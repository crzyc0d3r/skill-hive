//! Combine overlapping skills into one, with a model doing the writing.
//!
//! Two skills whose descriptions overlap make an agent pick between them at random, and two
//! bodies that say nearly the same thing cost context twice. [`Hive::propose_merge`] hands the
//! sources to a configured provider and asks for one `SKILL.md` that keeps every distinct
//! procedure, rule and bundled file, then validates the answer like any authored skill and
//! returns it with everything the user needs to judge it. [`Hive::apply_merge`] writes the
//! accepted proposal into the library, carries bundled files and evaluation cases across, and
//! can retire the sources while moving their deployments to the merged skill.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::conflicts::{self, AssetProfile};
use crate::error::{HiveError, IoContext, Result};
use crate::evals::suite;
use crate::hive::Hive;
use crate::model::{
    AssetKind, DeployFailure, DeployRequest, Deployment, EvalSuite, Finding, HiveEvent,
    LibraryItem, LinkMode, MergeCommit, MergeFile, MergeProposal, MergeRequest, MergeResult,
    MergeSource, ParsedDocument, ProviderConfig, ProviderRole, Scope, Severity, SkillFrontmatter,
    SourceRef, ValidationReport,
};
use crate::providers::CompletionRequest;
use crate::skill;
use crate::util;

/// Longest body of one source handed to the model, in characters.
const MAX_BODY_CHARS: usize = 24_000;
/// Longest excerpt of one bundled text file.
const MAX_FILE_EXCERPT_CHARS: usize = 1_200;
/// Most bundled files excerpted per source.
const MAX_FILE_EXCERPTS: usize = 8;
/// Output budget for the merged document.
const MAX_OUTPUT_TOKENS: u32 = 8_192;
/// Frontmatter key naming the sources of a merged skill.
pub const MERGED_FROM_KEY: &str = "merged-from";

const SYSTEM_PROMPT: &str = "You combine overlapping skills for AI coding agents into one skill. \
A skill is a directory with a SKILL.md: YAML frontmatter with `name` and `description`, then \
markdown instructions. Agents read only the name and description to decide whether to load a \
skill, so the description must say what the merged skill does and when to use it, covering \
every situation the sources covered, in one or two sentences of at most 1024 characters. The \
body must keep every distinct procedure, rule, command, file reference and caveat from the \
sources, say each shared thing once, resolve contradictions explicitly (state which rule wins \
and when), and refer to bundled files by the final paths listed. Do not invent capabilities the \
sources lack. Keep the body under 500 lines; move long reference material into the bundled \
files that already exist rather than repeating it. Reply with strict JSON and nothing else.";

/// A source skill with everything the prompt and the apply step need.
struct Loaded {
    item: LibraryItem,
    body: String,
    frontmatter: SkillFrontmatter,
    /// Bundled files: path relative to the skill root and size.
    files: Vec<(String, u64)>,
    suite: EvalSuite,
    deployments: Vec<Deployment>,
}

/// What the model answered, before validation.
#[derive(Debug, Default, PartialEq)]
struct Answer {
    name: String,
    description: String,
    body: String,
    notes: String,
    dropped: Vec<String>,
    /// Final paths of the bundled files the body still needs; `None` when the model did not
    /// say, which keeps every file.
    keep_files: Option<Vec<String>>,
}

/// Pull the merged skill out of the model's reply: the JSON object it was asked for, or a raw
/// `SKILL.md` when it answered with the document instead.
/// The first JSON object in `text`, wherever it starts and whatever follows it: the reply may
/// wrap it in a code fence or add a sentence with braces after it.
fn extract_json(text: &str) -> Option<serde_json::Value> {
    for (start, _) in text.char_indices().filter(|(_, c)| *c == '{').take(32) {
        let mut stream =
            serde_json::Deserializer::from_str(&text[start..]).into_iter::<serde_json::Value>();
        if let Some(Ok(value)) = stream.next() {
            if value.is_object() {
                return Some(value);
            }
        }
    }
    None
}

