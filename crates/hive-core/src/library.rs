//! The library: the one place skills, agents and commands live on this machine.
//!
//! Items are copied into `~/.skill-hive/library/<kind>s/<slug>` with a provenance sidecar in
//! `library/meta/<id>.json`. The SQLite index can always be rebuilt from those files.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::db::Database;
use crate::db::items::NewItem;
use crate::error::{HiveError, IoContext, Result};
use crate::hash;
use crate::model::{
    AssetKind, Candidate, CandidateStatus, FileEntry, ImportResult, ImportSkip, LibraryItem,
    ParsedDocument, SourceRef, ValidationReport,
};
use crate::paths::HivePaths;
use crate::skill;
use crate::util;

/// Sidecar written next to the library for each item so the index can be rebuilt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSidecar {
    pub id: String,
    pub kind: AssetKind,
    pub slug: String,
    pub source: SourceRef,
    pub tags: Vec<String>,
    pub created_at: i64,
}

pub struct Library<'a> {
    pub db: &'a Database,
    pub paths: &'a HivePaths,
}

/// Content-derived facts about an asset on disk.
#[derive(Debug, Clone)]
pub struct Inspection {
    pub doc: ParsedDocument,
    pub validation: ValidationReport,
    pub content_hash: String,
    pub size_bytes: u64,
}

/// Parse, validate and hash an asset at `path`.
pub fn inspect(kind: AssetKind, path: &Path) -> Result<Inspection> {
    let doc = skill::read_asset(kind, path)?;
    let dir_name = if kind == AssetKind::Skill {
        path.file_name().map(|n| n.to_string_lossy().to_string())
    } else {
        None
    };
    let validation = skill::validate(&doc, dir_name.as_deref());
    let (content_hash, size_bytes) = hash::hash_path(path)?;
    Ok(Inspection {
        doc,
        validation,
        content_hash,
        size_bytes,
    })
}

/// Detect what kind of asset a path is, if any.
pub fn detect_kind(path: &Path) -> Option<AssetKind> {
    if path.is_dir() {
        return skill::is_skill_dir(path).then_some(AssetKind::Skill);
    }
    if path.extension().and_then(|e| e.to_str()) == Some("md") {
        // A markdown file with `name:` + `description:` frontmatter is an agent, otherwise a command.
        let text = std::fs::read_to_string(path).ok()?;
        return Some(classify_markdown(&text));
    }
    None
}

/// Agents must declare `name` and `description`; everything else is a command.
pub fn classify_markdown(text: &str) -> AssetKind {
    match skill::split_frontmatter(text) {
        Ok((Some(yaml), _)) => {
            let map = skill::parse_frontmatter_json(&yaml).unwrap_or_default();
            if map.contains_key("name")
                && map.contains_key("description")
                && !map.contains_key("argument-hint")
            {
                AssetKind::Agent
            } else {
                AssetKind::Command
            }
        }
        _ => AssetKind::Command,
    }
}

impl<'a> Library<'a> {
    pub fn new(db: &'a Database, paths: &'a HivePaths) -> Self {
        Self { db, paths }
    }

    fn kind_dir(&self, kind: AssetKind) -> PathBuf {
        match kind {
            AssetKind::Skill => self.paths.skills_dir(),
            AssetKind::Agent => self.paths.agents_dir(),
            AssetKind::Command => self.paths.commands_dir(),
        }
    }

    /// Where an item of this kind and slug lives inside the library.
    pub fn item_path(&self, kind: AssetKind, slug: &str) -> PathBuf {
        match kind {
            AssetKind::Skill => self.kind_dir(kind).join(slug),
            _ => self.kind_dir(kind).join(format!("{slug}.md")),
        }
    }

    fn sidecar_path(&self, id: &str) -> PathBuf {
        self.paths.meta_dir().join(format!("{id}.json"))
    }

    /// Pick a free slug for a name; appends `-2`, `-3`, … on collision.
    fn free_slug(&self, kind: AssetKind, name: &str) -> Result<String> {
        let base = util::slugify(name);
        if !self.db.slug_exists(kind, &base)? && !self.item_path(kind, &base).exists() {
            return Ok(base);
        }
        for n in 2..1000 {
            let candidate = format!("{base}-{n}");
            if !self.db.slug_exists(kind, &candidate)? && !self.item_path(kind, &candidate).exists()
            {
                return Ok(candidate);
            }
        }
        Err(HiveError::msg(format!(
            "could not find a free slug for {name}"
        )))
    }

