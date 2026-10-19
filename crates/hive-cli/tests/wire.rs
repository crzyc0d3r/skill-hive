//! Local server that speaks the Anthropic Messages and OpenAI chat wire protocols.

use std::io::{Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

#[allow(dead_code)]
pub struct Wire {
    pub port: u16,
    pub hits: Arc<Mutex<Vec<String>>>,
}

pub fn spawn_wire() -> Wire {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let hits = Arc::new(Mutex::new(Vec::new()));
    let recorded = hits.clone();
    thread::spawn(move || {
        while let Ok((mut stream, _)) = listener.accept() {
            let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(2)));
            let mut buf = Vec::new();
            let mut tmp = [0_u8; 8192];
            loop {
                match stream.read(&mut tmp) {
                    Ok(0) | Err(_) => break,
                    Ok(count) => {
                        buf.extend_from_slice(&tmp[..count]);
                        if buf.windows(4).any(|window| window == b"\r\n\r\n") {
                            let header_end = buf
                                .windows(4)
                                .position(|window| window == b"\r\n\r\n")
                                .unwrap_or(buf.len());
                            let head = String::from_utf8_lossy(&buf[..header_end]);
                            let need = head.lines().find_map(|line| {
                                let (name, value) = line.split_once(':')?;
                                name.eq_ignore_ascii_case("content-length")
                                    .then(|| value.trim().parse::<usize>().ok())
                                    .flatten()
                            });
                            if buf.len() >= header_end + 4 + need.unwrap_or(0) {
                                break;
                            }
                        }
                    }
                }
            }
            let header_end = buf
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .unwrap_or(buf.len());
            let head = String::from_utf8_lossy(&buf[..header_end.min(buf.len())]).to_string();
            let body = if buf.len() > header_end + 4 {
                String::from_utf8_lossy(&buf[header_end + 4..]).to_string()
            } else {
                String::new()
            };
            recorded
                .lock()
                .unwrap()
                .push(format!("{head}\r\n\r\n{body}"));
            let payload = reply(&head, &body);
            let packet = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",
                payload.len()
            );
            let _ = stream.write_all(packet.as_bytes());
        }
    });
    Wire { port, hits }
}

fn reply(head: &str, body: &str) -> String {
    let path = head
        .lines()
        .next()
        .unwrap_or("")
        .split_whitespace()
        .nth(1)
        .unwrap_or("");
    if head.starts_with("GET") {
        return r#"{"data":[{"id":"gpt-4o-mini"},{"id":"claude-sonnet-4"},{"id":"claude-sonnet-5-5"},{"id":"gpt-test"}]}"#.to_string();
    }
    let openai = path.contains("chat/completions");
    if body.contains("You are Claude Code") {
        if body.contains("should-not") {
            return text(openai, "none");
        }
        return skill_tool(openai, "suite");
    }
    if body.contains("HIVE_ANALYST") {
        return text(
            openai,
            r#"{"notes":[{"text":"The expectation passes in both configurations."}]}"#,
        );
    }
    if body.contains("HIVE_INTERVIEW") {
        return text(
            openai,
            r#"{"questions":["Who is the reader?","Which files matter?"]}"#,
        );
    }
    if body.contains("GRADE") {
        return text(
            openai,
            r#"{"expectations":[{"text":"answer","passed":true,"evidence":"transcript"}]}"#,
        );
    }
    if body.contains("tool_result") || body.contains("toolu_") {
        return text(openai, "MOCK-GENERATED-BODY");
    }
    if body.contains("\"echo\"") {
        return echo_tool(openai);
    }
    if body.contains("write_file") {
        return write_tool(openai);
    }
    text(openai, "MOCK-GENERATED-BODY")
}

fn text(openai: bool, value: &str) -> String {
    let encoded = serde_json::to_string(value).unwrap_or_else(|_| "\"\"".into());
    if openai {
        format!(
            "data: {{\"id\":\"c\",\"choices\":[{{\"delta\":{{\"content\":{encoded}}}}}]}}\n\ndata: {{\"id\":\"c\",\"choices\":[{{\"finish_reason\":\"stop\"}}],\"usage\":{{\"prompt_tokens\":2,\"completion_tokens\":1}}}}\n\ndata: [DONE]\n\n"
        )
    } else {
        format!(
            "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"text_delta\",\"text\":{encoded}}}}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"end_turn\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n"
        )
    }
}

fn skill_tool(openai: bool, name: &str) -> String {
    if openai {
        return echo_tool(openai);
    }
    let partial = serde_json::json!({"skill": name}).to_string();
    let encoded = serde_json::to_string(&partial).unwrap();
    format!(
        "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":3}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"skill-1\",\"name\":\"Skill\"}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{encoded}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":2}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n"
    )
}

fn write_tool(openai: bool) -> String {
    if openai {
        return text(true, "MOCK-GENERATED-BODY");
    }
    let partial =
        serde_json::json!({"path": "outputs/answer.txt", "content": "PINECONE"}).to_string();
    let encoded = serde_json::to_string(&partial).unwrap();
    format!(
        "data: {{\"type\":\"message_start\",\"message\":{{\"id\":\"m\",\"usage\":{{\"input_tokens\":4}}}}}}\n\ndata: {{\"type\":\"content_block_start\",\"content_block\":{{\"type\":\"tool_use\",\"id\":\"tool-1\",\"name\":\"write_file\"}}}}\n\ndata: {{\"type\":\"content_block_delta\",\"delta\":{{\"type\":\"input_json_delta\",\"partial_json\":{encoded}}}}}\n\ndata: {{\"type\":\"content_block_stop\"}}\n\ndata: {{\"type\":\"message_delta\",\"delta\":{{\"stop_reason\":\"tool_use\"}},\"usage\":{{\"output_tokens\":3}}}}\n\ndata: {{\"type\":\"message_stop\"}}\n\n"
    )
}

fn echo_tool(openai: bool) -> String {
    if openai {
        return "data: {\"id\":\"c1\",\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"tool-1\",\"function\":{\"name\":\"echo\",\"arguments\":\"{\\\"word\\\":\\\"OK\\\"}\"}}]}}]}\n\ndata: {\"id\":\"c1\",\"choices\":[{\"finish_reason\":\"tool_calls\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2}}\n\ndata: [DONE]\n\n".to_string();
    }
    text(false, "OK")
}
