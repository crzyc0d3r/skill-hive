//! Loadouts: named sets of library items applied to an agent target as one unit.
//!
//! Applying a loadout is planned first (what gets added, removed, kept; which conflicts and
//! budget the resulting directory would have) and then executed. `Exclusive` makes the target
//! directory match the loadout (Hive-managed assets outside it are removed; unmanaged assets are
//! left alone and reported). `Additive` only adds.

use std::collections::HashMap;
use std::path::Path;

use crate::conflicts::{self, AssetProfile};
use crate::deploy::{Deployer, install_name};
use crate::error::{HiveError, Result};
use crate::hive::Hive;
use crate::model::{
    ApplyLoadoutRequest, ApplyLoadoutResult, ApplyStrategy, DeployFailure, HiveEvent, InstallState, LibraryItem, Loadout,
    LoadoutInput, LoadoutPlan, PlannedItem, Scope,
};

fn planned(item: &LibraryItem, path: Option<String>) -> PlannedItem {
    PlannedItem {
        item_id: Some(item.id.clone()),
        name: item.name.clone(),
        kind: item.kind,
        description_tokens: item.description_tokens,
        path,
    }
}

/// Everything the planner derives; kept so `apply` does not recompute it.
struct PlanInternals {
    plan: LoadoutPlan,
    /// (item, destination path) to deploy.
    to_deploy: Vec<(LibraryItem, String)>,
    /// Deployment ids to remove.
    to_remove: Vec<String>,
    /// Deployment ids that are stale copies and should be re-synced.
    to_resync: Vec<String>,
}

impl Hive {
    pub fn loadouts(&self) -> Result<Vec<Loadout>> {
        self.db.list_loadouts()
    }

    pub fn loadout(&self, id: &str) -> Result<Loadout> {
        self.db
            .get_loadout(id)?
            .ok_or_else(|| HiveError::not_found(format!("loadout {id}")))
    }

    /// Resolve a loadout by id or (case-insensitive) name.
    pub fn resolve_loadout(&self, reference: &str) -> Result<Loadout> {
        if let Some(l) = self.db.get_loadout(reference)? {
            return Ok(l);
        }
        self.db
            .list_loadouts()?
            .into_iter()
            .find(|l| l.name.eq_ignore_ascii_case(reference))
            .ok_or_else(|| HiveError::not_found(format!("no loadout matches '{reference}'")))
    }

    pub fn create_loadout(&self, input: &LoadoutInput) -> Result<Loadout> {
        let input = self.validated_loadout_input(input)?;
        let loadout = self.db.create_loadout(&input)?;
        self.db.audit("loadout.create", &serde_json::json!({"id": loadout.id, "name": loadout.name}))?;
        Ok(loadout)
    }

    pub fn update_loadout(&self, id: &str, input: &LoadoutInput) -> Result<Loadout> {
        let input = self.validated_loadout_input(input)?;
        self.db.update_loadout(id, &input)
    }

    pub fn delete_loadout(&self, id: &str) -> Result<()> {
        if !self.db.delete_loadout(id)? {
            return Err(HiveError::not_found(format!("loadout {id}")));
        }
        Ok(())
    }

    /// Add items to a loadout (deduplicated, order preserved).
    pub fn add_to_loadout(&self, id: &str, item_ids: &[String]) -> Result<Loadout> {
        let mut l = self.loadout(id)?;
        for item_id in item_ids {
            self.get_item(item_id)?;
            if !l.item_ids.contains(item_id) {
                l.item_ids.push(item_id.clone());
            }
        }
        self.db.update_loadout(
            id,
            &LoadoutInput {
                name: l.name,
                description: l.description,
                item_ids: l.item_ids,
                color: l.color,
            },
        )
    }

    pub fn remove_from_loadout(&self, id: &str, item_ids: &[String]) -> Result<Loadout> {
        let l = self.loadout(id)?;
        let remaining: Vec<String> = l.item_ids.into_iter().filter(|i| !item_ids.contains(i)).collect();
        self.db.update_loadout(
            id,
            &LoadoutInput {
                name: l.name,
                description: l.description,
                item_ids: remaining,
                color: l.color,
            },
        )
    }