    /// Build a [`Candidate`] for a path, classifying it against the library.
    pub fn candidate_for(&self, kind: AssetKind, path: &Path, source: SourceRef) -> Candidate {
        let path_str = path.to_string_lossy().to_string();
        let managed = self.paths.is_in_library(path)
            || std::fs::read_link(path)
                .map(|t| self.paths.is_in_library(&resolve_link_target(path, &t)))
                .unwrap_or(false);
        match inspect(kind, path) {
            Ok(ins) => {
                let (status, library_item_id) = if managed {
                    let id = self
                        .db
                        .find_item_by_path(&resolved_path(path))
                        .ok()
                        .flatten()
                        .map(|i| i.id);
                    (CandidateStatus::Managed, id)
                } else if let Ok(mut same) = self.db.find_items_by_hash(&ins.content_hash) {
                    if let Some(item) = same.drain(..).find(|i| i.kind == kind) {
                        (CandidateStatus::InLibrary, Some(item.id))
                    } else if let Ok(Some(item)) = self.db.find_item_by_name(kind, &ins.doc.name) {
                        (CandidateStatus::Modified, Some(item.id))
                    } else {
                        (CandidateStatus::New, None)
                    }
                } else {
                    (CandidateStatus::New, None)
                };
                let status = if ins.validation.is_ok() {
                    status
                } else {
                    CandidateStatus::Invalid
                };
                Candidate {
                    id: hash::hash_bytes(path_str.as_bytes())[..16].to_string(),
                    kind,
                    name: if ins.doc.name.is_empty() {
                        fallback_name(path)
                    } else {
                        ins.doc.name.clone()
                    },
                    description: ins.doc.description.clone(),
                    path: path_str,
                    content_hash: ins.content_hash,
                    size_bytes: ins.size_bytes,
                    token_estimate: util::estimate_tokens(&ins.doc.body),
                    source,
                    validation: ins.validation,
                    status,
                    library_item_id,
                }
            }
            Err(err) => Candidate {
                id: hash::hash_bytes(path_str.as_bytes())[..16].to_string(),
                kind,
                name: fallback_name(path),
                description: String::new(),
                path: path_str,
                content_hash: String::new(),
                size_bytes: 0,
                token_estimate: 0,
                source,
                validation: ValidationReport {
                    findings: vec![crate::model::Finding {
                        code: "parse.failed".into(),
                        severity: crate::model::Severity::Error,
                        message: err.to_string(),
                        field: None,
                    }],
                },
                status: CandidateStatus::Invalid,
                library_item_id: None,
            },
        }
    }

    /// Copy an asset into the library and index it. Returns the new item.
    pub fn import_path(
        &self,
        path: &Path,
        source: SourceRef,
        tags: &[String],
        overwrite: bool,
    ) -> Result<LibraryItem> {
        let kind = detect_kind(path).ok_or_else(|| {
            HiveError::invalid(format!(
                "{} is not a skill directory, agent or command file",
                path.display()
            ))
        })?;
        let ins = inspect(kind, path)?;
        if !ins.validation.is_ok() {
            let errors: Vec<String> = ins
                .validation
                .findings
                .iter()
                .filter(|f| f.severity == crate::model::Severity::Error)
                .map(|f| f.message.clone())
                .collect();
            return Err(HiveError::invalid(format!(
                "{} failed validation: {}",
                path.display(),
                errors.join("; ")
            )));
        }
        let name = if ins.doc.name.is_empty() {
            fallback_name(path)
        } else {
            ins.doc.name.clone()
        };

        if let Some(existing) = self.db.find_item_by_name(kind, &name)? {
            if existing.content_hash == ins.content_hash {
                return Err(HiveError::AlreadyExists(format!(
                    "{} '{}' (identical content)",
                    kind.label(),
                    name
                )));
            }
            if !overwrite {
                return Err(HiveError::AlreadyExists(format!(
                    "{} '{}' exists with different content; use overwrite to replace it",
                    kind.label(),
                    name
                )));
            }
            return self.replace_content(&existing, path, Some(source), tags);
        }

        let slug = self.free_slug(kind, &name)?;
        let dest = self.item_path(kind, &slug);
        copy_asset(kind, path, &dest)?;
        let (content_hash, size_bytes) = hash::hash_path(&dest)?;
        let id = util::new_id();
        let tags: Vec<String> = normalize_tags(tags);
        let new = NewItem {
            id: id.clone(),
            kind,
            name: name.clone(),
            slug: slug.clone(),
            description: ins.doc.description.clone(),
            path: dest.to_string_lossy().to_string(),
            content_hash,
            size_bytes,
            token_estimate: util::estimate_tokens(&ins.doc.body),
            description_tokens: util::estimate_tokens(&ins.doc.description),
            frontmatter: ins.doc.frontmatter.clone(),
            body: ins.doc.body.clone(),
            source: source.clone(),
            validation: ins.validation.clone(),
            tags: tags.clone(),
        };
        let item = match self.db.insert_item(&new) {
            Ok(item) => item,
            Err(err) => {
                let _ = util::remove_path(&dest);
                return Err(err);
            }
        };
        self.write_sidecar(&ItemSidecar {
            id,
            kind,
            slug,
            source,
            tags,
            created_at: item.created_at,
        })?;
        self.db.audit(
            "import",
            &serde_json::json!({"id": item.id, "name": item.name, "source": item.source}),
        )?;
        Ok(item)
    }

