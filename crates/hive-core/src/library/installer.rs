// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)
// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.
// Copyright (c) 2026 Tianliang Zhang

use anyhow::{anyhow, Context, Result};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Component, Path};
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

use crate::db::skill_store::{SkillRecord, SkillStore};
use crate::library::content_hash::hash_skill_dir;
use crate::library::index::SkillMetaFile;
use crate::library::skill_metadata::{read_skill_document, sanitize_skill_name, SkillDocument};

#[derive(Debug, Clone)]
pub struct InstalledSkill {
    pub record: SkillRecord,
    pub document: SkillDocument,
}

pub fn install_local(
    store: &SkillStore,
    skills_root: &Path,
    source: &Path,
) -> Result<InstalledSkill> {
    install_tree(
        store,
        skills_root,
        source,
        "local",
        Some(source.to_string_lossy().as_ref()),
    )
}

pub fn batch_import_folder(
    store: &SkillStore,
    skills_root: &Path,
    folder: &Path,
) -> Result<Vec<InstalledSkill>> {
    let mut installed = Vec::new();
    let mut dirs = crate::library::skill_metadata::find_skill_dirs(folder, 4);
    if dirs.first().is_some_and(|d| d == folder) {
        dirs = vec![folder.to_path_buf()];
    }
    for dir in dirs {
        installed.push(install_local(store, skills_root, &dir)?);
    }
    Ok(installed)
}

pub fn install_archive(
    store: &SkillStore,
    skills_root: &Path,
    archive: &Path,
) -> Result<InstalledSkill> {
    let tmp = skills_root.join(".extract-tmp");
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp)?;
    safe_extract(archive, &tmp)?;
    let found = crate::library::skill_metadata::find_skill_dirs(&tmp, 4);
    let source = found
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("archive has no SKILL.md within depth 4"))?;
    let installed = install_tree(
        store,
        skills_root,
        &source,
        "local",
        Some(archive.to_string_lossy().as_ref()),
    )?;
    let _ = fs::remove_dir_all(&tmp);
    Ok(installed)
}

pub fn safe_extract(archive: &Path, dest: &Path) -> Result<()> {
    let file = File::open(archive).with_context(|| format!("open {}", archive.display()))?;
    let mut zip = ZipArchive::new(file)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_string();
        if !is_safe_zip_name(&name) {
            return Err(anyhow!("refusing unsafe archive path: {name}"));
        }
        if entry.is_symlink() {
            return Err(anyhow!("refusing symlink entry: {name}"));
        }
        let out = dest.join(Path::new(&name));
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
            continue;
        }
        if let Some(parent) = out.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut file = File::create(&out)?;
        io::copy(&mut entry, &mut file)?;
    }
    Ok(())
}

fn is_safe_zip_name(name: &str) -> bool {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    let path = Path::new(name);
    path.components().all(|c| matches!(c, Component::Normal(_)))
}

pub fn write_stored_skill_archive(dir: &Path, archive: &Path) -> Result<()> {
    if let Some(parent) = archive.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(archive)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    write_tree(&mut zip, dir, dir, options)?;
    zip.finish()?;
    Ok(())
}

fn write_tree<W: Write + io::Seek>(
    zip: &mut ZipWriter<W>,
    root: &Path,
    dir: &Path,
    options: SimpleFileOptions,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(root)?
            .to_string_lossy()
            .replace('\\', "/");
        if path.is_dir() {
            zip.add_directory(format!("{rel}/"), options)?;
            write_tree(zip, root, &path, options)?;
        } else {
            zip.start_file(rel, options)?;
            let bytes = fs::read(&path)?;
            zip.write_all(&bytes)?;
        }
    }
    Ok(())
}

fn install_tree(
    store: &SkillStore,
    skills_root: &Path,
    source: &Path,
    source_type: &str,
    source_ref: Option<&str>,
) -> Result<InstalledSkill> {
    let document = read_skill_document(source)?;
    let dirname = unique_dirname(skills_root, &document.name)?;
    let dest = skills_root.join(&dirname);
    copy_dir(source, &dest)?;
    let hash = hash_skill_dir(&dest)?;
    let id = skill_id(&dirname, &hash);
    let now = chrono::Utc::now().timestamp_millis();
    let record = SkillRecord {
        id: id.clone(),
        name: document.name.clone(),
        description: document.description.clone(),
        source_type: source_type.to_string(),
        source_ref: source_ref.map(|s| s.to_string()),
        source_ref_resolved: None,
        source_subpath: None,
        source_branch: None,
        source_revision: None,
        remote_revision: None,
        central_path: dest.to_string_lossy().to_string(),
        content_hash: Some(hash),
        enabled: true,
        created_at: now,
        updated_at: now,
        status: "ok".into(),
        update_status: "unknown".into(),
        last_checked_at: None,
        last_check_error: None,
    };
    store.insert_skill(&record)?;
    let meta = SkillMetaFile {
        schema_version: 1,
        id: id.clone(),
        name: record.name.clone(),
        description: record.description.clone(),
        source_type: record.source_type.clone(),
        source_ref: record.source_ref.clone(),
        central_dirname: dirname,
        content_hash: record.content_hash.clone(),
        tags: Vec::new(),
        enabled: true,
    };
    let sidecar = skills_root.join(".skill-hive").join("skills");
    fs::create_dir_all(&sidecar)?;
    fs::write(
        sidecar.join(format!("{id}.json")),
        serde_json::to_vec_pretty(&meta)?,
    )?;
    Ok(InstalledSkill { record, document })
}

fn unique_dirname(skills_root: &Path, name: &str) -> Result<String> {
    let base = sanitize_skill_name(name);
    if !skills_root.join(&base).exists() {
        return Ok(base);
    }
    for n in 2..10_000 {
        let candidate = format!("{base}-{n}");
        if !skills_root.join(&candidate).exists() {
            return Ok(candidate);
        }
    }
    Err(anyhow!("no unique destination for {base}"))
}

fn skill_id(dirname: &str, hash: &str) -> String {
    format!("{dirname}-{}", &hash[..12])
}

pub fn copy_dir(from: &Path, to: &Path) -> io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let dest = to.join(entry.file_name());
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            copy_dir(&entry.path(), &dest)?;
        } else if file_type.is_file() {
            fs::copy(entry.path(), dest)?;
        }
    }
    Ok(())
}

/// Read every byte of a file. Used by tests that compare hashes.
pub fn read_all(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

pub fn archive_and_directory_hash_equal(dir: &Path) -> Result<bool> {
    let tmp = std::env::temp_dir().join(format!(
        "skill-hive-archive-{}",
        chrono::Utc::now().timestamp_millis()
    ));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp)?;
    let archive = tmp.join("skill.skill");
    write_stored_skill_archive(dir, &archive)?;
    let extracted = tmp.join("out");
    safe_extract(&archive, &extracted)?;
    let found = crate::library::skill_metadata::find_skill_dirs(&extracted, 4);
    let unpacked = found.first().cloned().unwrap_or(extracted);
    let equal = hash_skill_dir(dir)? == hash_skill_dir(&unpacked)?;
    let _ = fs::remove_dir_all(&tmp);
    Ok(equal)
}