fn parse_answer(text: &str) -> Result<Answer> {
    let text = text.trim();
    let json = extract_json(text);
    if let Some(value) = json.as_ref().and_then(|v| v.as_object()) {
        let field = |k: &str| {
            value
                .get(k)
                .and_then(|v| v.as_str())
                .map(|s| s.trim().to_string())
                .unwrap_or_default()
        };
        let list = |k: &str| {
            value.get(k).and_then(|v| v.as_array()).map(|a| {
                a.iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
        };
        let body = field("body");
        if !body.is_empty() || !field("description").is_empty() {
            return Ok(Answer {
                name: field("name"),
                description: field("description"),
                body,
                notes: field("notes"),
                dropped: list("dropped").unwrap_or_default(),
                keep_files: list("keep_files"),
            });
        }
    }
    // A document instead of JSON: accept it, optionally inside a code fence.
    let doc_text = text
        .strip_prefix("```markdown")
        .or_else(|| text.strip_prefix("```md"))
        .or_else(|| text.strip_prefix("```"))
        .map(|t| t.trim_end_matches("```").trim())
        .unwrap_or(text);
    if doc_text.starts_with("---") {
        if let Ok(doc) = skill::parse_skill_text(doc_text) {
            return Ok(Answer {
                name: doc.name,
                description: doc.description,
                body: doc.body.trim().to_string(),
                ..Default::default()
            });
        }
    }
    Err(HiveError::invalid(
        "the model did not return a merged skill (expected JSON with name, description and body)",
    ))
}

/// Decide the final path of every bundled file: the first source keeps its paths, a later
/// source that ships the same path with different content gets it under its own name, and an
/// identical file is carried once.
fn plan_files(sources: &[Loaded]) -> Vec<MergeFile> {
    // Final paths handed out so far, and (path, content hash) pairs already carried.
    let mut dests: BTreeSet<String> = BTreeSet::new();
    let mut carried: BTreeSet<(String, String)> = BTreeSet::new();
    let mut out = Vec::new();
    for src in sources {
        for (path, size) in &src.files {
            let full = Path::new(&src.item.path).join(path);
            let hash = crate::hash::hash_path(&full)
                .map(|(h, _)| h)
                .unwrap_or_default();
            if !hash.is_empty() && !carried.insert((path.clone(), hash.clone())) {
                continue; // the same file was already carried from an earlier source
            }
            let mut dest = if dests.contains(path) {
                format!("{}/{}", src.item.slug, path)
            } else {
                path.clone()
            };
            let mut n = 1;
            while dests.contains(&dest) {
                n += 1;
                dest = format!("{}-{n}/{}", src.item.slug, path);
            }
            dests.insert(dest.clone());
            out.push(MergeFile {
                from_item_id: src.item.id.clone(),
                from_name: src.item.name.clone(),
                path: path.clone(),
                dest,
                size_bytes: *size,
                keep: true,
            });
        }
    }
    out
}

fn is_text_file(path: &str) -> bool {
    let lower = path.to_lowercase();
    [
        ".md", ".txt", ".py", ".js", ".ts", ".sh", ".json", ".yaml", ".yml", ".toml", ".csv",
        ".rs", ".go", ".rb", ".sql", ".html", ".css",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

fn excerpt(path: &Path) -> Option<String> {
    let bytes = std::fs::read(path).ok()?;
    if bytes.len() > 512 * 1024 {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    let short = util::truncate_chars(text.trim(), MAX_FILE_EXCERPT_CHARS);
    (!short.is_empty()).then_some(short)
}

/// Build the user prompt: what to merge, why, and each source in full.
fn build_prompt(
    sources: &[Loaded],
    files: &[MergeFile],
    conflicts: &crate::model::ConflictReport,
    req: &MergeRequest,
) -> String {
    let mut p = String::new();
    p.push_str(&format!(
        "Merge the following {} skills into one skill.\n",
        sources.len()
    ));
    if let Some(name) = req.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
        p.push_str(&format!("The merged skill must be named `{name}`.\n"));
    }
    if let Some(g) = req
        .guidance
        .as_deref()
        .map(str::trim)
        .filter(|g| !g.is_empty())
    {
        p.push_str(&format!("\nGuidance from the user:\n{g}\n"));
    }
    if !conflicts.conflicts.is_empty() {
        p.push_str("\nWhy they overlap:\n");
        for c in &conflicts.conflicts {
            p.push_str(&format!("- {}: {}\n", c.names.join(" and "), c.detail));
        }
    }
    for (i, src) in sources.iter().enumerate() {
        p.push_str(&format!("\n## Source {}: {}\n", i + 1, src.item.name));
        p.push_str(&format!("Description: {}\n", src.item.description));
        if !src.frontmatter.allowed_tools.is_empty() {
            p.push_str(&format!(
                "Pre-approved tools: {}\n",
                src.frontmatter.allowed_tools.join(" ")
            ));
        }
        if let Some(l) = &src.frontmatter.license {
            p.push_str(&format!("License: {l}\n"));
        }
        let own: Vec<&MergeFile> = files
            .iter()
            .filter(|f| f.from_item_id == src.item.id)
            .collect();
        if own.is_empty() {
            p.push_str("Bundled files: none\n");
        } else {
            p.push_str("Bundled files (final path in the merged skill):\n");
            for f in &own {
                if f.dest == f.path {
                    p.push_str(&format!("- {} ({} bytes)\n", f.dest, f.size_bytes));
                } else {
                    p.push_str(&format!(
                        "- {} ({} bytes; was {} in this source)\n",
                        f.dest, f.size_bytes, f.path
                    ));
                }
            }
        }
        p.push_str("\n### SKILL.md body\n```markdown\n");
        let body = src.body.trim();
        if body.chars().count() > MAX_BODY_CHARS {
            p.push_str(&util::truncate_chars(body, MAX_BODY_CHARS));
            p.push_str("\n[… truncated …]");
        } else {
            p.push_str(body);
        }
        p.push_str("\n```\n");
        let mut shown = 0;
        for f in own.iter().filter(|f| is_text_file(&f.path)) {
            if shown >= MAX_FILE_EXCERPTS {
                break;
            }
            if let Some(ex) = excerpt(&Path::new(&src.item.path).join(&f.path)) {
                p.push_str(&format!("\n### Excerpt of {}\n```\n{ex}\n```\n", f.dest));
                shown += 1;
            }
        }
    }
    p.push_str(
        "\nAnswer with JSON only, in this shape: {\"name\": \"lowercase-hyphenated-name\", \
\"description\": \"what it does and when to use it\", \"body\": \"markdown body without \
frontmatter\", \"notes\": \"two or three sentences on how you combined the sources\", \
\"dropped\": [\"anything from the sources you left out, one item each\"], \"keep_files\": \
[\"final paths of the bundled files the body still refers to\"]}",
    );
    p
}

/// Frontmatter of the merged skill from the sources: the union of pre-approved tools, a licence
/// when every licensed source agrees, merged metadata and the list of sources.
fn merged_frontmatter(sources: &[Loaded], warnings: &mut Vec<String>) -> serde_json::Value {
    let mut fm = serde_json::Map::new();
    let mut tools: Vec<String> = Vec::new();
    for s in sources {
        for t in &s.frontmatter.allowed_tools {
            if !tools.contains(t) {
                tools.push(t.clone());
            }
        }
    }
    if !tools.is_empty() {
        fm.insert("allowed-tools".into(), serde_json::json!(tools.join(" ")));
    }
    let licenses: BTreeSet<&str> = sources
        .iter()
        .filter_map(|s| s.frontmatter.license.as_deref())
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    match licenses.len() {
        0 => {}
        1 => {
            fm.insert(
                "license".into(),
                serde_json::json!(licenses.iter().next().unwrap()),
            );
        }
        _ => warnings.push(format!(
            "The sources carry different licences ({}); the merged skill has none. Check what applies.",
            licenses.into_iter().collect::<Vec<_>>().join("; ")
        )),
    }
    let mut metadata: BTreeMap<String, String> = BTreeMap::new();
    for s in sources {
        for (k, v) in &s.frontmatter.metadata {
            metadata.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
    metadata.insert(
        MERGED_FROM_KEY.to_string(),
        sources
            .iter()
            .map(|s| s.item.name.as_str())
            .collect::<Vec<_>>()
            .join(", "),
    );
    fm.insert("metadata".into(), serde_json::json!(metadata));
    serde_json::Value::Object(fm)
}

/// The complete frontmatter object: name and description first, then the extras.
fn full_frontmatter(
    name: &str,
    description: &str,
    extras: &serde_json::Value,
) -> serde_json::Value {
    let mut fm = serde_json::Map::new();
    fm.insert("name".into(), serde_json::json!(name));
    fm.insert("description".into(), serde_json::json!(description));
    if let Some(obj) = extras.as_object() {
        for (k, v) in obj {
            if k != "name" && k != "description" {
                fm.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(fm)
}

/// A name for the merged skill: the user's, else the model's when valid, else the sources'
/// names joined.
fn resolve_name(
    requested: Option<&str>,
    proposed: &str,
    sources: &[Loaded],
    warnings: &mut Vec<String>,
) -> String {
    if let Some(n) = requested.map(str::trim).filter(|n| !n.is_empty()) {
        return n.to_string();
    }
    let slug = util::slugify(proposed);
    if !proposed.trim().is_empty() && skill::is_valid_name(&slug) && slug != "item" {
        if slug != proposed.trim() {
            warnings.push(format!(
                "The model proposed the name `{}`; it was normalised to `{slug}`.",
                proposed.trim()
            ));
        }
        return slug;
    }
    let joined = util::slugify(
        &sources
            .iter()
            .map(|s| s.item.name.as_str())
            .collect::<Vec<_>>()
            .join("-"),
    );
    let mut name: String = joined.chars().take(skill::MAX_NAME_LEN).collect();
    name = name.trim_end_matches('-').to_string();
    if name.is_empty() {
        name = "merged-skill".into();
    }
    warnings.push(format!(
        "The model proposed no usable name; `{name}` was used instead."
    ));
    name
}

/// Union of the sources' evaluation suites: cases by prompt, trigger cases by query.
fn merged_suite(sources: &[Loaded], name: &str) -> EvalSuite {
    let mut cases = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut trigger = Vec::new();
    let mut seen_queries: BTreeSet<String> = BTreeSet::new();
    for s in sources {
        for c in &s.suite.cases {
            if seen.insert(c.prompt.trim().to_lowercase()) {
                let mut c = c.clone();
                c.id = 0; // renumbered by normalize_suite
                cases.push(c);
            }
        }
        for t in &s.suite.trigger_cases {
            if seen_queries.insert(t.query.trim().to_lowercase()) {
                trigger.push(t.clone());
            }
        }
    }
    let exists = !cases.is_empty() || !trigger.is_empty();
    EvalSuite {
        item_id: String::new(),
        skill_name: name.to_string(),
        cases,
        trigger_cases: trigger,
        path: String::new(),
        exists,
    }
}

impl Hive {
    /// Load the skills to merge, refusing anything that is not two or more distinct skills.
    fn load_merge_sources(
        &self,
        ids: &[String],
        warnings: &mut Vec<String>,
    ) -> Result<Vec<Loaded>> {
        let mut distinct: Vec<&String> = Vec::new();
        for id in ids {
            if !distinct.contains(&id) {
                distinct.push(id);
            }
        }
        if distinct.len() < 2 {
            return Err(HiveError::invalid("pick at least two skills to merge"));
        }
        let lib = self.library();
        let mut out = Vec::new();
        for id in distinct {
            let rec = self
                .db
                .get_item_record(id)?
                .ok_or_else(|| HiveError::not_found(format!("item {id}")))?;
            if rec.item.kind != AssetKind::Skill {
                return Err(HiveError::invalid(format!(
                    "{} is {}; only skills can be merged",
                    rec.item.name,
                    rec.item.kind.label().to_lowercase()
                )));
            }
            let frontmatter = rec
                .item
                .frontmatter
                .as_object()
                .map(skill::skill_frontmatter_from_json)
                .unwrap_or_default();
            let files: Vec<(String, u64)> = lib
                .files(&rec.item)
                .into_iter()
                .filter(|f| !f.is_dir)
                .filter(|f| !skill::SKILL_FILE_NAMES.contains(&f.path.as_str()))
                .filter(|f| !f.path.starts_with(&format!("{}/", suite::SUITE_DIR)))
                .map(|f| (f.path, f.size_bytes))
                .collect();
            let suite = match suite::read_suite(
                &rec.item.id,
                &rec.item.name,
                Path::new(&rec.item.path),
            ) {
                Ok(s) => s,
                Err(err) => {
                    warnings.push(format!(
                        "The evaluation suite of {} could not be read and will not be carried over: {err}",
                        rec.item.name
                    ));
                    EvalSuite {
                        item_id: rec.item.id.clone(),
                        skill_name: rec.item.name.clone(),
                        cases: Vec::new(),
                        trigger_cases: Vec::new(),
                        path: String::new(),
                        exists: false,
                    }
                }
            };
            let deployments = self.with_deployer(|d| {
                d.deployments_with_state(self.db.deployments_for_item(&rec.item.id)?)
            })?;
            out.push(Loaded {
                item: rec.item,
                body: rec.body,
                frontmatter,
                files,
                suite,
                deployments,
            });
        }
        Ok(out)
    }

    /// The provider that writes merges: the one asked for, else the recommender, the grader,
    /// then the executor.
    fn merge_provider(&self, id: Option<&str>) -> Result<ProviderConfig> {
        if let Some(id) = id.map(str::trim).filter(|i| !i.is_empty()) {
            return self.get_provider(id);
        }
        for role in [
            ProviderRole::Recommender,
            ProviderRole::Grader,
            ProviderRole::Executor,
        ] {
            if let Some(p) = self.default_provider_for(role)? {
                return Ok(p);
            }
        }
        Err(HiveError::invalid(
            "no model provider is configured; add one under Settings → Providers",
        ))
    }

    /// Ask a model to combine the skills in `req` into one and return the proposal.
    pub async fn propose_merge(&self, req: &MergeRequest) -> Result<MergeProposal> {
        let mut warnings = Vec::new();
        if let Some(n) = req.name.as_deref().map(str::trim).filter(|n| !n.is_empty()) {
            if !skill::is_valid_name(n) {
                return Err(HiveError::invalid(
                    "name must be 1–64 lowercase letters, digits or single hyphens",
                ));
            }
        }
        let sources = self.load_merge_sources(&req.item_ids, &mut warnings)?;
        let config = self.merge_provider(req.provider_id.as_deref())?;
        let provider = self.provider(&config.id)?;

        let profiles: Vec<AssetProfile> = sources
            .iter()
            .map(|s| AssetProfile::from_item(&s.item, &s.body))
            .collect();
        let options = self.analysis_options(None)?;
        let conflicts = conflicts::analyze(&profiles, &options);
        let files = plan_files(&sources);

        let prompt = build_prompt(&sources, &files, &conflicts, req);
        let request = CompletionRequest {
            system: Some(SYSTEM_PROMPT.to_string()),
            prompt,
            max_tokens: Some(MAX_OUTPUT_TOKENS),
            temperature: None,
            timeout: Some(std::time::Duration::from_secs(
                config.timeout_secs.max(60) as u64
            )),
            cancel: Default::default(),
        };
        self.progress(
            "merge",
            format!("Asking {} to merge {} skills", config.name, sources.len()),
            None,
            None,
        );
        let response = provider.complete(request).await?;
        let answer = parse_answer(&response.text)?;

        let name = resolve_name(req.name.as_deref(), &answer.name, &sources, &mut warnings);
        let description = answer.description.trim().to_string();
        let body = answer.body.trim().to_string();
        if body.is_empty() {
            warnings.push("The model returned an empty body.".into());
        }
        let extras = merged_frontmatter(&sources, &mut warnings);
        let frontmatter = full_frontmatter(&name, &description, &extras);
        let doc = ParsedDocument {
            kind: AssetKind::Skill,
            name: name.clone(),
            description: description.clone(),
            frontmatter,
            body: body.clone(),
        };
        let mut validation: ValidationReport = skill::validate_skill(&doc, Some(&name));
        if let Some(existing) = self.db.find_item_by_name(AssetKind::Skill, &name)? {
            if !sources.iter().any(|s| s.item.id == existing.id) {
                validation.findings.push(Finding {
                    code: "name.taken".into(),
                    severity: Severity::Error,
                    message: format!(
                        "a skill named `{name}` already exists in the library and is not part of this merge; pick another name"
                    ),
                    field: Some("name".into()),
                });
            }
        }

        // Files: keep what the model still references; an unknown answer keeps everything.
        let mut files = files;
        if let Some(keep) = &answer.keep_files {
            let keep: BTreeSet<String> = keep
                .iter()
                .map(|k| k.trim_start_matches("./").to_string())
                .collect();
            for f in &mut files {
                f.keep = keep.contains(&f.dest) || keep.contains(&f.path);
            }
        }
        // A file the body names is kept whatever the model listed.
        for f in &mut files {
            if !f.keep && body.contains(&f.dest) {
                f.keep = true;
            }
        }

        let mut tags: Vec<String> = sources.iter().flat_map(|s| s.item.tags.clone()).collect();
        tags.sort();
        tags.dedup();

        let suite_after = merged_suite(&sources, &name);
        let proposal = MergeProposal {
            body_tokens_before: sources.iter().map(|s| s.item.token_estimate).sum(),
            body_tokens_after: util::estimate_tokens(&body),
            description_tokens_before: sources.iter().map(|s| s.item.description_tokens).sum(),
            description_tokens_after: util::estimate_tokens(&description),
            eval_cases: suite_after.cases.len() as u32,
            sources: sources
                .into_iter()
                .map(|s| MergeSource {
                    file_count: s.files.len() as u32,
                    eval_cases: s.suite.cases.len() as u32,
                    deployments: s.deployments,
                    item: s.item,
                })
                .collect(),
            conflicts,
            name,
            description,
            body,
            frontmatter: extras,
            files,
            tags,
            notes: answer.notes,
            dropped: answer.dropped,
            warnings,
            validation,
            provider_id: config.id.clone(),
            provider_name: config.name.clone(),
            model: response.model,
            input_tokens: response.input_tokens,
            output_tokens: response.output_tokens,
            latency_ms: response.latency_ms,
        };
        self.db.audit(
            "merge.propose",
            &serde_json::json!({
                "sources": proposal.sources.iter().map(|s| s.item.id.clone()).collect::<Vec<_>>(),
                "name": proposal.name,
                "provider": config.id,
            }),
        )?;
        Ok(proposal)
    }

    /// Write an accepted proposal into the library.
    pub fn apply_merge(&self, commit: &MergeCommit) -> Result<MergeResult> {
        let name = commit.name.trim();
        if !skill::is_valid_name(name) {
            return Err(HiveError::invalid(
                "name must be 1–64 lowercase letters, digits or single hyphens",
            ));
        }
        let mut warnings = Vec::new();
        let sources = self.load_merge_sources(&commit.source_ids, &mut warnings)?;
        let primary = self
            .db
            .find_item_by_name(AssetKind::Skill, name)?
            .filter(|existing| sources.iter().any(|s| s.item.id == existing.id));
        if primary.is_none() && self.db.find_item_by_name(AssetKind::Skill, name)?.is_some() {
            return Err(HiveError::AlreadyExists(format!(
                "a skill named `{name}` already exists and is not part of this merge"
            )));
        }

        // Stage the merged skill in a temporary directory.
        let tmp = tempfile::tempdir()?;
        let staging = tmp.path().join(name);
        std::fs::create_dir_all(&staging).at(&staging)?;
        let frontmatter = full_frontmatter(name, commit.description.trim(), &commit.frontmatter);
        let text = skill::render_document(&frontmatter, commit.body.trim())?;
        skill::parse_skill_text(&text)?;
        std::fs::write(staging.join("SKILL.md"), text).at(&staging)?;
        let mut dests: BTreeSet<String> = BTreeSet::new();
        for f in commit.files.iter().filter(|f| f.keep) {
            let path = f.path.replace('\\', "/");
            let dest = f.dest.replace('\\', "/");
            if !util::is_safe_relative(&path) || !util::is_safe_relative(&dest) {
                return Err(HiveError::invalid(format!(
                    "bundled file path `{}` escapes the skill",
                    f.dest
                )));
            }
            let top = dest.split('/').next().unwrap_or("");
            if skill::SKILL_FILE_NAMES.contains(&dest.as_str()) || top == suite::SUITE_DIR {
                return Err(HiveError::invalid(format!(
                    "bundled file `{dest}` would overwrite the merged SKILL.md or its evaluation suite"
                )));
            }
            if !dests.insert(dest.clone()) {
                return Err(HiveError::invalid(format!(
                    "two bundled files would both be written to `{dest}`"
                )));
            }
            let Some(src) = sources.iter().find(|s| s.item.id == f.from_item_id) else {
                return Err(HiveError::invalid(format!(
                    "bundled file {dest} comes from a skill that is not part of this merge"
                )));
            };
            let from = Path::new(&src.item.path).join(&path);
            let to = staging.join(&dest);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            std::fs::copy(&from, &to).at(&from)?;
        }
        let mut eval_cases = 0;
        if commit.carry_evals {
            let mut suite = merged_suite(&sources, name);
            if suite.exists {
                copy_eval_files(&sources, &mut suite, &staging)?;
                suite::normalize_suite(&mut suite)?;
                suite::write_suite(&staging, &suite)?;
                eval_cases = suite.cases.len() as u32;
            }
        } else if let Some(existing) = &primary {
            // A source replaced in place keeps its own suite when the union is not wanted.
            let evals = Path::new(&existing.path).join(suite::SUITE_DIR);
            if evals.is_dir() {
                util::copy_dir(&evals, &staging.join(suite::SUITE_DIR))?;
                eval_cases = suite::read_suite(&existing.id, name, &staging)
                    .map(|s| s.cases.len() as u32)
                    .unwrap_or(0);
            }
        }

        // From here on the merged skill is in the library: what fails afterwards is reported
        // in the result rather than raised, so the caller always learns what the library holds.
        let mut retired = Vec::new();
        let mut replaced_id = None;
        let mut deployed = Vec::new();
        let mut failed = Vec::new();
        let item = match &primary {
            Some(existing) => {
                let item = self.library().import_path(
                    &staging,
                    SourceRef::authored(),
                    &commit.tags,
                    true,
                )?;
                replaced_id = Some(existing.id.clone());
                // Copies of the replaced skill are stale until re-synced; links follow the
                // library automatically but their recorded hash moves too.
                match self.db.deployments_for_item(&item.id) {
                    Ok(deps) => {
                        for dep in deps {
                            if let Err(err) = self.with_deployer(|d| d.resync(&dep.id)) {
                                warnings.push(format!(
                                    "the copy at {} could not be re-synced: {err}",
                                    dep.path
                                ));
                            }
                        }
                    }
                    Err(err) => warnings.push(format!(
                        "the deployments of {} could not be listed: {err}",
                        item.name
                    )),
                }
                item
            }
            None => {
                self.library()
                    .import_path(&staging, SourceRef::authored(), &commit.tags, false)?
            }
        };
        // Where the retired sources were deployed: the merged skill goes there afterwards, once
        // per target, scope and project, in the mode of the first deployment seen.
        let mut targets: Vec<(String, Scope, Option<String>, LinkMode)> = Vec::new();
        if commit.retire_sources {
            for s in sources
                .iter()
                .filter(|s| Some(&s.item.id) != replaced_id.as_ref())
            {
                for dep in &s.deployments {
                    let seen = targets.iter().any(|(t, sc, p, _)| {
                        *t == dep.target_id && *sc == dep.scope && *p == dep.project_path
                    });
                    if !seen {
                        targets.push((
                            dep.target_id.clone(),
                            dep.scope,
                            dep.project_path.clone(),
                            dep.mode,
                        ));
                    }
                }
                if let Err(err) = self.with_deployer(|d| d.undeploy_item(&s.item.id)) {
                    warnings.push(format!(
                        "the deployments of {} could not be removed ({err}); it stays in the library",
                        s.item.name
                    ));
                    continue;
                }
                match self.library().remove(&s.item.id) {
                    Ok(removed) => retired.push(removed),
                    Err(err) => warnings.push(format!(
                        "{} could not be removed from the library: {err}",
                        s.item.name
                    )),
                }
            }
        }
        let default_mode = self
            .settings()
            .map(|s| s.default_link_mode)
            .unwrap_or(LinkMode::Link);
        for (target_id, scope, project_path, mode) in targets {
            let already = self.db.deployments_for_item(&item.id).unwrap_or_default();
            if already.iter().any(|d| {
                d.target_id == target_id && d.scope == scope && d.project_path == project_path
            }) {
                continue;
            }
            let request = DeployRequest {
                item_ids: vec![item.id.clone()],
                target_id: target_id.clone(),
                scope,
                project_path,
                mode: Some(mode),
                force: false,
            };
            match self.with_deployer(|d| d.deploy(&request, default_mode)) {
                Ok(result) => {
                    deployed.extend(result.deployed);
                    failed.extend(result.failed);
                }
                Err(err) => failed.push(DeployFailure {
                    item_id: item.id.clone(),
                    reason: format!("{target_id}: {err}"),
                }),
            }
        }
        let item = self.get_item(&item.id).unwrap_or(item);
        if let Err(err) = self.db.audit(
            "merge.apply",
            &serde_json::json!({
                "id": item.id,
                "name": item.name,
                "sources": commit.source_ids,
                "retired": retired.iter().map(|r| r.id.clone()).collect::<Vec<_>>(),
                "replaced": replaced_id,
                "warnings": warnings,
            }),
        ) {
            warnings.push(format!("the audit row could not be written: {err}"));
        }
        self.emit(HiveEvent::LibraryChanged);
        self.emit(HiveEvent::DeploymentsChanged);
        Ok(MergeResult {
            item,
            retired,
            replaced_id,
            deployed,
            failed,
            eval_cases,
            warnings,
        })
    }
}

/// Copy the input files the carried-over cases refer to into the staged skill. A path two
/// sources both use for different content is renamed for the later one and the case updated.
fn copy_eval_files(sources: &[Loaded], suite: &mut EvalSuite, staging: &Path) -> Result<()> {
    let mut placed: BTreeMap<String, String> = BTreeMap::new();
    for case in &mut suite.cases {
        // The case came from the source whose suite holds the same prompt.
        let Some(src) = sources.iter().find(|s| {
            s.suite
                .cases
                .iter()
                .any(|c| c.prompt.trim().eq_ignore_ascii_case(case.prompt.trim()))
        }) else {
            continue;
        };
        for file in &mut case.files {
            if !util::is_safe_relative(file) {
                continue;
            }
            let from = Path::new(&src.item.path).join(&*file);
            if !from.is_file() {
                continue;
            }
            let hash = crate::hash::hash_path(&from)
                .map(|(h, _)| h)
                .unwrap_or_default();
            let mut dest: String = file.clone();
            if let Some(existing) = placed.get(&dest) {
                if *existing != hash {
                    let p = PathBuf::from(&dest);
                    let stem = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let parent = p
                        .parent()
                        .map(|d| d.to_string_lossy().replace('\\', "/"))
                        .unwrap_or_default();
                    dest = if parent.is_empty() {
                        format!("{}-{stem}", src.item.slug)
                    } else {
                        format!("{parent}/{}-{stem}", src.item.slug)
                    };
                }
            }
            let to = staging.join(&dest);
            if let Some(parent) = to.parent() {
                std::fs::create_dir_all(parent).at(parent)?;
            }
            if !to.exists() {
                std::fs::copy(&from, &to).at(&from)?;
            }
            placed.insert(dest.clone(), hash);
            *file = dest;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::library::test_support::write_skill;
    use crate::model::{ProviderInput, ProviderKind, Scope};
    use std::sync::Arc;

    fn hive_with_provider(answer: &str) -> (tempfile::TempDir, Arc<Hive>, String) {
        let tmp = tempfile::tempdir().unwrap();
        let hive = Hive::open_temp(tmp.path()).unwrap();
        let home = tmp.path().join("home");
        std::fs::create_dir_all(home.join(".claude")).unwrap();
        hive.set_target_home(home).unwrap();
        let script = tmp.path().join("merge.json");
        std::fs::write(
            &script,
            serde_json::json!({"default": {"output": answer}}).to_string(),
        )
        .unwrap();
        let provider = hive
            .save_provider(
                None,
                &ProviderInput {
                    name: "Writer".into(),
                    kind: ProviderKind::Scripted,
                    model: Some("scripted-1".into()),
                    base_url: None,
                    command: Some(script.to_string_lossy().to_string()),
                    args: vec![],
                    env: Default::default(),
                    api_key_env: None,
                    api_key: None,
                    roles: vec![ProviderRole::Recommender],
                    timeout_secs: None,
                    max_concurrency: None,
                },
            )
            .unwrap();
        (tmp, hive, provider.id)
    }

    fn two_pdf_skills(tmp: &Path, hive: &Hive) -> (LibraryItem, LibraryItem) {
        let src = tmp.join("src");
        let a = write_skill(
            &src,
            "pdf-extract",
            "Extract text and tables from PDF documents. Use when the user shares a PDF.",
            "# Extract\n\nRun `scripts/extract.py` on the file.\n\n- Always check the first row for headers\n",
        );
        std::fs::create_dir_all(a.join("scripts")).unwrap();
        std::fs::write(a.join("scripts/extract.py"), "print('a')\n").unwrap();
        std::fs::write(a.join("README.md"), "shared\n").unwrap();
        std::fs::create_dir_all(a.join("evals")).unwrap();
        std::fs::write(
            a.join("evals/evals.json"),
            r#"{"skill_name":"pdf-extract","evals":[{"id":1,"prompt":"Extract the tables from invoice.pdf","expectations":["tables"]}]}"#,
        )
        .unwrap();
        let b = write_skill(
            &src,
            "pdf-reader",
            "Read and extract text and tables from PDF documents when the user shares a PDF file.",
            "# Reader\n\nUse `scripts/extract.py` (pdfplumber).\n\n- Never check the first row for headers\n",
        );
        std::fs::create_dir_all(b.join("scripts")).unwrap();
        std::fs::write(b.join("scripts/extract.py"), "print('b')\n").unwrap();
        std::fs::write(b.join("README.md"), "shared\n").unwrap();
        std::fs::create_dir_all(b.join("evals")).unwrap();
        std::fs::write(
            b.join("evals/evals.json"),
            r#"{"skill_name":"pdf-reader","evals":[{"id":1,"prompt":"extract the tables from invoice.pdf","expectations":["x"]},{"id":2,"prompt":"Read page 3 of report.pdf","expectations":["page 3"]}]}"#,
        )
        .unwrap();
        let item_a = hive.import_path(&a, &["docs".into()], false).unwrap();
        let item_b = hive.import_path(&b, &["pdf".into()], false).unwrap();
        (item_a, item_b)
    }

    const ANSWER: &str = r##"Here you go:
{"name": "PDF Tools", "description": "Extract text and tables from PDF documents. Use when the user shares a PDF or asks to read one.", "body": "# PDF tools\n\nRun `scripts/extract.py` on the file. The reader's copy lives at `pdf-reader/scripts/extract.py`.\n\n- Check the first row for headers unless the table has none.\n", "notes": "Combined both procedures; kept the header rule from pdf-extract.", "dropped": ["pdfplumber mention"], "keep_files": ["scripts/extract.py", "pdf-reader/scripts/extract.py"]}"##;

    #[tokio::test]
    async fn proposes_a_validated_merge_with_files_tokens_and_evals() {
        let (tmp, hive, provider_id) = hive_with_provider(ANSWER);
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone(), a.id.clone()],
                name: None,
                guidance: Some("keep it short".into()),
                provider_id: None,
            })
            .await
            .unwrap();
        assert_eq!(proposal.name, "pdf-tools");
        assert_eq!(proposal.provider_id, provider_id);
        assert_eq!(proposal.model.as_deref(), Some("scripted-1"));
        assert_eq!(proposal.sources.len(), 2);
        assert!(
            proposal
                .conflicts
                .conflicts
                .iter()
                .any(|c| c.kind == crate::model::ConflictKind::DescriptionOverlap)
        );
        assert!(
            proposal
                .conflicts
                .conflicts
                .iter()
                .any(|c| c.kind == crate::model::ConflictKind::ContradictoryDirective)
        );
        assert!(
            proposal
                .validation
                .findings
                .iter()
                .all(|f| f.severity != Severity::Error),
            "{:?}",
            proposal.validation
        );
        // The shared README is carried once; the clashing script gets a prefixed path.
        let dests: Vec<&str> = proposal.files.iter().map(|f| f.dest.as_str()).collect();
        assert_eq!(
            dests,
            vec![
                "README.md",
                "scripts/extract.py",
                "pdf-reader/scripts/extract.py"
            ]
        );
        let keep: Vec<bool> = proposal.files.iter().map(|f| f.keep).collect();
        assert_eq!(keep, vec![false, true, true]);
        assert_eq!(proposal.tags, vec!["docs", "pdf"]);
        assert_eq!(proposal.eval_cases, 2, "cases are deduplicated by prompt");
        assert!(proposal.body_tokens_before > 0 && proposal.body_tokens_after > 0);
        assert!(
            proposal.frontmatter["metadata"][MERGED_FROM_KEY]
                .as_str()
                .unwrap()
                .contains("pdf-extract")
        );
        assert!(
            proposal.warnings.iter().any(|w| w.contains("normalised")),
            "{:?}",
            proposal.warnings
        );
        assert_eq!(
            proposal.notes,
            "Combined both procedures; kept the header rule from pdf-extract."
        );
        assert_eq!(proposal.dropped, vec!["pdfplumber mention"]);
    }

    #[tokio::test]
    async fn applies_a_merge_retiring_sources_and_moving_deployments() {
        let (tmp, hive, _) = hive_with_provider(ANSWER);
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        hive.deploy(&DeployRequest {
            item_ids: vec![a.id.clone(), b.id.clone()],
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            mode: Some(crate::model::LinkMode::Copy),
            force: false,
        })
        .unwrap();
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone()],
                name: Some("pdf-tools".into()),
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        let result = hive
            .apply_merge(&MergeCommit {
                source_ids: vec![a.id.clone(), b.id.clone()],
                name: proposal.name.clone(),
                description: proposal.description.clone(),
                body: proposal.body.clone(),
                frontmatter: proposal.frontmatter.clone(),
                files: proposal.files.clone(),
                tags: proposal.tags.clone(),
                retire_sources: true,
                carry_evals: true,
            })
            .unwrap();
        assert_eq!(result.item.name, "pdf-tools");
        assert_eq!(result.retired.len(), 2);
        assert!(result.replaced_id.is_none());
        assert_eq!(result.deployed.len(), 1, "{:?}", result.failed);
        assert!(result.failed.is_empty());
        assert_eq!(result.eval_cases, 2);
        assert!(hive.get_item(&a.id).is_err());
        let detail = hive.item_detail(&result.item.id).unwrap();
        assert_eq!(detail.deployments.len(), 1);
        let paths: Vec<&str> = detail.files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"scripts/extract.py"), "{paths:?}");
        assert!(
            paths.contains(&"pdf-reader/scripts/extract.py"),
            "{paths:?}"
        );
        assert!(!paths.contains(&"README.md"), "{paths:?}");
        assert!(paths.contains(&"evals/evals.json"), "{paths:?}");
        let suite = hive.eval_suite(&result.item.id).unwrap();
        assert_eq!(suite.cases.len(), 2);
        assert_eq!(result.item.tags, vec!["docs", "pdf"]);
        let fm = skill::skill_frontmatter_from_json(result.item.frontmatter.as_object().unwrap());
        assert!(
            fm.metadata
                .get(MERGED_FROM_KEY)
                .unwrap()
                .contains("pdf-reader")
        );
        let status = hive
            .target_status("claude-code", Scope::User, None)
            .unwrap();
        assert_eq!(status.assets.len(), 1);
        assert!(status.conflicts.conflicts.is_empty());
    }

    #[tokio::test]
    async fn merging_under_a_source_name_replaces_it_in_place() {
        let (tmp, hive, _) = hive_with_provider(ANSWER);
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        hive.deploy(&DeployRequest {
            item_ids: vec![a.id.clone()],
            target_id: "claude-code".into(),
            scope: Scope::User,
            project_path: None,
            mode: Some(crate::model::LinkMode::Copy),
            force: false,
        })
        .unwrap();
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone()],
                name: Some("pdf-extract".into()),
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        let result = hive
            .apply_merge(&MergeCommit {
                source_ids: vec![a.id.clone(), b.id.clone()],
                name: "pdf-extract".into(),
                description: proposal.description.clone(),
                body: proposal.body.clone(),
                frontmatter: proposal.frontmatter.clone(),
                files: proposal.files.clone(),
                tags: vec![],
                retire_sources: true,
                carry_evals: false,
            })
            .unwrap();
        assert_eq!(result.item.id, a.id, "the primary keeps its id");
        assert_eq!(result.replaced_id.as_deref(), Some(a.id.as_str()));
        assert_eq!(result.retired.len(), 1);
        assert_eq!(result.retired[0].id, b.id);
        let detail = hive.item_detail(&a.id).unwrap();
        assert!(detail.body.contains("# PDF tools"));
        assert_eq!(detail.deployments.len(), 1);
        assert_eq!(
            detail.deployments[0].state,
            crate::model::InstallState::InSync
        );
        // Without carrying the union, the replaced source keeps its own suite.
        assert!(detail.files.iter().any(|f| f.path == "evals/evals.json"));
        assert_eq!(result.eval_cases, 1);
        assert_eq!(hive.eval_suite(&a.id).unwrap().cases.len(), 1);
        assert!(result.warnings.is_empty(), "{:?}", result.warnings);
    }

    #[tokio::test]
    async fn refuses_bad_input_and_taken_names() {
        let (tmp, hive, _) = hive_with_provider(ANSWER);
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        let one = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone()],
                name: None,
                guidance: None,
                provider_id: None,
            })
            .await;
        assert!(one.is_err());
        let other = hive
            .create_skill(
                "pdf-tools",
                "Something unrelated about pdf tools here",
                "# x",
                &[],
            )
            .unwrap();
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone()],
                name: None,
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        assert!(
            proposal
                .validation
                .findings
                .iter()
                .any(|f| f.code == "name.taken")
        );
        let err = hive
            .apply_merge(&MergeCommit {
                source_ids: vec![a.id.clone(), b.id.clone()],
                name: "pdf-tools".into(),
                description: proposal.description.clone(),
                body: proposal.body.clone(),
                frontmatter: proposal.frontmatter.clone(),
                files: vec![],
                tags: vec![],
                retire_sources: false,
                carry_evals: false,
            })
            .unwrap_err();
        assert!(matches!(err, HiveError::AlreadyExists(_)), "{err}");
        assert!(hive.get_item(&other.id).is_ok());
        // Keeping the sources leaves them (and their count) alone.
        let result = hive
            .apply_merge(&MergeCommit {
                source_ids: vec![a.id.clone(), b.id.clone()],
                name: "pdf-all".into(),
                description: proposal.description.clone(),
                body: proposal.body.clone(),
                frontmatter: proposal.frontmatter.clone(),
                files: vec![],
                tags: vec![],
                retire_sources: false,
                carry_evals: false,
            })
            .unwrap();
        assert_eq!(result.retired.len(), 0);
        assert_eq!(hive.stats().unwrap().skills, 4);
    }

    #[test]
    fn json_is_found_despite_prose_and_braces_around_it() {
        let trailing = parse_answer(
            "```json\n{\"name\":\"pdf-tools\",\"description\":\"d\",\"body\":\"# B\"}\n```\nNote: the body keeps the `{{args}}` placeholders of source 2.",
        )
        .unwrap();
        assert_eq!(trailing.name, "pdf-tools");
        assert_eq!(trailing.body, "# B");
        let leading = parse_answer(
            "I kept the {{args}} placeholder. {\"name\":\"x\",\"description\":\"d\",\"body\":\"b\"} done.",
        )
        .unwrap();
        assert_eq!(leading.body, "b");
        assert!(parse_answer("{ not json } and no document").is_err());
    }

    #[tokio::test]
    async fn long_joined_names_are_cut_without_an_ellipsis() {
        let (tmp, hive, _) = hive_with_provider(
            "---\nname: \ndescription: Everything about these three long-named skills, when any applies.\n---\n\n# Body\n",
        );
        let src = tmp.path().join("src");
        let mut ids = Vec::new();
        for n in [
            "test-driven-development",
            "systematic-debugging-and-tracing",
            "conventional-commit-messages",
        ] {
            let dir = write_skill(
                &src,
                n,
                &format!("Skill {n} for testing long names in merges"),
                "x",
            );
            ids.push(hive.import_path(&dir, &[], false).unwrap().id);
        }
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: ids,
                name: None,
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        assert!(skill::is_valid_name(&proposal.name), "{}", proposal.name);
        assert_eq!(proposal.name.len(), 64);
        assert!(
            proposal
                .validation
                .findings
                .iter()
                .all(|f| f.code != "name.invalid"),
            "{:?}",
            proposal.validation
        );
    }

    #[tokio::test]
    async fn identical_files_across_later_sources_are_carried_once_and_bad_dests_refused() {
        let (tmp, hive, _) = hive_with_provider(ANSWER);
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        // A third source shipping the reader's exact script and a file at the prefixed path.
        let c = write_skill(
            &tmp.path().join("src"),
            "pdf-third",
            "Yet another way to extract tables from PDF documents when a PDF is shared.",
            "# Third\n",
        );
        std::fs::create_dir_all(c.join("scripts")).unwrap();
        std::fs::write(c.join("scripts/extract.py"), "print('b')\n").unwrap();
        std::fs::create_dir_all(c.join("pdf-reader/scripts")).unwrap();
        std::fs::write(c.join("pdf-reader/scripts/extract.py"), "print('c')\n").unwrap();
        let item_c = hive.import_path(&c, &[], false).unwrap();
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone(), item_c.id.clone()],
                name: Some("pdf-all".into()),
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        let dests: Vec<&str> = proposal.files.iter().map(|f| f.dest.as_str()).collect();
        assert_eq!(
            dests,
            vec![
                "README.md",
                "scripts/extract.py",
                "pdf-reader/scripts/extract.py",
                "pdf-third/pdf-reader/scripts/extract.py",
            ],
            "the third source's identical script is carried once; its clashing path gets its own prefix"
        );
        let bad = |dest: &str, path: &str| MergeCommit {
            source_ids: vec![a.id.clone(), b.id.clone()],
            name: "pdf-all".into(),
            description: proposal.description.clone(),
            body: proposal.body.clone(),
            frontmatter: proposal.frontmatter.clone(),
            files: vec![MergeFile {
                from_item_id: a.id.clone(),
                from_name: "pdf-extract".into(),
                path: path.into(),
                dest: dest.into(),
                size_bytes: 1,
                keep: true,
            }],
            tags: vec![],
            retire_sources: false,
            carry_evals: false,
        };
        for (dest, path) in [
            ("SKILL.md", "SKILL.md"),
            ("evals/evals.json", "README.md"),
            ("..\\x", "README.md"),
            ("../x", "README.md"),
        ] {
            let err = hive.apply_merge(&bad(dest, path)).unwrap_err();
            assert!(matches!(err, HiveError::Invalid(_)), "{dest}: {err}");
        }
        let mut dup = bad("docs/a.md", "README.md");
        dup.files.push(dup.files[0].clone());
        assert!(hive.apply_merge(&dup).is_err());
        assert_eq!(hive.stats().unwrap().skills, 3, "nothing was written");
    }

    #[test]
    fn parses_json_and_raw_documents() {
        let a = parse_answer(
            "```json\n{\"name\":\"x\",\"description\":\"d\",\"body\":\"b {braces}\"}\n```",
        )
        .unwrap();
        assert_eq!(a.body, "b {braces}");
        assert_eq!(a.keep_files, None);
        let doc = parse_answer("---\nname: raw-one\ndescription: From a document\n---\n\n# Body\n")
            .unwrap();
        assert_eq!(doc.name, "raw-one");
        assert_eq!(doc.body, "# Body");
        let fenced =
            parse_answer("```markdown\n---\nname: raw-two\ndescription: Fenced\n---\n# B\n```")
                .unwrap();
        assert_eq!(fenced.name, "raw-two");
        assert!(parse_answer("I cannot do that").is_err());
        assert!(parse_answer("{\"notes\": \"only notes\"}").is_err());
    }

    #[tokio::test]
    async fn falls_back_to_a_joined_name_and_accepts_documents() {
        let (tmp, hive, _) = hive_with_provider(
            "---\nname: \ndescription: Everything about PDF files, when a PDF is shared or read.\n---\n\n# Merged body\n",
        );
        let (a, b) = two_pdf_skills(tmp.path(), &hive);
        let proposal = hive
            .propose_merge(&MergeRequest {
                item_ids: vec![a.id.clone(), b.id.clone()],
                name: None,
                guidance: None,
                provider_id: None,
            })
            .await
            .unwrap();
        assert_eq!(proposal.name, "pdf-extract-pdf-reader");
        assert_eq!(proposal.body, "# Merged body");
        assert!(
            proposal
                .warnings
                .iter()
                .any(|w| w.contains("no usable name"))
        );
        // Nothing listed by the model: every bundled file is kept.
        assert!(proposal.files.iter().all(|f| f.keep));
    }
}
