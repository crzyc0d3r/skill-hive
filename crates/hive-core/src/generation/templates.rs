//! Handlebars-style renderer for generation prompts P1 to P8 and P12.
//! `{{name}}`, `{{#if name}}`, and `{{#each name}}` (rows are `name|description`).

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Template {
    P1,
    P2,
    P3,
    P4,
    P5,
    P6,
    P7,
    P8,
    P12,
}

pub fn render(template: Template, vars: &BTreeMap<String, String>) -> String {
    render_str(template_text(template), vars)
}

pub fn render_str(template: &str, vars: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(rest);
            return out;
        };
        let tag = after[..end].trim();
        rest = &after[end + 2..];
        if let Some(name) = tag.strip_prefix("#if ") {
            let Some(close) = rest.find("{{/if}}") else {
                break;
            };
            let body = &rest[..close];
            rest = &rest[close + 7..];
            if vars.get(name.trim()).is_some_and(|value| !value.is_empty()) {
                out.push_str(&render_str(body, vars));
            }
        } else if let Some(name) = tag.strip_prefix("#each ") {
            let Some(close) = rest.find("{{/each}}") else {
                break;
            };
            let body = &rest[..close];
            rest = &rest[close + 9..];
            if let Some(rows) = vars.get(name.trim()) {
                for row in rows.lines().filter(|line| !line.is_empty()) {
                    let mut local = vars.clone();
                    let mut parts = row.splitn(2, '|');
                    local.insert("name".into(), parts.next().unwrap_or("").into());
                    local.insert("description".into(), parts.next().unwrap_or("").into());
                    out.push_str(&render_str(body, &local));
                }
            }
        } else {
            out.push_str(vars.get(tag).map(String::as_str).unwrap_or(""));
        }
    }
    out.push_str(rest);
    out
}

fn template_text(template: Template) -> &'static str {
    match template {
        Template::P1 => P1,
        Template::P2 => P2,
        Template::P3 => P3,
        Template::P4 => P4,
        Template::P5 => P5,
        Template::P6 => P6,
        Template::P7 => P7,
        Template::P8 => P8,
        Template::P12 => P12,
    }
}

const P1: &str = "You are helping a user design a {{kind}}. Do not draft anything yet.\n\
Goal:\n<<<\n{{goal}}\n>>>\n\
Context the app already knows (may be empty):\n\
- Skills already in the user's library (name: description): {{library_listing}}\n\
- Validation profile: {{profile}}\n\
{{#if base_subject}}\n- The user wants to revise an existing {{kind}}. Its current files:\n{{base_files_envelope}}\n{{/if}}\n\
Ask between 3 and 7 questions. Return JSON only:\n\
{\"assumed_name\": \"{{name}}\", \"questions\": [], \"skipped\": []}\n";

const P2: &str = "You are writing a complete first draft of an agent skill.\n\
Goal: {{goal}}\n\
Interview answers:\n{{answers_block}}\n\
Skill name (fixed; it must equal the directory name): {{name}}\n\
Validation profile: {{profile}}\n\
Rules the app checks deterministically; a draft that breaks them is sent back to you:\n\
- The file starts with \"---\" on line 1. Frontmatter has name: {{name}} and a description of\n\
  1 to 1024 characters containing no \"<\" or \">\". {{allowed_keys_rule}}\n\
- The SKILL.md body is under 500 lines.\n\
- Every relative path mentioned in SKILL.md is a file you include in this draft.\n\
- The skill must not surprise the user.\n\
Design guidance:\n\
1. Description. Say what the skill does AND when to use it. Be a little pushy.\n\
5. Evals alongside the skill. Write evals/evals.json with 2 or 3 prompts and evals/trigger.json\n\
with 8 to 10 should-trigger queries and 8 to 10 should-not-trigger near-misses.\n\
Output format. Emit the files in this envelope and nothing else.\n\
===== FILE: SKILL.md =====\n\
---\nname: {{name}}\ndescription: ...\n---\n\
===== FILE: evals/evals.json =====\n\
{\"skill_name\": \"{{name}}\", \"evals\": []}\n\
===== FILE: evals/trigger.json =====\n\
[{\"query\": \"...\", \"should_trigger\": true}]\n\
===== END =====\n";

const P3: &str = "You are writing a Claude Code subagent definition.\n\
Goal: {{goal}}\n\
Interview answers:\n{{answers_block}}\n\
Agent name (fixed): {{name}}\n\
Tools available: {{tool_catalog}}\n\
Return agent.md and evals/evals.json in the file envelope. Never list Skill in tools.\n";

const P4: &str = "Generate a trigger evaluation set for the skill below.\n\
Skill name: {{name}}\n\
Current description:\n<<<\n{{description}}\n>>>\n\
Write {{positives}} should-trigger queries and {{negatives}} should-not-trigger queries.\n\
Return a JSON array only.\n";

const P5: &str = "Draft expectations for the test prompts of this {{kind}}.\n\
{{kind}} name: {{name}}\n\
{{description}}\n\
{{evals_block}}\n\
Return JSON only with evals[].expectations.\n";

const P6: &str = "Revise the {{kind}} draft below based on the feedback.\n\
Current files:\n{{current_files_envelope}}\n\
Feedback from the user (may be empty):\n<<<\n{{user_feedback}}\n>>>\n\
Validation findings:\n{{validation_findings}}\n\
Evidence from test runs:\n{{run_evidence}}\n\
Output the complete set of files in the envelope format. Add .studio/changes.md listing each change.\n";

const P7: &str =
    "Read the transcripts and find repeated work.\n{{kind}}: {{name}}\n{{transcripts}}\n\
Return JSON only: {\"candidates\": []}\n";

const P8: &str = "You are improving the description of a skill.\n\
Skill name: {{name}}\n\
Current description:\n<<<\n{{description}}\n>>>\n\
Trigger results on the training split:\n{{train_results}}\n\
Return JSON only: {\"description\": \"...\", \"changes\": []}\n";

const P12: &str =
    "You are a blind comparator. Two outputs, A and B, were produced for the same task.\n\
You must not try to work out which configuration produced which.\n\
Task prompt: {{prompt}}\n\
Output A:\n{{outputs_a}}\n\
Output B:\n{{outputs_b}}\n\
Return JSON only with winner (\"A\" | \"B\" | \"TIE\").\n";
