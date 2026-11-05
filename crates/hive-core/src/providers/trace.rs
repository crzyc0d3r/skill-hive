//! Execution traces: what an agent did while it worked, read off the provider's event stream.
//!
//! A [`TraceBuilder`] absorbs the events a CLI agent prints (Claude Code's `stream-json`, Codex's
//! `exec --json` in both its current and legacy schemas) and turns them into an
//! [`ExecutionTrace`]: model turns, tool calls by category (read / search / write / command /
//! web / subagent / other), the files read and written, whether the installed skill was opened,
//! thinking and text blocks, the characters that flowed through tool arguments and tool results,
//! an estimated split of the output tokens, the prompt-cache breakdown and a timeline.
//!
//! Tool categories come from the tool name where it is unambiguous (`Read`, `Grep`, `Write`…)
//! and from the command line for shell tools (`cat` reads, `find` and `grep` search, a `>`
//! redirect writes, everything else is a command). The raw per-tool-name counts are kept too.
//!
//! Token figures that the stream reports (output tokens, cache reads and writes) are exact; the
//! split of the output tokens into thinking, text and tool arguments is proportional to the
//! characters of each, and tool-result tokens are estimated at four characters per token.

use std::collections::{BTreeSet, HashMap, HashSet};

use serde_json::Value;

use crate::model::{ExecutionTrace, ToolCategory, TraceStep, TraceStepKind};

/// Timeline steps kept per execution; the rest are counted in `steps_truncated`.
pub const STEP_CAP: usize = 400;
/// Characters kept in a step summary.
pub const SUMMARY_CHARS: usize = 160;
/// Distinct file paths kept per list.
pub const FILE_LIST_CAP: usize = 60;

/// Which agent's event stream the builder is reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone, Copy, Default)]
struct Usage {
    uncached: u64,
    cache_creation: u64,
    cache_read: u64,
}

#[derive(Debug, Clone)]
struct OpenTool {
    name: String,
    at_ms: Option<u64>,
}

