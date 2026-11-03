//! Deploying library items into agent directories, and reading back what is installed.
//!
//! Link mode creates a symlink (a junction for directories on Windows) so the agent always sees
//! the library copy. Copy mode writes an independent copy and records the hash so drift can be
//! detected later. Both are reversible with `undeploy`.

use std::path::{Path, PathBuf};

use crate::db::Database;
use crate::db::deployments::NewDeployment;
use crate::error::{HiveError, IoContext, Result};
use crate::hash;
use crate::library::copy_asset;
use crate::model::{
    AssetKind, DeployFailure, DeployRequest, DeployResult, Deployment, InstallState,
    InstalledAsset, LibraryItem, LinkMode, Scope, TargetSpec,
};
use crate::paths::HivePaths;
use crate::skill;
use crate::targets::TargetResolver;
use crate::util;

pub struct Deployer<'a> {
    pub db: &'a Database,
    pub paths: &'a HivePaths,
    pub resolver: &'a TargetResolver,
}

/// Name an item gets inside a target directory: skills use their `name`, markdown assets `name.md`.
pub fn install_name(item: &LibraryItem) -> String {
    match item.kind {
        AssetKind::Skill => item.name.clone(),
        _ => format!("{}.md", item.name),
    }
}

fn create_link(src: &Path, dest: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(src, dest).at(dest)
    }
    #[cfg(windows)]
    {
        if src.is_dir() {
            junction::create(src, dest).at(dest)
        } else {
            std::os::windows::fs::symlink_file(src, dest).at(dest)
        }
    }
}

/// Where a link points, resolved against the link's directory.
pub fn link_target(path: &Path) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        if let Ok(true) = junction::exists(path) {
            if let Ok(target) = junction::get_target(path) {
                return Some(target);
            }
        }
    }
    let target = std::fs::read_link(path).ok()?;
    Some(if target.is_absolute() {
        target
    } else {
        path.parent().map(|p| p.join(&target)).unwrap_or(target)
    })
}

fn is_link(path: &Path) -> bool {
    #[cfg(windows)]
    {
        if junction::exists(path).unwrap_or(false) {
            return true;
        }
    }
    std::fs::symlink_metadata(path)
        .map(|m| m.file_type().is_symlink())
        .unwrap_or(false)
}

impl<'a> Deployer<'a> {
    pub fn new(db: &'a Database, paths: &'a HivePaths, resolver: &'a TargetResolver) -> Self {
        Self {
            db,
            paths,
            resolver,
        }
    }

    fn target(&self, id: &str) -> Result<&TargetSpec> {
        self.resolver
            .get(id)
            .ok_or_else(|| HiveError::not_found(format!("target {id}")))
    }

    /// Destination directory for an item kind at a scope; errors when the target lacks one.
    pub fn destination_dir(
        &self,
        spec: &TargetSpec,
        kind: AssetKind,
        scope: Scope,
        project: Option<&Path>,
    ) -> Result<PathBuf> {
        if scope == Scope::Project && project.is_none() {
            return Err(HiveError::invalid("project scope needs a project path"));
        }
        self.resolver
            .asset_dir(spec, kind, scope, project)
            .ok_or_else(|| {
                HiveError::invalid(format!(
                    "{} has no {} directory at {} scope",
                    spec.name,
                    kind.label().to_lowercase(),
                    scope.as_str()
                ))
            })
    }

    pub fn deploy(&self, req: &DeployRequest, default_mode: LinkMode) -> Result<DeployResult> {
        let spec = self.target(&req.target_id)?.clone();
        let project = req.project_path.as_deref().map(Path::new);
        let mode = req.mode.unwrap_or(default_mode);
        let mut deployed = Vec::new();
        let mut failed = Vec::new();
        for item_id in &req.item_ids {
            match self.deploy_one(&spec, item_id, req.scope, project, mode, req.force) {
                Ok(d) => deployed.push(d),
                Err(e) => failed.push(DeployFailure {
                    item_id: item_id.clone(),
                    reason: e.to_string(),
                }),
            }
        }
        Ok(DeployResult { deployed, failed })
    }