    /// Replace the files of an existing item with the content at `path` (update from source, edit).
    pub fn replace_content(
        &self,
        existing: &LibraryItem,
        path: &Path,
        source: Option<SourceRef>,
        tags: &[String],
    ) -> Result<LibraryItem> {
        let dest = PathBuf::from(&existing.path);
        let same_place = dunce::canonicalize(path).ok() == dunce::canonicalize(&dest).ok();
        if !same_place {
            let staging = dest.with_extension("hive-staging");
            let _ = util::remove_path(&staging);
            copy_asset(existing.kind, path, &staging)?;
            util::remove_path(&dest)?;
            std::fs::rename(&staging, &dest).at(&dest)?;
        }
        let ins = inspect(existing.kind, &dest)?;
        let source = source.unwrap_or_else(|| existing.source.clone());
        let new = NewItem {
            id: existing.id.clone(),
            kind: existing.kind,
            name: if ins.doc.name.is_empty() {
                existing.name.clone()
            } else {
                ins.doc.name.clone()
            },
            slug: existing.slug.clone(),
            description: ins.doc.description.clone(),
            path: existing.path.clone(),
            content_hash: ins.content_hash,
            size_bytes: ins.size_bytes,
            token_estimate: util::estimate_tokens(&ins.doc.body),
            description_tokens: util::estimate_tokens(&ins.doc.description),
            frontmatter: ins.doc.frontmatter.clone(),
            body: ins.doc.body.clone(),
            source: source.clone(),
            validation: ins.validation.clone(),
            tags: normalize_tags(tags),
        };
        let item = self.db.update_item_content(&new)?;
        self.write_sidecar(&ItemSidecar {
            id: item.id.clone(),
            kind: item.kind,
            slug: item.slug.clone(),
            source,
            tags: item.tags.clone(),
            created_at: item.created_at,
        })?;
        Ok(item)
    }

    /// Re-read an item from disk after an in-place edit.
    pub fn refresh(&self, id: &str) -> Result<LibraryItem> {
        let existing = self
            .db
            .get_item(id)?
            .ok_or_else(|| HiveError::not_found(format!("item {id}")))?;
        let path = PathBuf::from(&existing.path);
        self.replace_content(&existing, &path, None, &[])
    }

    /// Write a new document for an item (frontmatter + body), then refresh.
    pub fn write_document(&self, id: &str, text: &str) -> Result<LibraryItem> {
        let existing = self
            .db
            .get_item(id)?
            .ok_or_else(|| HiveError::not_found(format!("item {id}")))?;
        let file = match existing.kind {
            AssetKind::Skill => skill::skill_file(Path::new(&existing.path))
                .unwrap_or_else(|| Path::new(&existing.path).join("SKILL.md")),
            _ => PathBuf::from(&existing.path),
        };
        // Validate before touching the file.
        match existing.kind {
            AssetKind::Skill => {
                skill::parse_skill_text(text)?;
            }
            other => {
                skill::parse_markdown_asset(other, text, &existing.name)?;
            }
        }
        util::write_atomic(&file, text.as_bytes())?;
        self.refresh(id)
    }