/// Builds an [`ExecutionTrace`] from events or from direct recording calls.
#[derive(Debug)]
pub struct TraceBuilder {
    dialect: Dialect,
    cwd: String,
    skill_marker: Option<String>,
    trace: ExecutionTrace,
    /// Claude Code repeats the same message (and its usage) once per content block.
    messages: HashSet<String>,
    usage_by_message: HashMap<String, Usage>,
    anonymous_usage: Usage,
    result_usage: Option<Usage>,
    open_tools: HashMap<String, OpenTool>,
    last_tool: Option<OpenTool>,
    files_read: BTreeSet<String>,
    files_written: BTreeSet<String>,
    tool_time_ms: u64,
    timed: bool,
    model_time_ms: Option<u64>,
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn ceil_div4(chars: u64) -> u64 {
    chars.div_ceil(4)
}

fn first_chars(text: &str, max: usize) -> String {
    crate::util::truncate_chars(
        text.split_whitespace().collect::<Vec<_>>().join(" ").trim(),
        max,
    )
}

/// Agents run shell scripts through a wrapper (`bash -lc '<script>'`, the way Codex and many
/// Claude Code commands do). Return the script when `command` is such a wrapper, with one
/// layer of quotes removed.
fn unwrap_shell(command: &str) -> Option<String> {
    let mut words = command.trim().splitn(3, char::is_whitespace);
    let program = words.next()?.rsplit('/').next()?;
    if !matches!(program, "bash" | "sh" | "zsh" | "dash" | "ksh") {
        return None;
    }
    let flag = words.next()?;
    if !(flag.starts_with('-') && flag.contains('c') && flag.len() <= 5 && !flag.starts_with("--"))
    {
        return None;
    }
    let script = words.next()?.trim();
    let unquoted = if script.len() >= 2 && script.starts_with('\'') && script.ends_with('\'') {
        script[1..script.len() - 1].replace("'\\''", "'")
    } else if script.len() >= 2 && script.starts_with('"') && script.ends_with('"') {
        script[1..script.len() - 1]
            .replace("\\\"", "\"")
            .replace("\\$", "$")
            .replace("\\\\", "\\")
    } else {
        script.to_string()
    };
    Some(unquoted)
}

/// The command line without a shell wrapper and without a heredoc body (the body is data,
/// not commands).
fn command_text(command: &str) -> String {
    let mut text = command.to_string();
    for _ in 0..2 {
        match unwrap_shell(&text) {
            Some(inner) => text = inner,
            None => break,
        }
    }
    if let Some(pos) = text.find("<<") {
        if let Some(newline) = text[pos..].find('\n') {
            text.truncate(pos + newline);
        }
    }
    text
}

/// Split a command line into words the way a shell would: quotes group, backslashes escape,
/// and the quote characters themselves are dropped.
fn shell_words(text: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_word = false;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for c in text.chars() {
        if escaped {
            current.push(c);
            escaped = false;
            in_word = true;
        } else if c == '\\' && quote != Some('\'') {
            escaped = true;
            in_word = true;
        } else if let Some(q) = quote {
            if c == q {
                quote = None;
            } else {
                current.push(c);
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
            in_word = true;
        } else if c.is_whitespace() {
            if in_word {
                words.push(std::mem::take(&mut current));
                in_word = false;
            }
        } else {
            current.push(c);
            in_word = true;
        }
    }
    if in_word {
        words.push(current);
    }
    words
}

/// Strip environment assignments, `sudo`, and leading `cd … &&` / `export … &&` segments
/// from a (wrapper-free) command line.
fn command_words(command: &str) -> Vec<String> {
    let mut segments: Vec<&str> = command.split("&&").map(str::trim).collect();
    // `cd dir && real-command`: the real command decides.
    while segments.len() > 1
        && ["cd", "export", "set", "source", "."]
            .iter()
            .any(|p| segments[0] == *p || segments[0].starts_with(&format!("{p} ")))
    {
        segments.remove(0);
    }
    let first = segments.first().copied().unwrap_or("");
    // Only the first pipeline stage decides the category.
    let stage = first.split('|').next().unwrap_or("").trim();
    shell_words(stage)
        .into_iter()
        .skip_while(|w| {
            w == "sudo"
                || w == "env"
                || w == "time"
                || (w.contains('=') && !w.starts_with('-') && !w.starts_with('>'))
        })
        .collect()
}

/// Is there a `>` redirect outside quotes, other than `2>&1`-style stderr plumbing?
fn has_redirect(command: &str) -> bool {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    let chars: Vec<char> = command.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if escaped {
            escaped = false;
        } else if c == '\\' && quote != Some('\'') {
            escaped = true;
        } else if let Some(q) = quote {
            if c == q {
                quote = None;
            }
        } else if c == '\'' || c == '"' {
            quote = Some(c);
        } else if c == '>' {
            let stderr_only = i > 0 && chars[i - 1] == '2';
            let rest: String = chars[i..].iter().collect();
            let target = rest
                .trim_start_matches('>')
                .trim_start_matches('|')
                .trim_start();
            if !(stderr_only
                || target.starts_with("&1")
                || target.starts_with("/dev/null")
                || target.starts_with("&2"))
            {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Does a command line write to the file system? A redirect, `| tee`, or a writing program.
fn command_writes(command: &str, words: &[String]) -> bool {
    if has_redirect(command) {
        return true;
    }
    if command.contains("| tee ") || command.contains("|tee ") {
        return true;
    }
    match words.first().map(|w| w.rsplit('/').next().unwrap_or(w)) {
        Some(
            "touch" | "mkdir" | "cp" | "mv" | "rm" | "rmdir" | "ln" | "tee" | "chmod" | "chown",
        ) => true,
        Some("sed") => words
            .iter()
            .any(|w| w == "-i" || w.starts_with("-i") || w == "--in-place"),
        _ => false,
    }
}

/// Classify a shell command line.
pub fn classify_command(command: &str) -> ToolCategory {
    let text = command_text(command);
    let words = command_words(&text);
    if command_writes(&text, &words) {
        return ToolCategory::Write;
    }
    let Some(program) = words.first() else {
        return ToolCategory::Command;
    };
    let program = program.rsplit('/').next().unwrap_or(program);
    match program {
        "cat" | "head" | "tail" | "less" | "more" | "bat" | "nl" | "strings" | "od" | "xxd"
        | "hexdump" | "jq" | "awk" | "sed" | "cut" | "diff" => ToolCategory::Read,
        "ls" | "find" | "grep" | "egrep" | "fgrep" | "rg" | "fd" | "fdfind" | "ag" | "ack"
        | "tree" | "wc" | "which" | "whereis" | "locate" | "type" | "stat" | "file" | "du"
        | "pwd" | "realpath" => ToolCategory::Search,
        "git" => match words.get(1).map(String::as_str) {
            Some("grep" | "ls-files" | "status" | "log" | "diff" | "show" | "blame") => {
                ToolCategory::Search
            }
            _ => ToolCategory::Command,
        },
        "curl" | "wget" => ToolCategory::Web,
        _ => ToolCategory::Command,
    }
}

/// The file a reading command targets (`cat path`, `head -n 5 path`), if any.
fn command_file(words: &[String]) -> Option<String> {
    let program = words.first()?.rsplit('/').next()?;
    if !matches!(
        program,
        "cat"
            | "head"
            | "tail"
            | "less"
            | "more"
            | "bat"
            | "nl"
            | "strings"
            | "od"
            | "xxd"
            | "hexdump"
    ) {
        return None;
    }
    let mut skip_next = false;
    for w in words.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if w == "-n" || w == "-c" || w == ">" || w == ">>" {
            // `-n 5`, `-c 400`, and `> target` / `>> target` take the next word.
            skip_next = true;
            continue;
        }
        if w.starts_with('-') || w.starts_with('<') || w.starts_with('>') {
            continue;
        }
        return Some(w.clone());
    }
    None
}

/// The file a reading command line targets, after unwrapping any shell wrapper.
fn command_read_file(command: &str) -> Option<String> {
    let text = command_text(command);
    command_file(&command_words(&text))
}

/// The skill a Claude Code `Skill` tool call invokes (`{"skill": "name"}`, or `"name args"`).
fn invoked_skill(input: &Value) -> Option<String> {
    for key in ["skill", "command", "name"] {
        if let Some(s) = input.get(key).and_then(Value::as_str) {
            let name = s.trim().trim_start_matches('/');
            let name = name.split_whitespace().next().unwrap_or("");
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }
    None
}

/// Category of a Claude Code tool call from its name and arguments.
pub fn categorize_tool(name: &str, input: &Value) -> ToolCategory {
    let lower = name.to_ascii_lowercase();
    match lower.as_str() {
        "read" | "notebookread" | "view" | "read_file" | "open" => ToolCategory::Read,
        "glob" | "grep" | "ls" | "list_dir" | "search" | "codebase_search" => ToolCategory::Search,
        "write"
        | "edit"
        | "multiedit"
        | "notebookedit"
        | "write_file"
        | "edit_file"
        | "apply_patch"
        | "patch"
        | "str_replace_editor"
        | "str_replace_based_edit_tool" => ToolCategory::Write,
        "bash" | "shell" | "exec" | "run_terminal_cmd" | "execute" => {
            let command = input
                .get("command")
                .and_then(Value::as_str)
                .or_else(|| input.get("cmd").and_then(Value::as_str))
                .unwrap_or("");
            classify_command(command)
        }
        "bashoutput" | "killshell" | "killbash" | "monitor" => ToolCategory::Command,
        "webfetch" | "websearch" | "fetch" | "web_search" | "web_fetch" => ToolCategory::Web,
        "task" | "agent" | "subagent" | "dispatch_agent" => ToolCategory::Subagent,
        // Claude Code loads a project skill through its `Skill` tool: that is how the agent
        // opens SKILL.md, so it counts as a read of the skill.
        "skill" => ToolCategory::Read,
        _ => ToolCategory::Other,
    }
}

/// The path a tool call touches, when its arguments name one.
fn tool_path(input: &Value) -> Option<String> {
    for key in ["file_path", "notebook_path", "path", "filePath", "file"] {
        if let Some(p) = input.get(key).and_then(Value::as_str) {
            if !p.trim().is_empty() {
                return Some(p.to_string());
            }
        }
    }
    None
}

/// One line describing a tool call.
fn describe_tool(name: &str, input: &Value) -> String {
    let lower = name.to_ascii_lowercase();
    let text = match lower.as_str() {
        "bash" | "shell" | "exec" | "run_terminal_cmd" | "execute" => input
            .get("command")
            .and_then(Value::as_str)
            .map(str::to_string),
        "glob" | "grep" => {
            let pattern = input.get("pattern").and_then(Value::as_str).unwrap_or("");
            let path = input.get("path").and_then(Value::as_str);
            Some(match path {
                Some(p) => format!("{pattern} in {p}"),
                None => pattern.to_string(),
            })
        }
        "webfetch" | "websearch" => input
            .get("url")
            .or_else(|| input.get("query"))
            .and_then(Value::as_str)
            .map(str::to_string),
        "task" | "agent" => input
            .get("description")
            .or_else(|| input.get("prompt"))
            .and_then(Value::as_str)
            .map(str::to_string),
        "skill" => invoked_skill(input).map(|s| format!("skill {s}")),
        _ => tool_path(input),
    };
    let text = text.unwrap_or_else(|| {
        let raw = serde_json::to_string(input).unwrap_or_default();
        if raw == "{}" || raw == "null" {
            String::new()
        } else {
            raw
        }
    });
    first_chars(&text, SUMMARY_CHARS)
}

fn u64_at(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(Value::as_u64).unwrap_or(0)
}

/// Split `total` across buckets in proportion to `weights` so the parts sum to `total`
/// exactly (largest-remainder rounding).
fn largest_remainder<const N: usize>(total: u64, weights: [u64; N]) -> [u64; N] {
    let sum: u64 = weights.iter().sum();
    let mut out = [0u64; N];
    if sum == 0 {
        return out;
    }
    let mut remainders = [0f64; N];
    let mut assigned = 0u64;
    for i in 0..N {
        let exact = total as f64 * weights[i] as f64 / sum as f64;
        out[i] = exact.floor() as u64;
        remainders[i] = exact - out[i] as f64;
        assigned += out[i];
    }
    let mut left = total.saturating_sub(assigned);
    while left > 0 {
        let (i, _) =
            remainders.iter().enumerate().fold(
                (0, -1.0),
                |best, (i, r)| if *r > best.1 { (i, *r) } else { best },
            );
        out[i] += 1;
        remainders[i] = -1.0;
        left -= 1;
    }
    out
}

/// Characters of a tool result's content (a string, or text blocks).
fn result_chars(content: &Value) -> (u64, String) {
    match content {
        Value::String(s) => (s.chars().count() as u64, s.clone()),
        Value::Array(items) => {
            let mut chars = 0u64;
            let mut first = String::new();
            for item in items {
                if let Some(t) = item.get("text").and_then(Value::as_str) {
                    chars += t.chars().count() as u64;
                    if first.is_empty() {
                        first = t.to_string();
                    }
                }
            }
            (chars, first)
        }
        Value::Null => (0, String::new()),
        other => {
            let s = other.to_string();
            (s.chars().count() as u64, s)
        }
    }
}

impl TraceBuilder {
    /// A builder for `dialect`, relativising paths against `cwd` and counting reads under
    /// `skill_marker` (the sandbox-relative directory the skill was installed in) as skill reads.
    pub fn new(dialect: Dialect, cwd: &std::path::Path, skill_marker: Option<&str>) -> Self {
        Self {
            dialect,
            cwd: normalize_path(&cwd.to_string_lossy()),
            skill_marker: skill_marker.map(normalize_path),
            trace: ExecutionTrace::default(),
            messages: HashSet::new(),
            usage_by_message: HashMap::new(),
            anonymous_usage: Usage::default(),
            result_usage: None,
            open_tools: HashMap::new(),
            last_tool: None,
            files_read: BTreeSet::new(),
            files_written: BTreeSet::new(),
            tool_time_ms: 0,
            timed: false,
            model_time_ms: None,
        }
    }

    /// Make a path sandbox-relative when it is inside the sandbox.
    fn relative(&self, path: &str) -> String {
        let p = normalize_path(path);
        if !self.cwd.is_empty() {
            if let Some(rest) = p.strip_prefix(&self.cwd) {
                // `/sb/x` is inside `/sb`; `/sb10/x` is a sibling and stays absolute.
                if let Some(inside) = rest.strip_prefix('/') {
                    let inside = inside.trim_start_matches('/');
                    if !inside.is_empty() {
                        return inside.to_string();
                    }
                }
            }
        }
        p
    }

    /// Is `text` a path inside the installed skill directory (`<marker>/…`), or the directory
    /// itself? A sibling skill whose name merely starts with the marker does not count.
    fn is_skill_path(&self, text: &str) -> bool {
        let Some(marker) = self.skill_marker.as_deref() else {
            return false;
        };
        let text = normalize_path(text);
        text.contains(&format!("{marker}/")) || text.ends_with(marker)
    }

    /// The installed skill's name: the last component of the marker.
    fn skill_name(&self) -> Option<&str> {
        self.skill_marker
            .as_deref()
            .and_then(|m| m.rsplit('/').next())
    }

    fn push_step(&mut self, step: TraceStep) {
        if self.trace.steps.len() < STEP_CAP {
            self.trace.steps.push(step);
        } else {
            self.trace.steps_truncated += 1;
        }
    }

    /// Count a model turn.
    pub fn turn(&mut self) {
        self.trace.turns += 1;
    }

    /// Record a thinking block.
    pub fn thinking(&mut self, text: &str, at_ms: Option<u64>) {
        let chars = text.chars().count() as u64;
        self.trace.thinking_blocks += 1;
        self.trace.thinking_chars += chars;
        self.push_step(TraceStep {
            at_ms,
            kind: TraceStepKind::Thinking,
            tool: None,
            category: None,
            summary: first_chars(text, SUMMARY_CHARS),
            chars,
            is_error: false,
        });
    }

    /// Record a redacted thinking block (its length is unknown).
    pub fn redacted_thinking(&mut self, at_ms: Option<u64>) {
        self.trace.thinking_blocks += 1;
        self.push_step(TraceStep {
            at_ms,
            kind: TraceStepKind::Thinking,
            tool: None,
            category: None,
            summary: "(redacted thinking)".into(),
            chars: 0,
            is_error: false,
        });
    }

    /// Record visible assistant text.
    pub fn text(&mut self, text: &str, at_ms: Option<u64>) {
        let chars = text.chars().count() as u64;
        self.trace.text_blocks += 1;
        self.trace.text_chars += chars;
        self.push_step(TraceStep {
            at_ms,
            kind: TraceStepKind::Text,
            tool: None,
            category: None,
            summary: first_chars(text, SUMMARY_CHARS),
            chars,
            is_error: false,
        });
    }

    /// Record a tool call. `path` is the file it reads or writes, when known; `summary` is the
    /// one-line description; `id` lets a later result close it.
    #[allow(clippy::too_many_arguments)]
    pub fn tool_use(
        &mut self,
        id: Option<&str>,
        name: &str,
        category: ToolCategory,
        summary: &str,
        path: Option<&str>,
        input_chars: u64,
        at_ms: Option<u64>,
    ) {
        self.trace.tool_calls += 1;
        *self.trace.tools.entry(name.to_string()).or_default() += 1;
        self.trace.tool_input_chars += input_chars;
        match category {
            ToolCategory::Read => self.trace.reads += 1,
            ToolCategory::Search => self.trace.searches += 1,
            ToolCategory::Write => self.trace.writes += 1,
            ToolCategory::Command => self.trace.commands += 1,
            ToolCategory::Web => self.trace.web += 1,
            ToolCategory::Subagent => self.trace.subagents += 1,
            ToolCategory::Other => self.trace.other_tools += 1,
        }
        if let Some(p) = path {
            let rel = self.relative(p);
            match category {
                ToolCategory::Read => {
                    self.files_read.insert(rel);
                }
                ToolCategory::Write => {
                    self.files_written.insert(rel);
                }
                _ => {}
            }
        }
        if category == ToolCategory::Read
            && (path.is_some_and(|p| self.is_skill_path(p)) || self.is_skill_path(summary))
        {
            self.trace.skill_reads += 1;
        }
        if self.trace.first_tool_ms.is_none() {
            self.trace.first_tool_ms = at_ms;
        }
        if at_ms.is_some() {
            self.timed = true;
        }
        let open = OpenTool {
            name: name.to_string(),
            at_ms,
        };
        match id {
            Some(id) => {
                self.open_tools.insert(id.to_string(), open);
            }
            None => self.last_tool = Some(open),
        }
        self.push_step(TraceStep {
            at_ms,
            kind: TraceStepKind::ToolUse,
            tool: Some(name.to_string()),
            category: Some(category),
            summary: first_chars(summary, SUMMARY_CHARS),
            chars: input_chars,
            is_error: false,
        });
    }

    /// Record an event of a subagent (Claude Code forwards them with `parent_tool_use_id`).
    /// They count in `subagent_turns` / `subagent_tool_calls` and appear in the timeline,
    /// but not in the headline counters, the files, or the tool time: the parent's own
    /// `Task` call already spans them.
    fn nested_step(
        &mut self,
        kind: TraceStepKind,
        tool: Option<&str>,
        summary: &str,
        chars: u64,
        is_error: bool,
        at_ms: Option<u64>,
    ) {
        self.push_step(TraceStep {
            at_ms,
            kind,
            tool: tool.map(str::to_string),
            category: None,
            summary: first_chars(&format!("↳ {summary}"), SUMMARY_CHARS),
            chars,
            is_error,
        });
    }

    /// Mark that the skill was in the model's context without a tool call (API providers put
    /// `SKILL.md` in the system prompt).
    pub fn set_skill_in_context(&mut self, in_context: bool) {
        self.trace.skill_in_context = in_context;
    }

    /// Record a tool result. With an `id` it closes that call; without one it closes the last
    /// call recorded without an id.
    pub fn tool_result(
        &mut self,
        id: Option<&str>,
        chars: u64,
        excerpt: &str,
        is_error: bool,
        at_ms: Option<u64>,
    ) {
        let open = match id {
            Some(id) => self.open_tools.remove(id),
            None => self.last_tool.take(),
        };
        if let (Some(open), Some(at)) = (&open, at_ms) {
            if let Some(started) = open.at_ms {
                self.tool_time_ms += at.saturating_sub(started);
            }
        }
        self.trace.tool_result_chars += chars;
        if is_error {
            self.trace.tool_errors += 1;
        }
        self.push_step(TraceStep {
            at_ms,
            kind: TraceStepKind::ToolResult,
            tool: open.map(|o| o.name),
            category: None,
            summary: first_chars(excerpt, SUMMARY_CHARS),
            chars,
            is_error,
        });
    }

    /// Record time the provider reports as spent waiting for the model.
    pub fn set_model_time(&mut self, ms: u64) {
        self.model_time_ms = Some(ms);
    }

    /// Absorb one event of the builder's dialect.
    pub fn absorb(&mut self, event: &Value, at_ms: Option<u64>) {
        match self.dialect {
            Dialect::ClaudeCode => self.absorb_claude(event, at_ms),
            Dialect::Codex => self.absorb_codex(event, at_ms),
        }
    }

    /// Absorb one JSONL line (non-JSON lines are ignored).
    pub fn absorb_line(&mut self, line: &str, at_ms: Option<u64>) {
        if let Ok(v) = serde_json::from_str::<Value>(line.trim()) {
            self.absorb(&v, at_ms);
        }
    }

    /// Events of a subagent: counted apart, shown in the timeline, otherwise left alone.
    fn absorb_nested_claude(&mut self, event: &Value, at_ms: Option<u64>) {
        let message = &event["message"];
        let Some(blocks) = message.get("content").and_then(Value::as_array) else {
            return;
        };
        match event.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                let counted = match message.get("id").and_then(Value::as_str) {
                    Some(id) => self.messages.insert(format!("nested:{id}")),
                    None => true,
                };
                if counted {
                    self.trace.subagent_turns += 1;
                }
                for block in blocks {
                    match block.get("type").and_then(Value::as_str) {
                        Some("thinking") => {
                            let t = block.get("thinking").and_then(Value::as_str).unwrap_or("");
                            self.nested_step(
                                TraceStepKind::Thinking,
                                None,
                                t,
                                t.chars().count() as u64,
                                false,
                                at_ms,
                            );
                        }
                        Some("text") => {
                            let t = block.get("text").and_then(Value::as_str).unwrap_or("");
                            self.nested_step(
                                TraceStepKind::Text,
                                None,
                                t,
                                t.chars().count() as u64,
                                false,
                                at_ms,
                            );
                        }
                        Some("tool_use") => {
                            self.trace.subagent_tool_calls += 1;
                            let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                            let input = block.get("input").cloned().unwrap_or(Value::Null);
                            let summary = describe_tool(name, &input);
                            let chars = serde_json::to_string(&input)
                                .map(|s| s.chars().count() as u64)
                                .unwrap_or(0);
                            self.nested_step(
                                TraceStepKind::ToolUse,
                                Some(name),
                                &summary,
                                chars,
                                false,
                                at_ms,
                            );
                        }
                        _ => {}
                    }
                }
            }
            Some("user") => {
                for block in blocks {
                    if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let (chars, excerpt) =
                        result_chars(block.get("content").unwrap_or(&Value::Null));
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    self.nested_step(
                        TraceStepKind::ToolResult,
                        None,
                        &excerpt,
                        chars,
                        is_error,
                        at_ms,
                    );
                }
            }
            _ => {}
        }
    }

    fn absorb_claude(&mut self, event: &Value, at_ms: Option<u64>) {
        if event
            .get("parent_tool_use_id")
            .is_some_and(|p| !p.is_null())
        {
            self.absorb_nested_claude(event, at_ms);
            return;
        }
        match event.get("type").and_then(Value::as_str) {
            Some("assistant") => {
                let message = &event["message"];
                let id = message
                    .get("id")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                let usage = message.get("usage").map(|u| Usage {
                    uncached: u64_at(u, "input_tokens"),
                    cache_creation: u64_at(u, "cache_creation_input_tokens"),
                    cache_read: u64_at(u, "cache_read_input_tokens"),
                });
                match id {
                    Some(id) => {
                        if self.messages.insert(id.clone()) {
                            self.turn();
                        }
                        if let Some(u) = usage {
                            self.usage_by_message.insert(id, u);
                        }
                    }
                    None => {
                        self.turn();
                        if let Some(u) = usage {
                            self.anonymous_usage.uncached += u.uncached;
                            self.anonymous_usage.cache_creation += u.cache_creation;
                            self.anonymous_usage.cache_read += u.cache_read;
                        }
                    }
                }
                let Some(blocks) = message.get("content").and_then(Value::as_array) else {
                    return;
                };
                for block in blocks {
                    match block.get("type").and_then(Value::as_str) {
                        Some("thinking") => {
                            let t = block.get("thinking").and_then(Value::as_str).unwrap_or("");
                            self.thinking(t, at_ms);
                        }
                        Some("redacted_thinking") => self.redacted_thinking(at_ms),
                        Some("text") => {
                            let t = block.get("text").and_then(Value::as_str).unwrap_or("");
                            self.text(t, at_ms);
                        }
                        Some("tool_use") => {
                            let name = block.get("name").and_then(Value::as_str).unwrap_or("tool");
                            let input = block.get("input").cloned().unwrap_or(Value::Null);
                            let mut category = categorize_tool(name, &input);
                            let path = if name.eq_ignore_ascii_case("skill") {
                                // The Skill tool opens the installed skill when it names it;
                                // any other skill (a user-level one) is just another tool.
                                let installed = invoked_skill(&input).is_some_and(|s| {
                                    self.skill_name()
                                        .is_some_and(|n| n.eq_ignore_ascii_case(&s))
                                });
                                if installed {
                                    self.skill_marker.as_ref().map(|m| format!("{m}/SKILL.md"))
                                } else {
                                    category = ToolCategory::Other;
                                    None
                                }
                            } else {
                                match tool_path(&input) {
                                    Some(p) => Some(p),
                                    None if category == ToolCategory::Read => command_read_file(
                                        input.get("command").and_then(Value::as_str).unwrap_or(""),
                                    ),
                                    None => None,
                                }
                            };
                            let input_chars = if input.is_null() {
                                0
                            } else {
                                serde_json::to_string(&input)
                                    .map(|s| s.chars().count() as u64)
                                    .unwrap_or(0)
                            };
                            let summary = describe_tool(name, &input);
                            self.tool_use(
                                block.get("id").and_then(Value::as_str),
                                name,
                                category,
                                &summary,
                                path.as_deref(),
                                input_chars,
                                at_ms,
                            );
                        }
                        _ => {}
                    }
                }
            }
            Some("user") => {
                let Some(blocks) = event["message"].get("content").and_then(Value::as_array) else {
                    return;
                };
                for block in blocks {
                    if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let (chars, excerpt) =
                        result_chars(block.get("content").unwrap_or(&Value::Null));
                    let is_error = block
                        .get("is_error")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    self.tool_result(
                        block.get("tool_use_id").and_then(Value::as_str),
                        chars,
                        &excerpt,
                        is_error,
                        at_ms,
                    );
                }
            }
            Some("result") => {
                if let Some(u) = event.get("usage") {
                    self.result_usage = Some(Usage {
                        uncached: u64_at(u, "input_tokens"),
                        cache_creation: u64_at(u, "cache_creation_input_tokens"),
                        cache_read: u64_at(u, "cache_read_input_tokens"),
                    });
                }
                if let Some(ms) = event.get("duration_api_ms").and_then(Value::as_u64) {
                    self.model_time_ms = Some(ms);
                }
                if self.trace.turns == 0 {
                    if let Some(n) = event.get("num_turns").and_then(Value::as_u64) {
                        self.trace.turns = n as u32;
                    }
                }
            }
            _ => {}
        }
    }

    fn codex_usage(&mut self, usage: &Value) {
        // Codex reports cached tokens as a subset of the input tokens (OpenAI API semantics)
        // and cache writes separately.
        let input = u64_at(usage, "input_tokens");
        let cached = u64_at(usage, "cached_input_tokens");
        self.anonymous_usage.cache_read += cached;
        self.anonymous_usage.uncached += input.saturating_sub(cached);
        self.anonymous_usage.cache_creation += u64_at(usage, "cache_write_input_tokens");
    }

    fn codex_command(
        &mut self,
        id: Option<&str>,
        command: &str,
        output: Option<&Value>,
        exit_code: Option<i64>,
        at_ms: Option<u64>,
        started_at: Option<u64>,
    ) {
        let category = classify_command(command);
        let path = if category == ToolCategory::Read {
            command_read_file(command)
        } else {
            None
        };
        self.turn();
        self.tool_use(
            id,
            "command_execution",
            category,
            &first_chars(command, SUMMARY_CHARS),
            path.as_deref(),
            command.chars().count() as u64,
            started_at.or(at_ms),
        );
        let (chars, excerpt) = output.map(result_chars).unwrap_or((0, String::new()));
        self.tool_result(
            id,
            chars,
            &excerpt,
            exit_code.is_some_and(|c| c != 0),
            at_ms,
        );
    }

    fn codex_file_change(&mut self, id: Option<&str>, item: &Value, at_ms: Option<u64>) {
        let paths: Vec<String> = match item.get("changes") {
            Some(Value::Array(changes)) => changes
                .iter()
                .filter_map(|c| c.get("path").and_then(Value::as_str))
                .map(str::to_string)
                .collect(),
            Some(Value::Object(map)) => map.keys().cloned().collect(),
            _ => Vec::new(),
        };
        let input_chars = serde_json::to_string(item)
            .map(|s| s.chars().count() as u64)
            .unwrap_or(0);
        self.turn();
        if paths.is_empty() {
            self.tool_use(
                id,
                "file_change",
                ToolCategory::Write,
                "",
                None,
                input_chars,
                at_ms,
            );
        } else {
            let summary = first_chars(&paths.join(", "), SUMMARY_CHARS);
            self.tool_use(
                id,
                "file_change",
                ToolCategory::Write,
                &summary,
                Some(&paths[0]),
                input_chars,
                at_ms,
            );
            for p in &paths[1..] {
                let rel = self.relative(p);
                self.files_written.insert(rel);
            }
        }
        let failed = item
            .get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| s == "failed");
        self.tool_result(id, 0, "", failed, at_ms);
    }

    fn absorb_codex(&mut self, event: &Value, at_ms: Option<u64>) {
        if let Some(kind) = event.get("type").and_then(Value::as_str) {
            match kind {
                "item.started" => {
                    let item = &event["item"];
                    if let Some(id) = item.get("id").and_then(Value::as_str) {
                        self.open_tools.insert(
                            format!("start:{id}"),
                            OpenTool {
                                name: item
                                    .get("type")
                                    .and_then(Value::as_str)
                                    .unwrap_or("item")
                                    .to_string(),
                                at_ms,
                            },
                        );
                    }
                }
                "item.completed" => {
                    let item = &event["item"];
                    let id = item.get("id").and_then(Value::as_str).map(str::to_string);
                    let started_at = id
                        .as_deref()
                        .and_then(|id| self.open_tools.remove(&format!("start:{id}")))
                        .and_then(|o| o.at_ms);
                    match item.get("type").and_then(Value::as_str) {
                        Some("agent_message") => {
                            self.turn();
                            let t = item.get("text").and_then(Value::as_str).unwrap_or("");
                            self.text(t, at_ms);
                        }
                        Some("reasoning") => {
                            let t = item.get("text").and_then(Value::as_str).unwrap_or("");
                            self.thinking(t, at_ms);
                        }
                        Some("command_execution") => {
                            let command = item.get("command").and_then(Value::as_str).unwrap_or("");
                            self.codex_command(
                                id.as_deref(),
                                command,
                                item.get("aggregated_output"),
                                item.get("exit_code").and_then(Value::as_i64),
                                at_ms,
                                started_at,
                            );
                        }
                        Some("file_change" | "patch_apply") => {
                            self.codex_file_change(id.as_deref(), item, at_ms);
                        }
                        Some("web_search") => {
                            self.turn();
                            let q = item.get("query").and_then(Value::as_str).unwrap_or("");
                            self.tool_use(
                                id.as_deref(),
                                "web_search",
                                ToolCategory::Web,
                                &first_chars(q, SUMMARY_CHARS),
                                None,
                                q.chars().count() as u64,
                                started_at.or(at_ms),
                            );
                            self.tool_result(id.as_deref(), 0, "", false, at_ms);
                        }
                        Some(t @ ("mcp_tool_call" | "tool_call")) => {
                            self.turn();
                            let name = match (
                                item.get("server").and_then(Value::as_str),
                                item.get("tool")
                                    .or_else(|| item.get("name"))
                                    .and_then(Value::as_str),
                            ) {
                                (Some(s), Some(n)) => format!("{s}.{n}"),
                                (None, Some(n)) => n.to_string(),
                                _ => t.to_string(),
                            };
                            let args = item
                                .get("arguments")
                                .or_else(|| item.get("input"))
                                .map(|a| a.to_string())
                                .unwrap_or_default();
                            self.tool_use(
                                id.as_deref(),
                                &name,
                                ToolCategory::Other,
                                &first_chars(&args, SUMMARY_CHARS),
                                None,
                                args.chars().count() as u64,
                                started_at.or(at_ms),
                            );
                            let (chars, excerpt) = item
                                .get("result")
                                .or_else(|| item.get("output"))
                                .map(result_chars)
                                .unwrap_or((0, String::new()));
                            let failed = item
                                .get("status")
                                .and_then(Value::as_str)
                                .is_some_and(|s| s == "failed")
                                || item.get("error").is_some_and(|e| !e.is_null());
                            self.tool_result(id.as_deref(), chars, &excerpt, failed, at_ms);
                        }
                        _ => {}
                    }
                }
                "turn.completed" => {
                    if let Some(u) = event.get("usage") {
                        self.codex_usage(u);
                    }
                }
                _ => {}
            }
            return;
        }
        let Some(msg) = event.get("msg") else {
            return;
        };
        let call_id = msg
            .get("call_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        match msg.get("type").and_then(Value::as_str) {
            Some("agent_message") => {
                self.turn();
                let t = msg.get("message").and_then(Value::as_str).unwrap_or("");
                self.text(t, at_ms);
            }
            Some("agent_reasoning") => {
                let t = msg.get("text").and_then(Value::as_str).unwrap_or("");
                self.thinking(t, at_ms);
            }
            Some("exec_command_begin") => {
                let command = match msg.get("command") {
                    Some(Value::Array(parts)) => parts
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(" "),
                    Some(Value::String(s)) => s.clone(),
                    _ => String::new(),
                };
                let category = classify_command(&command);
                let path = if category == ToolCategory::Read {
                    command_read_file(&command)
                } else {
                    None
                };
                self.turn();
                self.tool_use(
                    call_id.as_deref(),
                    "command_execution",
                    category,
                    &first_chars(&command, SUMMARY_CHARS),
                    path.as_deref(),
                    command.chars().count() as u64,
                    at_ms,
                );
            }
            Some("exec_command_end") => {
                let (chars, excerpt) = msg
                    .get("aggregated_output")
                    .or_else(|| msg.get("stdout"))
                    .map(result_chars)
                    .unwrap_or((0, String::new()));
                let failed = msg
                    .get("exit_code")
                    .and_then(Value::as_i64)
                    .is_some_and(|c| c != 0);
                self.tool_result(call_id.as_deref(), chars, &excerpt, failed, at_ms);
            }
            Some("patch_apply_begin") => {
                let paths: Vec<String> = match msg.get("changes") {
                    Some(Value::Object(map)) => map.keys().cloned().collect(),
                    Some(Value::Array(items)) => items
                        .iter()
                        .filter_map(|c| c.get("path").and_then(Value::as_str))
                        .map(str::to_string)
                        .collect(),
                    _ => Vec::new(),
                };
                let input_chars = msg
                    .get("changes")
                    .map(|c| c.to_string().chars().count() as u64)
                    .unwrap_or(0);
                self.turn();
                let summary = first_chars(&paths.join(", "), SUMMARY_CHARS);
                self.tool_use(
                    call_id.as_deref(),
                    "patch_apply",
                    ToolCategory::Write,
                    &summary,
                    paths.first().map(String::as_str),
                    input_chars,
                    at_ms,
                );
                for p in paths.iter().skip(1) {
                    let rel = self.relative(p);
                    self.files_written.insert(rel);
                }
            }
            Some("patch_apply_end") => {
                let failed = !msg.get("success").and_then(Value::as_bool).unwrap_or(true);
                self.tool_result(call_id.as_deref(), 0, "", failed, at_ms);
            }
            Some("mcp_tool_call_begin") => {
                let inv = msg.get("invocation").unwrap_or(msg);
                let name = match (
                    inv.get("server").and_then(Value::as_str),
                    inv.get("tool").and_then(Value::as_str),
                ) {
                    (Some(s), Some(t)) => format!("{s}.{t}"),
                    (_, Some(t)) => t.to_string(),
                    _ => "mcp_tool_call".to_string(),
                };
                let args = inv
                    .get("arguments")
                    .map(|a| a.to_string())
                    .unwrap_or_default();
                self.turn();
                self.tool_use(
                    call_id.as_deref(),
                    &name,
                    ToolCategory::Other,
                    &first_chars(&args, SUMMARY_CHARS),
                    None,
                    args.chars().count() as u64,
                    at_ms,
                );
            }
            Some("mcp_tool_call_end") => {
                let (chars, excerpt) = msg
                    .get("result")
                    .map(result_chars)
                    .unwrap_or((0, String::new()));
                let failed = msg.get("result").and_then(|r| r.get("Err")).is_some();
                self.tool_result(call_id.as_deref(), chars, &excerpt, failed, at_ms);
            }
            Some("web_search_begin") => {
                self.turn();
                let q = msg.get("query").and_then(Value::as_str).unwrap_or("");
                self.tool_use(
                    call_id.as_deref(),
                    "web_search",
                    ToolCategory::Web,
                    &first_chars(q, SUMMARY_CHARS),
                    None,
                    q.chars().count() as u64,
                    at_ms,
                );
            }
            Some("web_search_end") => {
                self.tool_result(call_id.as_deref(), 0, "", false, at_ms);
            }
            Some("custom_tool_call_begin") => {
                let name = msg
                    .get("tool_name")
                    .or_else(|| msg.get("name"))
                    .and_then(Value::as_str)
                    .unwrap_or("custom_tool");
                let args = msg
                    .get("parameters")
                    .or_else(|| msg.get("input"))
                    .map(|a| a.to_string())
                    .unwrap_or_default();
                self.turn();
                self.tool_use(
                    call_id.as_deref(),
                    name,
                    ToolCategory::Other,
                    &first_chars(&args, SUMMARY_CHARS),
                    None,
                    args.chars().count() as u64,
                    at_ms,
                );
            }
            Some("custom_tool_call_end") => {
                let (chars, excerpt) = msg
                    .get("result")
                    .map(result_chars)
                    .unwrap_or((0, String::new()));
                self.tool_result(call_id.as_deref(), chars, &excerpt, false, at_ms);
            }
            Some("token_count") => {
                // token_count is cumulative: keep the latest reading. Rate-limit-only updates
                // carry no usage and leave the last reading alone.
                let usage = match msg.get("info").and_then(|i| i.get("total_token_usage")) {
                    Some(u) if !u.is_null() => u,
                    _ if msg.get("input_tokens").is_some() => msg,
                    _ => return,
                };
                self.anonymous_usage = Usage::default();
                self.codex_usage(usage);
            }
            _ => {}
        }
    }

    /// Finish the trace. `output_tokens` is the total the provider reported (0 when unknown);
    /// it is split across thinking, text and tool arguments in proportion to their characters.
    pub fn finish(mut self, output_tokens: u64) -> ExecutionTrace {
        let mut t = std::mem::take(&mut self.trace);
        let total_chars = t.thinking_chars + t.text_chars + t.tool_input_chars;
        if output_tokens > 0 && total_chars > 0 {
            let split = largest_remainder(
                output_tokens,
                [t.thinking_chars, t.text_chars, t.tool_input_chars],
            );
            t.thinking_tokens = split[0];
            t.text_tokens = split[1];
            t.tool_input_tokens = split[2];
        } else {
            t.thinking_tokens = ceil_div4(t.thinking_chars);
            t.text_tokens = ceil_div4(t.text_chars);
            t.tool_input_tokens = ceil_div4(t.tool_input_chars);
        }
        t.tool_result_tokens = ceil_div4(t.tool_result_chars);
        let io = t.tool_input_tokens + t.tool_result_tokens;
        let all = io + t.thinking_tokens + t.text_tokens;
        t.io_share = if all > 0 { io as f64 / all as f64 } else { 0.0 };
        let usage = match self.result_usage {
            Some(u) => u,
            None => {
                let mut u = self.anonymous_usage;
                for m in self.usage_by_message.values() {
                    u.uncached += m.uncached;
                    u.cache_creation += m.cache_creation;
                    u.cache_read += m.cache_read;
                }
                u
            }
        };
        t.cache_read_tokens = usage.cache_read;
        t.cache_creation_tokens = usage.cache_creation;
        t.uncached_input_tokens = usage.uncached;
        t.model_time_ms = self.model_time_ms;
        t.tool_time_ms = self.timed.then_some(self.tool_time_ms);
        t.files_read_count = self.files_read.len() as u32;
        t.files_written_count = self.files_written.len() as u32;
        t.files_read = self.files_read.into_iter().take(FILE_LIST_CAP).collect();
        t.files_written = self.files_written.into_iter().take(FILE_LIST_CAP).collect();
        t
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn command_classification() {
        assert_eq!(classify_command("cat README.md"), ToolCategory::Read);
        assert_eq!(
            classify_command("head -n 20 ./src/main.rs | grep fn"),
            ToolCategory::Read
        );
        assert_eq!(classify_command("cd src && ls -la"), ToolCategory::Search);
        assert_eq!(
            classify_command("find . -name '*.py'"),
            ToolCategory::Search
        );
        assert_eq!(classify_command("rg -n TODO src/"), ToolCategory::Search);
        assert_eq!(classify_command("git status"), ToolCategory::Search);
        assert_eq!(classify_command("git commit -m x"), ToolCategory::Command);
        assert_eq!(classify_command("echo hi > out.txt"), ToolCategory::Write);
        assert_eq!(classify_command("mkdir -p out"), ToolCategory::Write);
        assert_eq!(
            classify_command("sed -i 's/a/b/' f.txt"),
            ToolCategory::Write
        );
        assert_eq!(classify_command("sed -n '1,5p' f.txt"), ToolCategory::Read);
        assert_eq!(
            classify_command("python3 run.py 2>&1"),
            ToolCategory::Command
        );
        assert_eq!(classify_command("FOO=1 npm test"), ToolCategory::Command);
        assert_eq!(classify_command("curl https://x.y"), ToolCategory::Web);
        assert_eq!(classify_command(""), ToolCategory::Command);
        assert_eq!(
            command_file(&command_words("head -n 5 'notes.txt'")),
            Some("notes.txt".into())
        );
        assert_eq!(command_file(&command_words("ls -la")), None);
    }

    #[test]
    fn tool_categories_by_name() {
        let v = serde_json::json!({});
        assert_eq!(categorize_tool("Read", &v), ToolCategory::Read);
        assert_eq!(categorize_tool("Grep", &v), ToolCategory::Search);
        assert_eq!(categorize_tool("Glob", &v), ToolCategory::Search);
        assert_eq!(categorize_tool("Write", &v), ToolCategory::Write);
        assert_eq!(categorize_tool("MultiEdit", &v), ToolCategory::Write);
        assert_eq!(categorize_tool("WebFetch", &v), ToolCategory::Web);
        assert_eq!(categorize_tool("Task", &v), ToolCategory::Subagent);
        assert_eq!(categorize_tool("TodoWrite", &v), ToolCategory::Other);
        assert_eq!(
            categorize_tool("mcp__github__get_me", &v),
            ToolCategory::Other
        );
        assert_eq!(
            categorize_tool("Bash", &serde_json::json!({"command": "cat x"})),
            ToolCategory::Read
        );
        assert_eq!(
            categorize_tool("Bash", &serde_json::json!({"command": "cargo test"})),
            ToolCategory::Command
        );
    }

    const CLAUDE_STREAM: &[&str] = &[
        r##"{"type":"system","subtype":"init","model":"claude-sonnet-4-5"}"##,
        r##"{"type":"assistant","message":{"id":"m1","content":[{"type":"thinking","thinking":"I should read the skill first, then the notes."}],"usage":{"input_tokens":10,"cache_creation_input_tokens":2000,"cache_read_input_tokens":0,"output_tokens":120}}}"##,
        r##"{"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Read","input":{"file_path":"/sb/.claude/skills/report-writer/SKILL.md"}}],"usage":{"input_tokens":10,"cache_creation_input_tokens":2000,"cache_read_input_tokens":0,"output_tokens":120}}}"##,
        r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"# Report writer\nWrite a markdown file with a heading."}]}}"##,
        r##"{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t2","name":"Bash","input":{"command":"cat notes.txt"}},{"type":"tool_use","id":"t3","name":"Grep","input":{"pattern":"shipped","path":"/sb"}}],"usage":{"input_tokens":5,"cache_creation_input_tokens":100,"cache_read_input_tokens":2000,"output_tokens":60}}}"##,
        r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t2","content":[{"type":"text","text":"shipped the thing"}]},{"type":"tool_result","tool_use_id":"t3","content":"notes.txt:1:shipped the thing","is_error":false}]}}"##,
        r##"{"type":"assistant","message":{"id":"m3","content":[{"type":"tool_use","id":"t4","name":"Write","input":{"file_path":"/sb/report.md","content":"# Status\n\nThe project status is green.\n"}}],"usage":{"input_tokens":5,"cache_creation_input_tokens":50,"cache_read_input_tokens":2100,"output_tokens":90}}}"##,
        r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t4","content":"File created successfully","is_error":false}]}}"##,
        r##"{"type":"assistant","message":{"id":"m4","content":[{"type":"tool_use","id":"t5","name":"Bash","input":{"command":"python3 check.py"}}],"usage":{"input_tokens":5,"cache_creation_input_tokens":20,"cache_read_input_tokens":2150,"output_tokens":30}}}"##,
        r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t5","content":"Traceback: No such file","is_error":true}]}}"##,
        r##"{"type":"assistant","message":{"id":"m5","content":[{"type":"text","text":"Wrote report.md as the skill instructs."}],"usage":{"input_tokens":5,"cache_creation_input_tokens":30,"cache_read_input_tokens":2170,"output_tokens":20}}}"##,
        r##"{"type":"result","subtype":"success","is_error":false,"duration_ms":9000,"duration_api_ms":6000,"num_turns":9,"result":"Wrote report.md as the skill instructs.","usage":{"input_tokens":35,"cache_creation_input_tokens":2200,"cache_read_input_tokens":8420,"output_tokens":320}}"##,
    ];

    #[test]
    fn claude_stream_builds_a_trace() {
        let mut b = TraceBuilder::new(
            Dialect::ClaudeCode,
            Path::new("/sb"),
            Some(".claude/skills/report-writer"),
        );
        for (i, line) in CLAUDE_STREAM.iter().enumerate() {
            b.absorb_line(line, Some(i as u64 * 1000));
        }
        let t = b.finish(320);
        assert_eq!(t.turns, 5, "one turn per distinct message id");
        assert_eq!(t.tool_calls, 5);
        assert_eq!((t.reads, t.searches, t.writes, t.commands), (2, 1, 1, 1));
        assert_eq!(t.tools["Bash"], 2);
        assert_eq!(t.tools["Read"], 1);
        assert_eq!(t.skill_reads, 1);
        assert_eq!(
            t.files_read,
            vec![".claude/skills/report-writer/SKILL.md", "notes.txt"]
        );
        assert_eq!(t.files_written, vec!["report.md"]);
        assert_eq!((t.files_read_count, t.files_written_count), (2, 1));
        assert_eq!(t.tool_errors, 1);
        assert_eq!(t.thinking_blocks, 1);
        assert_eq!(t.text_blocks, 1);
        assert!(t.thinking_chars > 0 && t.text_chars > 0 && t.tool_input_chars > 0);
        assert!(t.tool_result_chars > 0);
        assert_eq!(t.thinking_tokens + t.text_tokens + t.tool_input_tokens, 320);
        assert!(t.io_share > 0.0 && t.io_share < 1.0);
        assert_eq!(
            (
                t.cache_read_tokens,
                t.cache_creation_tokens,
                t.uncached_input_tokens
            ),
            (8420, 2200, 35)
        );
        assert_eq!(t.model_time_ms, Some(6000));
        assert_eq!(t.first_tool_ms, Some(2000));
        // t1 2000→3000, t2 and t3 4000→5000 each, t4 6000→7000, t5 8000→9000
        assert_eq!(t.tool_time_ms, Some(5000));
        assert_eq!(t.steps.len(), 12);
        assert_eq!(t.steps[0].kind, TraceStepKind::Thinking);
        assert_eq!(t.steps[1].tool.as_deref(), Some("Read"));
        assert_eq!(t.steps[1].category, Some(ToolCategory::Read));
        assert_eq!(t.steps[2].kind, TraceStepKind::ToolResult);
        assert_eq!(t.steps[2].tool.as_deref(), Some("Read"));
        assert!(
            t.steps
                .iter()
                .any(|s| s.is_error && s.tool.as_deref() == Some("Bash"))
        );
        assert_eq!(t.steps_truncated, 0);
    }

    #[test]
    fn claude_stream_without_result_sums_message_usage() {
        let mut b = TraceBuilder::new(Dialect::ClaudeCode, Path::new("/sb"), None);
        for line in CLAUDE_STREAM.iter().take(CLAUDE_STREAM.len() - 1) {
            b.absorb_line(line, None);
        }
        let t = b.finish(0);
        assert_eq!(t.turns, 5);
        assert_eq!(t.cache_read_tokens, 2000 + 2100 + 2150 + 2170);
        assert_eq!(t.cache_creation_tokens, 2000 + 100 + 50 + 20 + 30);
        assert_eq!(t.tool_time_ms, None);
        assert_eq!(t.model_time_ms, None);
        assert_eq!(
            t.skill_reads, 0,
            "no skill marker: nothing counts as a skill read"
        );
        assert!(
            t.thinking_tokens > 0,
            "estimated from characters when no total is known"
        );
    }

    #[test]
    fn codex_current_schema() {
        let lines = [
            r##"{"type":"thread.started","model":"gpt-5-codex"}"##,
            r##"{"type":"item.completed","item":{"id":"r0","type":"reasoning","text":"Look at the skill, then write."}}"##,
            r##"{"type":"item.started","item":{"id":"i0","type":"command_execution","command":"/bin/bash -lc 'cat .codex/skills/report-writer/SKILL.md'"}}"##,
            r##"{"type":"item.completed","item":{"id":"i0","type":"command_execution","command":"/bin/bash -lc 'cat .codex/skills/report-writer/SKILL.md'","aggregated_output":"# Report writer","exit_code":0}}"##,
            r##"{"type":"item.started","item":{"id":"i1","type":"command_execution","command":"ls"}}"##,
            r##"{"type":"item.completed","item":{"id":"i1","type":"command_execution","command":"ls","aggregated_output":"notes.txt","exit_code":0}}"##,
            r##"{"type":"item.completed","item":{"id":"i2","type":"file_change","changes":[{"path":"/sb/report.md","kind":"add"},{"path":"/sb/notes/extra.md","kind":"add"}]}}"##,
            r##"{"type":"item.completed","item":{"id":"i3","type":"command_execution","command":"python3 missing.py","aggregated_output":"not found","exit_code":1}}"##,
            r##"{"type":"item.completed","item":{"id":"i4","type":"agent_message","text":"Done, wrote report.md"}}"##,
            r##"{"type":"turn.completed","usage":{"input_tokens":1000,"cached_input_tokens":400,"output_tokens":80}}"##,
        ];
        let mut b = TraceBuilder::new(
            Dialect::Codex,
            Path::new("/sb"),
            Some(".codex/skills/report-writer"),
        );
        for (i, l) in lines.iter().enumerate() {
            b.absorb_line(l, Some(i as u64 * 100));
        }
        let t = b.finish(80);
        assert_eq!(t.tool_calls, 4);
        assert_eq!((t.reads, t.searches, t.writes, t.commands), (1, 1, 1, 1));
        assert_eq!(t.skill_reads, 1);
        assert_eq!(t.files_written, vec!["notes/extra.md", "report.md"]);
        assert_eq!(t.files_read, vec![".codex/skills/report-writer/SKILL.md"]);
        assert_eq!(t.tool_errors, 1);
        assert_eq!(t.thinking_blocks, 1);
        assert_eq!(t.text_blocks, 1);
        assert_eq!(t.turns, 5);
        assert_eq!((t.cache_read_tokens, t.uncached_input_tokens), (400, 600));
        assert_eq!(
            t.tool_time_ms,
            Some(200),
            "two timed command executions of 100 ms"
        );
        assert_eq!(t.thinking_tokens + t.text_tokens + t.tool_input_tokens, 80);
    }

    #[test]
    fn codex_legacy_schema() {
        let lines = [
            r##"{"id":"0","msg":{"type":"session_configured","model":"o4-mini"}}"##,
            r##"{"id":"1","msg":{"type":"agent_reasoning","text":"plan"}}"##,
            r##"{"id":"2","msg":{"type":"exec_command_begin","call_id":"c1","command":["bash","-lc","ls"]}}"##,
            r##"{"id":"3","msg":{"type":"exec_command_end","call_id":"c1","exit_code":0,"stdout":"a.txt"}}"##,
            r##"{"id":"4","msg":{"type":"patch_apply_begin","call_id":"c2","changes":{"a.txt":{"add":{"content":"x"}}}}}"##,
            r##"{"id":"5","msg":{"type":"patch_apply_end","call_id":"c2","success":true}}"##,
            r##"{"id":"6","msg":{"type":"agent_message","message":"All set."}}"##,
            r##"{"id":"7","msg":{"type":"token_count","info":{"total_token_usage":{"input_tokens":50,"cached_input_tokens":10,"output_tokens":5}}}}"##,
        ];
        let mut b = TraceBuilder::new(Dialect::Codex, Path::new("/sb"), None);
        for l in lines {
            b.absorb_line(l, None);
        }
        let t = b.finish(5);
        // `bash -lc ls`: the wrapper is unwrapped and the script classified.
        assert_eq!(t.tool_calls, 2);
        assert_eq!(t.searches, 1);
        assert_eq!(t.writes, 1);
        assert_eq!(t.files_written, vec!["a.txt"]);
        assert_eq!(t.thinking_blocks, 1);
        assert_eq!((t.cache_read_tokens, t.uncached_input_tokens), (10, 40));
        assert_eq!(t.tools["command_execution"], 1);
        assert_eq!(t.tools["patch_apply"], 1);
    }

    #[test]
    fn shell_wrappers_heredocs_and_quotes() {
        assert_eq!(unwrap_shell("bash -lc 'cat x'").as_deref(), Some("cat x"));
        assert_eq!(
            unwrap_shell("/bin/bash -c \"grep -n \\\"a\\\" f\"").as_deref(),
            Some("grep -n \"a\" f")
        );
        assert_eq!(
            unwrap_shell("bash -lc 'echo '\\''hi'\\'''").as_deref(),
            Some("echo 'hi'")
        );
        assert_eq!(unwrap_shell("bash script.sh"), None);
        assert_eq!(unwrap_shell("bashful -c x"), None);
        assert_eq!(
            classify_command("/bin/bash -lc 'cat .codex/skills/report-writer/SKILL.md'"),
            ToolCategory::Read
        );
        assert_eq!(
            command_read_file("bash -lc 'cat .codex/skills/x/SKILL.md'").as_deref(),
            Some(".codex/skills/x/SKILL.md")
        );
        assert_eq!(classify_command("sh -c \"ls -la\""), ToolCategory::Search);
        // Heredocs: the body is data; the redirect decides.
        assert_eq!(
            classify_command("cat > out.md <<'EOF'\nhello\nEOF"),
            ToolCategory::Write
        );
        assert_eq!(
            classify_command("cat <<'EOF' > out.md\nhello\nEOF"),
            ToolCategory::Write
        );
        assert_eq!(
            classify_command("cat << EOF >> notes.txt\nx\nEOF"),
            ToolCategory::Write
        );
        assert_eq!(
            classify_command("cat <<'EOF'\ncat > not-a-redirect\nEOF"),
            ToolCategory::Read
        );
        assert_eq!(command_read_file("cat > out.md <<'EOF'\nhello\nEOF"), None);
        assert_eq!(command_read_file("cat <<'EOF' > out.md\nx\nEOF"), None);
        // A `>` inside quotes is not a redirect.
        assert_eq!(
            classify_command("grep -n \"->\" src/"),
            ToolCategory::Search
        );
        assert_eq!(classify_command("rg '=>' src"), ToolCategory::Search);
        assert_eq!(
            classify_command("python -c \"print(1 > 0)\""),
            ToolCategory::Command
        );
        assert_eq!(classify_command("FOO=\"a > b\" ls"), ToolCategory::Search);
        assert_eq!(classify_command("npm test 2>&1"), ToolCategory::Command);
        assert_eq!(
            classify_command("npm test &>/dev/null"),
            ToolCategory::Command
        );
        assert_eq!(
            classify_command("npm test 2>/dev/null"),
            ToolCategory::Command
        );
        assert_eq!(classify_command("echo hi >out.txt"), ToolCategory::Write);
        assert_eq!(
            classify_command("sed --in-place 's/a/b/' f"),
            ToolCategory::Write
        );
        assert_eq!(classify_command("export X=1 && cat y"), ToolCategory::Read);
        assert_eq!(
            classify_command("ünïcödé > 日本語.txt"),
            ToolCategory::Write
        );
    }

    #[test]
    fn skill_tool_counts_as_opening_the_skill() {
        let lines = [
            r##"{"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Skill","input":{"skill":"report-writer"}}],"usage":{"input_tokens":1,"output_tokens":10}}}"##,
            r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"Launching skill: report-writer"}]}}"##,
            r##"{"type":"assistant","message":{"id":"m2","content":[{"type":"tool_use","id":"t2","name":"Skill","input":{"skill":"other-skill"}}],"usage":{"input_tokens":1,"output_tokens":10}}}"##,
            r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t2","content":"Launching skill: other-skill"}]}}"##,
        ];
        let mut b = TraceBuilder::new(
            Dialect::ClaudeCode,
            Path::new("/sb"),
            Some(".claude/skills/report-writer"),
        );
        for l in lines {
            b.absorb_line(l, None);
        }
        let t = b.finish(20);
        assert_eq!(t.skill_reads, 1);
        assert_eq!(t.reads, 1);
        assert_eq!(t.other_tools, 1, "another skill is just another tool");
        assert_eq!(t.files_read, vec![".claude/skills/report-writer/SKILL.md"]);
        assert_eq!(t.steps[0].summary, "skill report-writer");
        assert_eq!(t.tools["Skill"], 2);

        // A sibling skill whose name starts with the marker is not the skill.
        let mut b = TraceBuilder::new(
            Dialect::ClaudeCode,
            Path::new("/sb"),
            Some(".claude/skills/pdf"),
        );
        b.tool_use(
            None,
            "Read",
            ToolCategory::Read,
            "x",
            Some("/home/u/.claude/skills/pdf-forms/SKILL.md"),
            0,
            None,
        );
        b.tool_use(
            None,
            "Read",
            ToolCategory::Read,
            "x",
            Some("/sb/.claude/skills/pdf/SKILL.md"),
            0,
            None,
        );
        b.tool_use(
            None,
            "Bash",
            ToolCategory::Read,
            "cat .claude/skills/pdf/scripts/a.py",
            None,
            0,
            None,
        );
        let t = b.finish(0);
        assert_eq!(t.skill_reads, 2);
        assert_eq!(t.files_read_count, 2);
    }

    #[test]
    fn subagent_events_are_counted_apart() {
        let lines = [
            r##"{"type":"assistant","message":{"id":"m1","content":[{"type":"tool_use","id":"t1","name":"Task","input":{"description":"look around"}}]}}"##,
            r##"{"type":"assistant","parent_tool_use_id":"t1","message":{"id":"s1","content":[{"type":"tool_use","id":"s1a","name":"Read","input":{"file_path":"/sb/notes.txt"}}]}}"##,
            r##"{"type":"user","parent_tool_use_id":"t1","message":{"content":[{"type":"tool_result","tool_use_id":"s1a","content":"notes"}]}}"##,
            r##"{"type":"assistant","parent_tool_use_id":"t1","message":{"id":"s2","content":[{"type":"text","text":"found it"}]}}"##,
            r##"{"type":"user","message":{"content":[{"type":"tool_result","tool_use_id":"t1","content":"found it"}]}}"##,
        ];
        let mut b = TraceBuilder::new(Dialect::ClaudeCode, Path::new("/sb"), None);
        for (i, l) in lines.iter().enumerate() {
            b.absorb_line(l, Some(i as u64 * 100));
        }
        let t = b.finish(0);
        assert_eq!(t.turns, 1);
        assert_eq!(t.tool_calls, 1);
        assert_eq!(t.subagents, 1);
        assert_eq!(t.reads, 0, "the subagent's read is not the parent's");
        assert_eq!(t.subagent_turns, 2);
        assert_eq!(t.subagent_tool_calls, 1);
        assert_eq!(t.tool_time_ms, Some(400), "only the Task call's own span");
        assert_eq!(t.steps.len(), 5);
        assert!(t.steps[1].summary.starts_with("↳ "));
        assert_eq!(t.files_read_count, 0);
    }

    #[test]
    fn token_split_sums_exactly_and_paths_stay_outside_siblings() {
        assert_eq!(largest_remainder(5, [1, 1, 0]), [3, 2, 0]);
        assert_eq!(largest_remainder(100, [199, 199, 2]), [50, 50, 0]);
        assert_eq!(largest_remainder(100, [150, 150, 100]), [38, 37, 25]);
        assert_eq!(largest_remainder(7, [0, 0, 0]), [0, 0, 0]);
        let mut b = TraceBuilder::new(Dialect::ClaudeCode, Path::new("/tmp/hive/run1"), None);
        b.thinking("a", None);
        b.text("b", None);
        b.tool_use(
            None,
            "Read",
            ToolCategory::Read,
            "x",
            Some("/tmp/hive/run10/notes.txt"),
            0,
            None,
        );
        b.tool_use(
            None,
            "Read",
            ToolCategory::Read,
            "x",
            Some("/tmp/hive/run1/notes.txt"),
            0,
            None,
        );
        b.tool_use(
            None,
            "Read",
            ToolCategory::Read,
            &"é".repeat(500),
            Some("C:\\x\\y.txt"),
            0,
            None,
        );
        let t = b.finish(5);
        assert_eq!(t.thinking_tokens + t.text_tokens + t.tool_input_tokens, 5);
        assert_eq!(
            t.files_read,
            vec!["/tmp/hive/run10/notes.txt", "C:/x/y.txt", "notes.txt"]
        );
        assert!(t.steps[2].summary.chars().count() <= SUMMARY_CHARS);
        assert_eq!(t.tool_time_ms, None, "no timestamps");

        // A completion-style trace has text but no tool calls: no tool time at all.
        let mut b = TraceBuilder::new(Dialect::ClaudeCode, Path::new("/sb"), None);
        b.turn();
        b.text("answer", Some(1200));
        b.set_skill_in_context(true);
        let t = b.finish(3);
        assert_eq!(t.tool_time_ms, None);
        assert!(t.skill_in_context);

        // A token_count without usage leaves the last reading alone.
        let mut b = TraceBuilder::new(Dialect::Codex, Path::new("/sb"), None);
        b.absorb_line(r##"{"msg":{"type":"token_count","info":{"total_token_usage":{"input_tokens":50,"cached_input_tokens":10,"cache_write_input_tokens":5}}}}"##, None);
        b.absorb_line(
            r##"{"msg":{"type":"token_count","info":null,"rate_limits":{}}}"##,
            None,
        );
        let t = b.finish(0);
        assert_eq!(
            (
                t.cache_read_tokens,
                t.uncached_input_tokens,
                t.cache_creation_tokens
            ),
            (10, 40, 5)
        );
    }

    #[test]
    fn steps_are_capped_and_summaries_short() {
        let mut b = TraceBuilder::new(Dialect::ClaudeCode, Path::new("/sb"), None);
        let long = "x".repeat(1000);
        for _ in 0..(STEP_CAP + 5) {
            b.text(&long, None);
        }
        let t = b.finish(0);
        assert_eq!(t.steps.len(), STEP_CAP);
        assert_eq!(t.steps_truncated, 5);
        assert!(t.steps[0].summary.chars().count() <= SUMMARY_CHARS);
        assert_eq!(t.steps[0].chars, 1000);
        assert_eq!(t.text_blocks as usize, STEP_CAP + 5);
        assert_eq!(t.io_share, 0.0);
    }
}
