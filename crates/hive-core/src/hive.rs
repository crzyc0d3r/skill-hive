//! The [`Hive`] facade: one object that owns the database, paths, secrets and target table, and
//! exposes every operation the CLI and the desktop app need.
//!
//! Operations that belong to a feature module (sources, providers, evaluations, recommendations,
//! loadouts) are implemented as `impl Hive` blocks inside those modules.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use tokio::sync::broadcast;

use crate::conflicts::{self, AnalysisOptions, AssetProfile};
use crate::db::Database;
use crate::deploy::Deployer;
use crate::discovery::{self, ScanRoot};
use crate::error::{HiveError, Result};
use crate::library::Library;
use crate::model::{
    Dashboard, DeployRequest, DeployResult, Deployment, DoctorReport, HiveEvent, ImportRequest,
    ImportResult, LibraryItem, LibraryItemDetail, LibraryQuery, LibraryStats, ProjectRef,
    RunStatus, ScanReport, Scope, Settings, SettingsPatch, SourceRef, TargetSpec, TargetStatus,
};
use crate::paths::HivePaths;
use crate::secrets::SecretStore;
use crate::settings;
use crate::targets::TargetResolver;

pub struct Hive {
    pub paths: HivePaths,
    pub db: Database,
    pub secrets: SecretStore,
    pub http: reqwest::Client,
    pub(crate) runs: crate::evals::RunManager,
    resolver: RwLock<TargetResolver>,
    events: broadcast::Sender<HiveEvent>,
}

impl std::fmt::Debug for Hive {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Hive")
            .field("home", &self.paths.home)
            .finish()
    }
}

impl Hive {
    /// Open (or create) the Hive at `paths`.
    pub fn open(paths: HivePaths) -> Result<Arc<Self>> {
        paths.ensure()?;
        let db = Database::open(&paths.db_path())?;
        let secrets = SecretStore::new(&paths);
        Self::assemble(paths, db, secrets)
    }

    /// Open the default location (`HIVE_HOME` or `~/.skill-hive`).
    pub fn open_default() -> Result<Arc<Self>> {
        Self::open(HivePaths::resolve())
    }

    /// A throwaway Hive in a temporary directory with an in-memory database (tests, demos).
    pub fn open_temp(root: &Path) -> Result<Arc<Self>> {
        let paths = HivePaths::new(root.join("hive"));
        paths.ensure()?;
        let db = Database::open_memory()?;
        let secrets = SecretStore::file_only(&paths);
        Self::assemble(paths, db, secrets)
    }

    fn assemble(paths: HivePaths, db: Database, secrets: SecretStore) -> Result<Arc<Self>> {
        // `HIVE_TARGET_HOME` overrides where agent directories are looked up (tests, sandboxes);
        // Windows ignores `HOME`/`USERPROFILE` overrides in `dirs::home_dir`, so this is the one
        // portable switch.
        let home = std::env::var_os("HIVE_TARGET_HOME")
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .or_else(dirs::home_dir)
            .unwrap_or_else(|| paths.home.clone());
        let resolver = Self::build_resolver(&db, home)?;
        let (events, _) = broadcast::channel(1024);
        let http = reqwest::Client::builder()
            .user_agent(format!("skill-hive/{}", crate::version()))
            .timeout(std::time::Duration::from_secs(60))
            .build()?;
        let hive = Self {
            paths,
            db,
            secrets,
            http,
            runs: crate::evals::RunManager::new(),
            resolver: RwLock::new(resolver),
            events,
        };
        hive.mark_stale_runs()?;
        Ok(Arc::new(hive))
    }

    /// Use a different "home" for target resolution (tests point this at a temp dir).
    pub fn set_target_home(&self, home: PathBuf) -> Result<()> {
        let resolver = Self::build_resolver(&self.db, home)?;
        *self.resolver.write().unwrap_or_else(|p| p.into_inner()) = resolver;
        Ok(())
    }

    fn build_resolver(db: &Database, home: PathBuf) -> Result<TargetResolver> {
        let custom = db.list_custom_targets()?;
        let overrides: BTreeMap<String, PathBuf> = db
            .target_overrides()?
            .into_iter()
            .map(|(k, v)| (k, crate::paths::expand_tilde(&v)))
            .collect();
        Ok(TargetResolver::new(home, custom, overrides))
    }

