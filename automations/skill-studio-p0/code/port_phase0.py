#!/usr/bin/env python3
"""Deterministic Phase 0 porter.

Reads the vendored skills-manager sources and writes the Skill Studio
workspace. Each stage is one graph node. Stages are idempotent.
"""

from __future__ import annotations

import json
import os
import shutil
import sys
from pathlib import Path

ROOT = Path(os.environ.get("PROJECT_ROOT", Path(__file__).resolve().parents[3]))
REF = ROOT / "automations" / "skill-studio-p0" / "sources" / "reference"
HEADER = (
    "// Derived from skills-manager (https://github.com/xingkongliang/skills-manager)\n"
    "// commit 73d6b8f77bba6719924885c1f3be56b828d5a192, MIT License.\n"
    "// Copyright (c) 2026 Tianliang Zhang\n"
    "\n"
)


def write(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")
    print(f"wrote {path.relative_to(ROOT)}")


def read_ref(name: str) -> str:
    return (REF / name).read_text(encoding="utf-8")


def with_header(text: str) -> str:
    if text.startswith("// Derived from skills-manager"):
        return text
    return HEADER + text


def stage_inventory() -> None:
    files = sorted(p.name for p in REF.glob("*.rs"))
    payload = {
        "reference_commit": "73d6b8f77bba6719924885c1f3be56b828d5a192",
        "license": "MIT",
        "copyright": "Copyright (c) 2026 Tianliang Zhang",
        "files": files,
        "map": "graphify-out/graph.json",
        "note": "PLAN.md is prose; graphify extract needs an LLM key for documents, so the split uses the code map of these modules plus the Phase 0 task list.",
    }
    out = ROOT / "automations" / "skill-studio-p0" / "out" / "n1_inventory.json"
    write(out, json.dumps(payload, indent=2) + "\n")


def port_error(text: str) -> str:
    old = """    /// A write was refused because the target is not ours to replace. Always
    /// carries `ErrorDetails::TargetConflict`.
    TargetConflict,
}"""
    new = """    /// A write was refused because the target is not ours to replace. Always
    /// carries `ErrorDetails::TargetConflict`.
    TargetConflict,
    /// Provider configuration or transport failed.
    Provider,
    /// A skill or agent failed format validation.
    Validation,
    /// A run exceeded its budget.
    BudgetExceeded,
}"""
    if old not in text:
        raise SystemExit("error.rs: ErrorKind tail not found")
    text = text.replace(old, new, 1)
    old_from = """impl From<tauri::Error> for AppError {
    fn from(e: tauri::Error) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: e.to_string(),
            details: None,
        }
    }
}

"""
    if old_from not in text:
        raise SystemExit("error.rs: tauri From impl not found")
    text = text.replace(old_from, "", 1)
    tokio_from = """impl From<tokio::task::JoinError> for AppError {
    fn from(e: tokio::task::JoinError) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: e.to_string(),
            details: None,
        }
    }
}

"""
    if tokio_from not in text:
        raise SystemExit("error.rs: tokio From impl not found")
    return text.replace(tokio_from, "", 1)


def port_log_sanitize(text: str) -> str:
    needle = """        // Email addresses
        if let Ok(re) = Regex::new(r"[A-Za-z0-9._%+\\-]+@[A-Za-z0-9.\\-]+\\.[A-Za-z]{2,}") {
            rules.push((re, "<email>"));
        }"""
    insert = """        // Provider tokens named by the Skill Studio plan (Anthropic, OpenRouter,
        // Azure account keys, and Authorization bearer values).
        if let Ok(re) = Regex::new(r"\\bsk-ant-[A-Za-z0-9_\\-]+") {
            rules.push((re, "<token>"));
        }
        if let Ok(re) = Regex::new(r"\\bsk-or-[A-Za-z0-9_\\-]+") {
            rules.push((re, "<token>"));
        }
        if let Ok(re) = Regex::new(r"(?i)\\bAccountKey\\s*=\\s*\\S+") {
            rules.push((re, "AccountKey=<token>"));
        }
        if let Ok(re) = Regex::new(r"(?i)\\bBearer\\s+\\S+") {
            rules.push((re, "Bearer <token>"));
        }
        // Email addresses
        if let Ok(re) = Regex::new(r"[A-Za-z0-9._%+\\-]+@[A-Za-z0-9.\\-]+\\.[A-Za-z]{2,}") {
            rules.push((re, "<email>"));
        }"""
    if needle not in text:
        raise SystemExit("log_sanitize.rs: email rule not found")
    text = text.replace(needle, insert, 1)
    extra = """
    #[test]
    fn redacts_provider_tokens_and_bearer() {
        let s = sanitize(
            "sk-ant-api03-abcdefghijklmnopqrstuvwxyz sk-or-v1-abcdefghijklmnopqrstuvwxyz Bearer supersecretvalue AccountKey=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        );
        assert!(!s.contains("sk-ant-"));
        assert!(!s.contains("sk-or-"));
        assert!(!s.contains("supersecretvalue"));
        assert!(!s.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
        assert!(s.contains("<token>"));
        assert!(s.contains("Bearer <token>"));
    }
"""
    if "redacts_provider_tokens_and_bearer" not in text:
        text = text.rstrip() + "\n" + extra
    return text


PANIC_LOG = '''use std::backtrace::Backtrace;
use std::fs;
use std::io::Write;
use std::panic;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static LOG_DIR: OnceLock<PathBuf> = OnceLock::new();

pub fn last_panic_path(log_dir: &Path) -> PathBuf {
    log_dir.join("last_panic.log")
}

pub fn cached_last_panic_path() -> Option<PathBuf> {
    LOG_DIR.get().map(|dir| dir.join("last_panic.log"))
}

/// Install a panic hook that writes `last_panic.log` under `log_dir`.
///
/// The reference hooked this through `tauri::AppHandle`. Studio-core has no
/// Tauri dependency, so the app crate passes the log directory in.
pub fn install_panic_hook(log_dir: PathBuf) {
    let _ = fs::create_dir_all(&log_dir);
    let _ = LOG_DIR.set(log_dir);

    let prev = panic::take_hook();
    panic::set_hook(Box::new(move |info| {
        let backtrace = Backtrace::capture();
        let timestamp = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%.3f%:z");
        let location = info
            .location()
            .map(|l| format!("{}:{}:{}", l.file(), l.line(), l.column()))
            .unwrap_or_else(|| "<unknown>".into());
        let payload = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_string()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "<non-string panic payload>".to_string()
        };

        let body = format!(
            "[{timestamp}] PANIC at {location}\\n{payload}\\n\\nBacktrace:\\n{backtrace}\\n"
        );

        log::error!("panic: {payload} at {location}");

        if let Some(dir) = LOG_DIR.get() {
            let path = dir.join("last_panic.log");
            if let Ok(mut f) = fs::OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .open(&path)
            {
                let _ = f.write_all(body.as_bytes());
            }
        }

        prev(info);
    }));
}
'''

NOTIFIER = '''/// Replaces `tauri::AppHandle::emit` so studio-core never links Tauri.
///
/// The app crate implements this with `AppHandle::emit`. The CLI implements
/// it by writing a JSON line to stderr.
pub trait Notifier: Send + Sync {
    fn emit(&self, event: &str, payload: &str);
}

/// Test double that records events.
#[derive(Default)]
pub struct RecordingNotifier {
    pub events: std::sync::Mutex<Vec<(String, String)>>,
}

impl Notifier for RecordingNotifier {
    fn emit(&self, event: &str, payload: &str) {
        self.events
            .lock()
            .expect("notifier lock")
            .push((event.to_string(), payload.to_string()));
    }
}

/// CLI implementation: one JSON object per line on stderr.
pub struct StderrNotifier;

impl Notifier for StderrNotifier {
    fn emit(&self, event: &str, payload: &str) {
        eprintln!(
            "{}",
            serde_json::json!({"event": event, "payload": payload})
        );
    }
}
'''

SYNC_META = '''use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Preset sidecar. Field names match the reference `ScenarioMetaFile` so an
/// imported Skills Manager library reindexes without a rewrite.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioMetaFile {
    pub schema_version: u32,
    pub scenario_id: String,
    pub name: String,
    pub description: Option<String>,
    pub icon: Option<String>,
    pub sort_order: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSkillMetaFile {
    pub schema_version: u32,
    pub scenario_id: String,
    pub skill_id: String,
    pub sort_order: i32,
    pub tools: BTreeMap<String, bool>,
}
'''


def port_file_watcher(text: str) -> str:
    old_imports = """use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use tauri::Emitter;

use super::{central_repo, skill_store::SkillStore, tool_adapters};
"""
    new_imports = """use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};

use crate::db::skill_store::SkillStore;
use crate::deploy::tool_adapters;
use crate::library::central_repo;
use crate::util::notifier::Notifier;
"""
    if old_imports not in text:
        raise SystemExit("file_watcher.rs: imports not found")
    text = text.replace(old_imports, new_imports, 1)
    text = text.replace(
        "super::auto_backup::notify_central_change();",
        "notify_central_change();",
        1,
    )
    old_sig = "pub fn start_file_watcher<R: tauri::Runtime>(app: tauri::AppHandle<R>, store: Arc<SkillStore>) {"
    new_sig = """fn notify_central_change() {
    // Auto-backup lands in Phase 2. The call site stays so that reconnecting
    // it does not require another pass over the watch loop.
}

pub fn start_file_watcher(notifier: Arc<dyn Notifier>, store: Arc<SkillStore>) {"""
    if old_sig not in text:
        raise SystemExit("file_watcher.rs: signature not found")
    text = text.replace(old_sig, new_sig, 1)
    old_emit = """        let emit_now = |last_emit: &mut Instant| {
            if let Err(err) = app.emit(APP_FS_CHANGED_EVENT, ()) {
                log::debug!("Failed to emit app-files-changed: {err}");
            } else {
                *last_emit = Instant::now();
            }
        };"""
    new_emit = """        let emit_now = |last_emit: &mut Instant| {
            notifier.emit(APP_FS_CHANGED_EVENT, "");
            *last_emit = Instant::now();
        };"""
    if old_emit not in text:
        raise SystemExit("file_watcher.rs: emit closure not found")
    text = text.replace(old_emit, new_emit, 1)
    text = text.replace("crate::core::skill_store::", "crate::db::skill_store::")
    return text


def port_path_guard(text: str) -> str:
    old = """    let mut ancestors = path.ancestors();
    while let Some(anc) = ancestors.next() {"""
    new = """    let ancestors = path.ancestors();
    for anc in ancestors {"""
    if old not in text:
        raise SystemExit("path_guard.rs: ancestor loop not found")
    return text.replace(old, new, 1)


def port_skill_store(text: str) -> str:
    text = text.replace(
        "use super::audit_log::{AuditDraft, AuditEntry, MAX_ENTRIES as AUDIT_MAX_ENTRIES};",
        "use crate::util::audit_log::{AuditDraft, AuditEntry, MAX_ENTRIES as AUDIT_MAX_ENTRIES};",
    )
    text = text.replace("use super::crypto;", "use crate::util::crypto;")
    text = text.replace(
        "super::migrations::run_migrations",
        "crate::db::migrations::run_migrations",
    )
    text = text.replace("super::sync_metadata::", "crate::library::sync_metadata::")
    text = text.replace(
        "crate::core::sync_metadata::",
        "crate::library::sync_metadata::",
    )
    text = text.replace('map.get("b").is_none()', '!map.contains_key("b")')
    return text


def port_tool_adapters(text: str) -> str:
    return text.replace(
        "crate::core::skill_store::SkillStore",
        "crate::db::skill_store::SkillStore",
    )


def port_repo_lock(text: str) -> str:
    text = text.replace(
        "use super::central_repo;",
        "use crate::library::central_repo;",
    )
    text = text.replace(".skills-manager.lock", ".skill-studio.lock")
    text = text.replace(
        ".create(true)\n        .read(true)\n        .write(true)",
        ".create(true)\n        .truncate(false)\n        .read(true)\n        .write(true)",
    )
    return text


def port_cli_bridge(text: str) -> str:
    text = text.replace("skills-manager-cli", "skill-studio-cli")
    text = text.replace(
        "use super::central_repo;",
        "use crate::library::central_repo;",
    )
    return text


def port_central_repo(text: str) -> str:
    text = text.replace(".skills-manager.lock", ".skill-studio.lock")
    text = text.replace('skills-manager.db', "studio.db")
    text = text.replace("skills-manager-cli", "skill-studio-cli")
    text = text.replace('join(".skills-manager")', 'join(".skill-studio")')
    text = text.replace('join("skills-manager")', 'join("skill-studio")')
    text = text.replace("~/.skills-manager", "~/.skill-studio")
    text = text.replace("super::super::cli_bridge", "crate::util::cli_bridge")
    text = text.replace("super::cli_bridge", "crate::util::cli_bridge")
    old_default = """fn default_base_dir() -> PathBuf {
    home_base_dir()
}"""
    new_default = """fn default_base_dir() -> PathBuf {
    if let Some(dev) = std::env::var_os("SKILL_STUDIO_BASE") {
        if !dev.is_empty() {
            return PathBuf::from(dev);
        }
    }
    home_base_dir()
}

/// Sidecar directory Skill Studio writes. Readers fall back to a Skills
/// Manager `.skills-manager` directory when the new one is absent.
pub fn sidecar_dir() -> PathBuf {
    skills_dir().join(".skill-studio")
}

pub fn sidecar_read_dir() -> PathBuf {
    let modern = sidecar_dir();
    if modern.exists() {
        return modern;
    }
    let legacy = skills_dir().join(".skills-manager");
    if legacy.exists() {
        legacy
    } else {
        modern
    }
}"""
    if old_default not in text:
        raise SystemExit("central_repo.rs: default_base_dir not found")
    text = text.replace(old_default, new_default, 1)
    old_dirs = """    let dirs = [skills_dir(), scenarios_dir(), cache_dir(), logs_dir()];
    for d in &dirs {
        fs::create_dir_all(d)?;
    }

    Ok(())
}"""
    new_dirs = """    let dirs = [
        skills_dir(),
        skills_dir().join(".skill-studio"),
        scenarios_dir(),
        cache_dir(),
        cache_dir().join("repos"),
        logs_dir(),
    ];
    for d in &dirs {
        fs::create_dir_all(d)?;
    }

    let lock = current_base.join(".skill-studio.lock");
    if !lock.exists() {
        fs::File::create(&lock)?;
    }

    Ok(())
}"""
    if old_dirs not in text:
        raise SystemExit("central_repo.rs: ensure dirs not found")
    return text.replace(old_dirs, new_dirs, 1)


def emit_module(rel: str, body: str) -> None:
    write(ROOT / rel, with_header(body))


def stage_port_util() -> None:
    core = ROOT / "crates" / "studio-core"
    emit_module("crates/studio-core/src/util/error.rs", port_error(read_ref("error.rs")))
    emit_module(
        "crates/studio-core/src/util/path_guard.rs",
        port_path_guard(read_ref("path_guard.rs")),
    )
    emit_module("crates/studio-core/src/util/crypto.rs", read_ref("crypto.rs"))
    emit_module(
        "crates/studio-core/src/util/log_sanitize.rs",
        port_log_sanitize(read_ref("log_sanitize.rs")),
    )
    emit_module("crates/studio-core/src/util/panic_log.rs", PANIC_LOG)
    emit_module("crates/studio-core/src/util/timing.rs", read_ref("timing.rs"))
    emit_module("crates/studio-core/src/util/audit_log.rs", read_ref("audit_log.rs"))
    write(core / "src" / "util" / "notifier.rs", NOTIFIER)
    write(
        core / "src" / "util" / "mod.rs",
        """pub mod audit_log;
pub mod crypto;
pub mod error;
pub mod log_sanitize;
pub mod notifier;
pub mod panic_log;
pub mod path_guard;
pub mod timing;
""",
    )
    report = ROOT / "automations" / "skill-studio-p0" / "out" / "n2_port_util.md"
    write(
        report,
        "ported util modules: error, path_guard, crypto, log_sanitize, panic_log, timing, audit_log, notifier\n",
    )


def stage_port_db() -> None:
    emit_module("crates/studio-core/src/db/migrations.rs", read_ref("migrations.rs"))
    emit_module(
        "crates/studio-core/src/db/skill_store.rs",
        port_skill_store(read_ref("skill_store.rs")),
    )
    write(
        ROOT / "crates" / "studio-core" / "src" / "db" / "mod.rs",
        "pub mod migrations;\npub mod skill_store;\n",
    )
    write(
        ROOT / "automations" / "skill-studio-p0" / "out" / "n3_port_db.md",
        "ported db: migrations v1-v8, skill_store (database file name studio.db is chosen by central_repo)\n",
    )


def stage_port_repo() -> None:
    emit_module(
        "crates/studio-core/src/library/central_repo.rs",
        port_central_repo(read_ref("central_repo.rs")),
    )
    emit_module(
        "crates/studio-core/src/util/repo_lock.rs",
        port_repo_lock(read_ref("repo_lock.rs")),
    )
    emit_module(
        "crates/studio-core/src/util/cli_bridge.rs",
        port_cli_bridge(read_ref("cli_bridge.rs")),
    )
    emit_module("crates/studio-core/src/library/sync_metadata.rs", SYNC_META)
    # repo_lock and cli_bridge are declared from util/mod.rs; append if missing.
    mod_rs = ROOT / "crates" / "studio-core" / "src" / "util" / "mod.rs"
    text = mod_rs.read_text(encoding="utf-8")
    for line in ("pub mod cli_bridge;\n", "pub mod repo_lock;\n"):
        if line not in text:
            text += line
    write(mod_rs, text)
    write(
        ROOT / "crates" / "studio-core" / "src" / "library" / "mod.rs",
        HEADER + "pub mod central_repo;\npub mod sync_metadata;\n",
    )
    write(
        ROOT / "automations" / "skill-studio-p0" / "out" / "n4_port_repo.md",
        "ported central_repo, repo_lock, cli_bridge; sidecar .skill-studio with .skills-manager read fallback\n",
    )


def stage_port_watch() -> None:
    emit_module(
        "crates/studio-core/src/deploy/tool_adapters.rs",
        port_tool_adapters(read_ref("tool_adapters.rs")),
    )
    emit_module(
        "crates/studio-core/src/util/file_watcher.rs",
        port_file_watcher(read_ref("file_watcher.rs")),
    )
    write(
        ROOT / "crates" / "studio-core" / "src" / "deploy" / "mod.rs",
        HEADER + "pub mod tool_adapters;\n",
    )
    mod_rs = ROOT / "crates" / "studio-core" / "src" / "util" / "mod.rs"
    text = mod_rs.read_text(encoding="utf-8")
    if "pub mod file_watcher;\n" not in text:
        text += "pub mod file_watcher;\n"
    write(mod_rs, text)
    write(
        ROOT / "crates" / "studio-core" / "src" / "lib.rs",
        """//! Skill Studio core. No Tauri dependency: the app and the CLI both link this crate.
pub mod db;
pub mod deploy;
pub mod library;
pub mod util;

#[cfg(test)]
mod phase0_tests;
""",
    )
    write(
        ROOT / "automations" / "skill-studio-p0" / "out" / "n5_port_watch.md",
        "ported file_watcher (Notifier instead of AppHandle) and tool_adapters (required by the watcher's unit tests)\n",
    )


STAGES = {
    "inventory": stage_inventory,
    "port_util": stage_port_util,
    "port_db": stage_port_db,
    "port_repo": stage_port_repo,
    "port_watch": stage_port_watch,
}


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in STAGES:
        raise SystemExit(f"usage: port_phase0.py {'|'.join(STAGES)}")
    STAGES[sys.argv[1]]()


if __name__ == "__main__":
    main()