    fn validated_loadout_input(&self, input: &LoadoutInput) -> Result<LoadoutInput> {
        let mut ids = Vec::new();
        for id in &input.item_ids {
            let item = self.resolve_item(id)?;
            if !ids.contains(&item.id) {
                ids.push(item.id);
            }
        }
        Ok(LoadoutInput {
            name: input.name.trim().to_string(),
            description: input.description.trim().to_string(),
            item_ids: ids,
            color: input.color.clone(),
        })
    }

    fn plan_internal(&self, req: &ApplyLoadoutRequest) -> Result<PlanInternals> {
        let loadout = self.loadout(&req.loadout_id)?;
        let resolver = self.resolver();
        let spec = resolver
            .get(&req.target_id)
            .cloned()
            .ok_or_else(|| HiveError::not_found(format!("target {}", req.target_id)))?;
        let project = req.project_path.as_deref().map(Path::new);
        if req.scope == Scope::Project && project.is_none() {
            return Err(HiveError::invalid("project scope needs a project path"));
        }
        let deployer = Deployer::new(&self.db, &self.paths, &resolver);
        let installed = deployer.installed_assets(&spec, req.scope, project)?;
        let installed_by_path: HashMap<String, &crate::model::InstalledAsset> =
            installed.iter().map(|a| (a.path.clone(), a)).collect();

        let items = self.db.items_by_ids(&loadout.item_ids)?;
        let mut adds = Vec::new();
        let mut keeps = Vec::new();
        let mut removes = Vec::new();
        let mut unmanaged = Vec::new();
        let mut to_deploy = Vec::new();
        let mut to_remove = Vec::new();
        let mut to_resync = Vec::new();
        let mut wanted_paths: Vec<String> = Vec::new();
        // Profiles of what the directory will contain after apply.
        let mut result_profiles: Vec<AssetProfile> = Vec::new();

        for item in &items {
            let Ok(dir) = deployer.destination_dir(&spec, item.kind, req.scope, project) else {
                // Target cannot hold this kind (e.g. agents for Codex): report as a failure later.
                adds.push(planned(item, None));
                to_deploy.push((item.clone(), String::new()));
                continue;
            };
            let dest = dir.join(install_name(item)).to_string_lossy().to_string();
            wanted_paths.push(dest.clone());
            if let Some(rec) = self.db.get_item_record(&item.id)? {
                result_profiles.push(AssetProfile::from_item(&rec.item, &rec.body));
            }
            match installed_by_path.get(&dest) {
                Some(asset) if asset.library_item_id.as_deref() == Some(item.id.as_str()) => {
                    keeps.push(planned(item, Some(dest.clone())));
                    if asset.state == InstallState::Stale {
                        if let Some(dep_id) = &asset.deployment_id {
                            to_resync.push(dep_id.clone());
                        }
                    }
                }
                Some(asset) if asset.deployment_id.is_some() || asset.state == InstallState::Linked => {
                    // Managed, but a different item with the same name: replace it.
                    adds.push(planned(item, Some(dest.clone())));
                    to_deploy.push((item.clone(), dest));
                }
                Some(asset) => {
                    // Unmanaged asset in the way: do not touch it, report it.
                    unmanaged.push(PlannedItem {
                        item_id: None,
                        name: asset.name.clone(),
                        kind: asset.kind,
                        description_tokens: asset.description_tokens,
                        path: Some(asset.path.clone()),
                    });
                    adds.push(planned(item, Some(dest.clone())));
                    to_deploy.push((item.clone(), dest));
                }
                None => {
                    adds.push(planned(item, Some(dest.clone())));
                    to_deploy.push((item.clone(), dest));
                }
            }
        }

        for asset in &installed {
            if wanted_paths.contains(&asset.path) {
                continue;
            }
            let managed = asset.deployment_id.is_some();
            if managed && req.strategy == ApplyStrategy::Exclusive {
                removes.push(PlannedItem {
                    item_id: asset.library_item_id.clone(),
                    name: asset.name.clone(),
                    kind: asset.kind,
                    description_tokens: asset.description_tokens,
                    path: Some(asset.path.clone()),
                });
                if let Some(id) = &asset.deployment_id {
                    to_remove.push(id.clone());
                }
                continue;
            }
            if !managed {
                unmanaged.push(PlannedItem {
                    item_id: asset.library_item_id.clone(),
                    name: asset.name.clone(),
                    kind: asset.kind,
                    description_tokens: asset.description_tokens,
                    path: Some(asset.path.clone()),
                });
            }
            // Stays in the directory either way: part of the resulting set.
            let body = crate::skill::read_asset(asset.kind, Path::new(&asset.path))
                .map(|d| d.body)
                .unwrap_or_default();
            result_profiles.push(AssetProfile {
                id: asset.library_item_id.clone().unwrap_or_default(),
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

        let options = self.analysis_options(spec.description_budget_chars)?;
        let conflicts = conflicts::analyze(&result_profiles, &options);
        let budget = conflicts::budget(&result_profiles, options.budget_chars);
        Ok(PlanInternals {
            plan: LoadoutPlan {
                adds,
                removes,
                keeps,
                unmanaged,
                conflicts,
                budget,
            },
            to_deploy,
            to_remove,
            to_resync,
        })
    }

    /// Dry run: what applying the loadout would change.
    pub fn plan_loadout(&self, req: &ApplyLoadoutRequest) -> Result<LoadoutPlan> {
        Ok(self.plan_internal(req)?.plan)
    }

    /// Apply the loadout to the target.
    pub fn apply_loadout(&self, req: &ApplyLoadoutRequest) -> Result<ApplyLoadoutResult> {
        let internals = self.plan_internal(req)?;
        let settings = self.settings()?;
        let mode = req.mode.unwrap_or(settings.default_link_mode);
        let resolver = self.resolver();
        let spec = resolver
            .get(&req.target_id)
            .cloned()
            .ok_or_else(|| HiveError::not_found(format!("target {}", req.target_id)))?;
        let project = req.project_path.as_deref().map(Path::new);
        let deployer = Deployer::new(&self.db, &self.paths, &resolver);

        let mut removed = Vec::new();
        let mut deployed = Vec::new();
        let mut failed = Vec::new();

        for dep_id in &internals.to_remove {
            match deployer.undeploy(dep_id) {
                Ok(dep) => removed.push(dep.path),
                Err(err) => failed.push(DeployFailure {
                    item_id: dep_id.clone(),
                    reason: format!("remove failed: {err}"),
                }),
            }
        }
        for dep_id in &internals.to_resync {
            if let Err(err) = deployer.resync(dep_id) {
                failed.push(DeployFailure {
                    item_id: dep_id.clone(),
                    reason: format!("resync failed: {err}"),
                });
            }
        }
        for (item, _dest) in &internals.to_deploy {
            match deployer.deploy_one(&spec, &item.id, req.scope, project, mode, false) {
                Ok(dep) => deployed.push(dep),
                Err(err) => failed.push(DeployFailure {
                    item_id: item.id.clone(),
                    reason: err.to_string(),
                }),
            }
        }
        self.db.audit(
            "loadout.apply",
            &serde_json::json!({"loadout": req.loadout_id, "target": req.target_id, "scope": req.scope.as_str(), "deployed": deployed.len(), "removed": removed.len(), "failed": failed.len()}),
        )?;
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(ApplyLoadoutResult {
            plan: internals.plan,
            deployed,
            removed,
            failed,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;
    use crate::model::{DeployRequest, LinkMode};
    use std::sync::Arc;

    fn setup() -> (tempfile::TempDir, Arc<Hive>, Vec<LibraryItem>) {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        hive.set_target_home(home).unwrap();
        let src = tmp.path().join("src");
        let items = [
            ("pdf-tools", "Extract text and tables from PDF files when the user shares a PDF"),
            ("git-commit", "Write conventional commit messages when committing changes"),
            ("sql-tune", "Analyse slow SQL queries and propose indexes when a query is slow"),
        ]
        .iter()
        .map(|(n, d)| hive.import_path(&write_skill(&src, n, d, "body"), &[], false).unwrap())
        .collect();
        (tmp, hive, items)
    }

    #[test]
    fn crud_and_resolution() {
        let (_tmp, hive, items) = setup();
        let l = hive
            .create_loadout(&LoadoutInput {
                name: "Data work".into(),
                description: "d".into(),
                item_ids: vec![items[0].id.clone(), "git-commit".into(), items[0].id.clone()],
                color: None,
            })
            .unwrap();
        assert_eq!(l.item_ids.len(), 2);
        assert_eq!(hive.resolve_loadout("data WORK").unwrap().id, l.id);
        let l = hive.add_to_loadout(&l.id, &[items[2].id.clone()]).unwrap();
        assert_eq!(l.item_ids.len(), 3);
        let l = hive.remove_from_loadout(&l.id, &[items[1].id.clone()]).unwrap();
        assert_eq!(l.item_ids.len(), 2);
        assert!(hive
            .create_loadout(&LoadoutInput {
                name: "x".into(),
                description: String::new(),
                item_ids: vec!["missing".into()],
                color: None
            })
            .is_err());
        hive.delete_loadout(&l.id).unwrap();
        assert!(hive.loadout(&l.id).is_err());
    }

    #[test]
    fn plan_and_apply_exclusive() {
        let (tmp, hive, items) = setup();
        // git-commit is already deployed; pdf-tools + sql-tune are in the loadout.
        hive.deploy(&DeployRequest {
            item_ids: vec![items[1].id.clone()],
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            mode: Some(LinkMode::Link),
            force: false,
        })
        .unwrap();
        // An unmanaged skill sits in the directory too.
        write_skill(&tmp.path().join("home/.claude/skills"), "stranger", "An unmanaged skill nobody asked for", "x");

        let l = hive
            .create_loadout(&LoadoutInput {
                name: "Data".into(),
                description: String::new(),
                item_ids: vec![items[0].id.clone(), items[2].id.clone()],
                color: None,
            })
            .unwrap();
        let req = ApplyLoadoutRequest {
            loadout_id: l.id.clone(),
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            strategy: ApplyStrategy::Exclusive,
            mode: Some(LinkMode::Link),
        };
        let plan = hive.plan_loadout(&req).unwrap();
        assert_eq!(plan.adds.len(), 2);
        assert_eq!(plan.removes.len(), 1);
        assert_eq!(plan.removes[0].name, "git-commit");
        assert_eq!(plan.unmanaged.len(), 1);
        assert_eq!(plan.unmanaged[0].name, "stranger");
        assert!(plan.budget.description_tokens > 0);
        // Nothing on disk changed yet.
        assert!(tmp.path().join("home/.claude/skills/git-commit").exists());

        let result = hive.apply_loadout(&req).unwrap();
        assert!(result.failed.is_empty(), "{:?}", result.failed);
        assert_eq!(result.deployed.len(), 2);
        assert_eq!(result.removed.len(), 1);
        let skills = tmp.path().join("home/.claude/skills");
        assert!(!skills.join("git-commit").exists());
        assert!(skills.join("pdf-tools").exists());
        assert!(skills.join("sql-tune").exists());
        assert!(skills.join("stranger").exists());

        // Second apply is a no-op: everything is kept.
        let plan = hive.plan_loadout(&req).unwrap();
        assert_eq!(plan.keeps.len(), 2);
        assert!(plan.adds.is_empty());
        assert!(plan.removes.is_empty());

        // Additive keeps managed extras.
        hive.deploy(&DeployRequest {
            item_ids: vec![items[1].id.clone()],
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            mode: None,
            force: false,
        })
        .unwrap();
        let additive = ApplyLoadoutRequest {
            strategy: ApplyStrategy::Additive,
            ..req.clone()
        };
        let plan = hive.plan_loadout(&additive).unwrap();
        assert!(plan.removes.is_empty());
        let result = hive.apply_loadout(&additive).unwrap();
        assert!(result.removed.is_empty());
        assert!(skills.join("git-commit").exists());
    }

    #[test]
    fn plan_reports_unmanaged_collision_and_failure() {
        let (tmp, hive, items) = setup();
        write_skill(&tmp.path().join("home/.claude/skills"), "pdf-tools", "Someone else's pdf tools for pdfs", "x");
        let l = hive
            .create_loadout(&LoadoutInput {
                name: "L".into(),
                description: String::new(),
                item_ids: vec![items[0].id.clone()],
                color: None,
            })
            .unwrap();
        let req = ApplyLoadoutRequest {
            loadout_id: l.id,
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            strategy: ApplyStrategy::Exclusive,
            mode: None,
        };
        let plan = hive.plan_loadout(&req).unwrap();
        assert_eq!(plan.unmanaged.len(), 1);
        let result = hive.apply_loadout(&req).unwrap();
        assert_eq!(result.failed.len(), 1);
        assert!(result.failed[0].reason.contains("not managed"));
        assert!(hive
            .plan_loadout(&ApplyLoadoutRequest {
                scope: Scope::Project,
                ..req.clone()
            })
            .is_err());
    }
}