    fn mark_stale_runs(&self) -> Result<()> {
        for mut run in self.db.stale_runs()? {
            run.status = RunStatus::Failed;
            run.error =
                Some("interrupted: Skill Hive was closed while the run was in progress".into());
            run.finished_at = Some(crate::util::now_ms());
            self.db.update_run(&run)?;
        }
        Ok(())
    }

    // ---- events ---------------------------------------------------------------------------

    pub fn subscribe(&self) -> broadcast::Receiver<HiveEvent> {
        self.events.subscribe()
    }

    pub fn emit(&self, event: HiveEvent) {
        let _ = self.events.send(event);
    }

    pub fn progress(
        &self,
        op: &str,
        message: impl Into<String>,
        done: Option<u32>,
        total: Option<u32>,
    ) {
        self.emit(HiveEvent::Progress {
            op: op.to_string(),
            message: message.into(),
            done,
            total,
        });
    }

    // ---- composition helpers --------------------------------------------------------------

    pub fn library(&self) -> Library<'_> {
        Library::new(&self.db, &self.paths)
    }

    pub fn resolver(&self) -> TargetResolver {
        self.resolver
            .read()
            .unwrap_or_else(|p| p.into_inner())
            .clone()
    }

    pub(crate) fn with_deployer<T>(&self, f: impl FnOnce(&Deployer<'_>) -> Result<T>) -> Result<T> {
        let resolver = self.resolver();
        let deployer = Deployer::new(&self.db, &self.paths, &resolver);
        f(&deployer)
    }

    pub fn reload_targets(&self) -> Result<()> {
        let home = self.resolver().home().to_path_buf();
        self.set_target_home(home)
    }

    // ---- settings -------------------------------------------------------------------------

    pub fn settings(&self) -> Result<Settings> {
        settings::load(
            &self.db,
            &self.paths,
            self.secrets.has(settings::GITHUB_TOKEN_SECRET),
        )
    }

    pub fn update_settings(&self, patch: &SettingsPatch) -> Result<Settings> {
        settings::apply(&self.db, patch)?;
        if let Some(token) = &patch.github_token {
            self.secrets
                .set(settings::GITHUB_TOKEN_SECRET, token.trim())?;
        }
        self.settings()
    }

    pub fn github_token(&self) -> Option<String> {
        self.secrets
            .get(settings::GITHUB_TOKEN_SECRET)
            .ok()
            .flatten()
            .or_else(|| std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty()))
    }

    pub(crate) fn analysis_options(&self, budget_chars: Option<u32>) -> Result<AnalysisOptions> {
        let s = self.settings()?;
        Ok(AnalysisOptions {
            overlap_threshold: s.overlap_threshold,
            budget_chars: Some(budget_chars.unwrap_or(s.default_description_budget_chars)),
            idf: Some(self.library_idf()?),
        })
    }

    /// Document frequencies over the whole library (name + description per item).
    pub(crate) fn library_idf(&self) -> Result<conflicts::Idf> {
        let items = self.db.all_items()?;
        let texts: Vec<String> = items
            .iter()
            .map(|i| format!("{} {}", i.name.replace('-', " "), i.description))
            .collect();
        Ok(conflicts::Idf::from_texts(texts.iter().map(|s| s.as_str())))
    }

    // ---- targets --------------------------------------------------------------------------

    pub fn targets(&self) -> Vec<TargetSpec> {
        self.resolver().specs().to_vec()
    }

    pub fn target(&self, id: &str) -> Result<TargetSpec> {
        self.resolver()
            .get(id)
            .cloned()
            .ok_or_else(|| HiveError::not_found(format!("target {id}")))
    }

    pub fn add_custom_target(&self, mut spec: TargetSpec) -> Result<TargetSpec> {
        spec.id = crate::util::slugify(&spec.id);
        if spec.id.is_empty()
            || spec.name.trim().is_empty()
            || spec.user_skills_dir.trim().is_empty()
        {
            return Err(HiveError::invalid(
                "custom targets need an id, a name and a skills directory",
            ));
        }
        spec.custom = true;
        self.db.upsert_custom_target(&spec)?;
        self.reload_targets()?;
        Ok(spec)
    }

    pub fn remove_custom_target(&self, id: &str) -> Result<()> {
        self.db.remove_custom_target(id)?;
        self.reload_targets()
    }

    pub fn set_target_skills_dir(&self, id: &str, dir: Option<&str>) -> Result<()> {
        self.target(id)?;
        self.db
            .set_target_override(id, dir.map(str::trim).filter(|d| !d.is_empty()))?;
        self.reload_targets()
    }

    /// Live status of a target: what is installed, what it costs in context, and conflicts.
    pub fn target_status(
        &self,
        target_id: &str,
        scope: Scope,
        project_path: Option<&str>,
    ) -> Result<TargetStatus> {
        let resolver = self.resolver();
        let spec = resolver
            .get(target_id)
            .cloned()
            .ok_or_else(|| HiveError::not_found(format!("target {target_id}")))?;
        let project = project_path.map(Path::new);
        if scope == Scope::Project && project.is_none() {
            return Err(HiveError::invalid("project scope needs a project path"));
        }
        let deployer = Deployer::new(&self.db, &self.paths, &resolver);
        let assets = deployer.installed_assets(&spec, scope, project)?;
        let skills_path = resolver
            .skills_dir(&spec, scope, project)
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();
        let profiles = self.profiles_for_installed(&assets)?;
        let options = self.analysis_options(spec.description_budget_chars)?;
        let conflicts = conflicts::analyze(&profiles, &options);
        let budget = conflicts::budget(&profiles, options.budget_chars);
        Ok(TargetStatus {
            installed: resolver.is_installed(&spec),
            spec,
            scope,
            project_path: project_path.map(|p| p.to_string()),
            skills_path,
            assets,
            context_tokens: budget.description_tokens,
            context_chars: budget.description_chars,
            budget_used: budget.used_fraction,
            conflicts,
        })
    }

    fn profiles_for_installed(
        &self,
        assets: &[crate::model::InstalledAsset],
    ) -> Result<Vec<AssetProfile>> {
        let mut out = Vec::new();
        for asset in assets {
            if let Some(id) = &asset.library_item_id {
                if let Some(rec) = self.db.get_item_record(id)? {
                    let mut p = AssetProfile::from_item(&rec.item, &rec.body);
                    // Reflect the installed name/description (a stale copy may differ).
                    p.name = asset.name.clone();
                    p.description = asset.description.clone();
                    p.description_tokens = asset.description_tokens;
                    out.push(p);
                    continue;
                }
            }
            let body = crate::skill::read_asset(asset.kind, Path::new(&asset.path))
                .map(|d| d.body)
                .unwrap_or_default();
            out.push(AssetProfile {
                id: String::new(),
                kind: asset.kind,
                name: asset.name.clone(),
                description: asset.description.clone(),
                body_head: body.chars().take(4000).collect(),
                content_hash: asset.content_hash.clone(),
                allowed_tools: Vec::new(),
                description_tokens: asset.description_tokens,
                body_tokens: crate::util::estimate_tokens(&body),
            });
        }
        Ok(out)
    }

    /// User-scope status of every target that is installed (plus any with deployments).
    pub fn installed_target_statuses(&self) -> Result<Vec<TargetStatus>> {
        let resolver = self.resolver();
        let deployed: std::collections::HashSet<String> = self
            .db
            .all_deployments()?
            .into_iter()
            .map(|d| d.target_id)
            .collect();
        let mut out = Vec::new();
        for spec in resolver.specs() {
            if resolver.is_installed(spec) || deployed.contains(&spec.id) {
                out.push(self.target_status(&spec.id, Scope::User, None)?);
            }
        }
        Ok(out)
    }

    /// Profiles for arbitrary library items (used by loadout plans and recommendations).
    pub fn profiles_for_items(&self, ids: &[String]) -> Result<Vec<AssetProfile>> {
        let mut out = Vec::new();
        for id in ids {
            if let Some(rec) = self.db.get_item_record(id)? {
                out.push(AssetProfile::from_item(&rec.item, &rec.body));
            }
        }
        Ok(out)
    }

    /// Conflict report for a set of library items as if they were installed together.
    pub fn analyze_items(
        &self,
        ids: &[String],
        target_id: Option<&str>,
    ) -> Result<crate::model::ConflictReport> {
        let profiles = self.profiles_for_items(ids)?;
        let budget = target_id.and_then(|t| {
            self.resolver()
                .get(t)
                .and_then(|s| s.description_budget_chars)
        });
        let options = self.analysis_options(budget)?;
        Ok(conflicts::analyze(&profiles, &options))
    }

    // ---- projects -------------------------------------------------------------------------

    pub fn projects(&self) -> Result<Vec<ProjectRef>> {
        self.db.list_projects()
    }

    pub fn add_project(&self, path: &str) -> Result<ProjectRef> {
        let p = crate::paths::expand_tilde(path.trim());
        if !p.is_dir() {
            return Err(HiveError::invalid(format!(
                "{} is not a directory",
                p.display()
            )));
        }
        let canonical = dunce::canonicalize(&p).unwrap_or(p);
        let name = canonical
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| canonical.to_string_lossy().to_string());
        self.db.add_project(&canonical.to_string_lossy(), &name)
    }

    pub fn remove_project(&self, path: &str) -> Result<()> {
        self.db.remove_project(path)?;
        Ok(())
    }

    // ---- library --------------------------------------------------------------------------

    pub fn list_items(&self, query: &LibraryQuery) -> Result<Vec<LibraryItem>> {
        self.db.query_items(query)
    }

    pub fn get_item(&self, id: &str) -> Result<LibraryItem> {
        self.db
            .get_item(id)?
            .ok_or_else(|| HiveError::not_found(format!("item {id}")))
    }

    /// Resolve an item by id, or by `kind:name` / plain name (CLI convenience).
    pub fn resolve_item(&self, reference: &str) -> Result<LibraryItem> {
        if let Some(item) = self.db.get_item(reference)? {
            return Ok(item);
        }
        let (kind, name) = match reference.split_once(':') {
            Some((k, n)) if crate::model::AssetKind::parse(k).is_some() => {
                (crate::model::AssetKind::parse(k), n)
            }
            _ => (None, reference),
        };
        for k in [
            crate::model::AssetKind::Skill,
            crate::model::AssetKind::Agent,
            crate::model::AssetKind::Command,
        ] {
            if kind.is_some() && kind != Some(k) {
                continue;
            }
            if let Some(item) = self.db.find_item_by_name(k, name)? {
                return Ok(item);
            }
        }
        Err(HiveError::not_found(format!(
            "no library item matches '{reference}'"
        )))
    }

    pub fn item_detail(&self, id: &str) -> Result<LibraryItemDetail> {
        let rec = self
            .db
            .get_item_record(id)?
            .ok_or_else(|| HiveError::not_found(format!("item {id}")))?;
        let lib = self.library();
        let files = lib.files(&rec.item);
        let deployments =
            self.with_deployer(|d| d.deployments_with_state(self.db.deployments_for_item(id)?))?;
        let loadouts = self.db.loadouts_for_item(id)?;
        let history = self.db.history_for_item(id, 50)?;
        Ok(LibraryItemDetail {
            item: rec.item,
            body: rec.body,
            files,
            validation: rec.validation,
            deployments,
            loadouts,
            history,
        })
    }

    pub fn read_item_file(&self, id: &str, rel: &str) -> Result<String> {
        let item = self.get_item(id)?;
        self.library().read_file(&item, rel)
    }

    pub fn stats(&self) -> Result<LibraryStats> {
        self.db.library_stats()
    }

    pub fn import(&self, req: &ImportRequest) -> Result<ImportResult> {
        let result = self
            .library()
            .import_many(&req.paths, &req.source, &req.tags, req.overwrite);
        if !result.imported.is_empty() {
            self.emit(HiveEvent::LibraryChanged);
        }
        Ok(result)
    }

    pub fn import_path(
        &self,
        path: &Path,
        tags: &[String],
        overwrite: bool,
    ) -> Result<LibraryItem> {
        let source = SourceRef::local(path.to_string_lossy());
        let item = self.library().import_path(path, source, tags, overwrite)?;
        self.emit(HiveEvent::LibraryChanged);
        Ok(item)
    }

    pub fn create_skill(
        &self,
        name: &str,
        description: &str,
        body: &str,
        tags: &[String],
    ) -> Result<LibraryItem> {
        let item = self.library().create_skill(name, description, body, tags)?;
        self.emit(HiveEvent::LibraryChanged);
        Ok(item)
    }

    pub fn write_document(&self, id: &str, text: &str) -> Result<LibraryItem> {
        let item = self.library().write_document(id, text)?;
        self.emit(HiveEvent::LibraryChanged);
        Ok(item)
    }

    pub fn refresh_item(&self, id: &str) -> Result<LibraryItem> {
        let item = self.library().refresh(id)?;
        self.emit(HiveEvent::LibraryChanged);
        Ok(item)
    }

    /// Remove an item and every deployment of it.
    pub fn remove_item(&self, id: &str) -> Result<LibraryItem> {
        self.with_deployer(|d| d.undeploy_item(id))?;
        let item = self.library().remove(id)?;
        self.emit(HiveEvent::LibraryChanged);
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(item)
    }

    pub fn set_tags(&self, id: &str, tags: &[String]) -> Result<LibraryItem> {
        self.db.set_item_tags(id, tags)?;
        self.emit(HiveEvent::LibraryChanged);
        self.get_item(id)
    }

    pub fn set_favorite(&self, id: &str, favorite: bool) -> Result<LibraryItem> {
        self.db.set_item_favorite(id, favorite)?;
        self.get_item(id)
    }

    pub fn set_notes(&self, id: &str, notes: Option<&str>) -> Result<LibraryItem> {
        self.db
            .set_item_notes(id, notes.map(str::trim).filter(|n| !n.is_empty()))?;
        self.get_item(id)
    }

    // ---- discovery ------------------------------------------------------------------------

    /// Scan every known agent directory, registered project and extra directory.
    pub fn scan_local(&self, extra_roots: &[String]) -> Result<ScanReport> {
        let settings = self.settings()?;
        let resolver = self.resolver();
        let projects: Vec<PathBuf> = self
            .db
            .list_projects()?
            .into_iter()
            .map(|p| PathBuf::from(p.path))
            .collect();
        let mut extra: Vec<PathBuf> = settings
            .extra_scan_dirs
            .iter()
            .map(|d| crate::paths::expand_tilde(d))
            .collect();
        extra.extend(extra_roots.iter().map(|d| crate::paths::expand_tilde(d)));
        let roots = discovery::default_roots(&resolver, &projects, &extra);
        self.progress(
            "scan",
            format!("Scanning {} directories", roots.len()),
            None,
            Some(roots.len() as u32),
        );
        let lib = self.library();
        Ok(discovery::scan(&lib, &roots))
    }

    /// Scan only the given directories (a folder the user picked).
    pub fn scan_paths(&self, paths: &[String]) -> Result<ScanReport> {
        let roots: Vec<ScanRoot> = paths
            .iter()
            .map(|p| ScanRoot {
                path: crate::paths::expand_tilde(p),
                target_id: None,
                kind: None,
            })
            .collect();
        let lib = self.library();
        Ok(discovery::scan(&lib, &roots))
    }

    // ---- deployment -----------------------------------------------------------------------

    pub fn deploy(&self, req: &DeployRequest) -> Result<DeployResult> {
        let default_mode = self.settings()?.default_link_mode;
        let result = self.with_deployer(|d| d.deploy(req, default_mode))?;
        if !result.deployed.is_empty() {
            self.emit(HiveEvent::DeploymentsChanged);
        }
        Ok(result)
    }

    pub fn undeploy(&self, deployment_id: &str) -> Result<Deployment> {
        let dep = self.with_deployer(|d| d.undeploy(deployment_id))?;
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(dep)
    }

    pub fn undeploy_item(&self, item_id: &str) -> Result<Vec<Deployment>> {
        let deps = self.with_deployer(|d| d.undeploy_item(item_id))?;
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(deps)
    }

    pub fn resync(&self, deployment_id: &str) -> Result<Deployment> {
        let dep = self.with_deployer(|d| d.resync(deployment_id))?;
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(dep)
    }

    pub fn deployments(&self) -> Result<Vec<Deployment>> {
        self.with_deployer(|d| d.deployments_with_state(self.db.all_deployments()?))
    }

    /// Remove an unmanaged asset from a target directory (the user asked to clean it up).
    pub fn remove_installed_path(&self, path: &str) -> Result<()> {
        let p = Path::new(path);
        let resolver = self.resolver();
        let inside_target = resolver.specs().iter().any(|spec| {
            [
                crate::model::AssetKind::Skill,
                crate::model::AssetKind::Agent,
                crate::model::AssetKind::Command,
            ]
            .into_iter()
            .filter_map(|k| resolver.asset_dir(spec, k, Scope::User, None))
            .any(|dir| p.starts_with(&dir))
        }) || self
            .db
            .list_projects()?
            .iter()
            .any(|proj| p.starts_with(&proj.path));
        if !inside_target {
            return Err(HiveError::invalid(
                "refusing to delete a path outside known agent directories",
            ));
        }
        crate::util::remove_path(p)?;
        self.db.delete_deployment_by_path(path)?;
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(())
    }

    // ---- dashboard & doctor ---------------------------------------------------------------

    pub fn dashboard(&self) -> Result<Dashboard> {
        Ok(Dashboard {
            stats: self.db.library_stats()?,
            targets: self.installed_target_statuses()?,
            recent_runs: self.db.list_runs(None, 8)?,
            recent_items: self.db.query_items(&LibraryQuery {
                sort: crate::model::LibrarySort::Updated,
                limit: Some(8),
                ..Default::default()
            })?,
            loadouts: self.db.list_loadouts()?,
            providers: self.db.list_providers()?,
            detected_clis: crate::providers::detect_clis(),
        })
    }

    pub fn doctor(&self, reindex: bool) -> Result<DoctorReport> {
        let mut notes = Vec::new();
        if reindex {
            let (indexed, removed) = self.library().reindex()?;
            notes.push(format!("reindex: {indexed} indexed, {removed} removed"));
        }
        let (orphan_dirs, missing_dirs) = self.library().orphans()?;
        let deployments = self.deployments()?;
        let broken: Vec<Deployment> = deployments
            .into_iter()
            .filter(|d| d.state == crate::model::InstallState::Broken)
            .collect();
        let git_available = which::which("git").is_ok();
        if !git_available {
            notes.push("git is not on PATH: online sources that need a clone will not work".into());
        }
        notes.push(format!("secrets backend: {}", self.secrets.backend_name()));
        Ok(DoctorReport {
            home: self.paths.home.to_string_lossy().to_string(),
            db_ok: self.db.with(|c| {
                Ok(c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))?)
            })? == "ok",
            library_items: self.db.library_stats()?.total,
            orphan_dirs,
            missing_dirs,
            broken_deployments: broken,
            git_available,
            detected_clis: crate::providers::detect_clis(),
            notes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;
    use crate::model::{AssetKind, LinkMode};

    fn temp_hive() -> (tempfile::TempDir, Arc<Hive>) {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        hive.set_target_home(home).unwrap();
        (tmp, hive)
    }

    #[test]
    fn end_to_end_import_deploy_status_remove() {
        let (tmp, hive) = temp_hive();
        let src = tmp.path().join("src");
        let a = write_skill(
            &src,
            "alpha",
            "Extracts tables from PDF files when the user shares a PDF",
            "a",
        );
        let b = write_skill(
            &src,
            "beta",
            "Extracts tables from PDF documents when the user shares a PDF",
            "b",
        );
        let mut rx = hive.subscribe();

        let item_a = hive.import_path(&a, &["docs".into()], false).unwrap();
        let item_b = hive.import_path(&b, &[], false).unwrap();
        assert!(matches!(rx.try_recv(), Ok(HiveEvent::LibraryChanged)));
        assert_eq!(hive.stats().unwrap().skills, 2);
        assert_eq!(hive.resolve_item("alpha").unwrap().id, item_a.id);
        assert_eq!(hive.resolve_item("skill:beta").unwrap().id, item_b.id);
        assert!(hive.resolve_item("nope").is_err());

        let result = hive
            .deploy(&DeployRequest {
                item_ids: vec![item_a.id.clone(), item_b.id.clone()],
                target_id: "claude-code".into(),
                scope: Scope::User,
                project_path: None,
                mode: Some(LinkMode::Link),
                force: false,
            })
            .unwrap();
        assert_eq!(result.deployed.len(), 2);

        let status = hive
            .target_status("claude-code", Scope::User, None)
            .unwrap();
        assert!(status.installed);
        assert_eq!(status.assets.len(), 2);
        assert!(status.context_tokens > 0);
        // alpha and beta describe the same thing -> overlap conflict surfaces on the target
        assert!(
            status
                .conflicts
                .conflicts
                .iter()
                .any(|c| c.kind == crate::model::ConflictKind::DescriptionOverlap),
            "{:?}",
            status.conflicts
        );

        let detail = hive.item_detail(&item_a.id).unwrap();
        assert_eq!(detail.deployments.len(), 1);
        assert_eq!(detail.files.len(), 1);
        assert!(detail.body.contains('a'));

        let dash = hive.dashboard().unwrap();
        assert_eq!(dash.stats.deployed, 2);
        assert_eq!(dash.targets.len(), 1);

        let report = hive
            .analyze_items(&[item_a.id.clone(), item_b.id.clone()], Some("claude-code"))
            .unwrap();
        assert!(!report.conflicts.is_empty());

        hive.remove_item(&item_a.id).unwrap();
        assert_eq!(
            hive.target_status("claude-code", Scope::User, None)
                .unwrap()
                .assets
                .len(),
            1
        );
        assert_eq!(hive.doctor(false).unwrap().library_items, 1);
    }

    #[test]
    fn scan_imports_and_settings() {
        let (tmp, hive) = temp_hive();
        let home = tmp.path().join("home");
        write_skill(
            &home.join(".claude/skills"),
            "local-one",
            "A local skill found by the scanner when testing",
            "x",
        );
        let report = hive.scan_local(&[]).unwrap();
        assert_eq!(report.candidates.len(), 1);
        let cand = &report.candidates[0];
        let imported = hive
            .import(&ImportRequest {
                paths: vec![cand.path.clone()],
                source: cand.source.clone(),
                tags: vec![],
                overwrite: false,
            })
            .unwrap();
        assert_eq!(imported.imported.len(), 1);
        assert_eq!(
            imported.imported[0].source.kind,
            crate::model::SourceKind::AgentDir
        );
        // After import the installed copy is "in library" and the scan says so.
        let report = hive.scan_local(&[]).unwrap();
        assert_eq!(
            report.candidates[0].status,
            crate::model::CandidateStatus::InLibrary
        );

        let s = hive
            .update_settings(&SettingsPatch {
                github_token: Some("ghp_x".into()),
                theme: Some("light".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(s.github_token_set);
        assert_eq!(hive.github_token().as_deref(), Some("ghp_x"));
        let s = hive
            .update_settings(&SettingsPatch {
                github_token: Some(String::new()),
                ..Default::default()
            })
            .unwrap();
        assert!(!s.github_token_set);
    }

    #[test]
    fn custom_targets_projects_and_authoring() {
        let (tmp, hive) = temp_hive();
        let spec = TargetSpec {
            id: "My Tool".into(),
            name: "My Tool".into(),
            category: crate::model::TargetCategory::Coding,
            user_skills_dir: ".mytool/skills".into(),
            project_skills_dir: None,
            user_agents_dir: None,
            project_agents_dir: None,
            user_commands_dir: None,
            project_commands_dir: None,
            detect_dir: ".mytool".into(),
            description_budget_chars: None,
            docs_url: None,
            custom: false,
        };
        let saved = hive.add_custom_target(spec).unwrap();
        assert_eq!(saved.id, "my-tool");
        assert!(hive.targets().iter().any(|t| t.id == "my-tool" && t.custom));
        hive.set_target_skills_dir("codex", Some("~/alt-codex"))
            .unwrap();
        let status = hive.target_status("codex", Scope::User, None).unwrap();
        assert!(status.installed);
        assert!(status.skills_path.ends_with("alt-codex"));
        hive.remove_custom_target("my-tool").unwrap();
        assert!(!hive.targets().iter().any(|t| t.id == "my-tool"));

        let proj = tmp.path().join("proj");
        std::fs::create_dir_all(&proj).unwrap();
        let p = hive.add_project(&proj.to_string_lossy()).unwrap();
        assert_eq!(p.name, "proj");
        assert!(hive.add_project("/definitely/not/here").is_err());

        let item = hive
            .create_skill(
                "notes",
                "Tidy meeting notes when asked for a summary",
                "# Notes",
                &[],
            )
            .unwrap();
        assert_eq!(item.kind, AssetKind::Skill);
        let item = hive.set_tags(&item.id, &["Meetings".into()]).unwrap();
        assert_eq!(item.tags, vec!["meetings"]);
        let item = hive.set_notes(&item.id, Some("  my note ")).unwrap();
        assert_eq!(item.notes.as_deref(), Some("my note"));
        assert!(
            hive.read_item_file(&item.id, "SKILL.md")
                .unwrap()
                .contains("# Notes")
        );
        hive.remove_project(&p.path).unwrap();
    }
}
