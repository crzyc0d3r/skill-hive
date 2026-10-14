//! Safety scan for generated scripts. The execute bit is never set by the writer.

use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyFlag {
    pub file: String,
    pub line: usize,
    pub text: String,
}

pub fn scan_scripts(files: &BTreeMap<String, String>) -> Vec<SafetyFlag> {
    let mut flags = Vec::new();
    for (path, body) in files {
        if !(path.ends_with(".py") || path.ends_with(".sh")) {
            continue;
        }
        for (index, line) in body.lines().enumerate() {
            let lower = line.to_ascii_lowercase();
            let flagged = lower.contains("curl")
                || lower.contains("rm -rf")
                || lower.contains("sudo")
                || lower.contains("wget")
                || line.contains("../");
            if flagged {
                flags.push(SafetyFlag {
                    file: path.clone(),
                    line: index + 1,
                    text: line.to_string(),
                });
            }
        }
    }
    flags
}
