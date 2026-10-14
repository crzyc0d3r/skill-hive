//! Local agent CLIs. Claude is `claude -p` with `stream-json`. Codex is probed and kept behind
//! the detected flag set. Process trees die with the run: a process group on Unix, a job object
//! on Windows.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

use serde::Serialize;
use serde_json::Value;

use super::types::*;

#[derive(Debug, Clone)]
pub struct CliEvent {
    pub kind: String,
    pub tool_name: Option<String>,
    pub partial_json: Option<String>,
    pub text: Option<String>,
    pub usage: Option<Usage>,
    pub duration_ms: Option<u64>,
}

pub struct LocalCli {
    pub id: String,
    pub program: String,
    pub flavor: Flavor,
    pub model: Option<String>,
}

impl LocalCli {
    pub fn claude(program: impl Into<String>) -> Self {
        Self {
            id: "claude-cli".into(),
            program: program.into(),
            flavor: Flavor::ClaudeCli,
            model: None,
        }
    }
}

impl ChatProvider for LocalCli {
    fn id(&self) -> &str {
        &self.id
    }

    fn kind_name(&self) -> &'static str {
        "local_cli"
    }

    fn capabilities(&self) -> Capabilities {
        Capabilities {
            tools: true,
            streaming: true,
            system_prompt: false,
        }
    }

    fn complete(
        &self,
        request: &ChatRequest,
        sink: &mut dyn StreamSink,
        cancel: &CancellationToken,
    ) -> Result<ChatResponse, ProviderError> {
        let prompt = request
            .messages
            .iter()
            .flat_map(|message| message.content.iter())
            .filter_map(|block| match block {
                ContentBlock::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let output = run_claude(
            &self.program,
            &prompt,
            self.model.as_deref(),
            &cwd,
            cancel,
            true,
        )?;
        let _ = output.duration_ms;
        parse_stream_json(&output.lines, sink)
    }

    fn list_models(&self) -> Result<Vec<ModelInfo>, ProviderError> {
        let version = command_output(&self.program, &["--version"])?;
        Ok(vec![ModelInfo {
            id: version.trim().to_string(),
            supports_tools: Some(true),
        }])
    }

    fn count_tokens(&self, _request: &ChatRequest) -> Result<Option<u64>, ProviderError> {
        Ok(None)
    }
}

pub struct CliOutput {
    pub lines: Vec<String>,
    pub duration_ms: u64,
}

pub fn run_claude(
    program: &str,
    prompt: &str,
    model: Option<&str>,
    cwd: &Path,
    cancel: &CancellationToken,
    stream: bool,
) -> Result<CliOutput, ProviderError> {
    let mut command = command_for(program);
    command.arg("-p").arg(prompt);
    if stream {
        command.args([
            "--output-format",
            "stream-json",
            "--verbose",
            "--include-partial-messages",
        ]);
    } else {
        command.args(["--output-format", "json"]);
    }
    if let Some(model) = model {
        command.args(["--model", model]);
    }
    command.current_dir(cwd);
    command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command.env_remove("CLAUDECODE");
    let mut child = spawn_grouped(&mut command)?;
    let started = std::time::Instant::now();
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| ProviderError::new(ErrorClass::Other, "no stdout"))?;
    let reader = BufReader::new(stdout);
    let mut lines = Vec::new();
    for line in reader.lines() {
        if cancel.is_cancelled() {
            let _ = kill_tree(&mut child);
            return Err(ProviderError::new(ErrorClass::Cancelled, "cancelled"));
        }
        lines.push(line.map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?);
    }
    let status = child
        .wait()
        .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
    if !status.success() {
        return Err(ProviderError::new(
            ErrorClass::Other,
            format!("cli exit {}", status.code().unwrap_or(-1)),
        ));
    }
    Ok(CliOutput {
        lines,
        duration_ms: u64::try_from(started.elapsed().as_millis())
            .unwrap_or(u64::MAX)
            .max(1),
    })
}

