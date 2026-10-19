//! Generation session store. Replies come from recorded fixtures, never a live endpoint.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use rusqlite::{params, Connection};
use serde_json::{json, Value};

use crate::authoring::versions::{VersionSource, VersionStore};
use crate::authoring::{validate_agent, validate_skill, Profile, Severity};
use crate::db::migrations::run_migrations;
use crate::providers::{
    CancellationToken, ChatProvider, ChatRequest, ContentBlock, Message, RequestMetadata,
    ThinkingMode, VecSink,
};

use super::envelope::{parse_envelope, write_files, EnvelopeError, RequiredFile};
use super::safety::{scan_scripts, SafetyFlag};
use super::templates::{render, Template};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Skill,
    Agent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Interview,
    Review,
    NeedsRetry,
    Cancelled,
    Saved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    Retry(EnvelopeError),
    Surfaced(EnvelopeError),
    Refused(String),
}

pub struct GenerationSession {
    pub id: String,
    pub kind: Kind,
    pub goal: String,
    pub name: String,
    pub profile: Profile,
    pub status: Status,
    pub files: BTreeMap<String, String>,
    pub base: BTreeMap<String, String>,
    pub sent: Vec<(String, String)>,
    pub endpoint_payloads: Vec<String>,
    pub findings_errors: usize,
    pub safety: Vec<SafetyFlag>,
    pub changes: String,
    pub retries: u32,
    pub library: PathBuf,
    pub staging: PathBuf,
    pub cancelled: bool,
    pub saved: bool,
}

pub struct Attempt {
    pub name: String,
    pub errors: usize,
    pub saved: bool,
    pub status: Status,
}

impl GenerationSession {
    pub fn start(
        root: &Path,
        db: Option<&Path>,
        kind: Kind,
        goal: &str,
        name: &str,
        profile: Profile,
    ) -> Result<Self, String> {
        let id = format!("gen-{name}");
        let library = root.join(name);
        let staging = root.join("staging").join(name);
        fs::create_dir_all(root).map_err(|err| err.to_string())?;
        if let Some(db) = db {
            if let Some(parent) = db.parent() {
                fs::create_dir_all(parent).map_err(|err| err.to_string())?;
            }
            let conn = Connection::open(db).map_err(|err| err.to_string())?;
            run_migrations(&conn).map_err(|err| err.to_string())?;
            conn.execute(
                "INSERT INTO generation_sessions (id, kind, goal, status, endpoint_id, model, profile, cost_usd, created_at)
                 VALUES (?1, ?2, ?3, 'interview', 'replay', 'fixture-model', ?4, 0, ?5)",
                params![
                    id,
                    kind_name(kind),
                    goal,
                    profile_name(profile),
                    unix_now() as i64
                ],
            )
            .map_err(|err| err.to_string())?;
        }
        Ok(Self {
            id,
            kind,
            goal: goal.to_string(),
            name: name.to_string(),
            profile,
            status: Status::Interview,
            files: BTreeMap::new(),
            base: BTreeMap::new(),
            sent: Vec::new(),
            endpoint_payloads: Vec::new(),
            findings_errors: 0,
            safety: Vec::new(),
            changes: String::new(),
            retries: 0,
            library,
            staging,
            cancelled: false,
            saved: false,
        })
    }

    pub fn interview(&mut self, provider: &dyn ChatProvider, answers: &str) -> Result<(), String> {
        let prompt = render(Template::P1, &self.vars(""));
        self.sent.push(("P1".into(), prompt.clone()));
        self.endpoint_payloads.push(prompt.clone());
        let request = ChatRequest {
            model: "fixture-model".into(),
            system: None,
            messages: vec![Message {
                role: "user".into(),
                content: vec![ContentBlock::Text(prompt)],
            }],
            tools: Vec::new(),
            max_tokens: 400,
            effort: None,
            thinking: ThinkingMode::Off,
            stream: false,
            metadata: RequestMetadata {
                purpose: "interview".into(),
                ref_id: Some(self.id.clone()),
                provider_id: None,
            },
        };
        let mut sink = VecSink::default();
        let cancel = CancellationToken::new();
        let response = provider
            .complete(&request, &mut sink, &cancel)
            .map_err(|err| err.message)?;
        let parsed: Value = serde_json::from_str(&response.text).unwrap_or(json!({}));
        let questions = parsed.get("questions").cloned().unwrap_or(json!([]));
        fs::create_dir_all(&self.staging).map_err(|err| err.to_string())?;
        let body = json!({
            "questions": questions,
            "answers": answers,
            "provider_text": response.text,
        });
        fs::write(
            self.staging.join("interview.json"),
            serde_json::to_string_pretty(&body).map_err(|err| err.to_string())?,
        )
        .map_err(|err| err.to_string())?;
        Ok(())
    }

