//! Skill drafts: files, templates, preview, packaging, rename, and publish.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use super::frontmatter::{parse_document, Document};
use super::validator::{directory_is_safe, validate_skill, Finding, Profile, Severity};
use crate::util::error::AppError;
use crate::util::path_guard::is_path_safe;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuiltInTemplate {
    Blank,
    Workflow,
    ReferenceHeavy,
    ScriptBacked,
    MultiVariant,
    DocumentProducer,
}

pub fn template_ids() -> &'static [&'static str] {
    &[
        "blank",
        "workflow",
        "reference-heavy",
        "script-backed",
        "multi-variant",
        "document-producer",
    ]
}

impl BuiltInTemplate {
    pub fn parse(id: &str) -> Option<Self> {
        match id {
            "blank" => Some(Self::Blank),
            "workflow" => Some(Self::Workflow),
            "reference-heavy" => Some(Self::ReferenceHeavy),
            "script-backed" => Some(Self::ScriptBacked),
            "multi-variant" => Some(Self::MultiVariant),
            "document-producer" => Some(Self::DocumentProducer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Deployment {
    pub agent: String,
    pub mode: String,
    pub target: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishReport {
    pub wrote: bool,
    pub confirm_agents: Vec<String>,
    pub center_changed: Vec<String>,
}

pub fn plan_publish(deployments: &[Deployment]) -> Vec<String> {
    deployments
        .iter()
        .filter(|item| item.mode == "symlink")
        .map(|item| item.agent.clone())
        .collect()
}

/// Publishing updates the central skill. Symlinked agents must be confirmed
/// first because they see the write immediately. Copied deployments are left
/// untouched and reported as center-changed.
pub fn publish_draft(
    draft: &Path,
    center: &Path,
    deployments: &[Deployment],
    confirmed: bool,
) -> Result<PublishReport, AppError> {
    let confirm_agents = plan_publish(deployments);
    if !confirm_agents.is_empty() && !confirmed {
        return Ok(PublishReport {
            wrote: false,
            confirm_agents,
            center_changed: Vec::new(),
        });
    }
    copy_tree(draft, center)?;
    let mut center_changed = Vec::new();
    for deployment in deployments.iter().filter(|item| item.mode == "copy") {
        if deployment_status(center, &deployment.target) == "center_changed" {
            center_changed.push(deployment.agent.clone());
        }
    }
    Ok(PublishReport {
        wrote: true,
        confirm_agents,
        center_changed,
    })
}

pub fn deployment_status(center: &Path, deployed: &Path) -> &'static str {
    if !deployed.exists() {
        return "missing";
    }
    if hash_tree(center) == hash_tree(deployed) {
        "in_sync"
    } else {
        "center_changed"
    }
}

pub fn instantiate_template(id: BuiltInTemplate, dest: &Path, name: &str) -> Result<(), AppError> {
    if !directory_is_safe(name) || reserved(name) {
        return Err(AppError::invalid_input("skill name is not safe to create"));
    }
    if dest.exists() {
        return Err(AppError::invalid_input("skill directory already exists"));
    }
    fs::create_dir_all(dest)?;
    let skill = skill_markdown(name, id);
    fs::write(dest.join("SKILL.md"), skill)?;
    match id {
        BuiltInTemplate::ReferenceHeavy => {
            fs::create_dir_all(dest.join("references"))?;
            fs::write(
                dest.join("references").join("overview.md"),
                "# Overview\n\nRead this when the task needs the long reference.\n",
            )?;
        }
        BuiltInTemplate::ScriptBacked => {
            fs::create_dir_all(dest.join("scripts"))?;
            let script = dest.join("scripts").join("run.py");
            fs::write(&script, "#!/usr/bin/env python3\nimport sys\nif len(sys.argv) < 2:\n    sys.exit(1)\nprint(sys.argv[1])\n")?;
            set_executable(&script)?;
        }
        BuiltInTemplate::MultiVariant => {
            fs::create_dir_all(dest.join("references"))?;
            fs::write(
                dest.join("references").join("claude.md"),
                "# Claude\n\nRead this for the Claude variant.\n",
            )?;
            fs::write(
                dest.join("references").join("openai.md"),
                "# OpenAI\n\nRead this for the OpenAI variant.\n",
            )?;
        }
        _ => {}
    }
    Ok(())
}

fn skill_markdown(name: &str, id: BuiltInTemplate) -> String {
    let body = match id {
        BuiltInTemplate::Blank => "\n",
        BuiltInTemplate::Workflow => "\n1. Read the task.\n2. Do the work.\n3. Explain why each step is there.\n\n## Output format\n\n- Result\n",
        BuiltInTemplate::ReferenceHeavy => "\nRead `references/overview.md` when the task needs the long reference.\n",
        BuiltInTemplate::ScriptBacked => "\nRun `scripts/run.py` with one argument. It exits non-zero when the argument is missing.\n",
        BuiltInTemplate::MultiVariant => "\nRead `references/claude.md` or `references/openai.md` for the matching provider.\n",
        BuiltInTemplate::DocumentProducer => "\n## Output format\n\n# Title\n\n## Examples\n\nOne short example of the finished document.\n",
    };
    format!(
        "---\nname: {name}\ndescription: Use this skill when the task matches {name}.\n---{body}"
    )
}

pub fn rename_skill(dir: &Path, new_name: &str) -> Result<PathBuf, AppError> {
    if !directory_is_safe(new_name) || reserved(new_name) {
        return Err(AppError::invalid_input(
            "rename refused because sanitize_skill_name would change it or the name is reserved",
        ));
    }
    let parent = dir
        .parent()
        .ok_or_else(|| AppError::invalid_input("skill has no parent"))?;
    let dest = parent.join(new_name);
    if dest != dir && dest.exists() {
        return Err(AppError::invalid_input(
            "a skill with that name already exists",
        ));
    }
    if dest != dir {
        fs::rename(dir, &dest)?;
    }
    let skill = dest.join("SKILL.md");
    if skill.is_file() {
        let text = fs::read_to_string(&skill)?;
        let mut doc = parse_document(&text);
        if doc.string("name").is_some() {
            doc.set_string("name", new_name);
            fs::write(&skill, doc.serialize())?;
        }
    }
    let evals = dest.join("evals").join("evals.json");
    if evals.is_file() {
        if let Ok(mut json) =
            serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&evals)?)
        {
            if let Some(object) = json.as_object_mut() {
                object.insert(
                    "skill_name".to_string(),
                    serde_json::Value::String(new_name.to_string()),
                );
                fs::write(&evals, serde_json::to_vec_pretty(&json).unwrap_or_default())?;
            }
        }
    }
    Ok(dest)
}

pub fn resolve_inside(root: &Path, rel: &str) -> Result<PathBuf, AppError> {
    if rel.is_empty()
        || rel.starts_with('/')
        || rel.starts_with('\\')
        || rel.contains("..")
        || Path::new(rel).is_absolute()
    {
        return Err(AppError::invalid_input("path escapes the skill directory"));
    }
    let target = root.join(rel);
    if !is_path_safe(root, &target) {
        return Err(AppError::invalid_input("path escapes the skill directory"));
    }
    Ok(target)
}

pub fn write_skill_file(root: &Path, rel: &str, content: &str) -> Result<(), AppError> {
    let target = resolve_inside(root, rel)?;
    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(target, content)?;
    Ok(())
}

pub fn set_skill_executable(root: &Path, rel: &str, executable: bool) -> Result<(), AppError> {
    let target = resolve_inside(root, rel)?;
    if !target.is_file() {
        return Err(AppError::invalid_input("file does not exist"));
    }
    if executable {
        set_executable(&target)?;
    }
    Ok(())
}

pub fn apply_skill_fix(root: &Path, finding: &Finding) -> Result<bool, AppError> {
    match finding.fix_id.as_deref() {
        Some("rename-skill-md") => {
            let from = root.join("skill.md");
            let to = root.join("SKILL.md");
            if from.is_file() && !to.exists() {
                fs::rename(from, to)?;
                return Ok(true);
            }
            Ok(false)
        }
        Some("remove-unknown-key") => {
            let path = root.join("SKILL.md");
            let mut doc = parse_document(&fs::read_to_string(&path)?);
            let key = finding.message.split('`').nth(1).unwrap_or("");
            if doc.remove_key(key) {
                fs::write(path, doc.serialize())?;
                return Ok(true);
            }
            Ok(false)
        }
        Some("add-toc") => {
            let path = resolve_inside(root, &finding.path)?;
            let text = fs::read_to_string(&path)?;
            if !text.to_ascii_lowercase().contains("table of contents") {
                fs::write(
                    &path,
                    format!("# Table of contents\n\n- Overview\n\n{text}"),
                )?;
            }
            Ok(true)
        }
        Some("create-missing-file") => {
            let reference = finding.message.split('`').nth(1).unwrap_or("");
            if reference.is_empty() {
                return Ok(false);
            }
            write_skill_file(root, reference, "")?;
            Ok(true)
        }
        Some("set-executable") => {
            set_skill_executable(root, &finding.path, true)?;
            Ok(true)
        }
        Some("sync-skill-name") => {
            let skill = fs::read_to_string(root.join("SKILL.md")).unwrap_or_default();
            let name = parse_document(&skill)
                .string("name")
                .unwrap_or("")
                .to_string();
            let path = root.join("evals").join("evals.json");
            let mut json = serde_json::from_str::<serde_json::Value>(&fs::read_to_string(&path)?)
                .unwrap_or(serde_json::json!({}));
            if let Some(object) = json.as_object_mut() {
                object.insert("skill_name".into(), serde_json::Value::String(name));
                let bytes = serde_json::to_vec_pretty(&json)
                    .map_err(|err| AppError::invalid_input(err.to_string()))?;
                fs::write(path, bytes)?;
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

pub fn package_skill(root: &Path, out: &Path) -> Result<(), AppError> {
    let findings = validate_skill(root, Profile::Portable);
    let errors = findings
        .iter()
        .filter(|finding| finding.severity == Severity::Error)
        .count();
    if errors > 0 {
        return Err(AppError::invalid_input(format!(
            "package blocked by {errors} errors"
        )));
    }
    if let Some(parent) = out.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = fs::File::create(out)?;
    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default();
    for (rel, path) in list_packaged(root) {
        if path.is_dir() {
            continue;
        }
        zip.start_file(rel, options)
            .map_err(|err| AppError::io(err.to_string()))?;
        let bytes = fs::read(path)?;
        zip.write_all(&bytes)?;
    }
    zip.finish().map_err(|err| AppError::io(err.to_string()))?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    pub listing: String,
    pub loaded: String,
    pub commands: Vec<String>,
    pub shell_enabled: bool,
}

pub fn render_preview(
    skill_dir: &Path,
    sample_arguments: &str,
    project_dir: &Path,
    shell_enabled: bool,
) -> Result<Preview, AppError> {
    let text = fs::read_to_string(skill_dir.join("SKILL.md"))?;
    let doc = parse_document(&text);
    let name = doc.string("name").unwrap_or("").to_string();
    let description = doc.string("description").unwrap_or("").to_string();
    let when = doc.string("when_to_use").unwrap_or("");
    let mut listing = format!("{name}: {description}");
    if !when.is_empty() {
        listing.push(' ');
        listing.push_str(when);
    }
    if listing.chars().count() > 1536 {
        listing = listing.chars().take(1536).collect();
    }
    let loaded = render_loaded(&doc, skill_dir, sample_arguments, project_dir);
    let commands = preview_commands(&doc.body);
    Ok(Preview {
        listing,
        loaded,
        commands,
        shell_enabled,
    })
}

pub fn render_loaded(
    doc: &Document,
    skill_dir: &Path,
    sample_arguments: &str,
    project_dir: &Path,
) -> String {
    let mut body = doc.body.clone();
    let skill = skill_dir.display().to_string();
    let project = project_dir.display().to_string();
    body = body
        .replace("${CLAUDE_SKILL_DIR}", &skill)
        .replace("${CLAUDE_PROJECT_DIR}", &project);
    let args: Vec<&str> = sample_arguments.split_whitespace().collect();
    body = body.replace("$ARGUMENTS", sample_arguments);
    for (index, arg) in args.iter().enumerate() {
        body = body.replace(&format!("${}", index + 1), arg);
    }
    if let Some(first) = args.first() {
        body = body.replace("$name", first);
    }
    body
}

pub fn preview_commands(body: &str) -> Vec<String> {
    let mut commands = Vec::new();
    let mut rest = body;
    while let Some(start) = rest.find("!`") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find('`') {
            commands.push(after[..end].to_string());
            rest = &after[end + 1..];
        } else {
            break;
        }
    }
    commands
}

fn reserved(name: &str) -> bool {
    name == "synced" || name == "anthropic-skills" || name.starts_with("anthropic-skills:")
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), AppError> {
    if to.exists() {
        fs::remove_dir_all(to)?;
    }
    fs::create_dir_all(to)?;
    copy_into(from, from, to)
}

fn copy_into(root: &Path, from: &Path, to_root: &Path) -> Result<(), AppError> {
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path.strip_prefix(root).unwrap_or(&path);
        let dest = to_root.join(rel);
        if path.is_dir() {
            fs::create_dir_all(&dest)?;
            copy_into(root, &path, to_root)?;
        } else {
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(&path, &dest)?;
        }
    }
    Ok(())
}

fn list_packaged(root: &Path) -> Vec<(String, PathBuf)> {
    let mut out = Vec::new();
    walk_packaged(root, root, &mut out);
    out
}

fn walk_packaged(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "__pycache__" || name == "node_modules" || (dir == root && name == "evals") {
            continue;
        }
        if path.is_dir() {
            walk_packaged(root, &path, out);
        } else if let Ok(rel) = path.strip_prefix(root) {
            out.push((rel.to_string_lossy().replace('\\', "/"), path));
        }
    }
}

fn hash_tree(root: &Path) -> String {
    let mut files = list_packaged(root);
    files.sort_by(|a, b| a.0.cmp(&b.0));
    let mut hasher = Sha256::new();
    for (rel, path) in files {
        hasher.update(rel.as_bytes());
        hasher.update(fs::read(path).unwrap_or_default());
    }
    hex_encode(&hasher.finalize())
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

fn set_executable(path: &Path) -> Result<(), AppError> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)?;
    }
    #[cfg(not(unix))]
    {
        let _ = path;
    }
    Ok(())
}

pub fn read_zip_names(path: &Path) -> Result<Vec<String>, AppError> {
    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|err| AppError::io(err.to_string()))?;
    let mut names = Vec::new();
    for index in 0..archive.len() {
        let file = archive
            .by_index(index)
            .map_err(|err| AppError::io(err.to_string()))?;
        names.push(file.name().to_string());
    }
    Ok(names)
}