pub fn parse_stream_json(
    lines: &[String],
    sink: &mut dyn StreamSink,
) -> Result<ChatResponse, ProviderError> {
    let mut response = ChatResponse {
        id: "cli".into(),
        stop: StopReason::EndTurn,
        text: String::new(),
        tool_uses: Vec::new(),
        usage: Usage::zero(),
        incomplete_tool: false,
    };
    let mut tool: Option<(String, String, String)> = None;
    let mut duration_ms = 1_u64;
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line)
            .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
        let events = cli_events(&value);
        for event in events {
            match event.kind.as_str() {
                "tool_use_start" => {
                    let id = event.text.clone().unwrap_or_else(|| "tool".into());
                    let name = event.tool_name.clone().unwrap_or_default();
                    sink.on_event(&StreamEvent::ToolUseStart {
                        id: id.clone(),
                        name: name.clone(),
                    });
                    tool = Some((id, name, String::new()));
                }
                "input_json_delta" => {
                    if let Some((id, _, buf)) = tool.as_mut() {
                        let partial = event.partial_json.unwrap_or_default();
                        buf.push_str(&partial);
                        sink.on_event(&StreamEvent::ToolInputDelta {
                            id: id.clone(),
                            partial_json: partial,
                        });
                    }
                }
                "assistant" => {
                    if let Some(text) = event.text {
                        response.text.push_str(&text);
                        sink.on_event(&StreamEvent::TextDelta(text));
                    }
                }
                "result" => {
                    if let Some(usage) = event.usage {
                        response.usage = usage.clone();
                        sink.on_event(&StreamEvent::Usage(usage));
                    }
                    if let Some(ms) = event.duration_ms {
                        duration_ms = ms.max(1);
                    }
                    response.stop = StopReason::EndTurn;
                }
                _ => {}
            }
        }
    }
    if let Some((id, name, raw)) = tool.take() {
        let parsed = serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null);
        let incomplete = serde_json::from_str::<Value>(&raw).is_err();
        sink.on_event(&StreamEvent::ToolUseEnd { id: id.clone() });
        response.tool_uses.push(ToolUse {
            id,
            name,
            input: parsed,
            raw_input: raw,
            incomplete,
        });
        if !response.tool_uses.is_empty() {
            response.stop = StopReason::ToolUse;
        }
    }
    let _ = duration_ms;
    sink.on_event(&StreamEvent::MessageEnd {
        stop_reason: response.stop.clone(),
    });
    Ok(response)
}