    pub fn deploy_one(
        &self,
        spec: &TargetSpec,
        item_id: &str,
        scope: Scope,
        project: Option<&Path>,
        mode: LinkMode,
        force: bool,
    ) -> Result<Deployment> {
        let item = self
            .db
            .get_item(item_id)?
            .ok_or_else(|| HiveError::not_found(format!("item {item_id}")))?;
        let src = PathBuf::from(&item.path);
        if !src.exists() {
            return Err(HiveError::not_found(format!(
                "library files for {} are missing",
                item.name
            )));
        }
        let dir = self.destination_dir(spec, item.kind, scope, project)?;
        std::fs::create_dir_all(&dir).at(&dir)?;
        let dest = dir.join(install_name(&item));
        let dest_str = dest.to_string_lossy().to_string();

        if dest.exists() || is_link(&dest) {
            let existing = self.db.deployment_by_path(&dest_str)?;
            let managed = existing.is_some()
                || link_target(&dest)
                    .map(|t| self.paths.is_in_library(&t))
                    .unwrap_or(false);
            if !managed && !force {
                return Err(HiveError::AlreadyExists(format!(
                    "{} already exists in {} and is not managed by Skill Hive (use force to replace)",
                    install_name(&item),
                    dir.display()
                )));
            }
            util::remove_path(&dest)?;
        }

        match mode {
            LinkMode::Link => {
                if let Err(err) = create_link(&src, &dest) {
                    // Symlinks can fail on Windows without developer mode; fall back to a copy.
                    tracing::warn!("link failed ({err}); falling back to copy");
                    copy_asset(item.kind, &src, &dest)?;
                    return self.record(&item, spec, scope, project, &dest_str, LinkMode::Copy);
                }
            }
            LinkMode::Copy => copy_asset(item.kind, &src, &dest)?,
        }
        self.record(&item, spec, scope, project, &dest_str, mode)
    }

    fn record(
        &self,
        item: &LibraryItem,
        spec: &TargetSpec,
        scope: Scope,
        project: Option<&Path>,
        dest: &str,
        mode: LinkMode,
    ) -> Result<Deployment> {
        let mut dep = self.db.upsert_deployment(&NewDeployment {
            item_id: item.id.clone(),
            target_id: spec.id.clone(),
            scope,
            project_path: project.map(|p| p.to_string_lossy().to_string()),
            path: dest.to_string(),
            mode,
            deployed_hash: item.content_hash.clone(),
        })?;
        dep.state = if mode == LinkMode::Link {
            InstallState::Linked
        } else {
            InstallState::InSync
        };
        self.db.audit(
            "deploy",
            &serde_json::json!({"item": item.name, "target": spec.id, "scope": scope.as_str(), "path": dest, "mode": mode.as_str()}),
        )?;
        Ok(dep)
    }

    /// Remove a deployment (the link or copy on disk and the record).
    pub fn undeploy(&self, deployment_id: &str) -> Result<Deployment> {
        let dep = self
            .db
            .deployment(deployment_id)?
            .ok_or_else(|| HiveError::not_found("deployment"))?;
        self.remove_installed(&dep.path)?;
        self.db.delete_deployment(deployment_id)?;
        self.db.audit(
            "undeploy",
            &serde_json::json!({"item": dep.item_name, "path": dep.path}),
        )?;
        Ok(dep)
    }

    /// Remove every deployment of an item (used before deleting it from the library).
    pub fn undeploy_item(&self, item_id: &str) -> Result<Vec<Deployment>> {
        let deps = self.db.deployments_for_item(item_id)?;
        for dep in &deps {
            self.remove_installed(&dep.path)?;
            self.db.delete_deployment(&dep.id)?;
        }
        Ok(deps)
    }

    /// Remove an installed path only when it is safe: a link, or a copy recorded by Hive.
    fn remove_installed(&self, path: &str) -> Result<()> {
        let p = Path::new(path);
        if !p.exists() && !is_link(p) {
            return Ok(());
        }
        util::remove_path(p)
    }