    /// Create a brand-new skill in the library from a name, description and body.
    pub fn create_skill(
        &self,
        name: &str,
        description: &str,
        body: &str,
        tags: &[String],
    ) -> Result<LibraryItem> {
        if !skill::is_valid_name(name) {
            return Err(HiveError::invalid(
                "name must be 1–64 lowercase letters, digits or single hyphens",
            ));
        }
        let tmp = tempfile::tempdir()?;
        let dir = tmp.path().join(name);
        std::fs::create_dir_all(&dir).at(&dir)?;
        let fm = serde_json::json!({"name": name, "description": description});
        let text = skill::render_document(&fm, body)?;
        std::fs::write(dir.join("SKILL.md"), text).at(&dir)?;
        self.import_path(&dir, SourceRef::authored(), tags, false)
    }

    /// Remove an item from the library (its files, sidecar and index row). Deployments are the
    /// caller's job: remove them first so no dangling links remain.
    pub fn remove(&self, id: &str) -> Result<LibraryItem> {
        let item = self
            .db
            .get_item(id)?
            .ok_or_else(|| HiveError::not_found(format!("item {id}")))?;
        let path = PathBuf::from(&item.path);
        if self.paths.is_in_library(&path) {
            util::remove_path(&path)?;
        }
        let _ = std::fs::remove_file(self.sidecar_path(id));
        self.db.delete_item(id)?;
        self.db.audit(
            "remove",
            &serde_json::json!({"id": item.id, "name": item.name}),
        )?;
        Ok(item)
    }

    pub fn import_many(
        &self,
        paths: &[String],
        source: &SourceRef,
        tags: &[String],
        overwrite: bool,
    ) -> ImportResult {
        let mut imported = Vec::new();
        let mut skipped = Vec::new();
        for p in paths {
            let path = Path::new(p);
            let mut src = source.clone();
            if src.kind == crate::model::SourceKind::LocalPath && src.locator.is_empty() {
                src.locator = p.clone();
            }
            match self.import_path(path, src, tags, overwrite) {
                Ok(item) => imported.push(item),
                Err(err) => skipped.push(ImportSkip {
                    path: p.clone(),
                    reason: err.to_string(),
                }),
            }
        }
        ImportResult { imported, skipped }
    }

    /// Files inside an item, relative to its root.
    pub fn files(&self, item: &LibraryItem) -> Vec<FileEntry> {
        let root = Path::new(&item.path);
        if root.is_file() {
            return vec![FileEntry {
                path: root
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size_bytes: std::fs::metadata(root).map(|m| m.len()).unwrap_or(0),
                is_dir: false,
            }];
        }
        let mut out: Vec<FileEntry> = walkdir::WalkDir::new(root)
            .min_depth(1)
            .sort_by_file_name()
            .into_iter()
            .filter_entry(|e| {
                !hash::IGNORED_DIRS.contains(&e.file_name().to_string_lossy().as_ref())
            })
            .filter_map(|e| e.ok())
            .map(|e| FileEntry {
                path: e
                    .path()
                    .strip_prefix(root)
                    .unwrap_or(e.path())
                    .to_string_lossy()
                    .replace('\\', "/"),
                size_bytes: e.metadata().map(|m| m.len()).unwrap_or(0),
                is_dir: e.file_type().is_dir(),
            })
            .collect();
        out.sort_by(|a, b| a.path.cmp(&b.path));
        out
    }