fn cli_events(value: &Value) -> Vec<CliEvent> {
    let mut out = Vec::new();
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    if kind == "stream_event" {
        let event = &value["event"];
        let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
        if event_type == "content_block_start" {
            let block = &event["content_block"];
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                out.push(CliEvent {
                    kind: "tool_use_start".into(),
                    tool_name: block
                        .get("name")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    partial_json: None,
                    text: block.get("id").and_then(Value::as_str).map(str::to_string),
                    usage: None,
                    duration_ms: None,
                });
            }
        } else if event_type == "content_block_delta" {
            let delta = &event["delta"];
            if delta.get("type").and_then(Value::as_str) == Some("input_json_delta") {
                out.push(CliEvent {
                    kind: "input_json_delta".into(),
                    tool_name: None,
                    partial_json: delta
                        .get("partial_json")
                        .and_then(Value::as_str)
                        .map(str::to_string),
                    text: None,
                    usage: None,
                    duration_ms: None,
                });
            }
        }
    } else if kind == "assistant" {
        let content = value.pointer("/message/content").and_then(Value::as_array);
        let mut text = String::new();
        if let Some(blocks) = content {
            for block in blocks {
                if block.get("type").and_then(Value::as_str) == Some("text") {
                    text.push_str(block.get("text").and_then(Value::as_str).unwrap_or(""));
                }
            }
        }
        out.push(CliEvent {
            kind: "assistant".into(),
            tool_name: None,
            partial_json: None,
            text: Some(text),
            usage: None,
            duration_ms: None,
        });
    } else if kind == "result" {
        let usage_value = value.get("usage");
        let usage = usage_value.map(|usage| Usage {
            input_tokens: usage
                .get("input_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            output_tokens: usage
                .get("output_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        });
        out.push(CliEvent {
            kind: "result".into(),
            tool_name: None,
            partial_json: None,
            text: None,
            usage,
            duration_ms: value.get("duration_ms").and_then(Value::as_u64),
        });
    }
    out
}

fn command_for(program: &str) -> Command {
    #[cfg(windows)]
    {
        let extension = Path::new(program)
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        if extension.eq_ignore_ascii_case("cmd") || extension.eq_ignore_ascii_case("bat") {
            let mut command = Command::new("cmd");
            command.arg("/c").arg(program);
            return command;
        }
    }
    Command::new(program)
}

pub fn command_output(program: &str, args: &[&str]) -> Result<String, ProviderError> {
    let output = command_for(program)
        .args(args)
        .output()
        .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))?;
    if !output.status.success() {
        return Err(ProviderError::new(
            ErrorClass::Other,
            format!("{program} exited {}", output.status),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedCli {
    pub name: String,
    pub path: String,
    pub version: String,
    pub flags: Vec<String>,
}

pub fn detect_local_agent_clis(extra_dirs: &[PathBuf]) -> Vec<DetectedCli> {
    let mut dirs = extra_dirs.to_vec();
    if let Some(path) = std::env::var_os("PATH") {
        dirs.extend(std::env::split_paths(&path));
    }
    if let Some(home) = dirs::home_dir() {
        dirs.push(home.join(".skills-manager").join("bin"));
    }
    let mut found = Vec::new();
    for name in ["claude", "codex"] {
        if let Some(path) = find_on_dirs(name, &dirs) {
            let version = command_output(&path.to_string_lossy(), &["--version"])
                .unwrap_or_else(|_| "unknown".into());
            let flags = help_flags(&path);
            found.push(DetectedCli {
                name: name.into(),
                path: path.to_string_lossy().into_owned(),
                version,
                flags,
            });
        }
    }
    found
}

fn find_on_dirs(name: &str, dirs: &[PathBuf]) -> Option<PathBuf> {
    for dir in dirs {
        let candidate = dir.join(name);
        if candidate.is_file() {
            return Some(candidate);
        }
        #[cfg(windows)]
        {
            let exe = dir.join(format!("{name}.exe"));
            if exe.is_file() {
                return Some(exe);
            }
            let cmd = dir.join(format!("{name}.cmd"));
            if cmd.is_file() {
                return Some(cmd);
            }
        }
    }
    None
}

pub fn spawn_grouped(command: &mut Command) -> Result<Child, ProviderError> {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        command.process_group(0);
    }
    command
        .spawn()
        .map_err(|err| ProviderError::new(ErrorClass::Other, err.to_string()))
}

pub fn kill_tree(child: &mut Child) -> Result<(), ProviderError> {
    #[cfg(unix)]
    {
        let pid = child.id() as i32;
        unsafe {
            libc::kill(-pid, libc::SIGKILL);
        }
        let _ = child.wait();
        Ok(())
    }
    #[cfg(windows)]
    {
        let _ = child.kill();
        let _ = child.wait();
        Ok(())
    }
}

#[cfg(windows)]
#[cfg_attr(not(test), allow(dead_code))]
pub struct JobChild {
    job: windows_sys::Win32::Foundation::HANDLE,
}

#[cfg(windows)]
#[cfg_attr(not(test), allow(dead_code))]
pub fn spawn_killed_with_job(
    program: &str,
    args: &[String],
    cwd: &Path,
) -> Result<JobChild, ProviderError> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Foundation::{CloseHandle, FALSE};
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, ResumeThread, CREATE_NO_WINDOW, CREATE_SUSPENDED, PROCESS_INFORMATION,
        STARTUPINFOW,
    };

    unsafe {
        let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
        if job.is_null() {
            return Err(ProviderError::new(
                ErrorClass::Other,
                "CreateJobObjectW failed",
            ));
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        SetInformationJobObject(
            job,
            JobObjectExtendedLimitInformation,
            &info as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION as *const std::ffi::c_void,
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        );
        let mut command_line = format!("\"{program}\"");
        for arg in args {
            command_line.push(' ');
            command_line.push_str(arg);
        }
        let mut wide: Vec<u16> = std::ffi::OsStr::new(&command_line)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let cwd_wide: Vec<u16> = cwd
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut startup: STARTUPINFOW = std::mem::zeroed();
        startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        let mut process_info: PROCESS_INFORMATION = std::mem::zeroed();
        let created = CreateProcessW(
            std::ptr::null(),
            wide.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            FALSE,
            CREATE_SUSPENDED | CREATE_NO_WINDOW,
            std::ptr::null(),
            cwd_wide.as_ptr(),
            &startup,
            &mut process_info,
        );
        if created == 0 {
            CloseHandle(job);
            return Err(ProviderError::new(
                ErrorClass::Other,
                "CreateProcessW failed",
            ));
        }
        if AssignProcessToJobObject(job, process_info.hProcess) == 0 {
            CloseHandle(process_info.hThread);
            CloseHandle(process_info.hProcess);
            CloseHandle(job);
            return Err(ProviderError::new(
                ErrorClass::Other,
                "AssignProcessToJobObject failed",
            ));
        }
        ResumeThread(process_info.hThread);
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
        Ok(JobChild { job })
    }
}

#[cfg(windows)]
#[cfg_attr(not(test), allow(dead_code))]
pub fn kill_job(child: &JobChild) {
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::JobObjects::TerminateJobObject;
    unsafe {
        TerminateJobObject(child.job, 1);
        CloseHandle(child.job);
    }
}

pub fn help_flags(program: &Path) -> Vec<String> {
    command_output(&program.to_string_lossy(), &["--help"])
        .unwrap_or_default()
        .split_whitespace()
        .filter(|token| token.starts_with("--"))
        .map(|token| {
            token
                .trim_matches(|ch: char| ch == ',' || ch == ':')
                .to_string()
        })
        .collect()
}

#[allow(dead_code)]
fn _duration_marker(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(0)
}