    /// Bring a copy-mode deployment back in sync with the library version.
    pub fn resync(&self, deployment_id: &str) -> Result<Deployment> {
        let dep = self
            .db
            .deployment(deployment_id)?
            .ok_or_else(|| HiveError::not_found("deployment"))?;
        let item = self
            .db
            .get_item(&dep.item_id)?
            .ok_or_else(|| HiveError::not_found("item"))?;
        let dest = Path::new(&dep.path);
        if dep.mode == LinkMode::Copy {
            util::remove_path(dest)?;
            copy_asset(item.kind, Path::new(&item.path), dest)?;
        }
        self.db
            .update_deployment_hash(&dep.id, &item.content_hash)?;
        let mut dep = self
            .db
            .deployment(deployment_id)?
            .ok_or_else(|| HiveError::not_found("deployment"))?;
        dep.state = self.deployment_state(&dep, Some(&item));
        Ok(dep)
    }

    /// Compute the live state of a deployment by looking at disk.
    pub fn deployment_state(&self, dep: &Deployment, item: Option<&LibraryItem>) -> InstallState {
        let path = Path::new(&dep.path);
        if is_link(path) {
            return match link_target(path) {
                Some(target) if target.exists() => InstallState::Linked,
                _ => InstallState::Broken,
            };
        }
        if !path.exists() {
            return InstallState::Broken;
        }
        let Ok((disk_hash, _)) = hash::hash_path(path) else {
            return InstallState::Broken;
        };
        let library_hash = item.map(|i| i.content_hash.as_str());
        if disk_hash != dep.deployed_hash {
            InstallState::Modified
        } else if library_hash
            .map(|h| h != dep.deployed_hash)
            .unwrap_or(false)
        {
            InstallState::Stale
        } else {
            InstallState::InSync
        }
    }

    /// Deployments with live state filled in.
    pub fn deployments_with_state(&self, deps: Vec<Deployment>) -> Result<Vec<Deployment>> {
        let mut out = Vec::with_capacity(deps.len());
        for mut dep in deps {
            let item = self.db.get_item(&dep.item_id)?;
            dep.state = self.deployment_state(&dep, item.as_ref());
            out.push(dep);
        }
        Ok(out)
    }

    /// Everything installed in a target directory, managed or not.
    pub fn installed_assets(
        &self,
        spec: &TargetSpec,
        scope: Scope,
        project: Option<&Path>,
    ) -> Result<Vec<InstalledAsset>> {
        let mut out = Vec::new();
        for kind in [AssetKind::Skill, AssetKind::Agent, AssetKind::Command] {
            let Some(dir) = self.resolver.asset_dir(spec, kind, scope, project) else {
                continue;
            };
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            let mut paths: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            paths.sort();
            for path in paths {
                let is_skill_entry = kind == AssetKind::Skill && (path.is_dir() || is_link(&path));
                let is_md_entry = kind != AssetKind::Skill
                    && path.extension().and_then(|e| e.to_str()) == Some("md");
                if !(is_skill_entry || is_md_entry) {
                    continue;
                }
                if kind == AssetKind::Skill && !skill::is_skill_dir(&path) && !is_link(&path) {
                    continue;
                }
                if let Some(asset) = self.describe_installed(kind, &path) {
                    out.push(asset);
                }
            }
        }
        Ok(out)
    }