    /// Read a text file inside an item (path relative to the item root).
    pub fn read_file(&self, item: &LibraryItem, rel: &str) -> Result<String> {
        if !util::is_safe_relative(rel) {
            return Err(HiveError::invalid("path escapes the item"));
        }
        let root = Path::new(&item.path);
        let file = if root.is_file() {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        let bytes = std::fs::read(&file).at(&file)?;
        if bytes.len() > 2 * 1024 * 1024 {
            return Err(HiveError::invalid("file is larger than 2 MB"));
        }
        String::from_utf8(bytes).map_err(|_| HiveError::invalid("file is not UTF-8 text"))
    }

    fn write_sidecar(&self, sidecar: &ItemSidecar) -> Result<()> {
        util::write_json_atomic(&self.sidecar_path(&sidecar.id), sidecar)
    }

    /// Rebuild the index from the library directory. Returns `(indexed, removed)`.
    pub fn reindex(&self) -> Result<(u32, u32)> {
        let mut indexed = 0u32;
        let mut removed = 0u32;
        // Drop rows whose files vanished.
        for item in self.db.all_items()? {
            if !Path::new(&item.path).exists() {
                self.db.delete_item(&item.id)?;
                let _ = std::fs::remove_file(self.sidecar_path(&item.id));
                removed += 1;
            }
        }
        // Index files that have no row.
        for kind in [AssetKind::Skill, AssetKind::Agent, AssetKind::Command] {
            let dir = self.kind_dir(kind);
            let Ok(entries) = std::fs::read_dir(&dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if detect_kind(&path).map(|k| k == kind).unwrap_or(false) {
                    let path_str = path.to_string_lossy().to_string();
                    if self.db.find_item_by_path(&path_str)?.is_some() {
                        continue;
                    }
                    let slug = path
                        .file_stem()
                        .map(|s| s.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let sidecar = self.find_sidecar(kind, &slug);
                    let ins = match inspect(kind, &path) {
                        Ok(i) => i,
                        Err(_) => continue,
                    };
                    let (id, source, tags, created_at) = match sidecar {
                        Some(s) => (s.id, s.source, s.tags, Some(s.created_at)),
                        None => (
                            util::new_id(),
                            SourceRef::local(&path_str),
                            Vec::new(),
                            None,
                        ),
                    };
                    let name = if ins.doc.name.is_empty() {
                        slug.clone()
                    } else {
                        ins.doc.name.clone()
                    };
                    let new = NewItem {
                        id: id.clone(),
                        kind,
                        name,
                        slug: slug.clone(),
                        description: ins.doc.description.clone(),
                        path: path_str,
                        content_hash: ins.content_hash,
                        size_bytes: ins.size_bytes,
                        token_estimate: util::estimate_tokens(&ins.doc.body),
                        description_tokens: util::estimate_tokens(&ins.doc.description),
                        frontmatter: ins.doc.frontmatter.clone(),
                        body: ins.doc.body.clone(),
                        source: source.clone(),
                        validation: ins.validation.clone(),
                        tags: tags.clone(),
                    };
                    if self.db.insert_item(&new).is_ok() {
                        indexed += 1;
                        let _ = self.write_sidecar(&ItemSidecar {
                            id,
                            kind,
                            slug,
                            source,
                            tags,
                            created_at: created_at.unwrap_or_else(util::now_ms),
                        });
                    }
                }
            }
        }
        Ok((indexed, removed))
    }

    fn find_sidecar(&self, kind: AssetKind, slug: &str) -> Option<ItemSidecar> {
        let entries = std::fs::read_dir(self.paths.meta_dir()).ok()?;
        for entry in entries.flatten() {
            if let Ok(sidecar) = util::read_json::<ItemSidecar>(&entry.path()) {
                if sidecar.kind == kind && sidecar.slug == slug {
                    return Some(sidecar);
                }
            }
        }
        None
    }

    /// Library directories that have no index row, and index rows whose files are missing.
    pub fn orphans(&self) -> Result<(Vec<String>, Vec<String>)> {
        let mut orphan_dirs = Vec::new();
        let mut missing = Vec::new();
        for item in self.db.all_items()? {
            if !Path::new(&item.path).exists() {
                missing.push(item.path);
            }
        }
        for kind in [AssetKind::Skill, AssetKind::Agent, AssetKind::Command] {
            if let Ok(entries) = std::fs::read_dir(self.kind_dir(kind)) {
                for entry in entries.flatten() {
                    let path = entry.path().to_string_lossy().to_string();
                    if self.db.find_item_by_path(&path)?.is_none() {
                        orphan_dirs.push(path);
                    }
                }
            }
        }
        Ok((orphan_dirs, missing))
    }
}

fn normalize_tags(tags: &[String]) -> Vec<String> {
    let mut out: Vec<String> = tags
        .iter()
        .map(|t| t.trim().to_lowercase())
        .filter(|t| !t.is_empty())
        .collect();
    out.sort();
    out.dedup();
    out
}

fn fallback_name(path: &Path) -> String {
    path.file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "item".into())
}

fn resolve_link_target(link: &Path, target: &Path) -> PathBuf {
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        link.parent()
            .map(|p| p.join(target))
            .unwrap_or_else(|| target.to_path_buf())
    }
}