    pub fn run_review(&mut self, draft_reply: &str) -> Result<(), SessionError> {
        self.send(Template::P1, "");
        self.send(Template::P2, "");
        self.send(Template::P4, "");
        self.send(Template::P5, "");
        self.apply_draft(draft_reply)?;
        self.status = Status::Review;
        Ok(())
    }

    pub fn apply_draft(&mut self, reply: &str) -> Result<(), SessionError> {
        let files = self.take_envelope(reply)?;
        self.files = files;
        self.refresh_validation();
        Ok(())
    }

    pub fn revise(&mut self, feedback: &str, reply: &str) -> Result<(), SessionError> {
        self.send(Template::P6, feedback);
        let parsed = self.take_envelope(reply)?;
        let changes = parsed
            .get(".studio/changes.md")
            .cloned()
            .unwrap_or_default();
        let named = named_files(&changes);
        let before = self.files.clone();
        let mut next = before.clone();
        for (path, body) in &parsed {
            if path == ".studio/changes.md" {
                continue;
            }
            let changed = before.get(path) != Some(body);
            if changed && !named.iter().any(|item| item == path) {
                continue;
            }
            next.insert(path.clone(), body.clone());
        }
        self.files = next;
        self.changes = changes;
        self.refresh_validation();
        self.status = Status::Review;
        Ok(())
    }

    pub fn accept(&mut self) -> Result<PathBuf, SessionError> {
        if self.cancelled {
            return Err(SessionError::Refused("cancelled".into()));
        }
        if self.findings_errors > 0 || self.status == Status::NeedsRetry {
            return Err(SessionError::Refused(
                "refusing to save a draft that still has validation errors".into(),
            ));
        }
        write_files(&self.library, &self.files)
            .map_err(|err| SessionError::Refused(err.to_string()))?;
        let _version = VersionStore::open(&self.library)
            .save_with_source(VersionSource::Generated, None)
            .map_err(|err| SessionError::Refused(err.to_string()))?;
        self.saved = true;
        self.status = Status::Saved;
        Ok(self.library.clone())
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.status = Status::Cancelled;
        let _ = fs::remove_dir_all(&self.staging);
        self.saved = false;
    }

    pub fn review_diff(&self) -> String {
        let mut lines = Vec::new();
        if self.base.is_empty() {
            lines.push("--- empty template".into());
            for path in self.files.keys() {
                if path.starts_with(".studio/") {
                    continue;
                }
                lines.push(format!("+++ {path}"));
            }
        } else {
            for (path, body) in &self.files {
                let previous = self.base.get(path).map(String::as_str).unwrap_or("");
                if previous != body {
                    lines.push(format!("--- {path}"));
                    lines.push(format!("+++ {path}"));
                }
            }
        }
        lines.join("\n")
    }

    pub fn inspector_covers_endpoint(&self) -> bool {
        self.endpoint_payloads
            .iter()
            .all(|payload| self.sent.iter().any(|(_, text)| text == payload))
    }

    pub fn prompt_contains_goal(&self) -> bool {
        self.sent.iter().any(|(_, text)| text.contains(&self.goal))
    }

    fn send(&mut self, template: Template, feedback: &str) {
        let text = render(template, &self.vars(feedback));
        self.sent
            .push((template_name(template).to_string(), text.clone()));
        self.endpoint_payloads.push(text);
    }

