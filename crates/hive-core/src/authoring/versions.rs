//! Immutable draft versions under `<name>-workspace/versions/<id>/`.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::util::error::AppError;

const AUTOSAVE_KEEP: usize = 50;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionSource {
    Manual,
    Autosave,
    Generated,
    Optimized,
    Restored,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileHash {
    pub path: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DraftVersion {
    pub id: String,
    pub subject_id: String,
    pub parent_id: Option<String>,
    pub label: Option<String>,
    pub source: VersionSource,
    pub created_at: u64,
    pub content_hash: String,
    pub files: Vec<FileHash>,
    pub pinned_runs: Vec<String>,
    pub hidden: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Manifest {
    versions: Vec<DraftVersion>,
    current_best: Option<String>,
}

pub struct VersionStore {
    subject_id: String,
    subject_dir: PathBuf,
    workspace: PathBuf,
}

impl VersionStore {
    pub fn open(subject_dir: &Path) -> Self {
        let name = subject_dir
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("subject");
        let workspace = subject_dir
            .parent()
            .unwrap_or(Path::new("."))
            .join(format!("{name}-workspace"));
        Self {
            subject_id: name.to_string(),
            subject_dir: subject_dir.to_path_buf(),
            workspace,
        }
    }

    pub fn versions(&self) -> Result<Vec<DraftVersion>, AppError> {
        Ok(self.manifest()?.versions)
    }

    pub fn save_autosave(&self) -> Result<DraftVersion, AppError> {
        self.save(VersionSource::Autosave, None, None)
    }

    pub fn snapshot(&self, label: Option<&str>) -> Result<DraftVersion, AppError> {
        self.save(VersionSource::Manual, label, None)
    }

    pub fn save_with_source(
        &self,
        source: VersionSource,
        parent: Option<&str>,
    ) -> Result<DraftVersion, AppError> {
        self.save(source, None, parent)
    }

    fn save(
        &self,
        source: VersionSource,
        label: Option<&str>,
        parent_override: Option<&str>,
    ) -> Result<DraftVersion, AppError> {
        let mut manifest = self.manifest()?;
        let files = hash_files(&self.subject_dir)?;
        let content_hash = hash_manifest(&files);
        if source == VersionSource::Autosave {
            if let Some(latest) = manifest
                .versions
                .iter()
                .rev()
                .find(|version| !version.hidden)
            {
                if latest.content_hash == content_hash {
                    return Ok(latest.clone());
                }
            }
        }
        let parent_id = parent_override.map(str::to_string).or_else(|| {
            manifest
                .versions
                .iter()
                .rev()
                .find(|version| !version.hidden)
                .map(|version| version.id.clone())
        });
        let id = format!("{}-{}", unix_now(), manifest.versions.len() + 1);
        let version = DraftVersion {
            id: id.clone(),
            subject_id: self.subject_id.clone(),
            parent_id,
            label: label.map(str::to_string),
            source,
            created_at: unix_now(),
            content_hash,
            files: files.clone(),
            pinned_runs: Vec::new(),
            hidden: false,
        };
        let dest = self.workspace.join("versions").join(&id);
        copy_subject(&self.subject_dir, &dest)?;
        manifest.versions.push(version.clone());
        if manifest.current_best.is_none() {
            manifest.current_best = Some(id);
        }
        self.prune(&mut manifest);
        self.write_manifest(&manifest)?;
        Ok(version)
    }

    pub fn label(&self, id: &str, label: &str) -> Result<(), AppError> {
        let mut manifest = self.manifest()?;
        let version = manifest
            .versions
            .iter_mut()
            .find(|version| version.id == id)
            .ok_or_else(|| AppError::not_found(id))?;
        version.label = Some(label.to_string());
        self.write_manifest(&manifest)
    }

    pub fn pin(&self, id: &str, run_id: &str) -> Result<(), AppError> {
        let mut manifest = self.manifest()?;
        let version = manifest
            .versions
            .iter_mut()
            .find(|version| version.id == id)
            .ok_or_else(|| AppError::not_found(id))?;
        if !version
            .pinned_runs
            .iter()
            .any(|existing| existing == run_id)
        {
            version.pinned_runs.push(run_id.to_string());
        }
        self.write_manifest(&manifest)
    }

    pub fn hide(&self, id: &str) -> Result<(), AppError> {
        let mut manifest = self.manifest()?;
        let version = manifest
            .versions
            .iter()
            .find(|version| version.id == id)
            .ok_or_else(|| AppError::not_found(id))?;
        if !version.pinned_runs.is_empty() {
            return Err(AppError::invalid_input(format!(
                "version {id} is pinned by runs: {}",
                version.pinned_runs.join(",")
            )));
        }
        let version = manifest
            .versions
            .iter_mut()
            .find(|version| version.id == id)
            .unwrap();
        version.hidden = true;
        self.write_manifest(&manifest)
    }

    pub fn restore(&self, id: &str) -> Result<DraftVersion, AppError> {
        let manifest = self.manifest()?;
        if !manifest.versions.iter().any(|version| version.id == id) {
            return Err(AppError::not_found(id));
        }
        let current_hash = hash_manifest(&hash_files(&self.subject_dir)?);
        let latest_hash = manifest
            .versions
            .iter()
            .rev()
            .find(|version| !version.hidden)
            .map(|version| version.content_hash.clone());
        if latest_hash.as_deref() != Some(current_hash.as_str()) {
            self.save_autosave()?;
        }
        let snapshot = self.workspace.join("versions").join(id);
        if self.subject_dir.exists() {
            fs::remove_dir_all(&self.subject_dir)?;
        }
        copy_subject(&snapshot, &self.subject_dir)?;
        self.save(VersionSource::Restored, None, Some(id))
    }

    pub fn export_history(&self, out: &Path) -> Result<serde_json::Value, AppError> {
        let manifest = self.manifest()?;
        let visible: Vec<&DraftVersion> = manifest
            .versions
            .iter()
            .filter(|version| !version.hidden)
            .collect();
        let labels: Vec<(String, String)> = visible
            .iter()
            .enumerate()
            .map(|(index, version)| (version.id.clone(), format!("v{index}")))
            .collect();
        let label_of = |id: &str| {
            labels
                .iter()
                .find(|(version_id, _)| version_id == id)
                .map(|(_, label)| label.clone())
        };
        let current = manifest
            .current_best
            .clone()
            .or_else(|| visible.first().map(|version| version.id.clone()));
        let measured: Vec<Option<f64>> = visible
            .iter()
            .map(|version| {
                version
                    .pinned_runs
                    .iter()
                    .find_map(|run_id| read_run_pass_rate(&self.workspace, run_id))
            })
            .collect();
        let iterations: Vec<serde_json::Value> = visible
            .iter()
            .enumerate()
            .map(|(index, version)| {
                let label = format!("v{index}");
                let rate = measured[index];
                let grading = if index == 0 || rate.is_none() {
                    "baseline"
                } else {
                    match (rate, measured[..index].iter().rev().find_map(|rate| *rate)) {
                        (Some(rate), Some(previous)) if rate > previous => "won",
                        (Some(rate), Some(previous)) if rate < previous => "lost",
                        (Some(_), Some(_)) => "tie",
                        _ => "baseline",
                    }
                };
                serde_json::json!({
                    "version": label,
                    "parent": version.parent_id.as_deref().and_then(|id| label_of(id).or_else(|| if index > 0 { Some(format!("v{}", index - 1)) } else { None })),
                    "expectation_pass_rate": rate,
                    "grading_result": grading,
                    "is_current_best": current.as_deref() == Some(version.id.as_str()),
                })
            })
            .collect();
        if iterations
            .iter()
            .filter(|item| item["is_current_best"] == true)
            .count()
            != 1
            && !iterations.is_empty()
        {
            return Err(AppError::invalid_input(
                "history.json must have exactly one is_current_best",
            ));
        }
        let current_label = current.as_deref().and_then(label_of);
        let value = serde_json::json!({
            "started_at": "1970-01-01T00:00:00Z",
            "skill_name": self.subject_id,
            "current_best": current_label,
            "iterations": iterations,
        });
        validate_history(&value)?;
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(&value)
            .map_err(|err| AppError::invalid_input(err.to_string()))?;
        fs::write(out, bytes)?;
        Ok(value)
    }

    pub fn set_current_best(&self, id: &str) -> Result<(), AppError> {
        let mut manifest = self.manifest()?;
        if !manifest
            .versions
            .iter()
            .any(|version| version.id == id && !version.hidden)
        {
            return Err(AppError::not_found(id));
        }
        manifest.current_best = Some(id.to_string());
        self.write_manifest(&manifest)
    }

    fn prune(&self, manifest: &mut Manifest) {
        let autosaves: Vec<String> = manifest
            .versions
            .iter()
            .filter(|version| {
                version.source == VersionSource::Autosave
                    && version.label.is_none()
                    && version.pinned_runs.is_empty()
            })
            .map(|version| version.id.clone())
            .collect();
        if autosaves.len() <= AUTOSAVE_KEEP {
            return;
        }
        let drop: std::collections::HashSet<String> = autosaves[..autosaves.len() - AUTOSAVE_KEEP]
            .iter()
            .cloned()
            .collect();
        manifest
            .versions
            .retain(|version| !drop.contains(&version.id));
    }

    fn manifest(&self) -> Result<Manifest, AppError> {
        let path = self.workspace.join("versions").join("manifest.json");
        if !path.is_file() {
            return Ok(Manifest {
                versions: Vec::new(),
                current_best: None,
            });
        }
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text).unwrap_or(Manifest {
            versions: Vec::new(),
            current_best: None,
        }))
    }

    fn write_manifest(&self, manifest: &Manifest) -> Result<(), AppError> {
        let dir = self.workspace.join("versions");
        fs::create_dir_all(&dir)?;
        let bytes = serde_json::to_vec_pretty(manifest)
            .map_err(|err| AppError::invalid_input(err.to_string()))?;
        fs::write(dir.join("manifest.json"), bytes)?;
        Ok(())
    }
}

pub fn save_autosave(subject_dir: &Path) -> Result<DraftVersion, AppError> {
    VersionStore::open(subject_dir).save_autosave()
}

pub fn snapshot_version(subject_dir: &Path, label: Option<&str>) -> Result<DraftVersion, AppError> {
    VersionStore::open(subject_dir).snapshot(label)
}

pub fn label_version(subject_dir: &Path, id: &str, label: &str) -> Result<(), AppError> {
    VersionStore::open(subject_dir).label(id, label)
}

pub fn restore_version(subject_dir: &Path, id: &str) -> Result<DraftVersion, AppError> {
    VersionStore::open(subject_dir).restore(id)
}

pub fn hide_version(subject_dir: &Path, id: &str) -> Result<(), AppError> {
    VersionStore::open(subject_dir).hide(id)
}

pub fn export_history_json(subject_dir: &Path, out: &Path) -> Result<serde_json::Value, AppError> {
    VersionStore::open(subject_dir).export_history(out)
}

fn read_run_pass_rate(workspace: &Path, run_id: &str) -> Option<f64> {
    let path = workspace.join("runs").join(format!("{run_id}.json"));
    let value: serde_json::Value = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    value
        .get("expectation_pass_rate")
        .and_then(|rate| rate.as_f64())
        .or_else(|| value.get("pass_rate").and_then(|rate| rate.as_f64()))
        .or_else(|| {
            value
                .pointer("/summary/pass_rate")
                .and_then(|rate| rate.as_f64())
        })
}

pub fn set_current_best(subject_dir: &Path, id: &str) -> Result<(), AppError> {
    VersionStore::open(subject_dir).set_current_best(id)
}

fn validate_history(value: &serde_json::Value) -> Result<(), AppError> {
    let iterations = value
        .get("iterations")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::invalid_input("iterations"))?;
    let mut best = 0;
    for item in iterations {
        let grading = item
            .get("grading_result")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if !matches!(grading, "baseline" | "won" | "lost" | "tie") {
            return Err(AppError::invalid_input(
                "grading_result is not one of baseline, won, lost, tie",
            ));
        }
        if item.get("is_current_best") == Some(&serde_json::Value::Bool(true)) {
            best += 1;
            let version = item.get("version").and_then(|v| v.as_str());
            if value.get("current_best").and_then(|v| v.as_str()) != version {
                return Err(AppError::invalid_input(
                    "is_current_best does not match current_best",
                ));
            }
        }
    }
    if !iterations.is_empty() && best != 1 {
        return Err(AppError::invalid_input(
            "exactly one iteration must be current best",
        ));
    }
    Ok(())
}

fn hash_files(root: &Path) -> Result<Vec<FileHash>, AppError> {
    let mut files = Vec::new();
    walk_hash(root, root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

fn walk_hash(root: &Path, dir: &Path, files: &mut Vec<FileHash>) -> Result<(), AppError> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            walk_hash(root, &path, files)?;
        } else if let Ok(rel) = path.strip_prefix(root) {
            let bytes = fs::read(&path)?;
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            files.push(FileHash {
                path: rel.to_string_lossy().replace('\\', "/"),
                hash: hex_encode(&hasher.finalize()),
            });
        }
    }
    Ok(())
}

fn hash_manifest(files: &[FileHash]) -> String {
    let mut hasher = Sha256::new();
    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update(file.hash.as_bytes());
    }
    hex_encode(&hasher.finalize())
}

fn copy_subject(from: &Path, to: &Path) -> Result<(), AppError> {
    fs::create_dir_all(to)?;
    if !from.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let dest = to.join(entry.file_name());
        if path.is_dir() {
            copy_subject(&path, &dest)?;
        } else {
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0xf) as usize] as char);
    }
    out
}