    fn describe_installed(&self, kind: AssetKind, path: &Path) -> Option<InstalledAsset> {
        let path_str = path.to_string_lossy().to_string();
        let deployment = self.db.deployment_by_path(&path_str).ok().flatten();
        let linked_target = link_target(path);
        if is_link(path) && !linked_target.as_ref().map(|t| t.exists()).unwrap_or(false) {
            return Some(InstalledAsset {
                kind,
                name: path
                    .file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                description: String::new(),
                path: path_str,
                content_hash: String::new(),
                description_tokens: 0,
                library_item_id: deployment.as_ref().map(|d| d.item_id.clone()),
                deployment_id: deployment.as_ref().map(|d| d.id.clone()),
                state: InstallState::Broken,
            });
        }
        let doc = skill::read_asset(kind, path).ok();
        let (name, description) = match &doc {
            Some(d) => (
                if d.name.is_empty() {
                    path.file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default()
                } else {
                    d.name.clone()
                },
                d.description.clone(),
            ),
            None => (
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default(),
                String::new(),
            ),
        };
        let content_hash = hash::hash_path(path).map(|(h, _)| h).unwrap_or_default();
        let (library_item_id, deployment_id, state) = match &deployment {
            Some(dep) => {
                let item = self.db.get_item(&dep.item_id).ok().flatten();
                (
                    Some(dep.item_id.clone()),
                    Some(dep.id.clone()),
                    self.deployment_state(dep, item.as_ref()),
                )
            }
            None => {
                // A link into the library that Hive did not record (e.g. made by hand).
                if let Some(target) = &linked_target {
                    if self.paths.is_in_library(target) {
                        let item = self
                            .db
                            .find_item_by_path(&target.to_string_lossy())
                            .ok()
                            .flatten();
                        (item.map(|i| i.id), None, InstallState::Linked)
                    } else {
                        (None, None, InstallState::Unmanaged)
                    }
                } else {
                    let same = self
                        .db
                        .find_items_by_hash(&content_hash)
                        .ok()
                        .unwrap_or_default();
                    (
                        same.into_iter().find(|i| i.kind == kind).map(|i| i.id),
                        None,
                        InstallState::Unmanaged,
                    )
                }
            }
        };
        Some(InstalledAsset {
            kind,
            name,
            description: description.clone(),
            path: path_str,
            content_hash,
            description_tokens: util::estimate_tokens(&description),
            library_item_id,
            deployment_id,
            state,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::Library;
    use crate::library::test_support::write_skill;
    use crate::model::SourceRef;

    struct Fx {
        _tmp: tempfile::TempDir,
        paths: HivePaths,
        db: Database,
        home: PathBuf,
        src: PathBuf,
    }

    fn fx() -> Fx {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path().join("hive"));
        paths.ensure().unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        Fx {
            _tmp: tmp,
            paths,
            db: Database::open_memory().unwrap(),
            home,
            src,
        }
    }

    #[test]
    fn link_deploy_and_undeploy() {
        let f = fx();
        let lib = Library::new(&f.db, &f.paths);
        let resolver = TargetResolver::with_home(f.home.clone());
        let deployer = Deployer::new(&f.db, &f.paths, &resolver);
        let dir = write_skill(&f.src, "alpha", "Alpha skill for testing deployments", "a");
        let item = lib
            .import_path(&dir, SourceRef::local("x"), &[], false)
            .unwrap();

        let result = deployer
            .deploy(
                &DeployRequest {
                    item_ids: vec![item.id.clone()],
                    target_id: "claude-code".into(),
                    scope: Scope::User,
                    project_path: None,
                    mode: Some(LinkMode::Link),
                    force: false,
                },
                LinkMode::Link,
            )
            .unwrap();
        assert!(result.failed.is_empty(), "{:?}", result.failed);
        let dep = &result.deployed[0];
        let installed = f.home.join(".claude/skills/alpha");
        assert!(installed.join("SKILL.md").exists());
        assert!(is_link(&installed));
        assert_eq!(dep.state, InstallState::Linked);

        let spec = resolver.get("claude-code").unwrap();
        let assets = deployer.installed_assets(spec, Scope::User, None).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].state, InstallState::Linked);
        assert_eq!(assets[0].library_item_id.as_deref(), Some(item.id.as_str()));

        // Deploying again over our own link is fine.
        let again = deployer
            .deploy_one(spec, &item.id, Scope::User, None, LinkMode::Link, false)
            .unwrap();
        assert_eq!(again.path, dep.path);

        deployer.undeploy(&dep.id).unwrap();
        assert!(!installed.exists());
        assert!(
            deployer
                .installed_assets(spec, Scope::User, None)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn copy_deploy_tracks_drift() {
        let f = fx();
        let lib = Library::new(&f.db, &f.paths);
        let resolver = TargetResolver::with_home(f.home.clone());
        let deployer = Deployer::new(&f.db, &f.paths, &resolver);
        let dir = write_skill(
            &f.src,
            "beta",
            "Beta skill for testing copy deployments",
            "b",
        );
        let item = lib
            .import_path(&dir, SourceRef::local("x"), &[], false)
            .unwrap();
        let spec = resolver.get("codex").unwrap();
        let project = f.home.join("proj");
        std::fs::create_dir_all(&project).unwrap();
        let dep = deployer
            .deploy_one(
                spec,
                &item.id,
                Scope::Project,
                Some(&project),
                LinkMode::Copy,
                false,
            )
            .unwrap();
        assert_eq!(dep.state, InstallState::InSync);
        let installed = project.join(".codex/skills/beta");
        assert!(installed.join("SKILL.md").is_file());
        assert!(!is_link(&installed));

        // Library changes -> stale
        std::fs::write(
            Path::new(&item.path).join("SKILL.md"),
            "---\nname: beta\ndescription: Beta skill for testing copy deployments v2\n---\nb2",
        )
        .unwrap();
        lib.refresh(&item.id).unwrap();
        let deps = deployer
            .deployments_with_state(f.db.deployments_for_item(&item.id).unwrap())
            .unwrap();
        assert_eq!(deps[0].state, InstallState::Stale);
        let resynced = deployer.resync(&deps[0].id).unwrap();
        assert_eq!(resynced.state, InstallState::InSync);

        // Local edit -> modified
        std::fs::write(
            installed.join("SKILL.md"),
            "---\nname: beta\ndescription: edited locally by the user\n---\nb3",
        )
        .unwrap();
        let assets = deployer
            .installed_assets(spec, Scope::Project, Some(&project))
            .unwrap();
        assert_eq!(assets[0].state, InstallState::Modified);
    }

    #[test]
    fn refuses_to_clobber_unmanaged_without_force() {
        let f = fx();
        let lib = Library::new(&f.db, &f.paths);
        let resolver = TargetResolver::with_home(f.home.clone());
        let deployer = Deployer::new(&f.db, &f.paths, &resolver);
        let dir = write_skill(
            &f.src,
            "gamma",
            "Gamma skill for testing force deploys",
            "g",
        );
        let item = lib
            .import_path(&dir, SourceRef::local("x"), &[], false)
            .unwrap();
        // A foreign skill with the same name already installed.
        write_skill(
            &f.home.join(".claude/skills"),
            "gamma",
            "Someone else's gamma",
            "other",
        );
        let spec = resolver.get("claude-code").unwrap();
        let assets = deployer.installed_assets(spec, Scope::User, None).unwrap();
        assert_eq!(assets[0].state, InstallState::Unmanaged);
        let err = deployer
            .deploy_one(spec, &item.id, Scope::User, None, LinkMode::Link, false)
            .unwrap_err();
        assert!(matches!(err, HiveError::AlreadyExists(_)));
        let dep = deployer
            .deploy_one(spec, &item.id, Scope::User, None, LinkMode::Link, true)
            .unwrap();
        assert_eq!(dep.state, InstallState::Linked);
        // Removing the library item's deployments cleans up.
        deployer.undeploy_item(&item.id).unwrap();
        assert!(!f.home.join(".claude/skills/gamma").exists());
    }

    #[test]
    fn project_scope_requires_path_and_agents_need_support() {
        let f = fx();
        let lib = Library::new(&f.db, &f.paths);
        let resolver = TargetResolver::with_home(f.home.clone());
        let deployer = Deployer::new(&f.db, &f.paths, &resolver);
        let agent = f.src.join("rev.md");
        std::fs::write(
            &agent,
            "---\nname: rev\ndescription: Reviews things carefully\n---\nbody",
        )
        .unwrap();
        let item = lib
            .import_path(&agent, SourceRef::local("x"), &[], false)
            .unwrap();
        let claude = resolver.get("claude-code").unwrap();
        assert!(
            deployer
                .deploy_one(
                    claude,
                    &item.id,
                    Scope::Project,
                    None,
                    LinkMode::Link,
                    false
                )
                .is_err()
        );
        let dep = deployer
            .deploy_one(claude, &item.id, Scope::User, None, LinkMode::Copy, false)
            .unwrap();
        assert!(dep.path.ends_with("rev.md"));
        let codex = resolver.get("codex").unwrap();
        let err = deployer
            .deploy_one(codex, &item.id, Scope::User, None, LinkMode::Link, false)
            .unwrap_err();
        assert!(err.to_string().contains("no agent directory"), "{err}");
    }
}