    fn vars(&self, feedback: &str) -> BTreeMap<String, String> {
        let mut vars = BTreeMap::new();
        vars.insert("kind".into(), kind_name(self.kind).into());
        vars.insert("goal".into(), self.goal.clone());
        vars.insert("name".into(), self.name.clone());
        vars.insert("profile".into(), profile_name(self.profile).into());
        vars.insert(
            "allowed_keys_rule".into(),
            "Only name, description, license, compatibility, metadata and allowed-tools are allowed.".into(),
        );
        vars.insert("answers_block".into(), "accepted defaults".into());
        vars.insert("library_listing".into(), String::new());
        vars.insert("description".into(), String::new());
        vars.insert("positives".into(), "8".into());
        vars.insert("negatives".into(), "8".into());
        vars.insert("evals_block".into(), String::new());
        vars.insert("user_feedback".into(), feedback.into());
        vars.insert("validation_findings".into(), String::new());
        vars.insert("run_evidence".into(), feedback.into());
        vars.insert("current_files_envelope".into(), String::new());
        vars.insert("tool_catalog".into(), "Read, Grep, Glob".into());
        vars.insert("base_subject".into(), String::new());
        vars
    }

    fn take_envelope(&mut self, reply: &str) -> Result<BTreeMap<String, String>, SessionError> {
        let required = match self.kind {
            Kind::Skill => RequiredFile::Skill,
            Kind::Agent => RequiredFile::Agent,
        };
        match parse_envelope(reply, required) {
            Ok(files) => Ok(files),
            Err(err) => {
                if self.retries == 0 {
                    self.retries = 1;
                    Err(SessionError::Retry(err))
                } else {
                    self.status = Status::NeedsRetry;
                    Err(SessionError::Surfaced(err))
                }
            }
        }
    }

    fn refresh_validation(&mut self) {
        let _ = fs::remove_dir_all(&self.staging);
        if write_files(&self.staging, &self.files).is_err() {
            self.findings_errors = 1;
            return;
        }
        let findings = match self.kind {
            Kind::Skill => validate_skill(&self.staging, self.profile),
            Kind::Agent => {
                let path = self.staging.join("agent.md");
                validate_agent(&path, false)
            }
        };
        self.findings_errors = findings
            .iter()
            .filter(|finding| finding.severity == Severity::Error)
            .count();
        self.safety = scan_scripts(&self.files);
    }
}

pub fn named_files(changes: &str) -> Vec<String> {
    let mut names = Vec::new();
    for token in changes.split_whitespace() {
        let token = token.trim_matches(|ch: char| matches!(ch, ',' | ':' | '`' | '*' | '(' | ')'));
        if token.contains('.') && !token.contains("..") && !names.iter().any(|item| item == token) {
            names.push(token.to_string());
        }
    }
    names
}

pub fn draft_trigger_set(reply: &str) -> Result<Vec<Value>, String> {
    let value: Value = serde_json::from_str(reply).map_err(|err| err.to_string())?;
    let rows = value.as_array().ok_or("trigger set must be an array")?;
    if rows.len() < 16 {
        return Err(format!("trigger set has {} queries", rows.len()));
    }
    Ok(rows.clone())
}

pub fn draft_expectations(reply: &str) -> Result<Value, String> {
    serde_json::from_str(reply).map_err(|err| err.to_string())
}

pub fn replay_goal_attempts(root: &Path) -> Result<Vec<Attempt>, String> {
    let mut attempts = Vec::new();
    for index in 1..=10 {
        let broken = index == 10;
        let name = if broken {
            "invoice-bad".to_string()
        } else {
            format!("invoice-{index}")
        };
        let mut session = GenerationSession::start(
            root,
            None,
            Kind::Skill,
            "Extract line items from supplier invoice PDFs into a CSV",
            &name,
            Profile::Portable,
        )?;
        session
            .run_review(&skill_envelope(&name, broken))
            .map_err(|err| format!("{err:?}"))?;
        let saved = session.accept().is_ok();
        attempts.push(Attempt {
            name,
            errors: session.findings_errors,
            saved,
            status: session.status,
        });
    }
    Ok(attempts)
}