fn resolved_path(path: &Path) -> String {
    dunce::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Copy a skill directory or a markdown file to `dest`.
pub fn copy_asset(kind: AssetKind, src: &Path, dest: &Path) -> Result<()> {
    match kind {
        AssetKind::Skill => {
            util::copy_dir(src, dest)?;
        }
        _ => {
            if let Some(parent) = dest.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            std::fs::copy(src, dest).at(src)?;
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::{Path, PathBuf};

    pub fn write_skill(root: &Path, name: &str, description: &str, body: &str) -> PathBuf {
        let dir = root.join(name);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\n\n{body}\n"),
        )
        .unwrap();
        dir
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::write_skill;
    use super::*;

    struct Fixture {
        _tmp: tempfile::TempDir,
        paths: HivePaths,
        db: Database,
        src: PathBuf,
    }

    fn fixture() -> Fixture {
        let tmp = tempfile::tempdir().unwrap();
        let paths = HivePaths::new(tmp.path().join("hive"));
        paths.ensure().unwrap();
        let db = Database::open_memory().unwrap();
        let src = tmp.path().join("src");
        std::fs::create_dir_all(&src).unwrap();
        Fixture {
            _tmp: tmp,
            paths,
            db,
            src,
        }
    }

    #[test]
    fn import_skill_copies_and_indexes() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let dir = write_skill(
            &f.src,
            "pdf-tools",
            "Extract text from PDF files when asked about PDFs",
            "# Body\nDo it.",
        );
        std::fs::create_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("scripts/run.py"), "print(1)").unwrap();

        let item = lib
            .import_path(
                &dir,
                SourceRef::local(dir.to_string_lossy()),
                &["Docs".into()],
                false,
            )
            .unwrap();
        assert_eq!(item.name, "pdf-tools");
        assert_eq!(item.slug, "pdf-tools");
        assert_eq!(item.tags, vec!["docs"]);
        assert!(Path::new(&item.path).join("scripts/run.py").is_file());
        assert!(
            f.paths
                .meta_dir()
                .join(format!("{}.json", item.id))
                .is_file()
        );
        assert_eq!(item.validation, crate::model::ValidationStatus::Ok);

        // identical content again -> already exists
        assert!(matches!(
            lib.import_path(&dir, SourceRef::local("x"), &[], false),
            Err(HiveError::AlreadyExists(_))
        ));

        // modified content without overwrite -> error; with overwrite -> replaced (old files gone)
        std::fs::remove_dir_all(dir.join("scripts")).unwrap();
        std::fs::write(dir.join("SKILL.md"), "---\nname: pdf-tools\ndescription: Extract text and tables from PDF files when asked\n---\n\nNew body\n").unwrap();
        assert!(matches!(
            lib.import_path(&dir, SourceRef::local("x"), &[], false),
            Err(HiveError::AlreadyExists(_))
        ));
        let updated = lib
            .import_path(&dir, SourceRef::local("x"), &[], true)
            .unwrap();
        assert_eq!(updated.id, item.id);
        assert_ne!(updated.content_hash, item.content_hash);
        assert!(updated.description.contains("tables"));
        assert!(!Path::new(&updated.path).join("scripts/run.py").exists());

        let files = lib.files(&updated);
        assert_eq!(files.len(), 1);
        assert_eq!(
            lib.read_file(&updated, "SKILL.md")
                .unwrap()
                .contains("New body"),
            true
        );
        assert!(lib.read_file(&updated, "../x").is_err());

        let removed = lib.remove(&item.id).unwrap();
        assert!(!Path::new(&removed.path).exists());
        assert!(f.db.get_item(&item.id).unwrap().is_none());
    }

    #[test]
    fn slug_collisions_get_suffixes() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let a = write_skill(
            &f.src.join("a"),
            "fmt",
            "Format code with the house style when asked",
            "A",
        );
        let b = write_skill(
            &f.src.join("b"),
            "fmt",
            "Format code with a different style when asked",
            "B",
        );
        let first = lib
            .import_path(&a, SourceRef::local("a"), &[], false)
            .unwrap();
        // same name, different content, no overwrite: refused (names must be unique per kind)
        assert!(
            lib.import_path(&b, SourceRef::local("b"), &[], false)
                .is_err()
        );
        assert_eq!(first.slug, "fmt");
    }

    #[test]
    fn invalid_skill_is_refused() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let dir = f.src.join("Bad Name");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("SKILL.md"),
            "---\nname: Bad Name\ndescription: x\n---\nbody",
        )
        .unwrap();
        let err = lib
            .import_path(&dir, SourceRef::local("x"), &[], false)
            .unwrap_err();
        assert!(matches!(err, HiveError::Invalid(_)), "{err}");
        let cand = lib.candidate_for(AssetKind::Skill, &dir, SourceRef::local("x"));
        assert_eq!(cand.status, CandidateStatus::Invalid);
    }

    #[test]
    fn candidates_classify_against_library() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let dir = write_skill(
            &f.src,
            "git-commit",
            "Write conventional commit messages when committing",
            "Body",
        );
        let c = lib.candidate_for(AssetKind::Skill, &dir, SourceRef::local("x"));
        assert_eq!(c.status, CandidateStatus::New);
        let item = lib
            .import_path(&dir, SourceRef::local("x"), &[], false)
            .unwrap();
        let c = lib.candidate_for(AssetKind::Skill, &dir, SourceRef::local("x"));
        assert_eq!(c.status, CandidateStatus::InLibrary);
        assert_eq!(c.library_item_id.as_deref(), Some(item.id.as_str()));
        std::fs::write(dir.join("extra.txt"), "x").unwrap();
        let c = lib.candidate_for(AssetKind::Skill, &dir, SourceRef::local("x"));
        assert_eq!(c.status, CandidateStatus::Modified);
        let c = lib.candidate_for(
            AssetKind::Skill,
            Path::new(&item.path),
            SourceRef::local("x"),
        );
        assert_eq!(c.status, CandidateStatus::Managed);
    }

    #[test]
    fn agents_and_commands() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let agent = f.src.join("reviewer.md");
        std::fs::write(&agent, "---\nname: reviewer\ndescription: Reviews pull requests for bugs\n---\nYou review code.").unwrap();
        let cmd = f.src.join("standup.md");
        std::fs::write(
            &cmd,
            "---\ndescription: Daily standup summary\n---\nSummarise $ARGUMENTS",
        )
        .unwrap();
        assert_eq!(detect_kind(&agent), Some(AssetKind::Agent));
        assert_eq!(detect_kind(&cmd), Some(AssetKind::Command));
        let a = lib
            .import_path(&agent, SourceRef::local("x"), &[], false)
            .unwrap();
        let c = lib
            .import_path(&cmd, SourceRef::local("x"), &[], false)
            .unwrap();
        assert_eq!(a.kind, AssetKind::Agent);
        assert_eq!(c.kind, AssetKind::Command);
        assert_eq!(c.name, "standup");
        assert!(Path::new(&a.path).ends_with("agents/reviewer.md"));
        let files = lib.files(&c);
        assert_eq!(files[0].path, "standup.md");
    }

    #[test]
    fn create_write_and_refresh() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let item = lib
            .create_skill(
                "notes",
                "Keep meeting notes tidy when asked to summarise",
                "# Notes\nbody",
                &[],
            )
            .unwrap();
        assert_eq!(item.source.kind, crate::model::SourceKind::Authored);
        assert!(lib.create_skill("Bad", "d", "b", &[]).is_err());
        let text = "---\nname: notes\ndescription: Keep meeting notes tidy and short when asked\n---\n\n# Notes v2\n";
        let updated = lib.write_document(&item.id, text).unwrap();
        assert!(updated.description.contains("short"));
        assert!(lib.write_document(&item.id, "no frontmatter").is_err());
    }

    #[test]
    fn reindex_rebuilds_from_disk() {
        let f = fixture();
        let lib = Library::new(&f.db, &f.paths);
        let dir = write_skill(&f.src, "alpha", "Alpha does alpha things when asked", "A");
        let item = lib
            .import_path(&dir, SourceRef::local("x"), &["t".into()], false)
            .unwrap();
        // Simulate a lost index.
        let db2 = Database::open_memory().unwrap();
        let lib2 = Library::new(&db2, &f.paths);
        let (indexed, removed) = lib2.reindex().unwrap();
        assert_eq!((indexed, removed), (1, 0));
        let again = db2.get_item(&item.id).unwrap().unwrap();
        assert_eq!(again.id, item.id);
        assert_eq!(again.tags, vec!["t"]);
        // Remove files behind the index's back.
        util::remove_path(Path::new(&item.path)).unwrap();
        let (indexed, removed) = lib2.reindex().unwrap();
        assert_eq!((indexed, removed), (0, 1));
        assert!(db2.get_item(&item.id).unwrap().is_none());
    }
}