pub fn skill_files(name: &str, broken: bool) -> BTreeMap<String, String> {
    let description = if broken {
        "Extract invoices <bad> into a CSV."
    } else {
        "Extract line items from supplier invoice PDFs into a CSV. Use this whenever the user mentions invoices, PDFs, or bookkeeper imports, even if they do not say extract."
    };
    let skill = format!(
        "---\nname: {name}\ndescription: {description}\n---\n\nRead the invoice and write the CSV with scripts/extract.py.\nSee references/columns.md for the columns.\n"
    );
    let evals = json!({
        "skill_name": name,
        "evals": [
            {
                "id": 1,
                "prompt": "Extract the October invoices in inbox/oct into a CSV",
                "expected_output": "A CSV the bookkeeper can import",
                "files": [],
                "expectations": [
                    "The CSV has the columns vendor, date and total",
                    "The script scripts/extract.py was run"
                ]
            },
            {
                "id": 2,
                "prompt": "Turn supplier-invoice.pdf into rows for the ledger",
                "expected_output": "CSV rows",
                "files": [],
                "expectations": [
                    "Twelve data rows are present",
                    "The date column uses the format from the interview"
                ]
            }
        ]
    });
    let mut triggers = Vec::new();
    for index in 0..8 {
        triggers.push(json!({"query": format!("please pull line items from invoice batch {index} into a CSV for the bookkeeper"), "should_trigger": true}));
    }
    for index in 0..8 {
        triggers.push(json!({"query": format!("scan these coffee receipts {index} for an expense report, not the supplier invoices"), "should_trigger": false}));
    }
    let mut files = BTreeMap::new();
    files.insert("SKILL.md".into(), skill);
    files.insert(
        "scripts/extract.py".into(),
        "import sys\nprint('rows')\n".into(),
    );
    files.insert(
        "references/columns.md".into(),
        "# Columns\n\nvendor, date, total\n".into(),
    );
    files.insert("evals/evals.json".into(), format!("{evals}\n"));
    files.insert(
        "evals/trigger.json".into(),
        format!("{}\n", json!(triggers)),
    );
    files
}

pub fn skill_envelope(name: &str, broken: bool) -> String {
    let mut out = String::new();
    for (path, body) in skill_files(name, broken) {
        out.push_str("===== FILE: ");
        out.push_str(&path);
        out.push_str(" =====\n");
        out.push_str(&body);
        if !body.ends_with('\n') {
            out.push('\n');
        }
    }
    out.push_str("===== END =====\n");
    out
}

pub fn agent_envelope(name: &str) -> String {
    format!(
        "===== FILE: agent.md =====\n---\nname: {name}\ndescription: Reviews a pull request diff for security problems and reports findings with file and line. Use when the user asks for a security review of a diff.\ntools: Read, Grep, Glob\nmodel: inherit\nskills: security-checklist\n---\n\nYou review diffs for security problems and you do not edit files.\n\n1. Read the diff.\n2. Report findings with severity, file, line, explanation, and a suggested fix.\n===== FILE: evals/evals.json =====\n{{\"skill_name\": \"{name}\", \"evals\": [{{\"id\": 1, \"prompt\": \"Review this diff for the seeded vulnerability\", \"expected_output\": \"A findings list\", \"files\": [], \"expectations\": [\"The findings file names the seeded vulnerability\", \"The agent did not edit the diff\"]}}, {{\"id\": 2, \"prompt\": \"Check the auth change for injection\", \"expected_output\": \"Findings\", \"files\": [], \"expectations\": [\"A severity is present\", \"A file and line are present\"]}}, {{\"id\": 3, \"prompt\": \"Look at the token parser\", \"expected_output\": \"Findings\", \"files\": [], \"expectations\": [\"The report explains the issue\", \"No files were written\"]}}]}}\n===== END =====\n"
    )
}

fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Skill => "skill",
        Kind::Agent => "agent",
    }
}

fn profile_name(profile: Profile) -> &'static str {
    match profile {
        Profile::Portable => "portable",
        Profile::ClaudeCode => "claude-code",
    }
}

fn template_name(template: Template) -> &'static str {
    match template {
        Template::P1 => "P1",
        Template::P2 => "P2",
        Template::P3 => "P3",
        Template::P4 => "P4",
        Template::P5 => "P5",
        Template::P6 => "P6",
        Template::P7 => "P7",
        Template::P8 => "P8",
        Template::P9 => "P9",
        Template::P10 => "P10",
        Template::P11 => "P11",
        Template::P12 => "P12",
        Template::P13 => "P13",
    }
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}
