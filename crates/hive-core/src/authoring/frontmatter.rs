//! YAML frontmatter that keeps key order, quoting, and block scalars.
//!
//! An unedited document serializes back to the original bytes. Editing one key
//! rewrites only that key, in the quoting style the author already used.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Document {
    pub original: String,
    pub keys: Vec<Entry>,
    pub body: String,
    pub opened_on_line_1: bool,
    pub closed: bool,
    pub yaml_error: Option<String>,
    pub late_marker_line: Option<usize>,
    pub(crate) lines: Vec<SrcLine>,
    open_line: Option<usize>,
    close_line: Option<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SrcLine {
    pub(crate) text: String,
    pub(crate) nl: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub line: usize,
    pub value: Scalar,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value")]
pub enum Scalar {
    Str { text: String, style: Style },
    Bool(bool),
    Int(i64),
    Seq(Vec<String>),
    Map(Vec<(String, String)>),
    Null,
    Invalid(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Style {
    Plain,
    Double,
    Single,
    Fold,
    FoldStrip,
    FoldKeep,
    Literal,
    LiteralStrip,
    LiteralKeep,
}

impl Document {
    pub fn serialize(&self) -> String {
        self.lines
            .iter()
            .map(|line| format!("{}{}", line.text, line.nl))
            .collect()
    }

    pub fn get(&self, name: &str) -> Option<&Scalar> {
        self.keys
            .iter()
            .find(|key| key.name == name)
            .map(|key| &key.value)
    }

    pub fn string(&self, name: &str) -> Option<&str> {
        match self.get(name) {
            Some(Scalar::Str { text, .. }) => Some(text),
            _ => None,
        }
    }

    pub fn set_string(&mut self, name: &str, value: &str) -> bool {
        let Some(index) = self.keys.iter().position(|key| key.name == name) else {
            return false;
        };
        let style = match &self.keys[index].value {
            Scalar::Str { style, .. } => *style,
            _ => Style::Plain,
        };
        let start = self.keys[index].start_line;
        let end = self.keys[index].end_line;
        let rendered = render_string_key(name, value, style, &self.lines[start].nl);
        let nl = if self.lines[start].nl.is_empty() {
            "\n".to_string()
        } else {
            self.lines[start].nl.clone()
        };
        let mut replacement = Vec::new();
        for (offset, text) in rendered.into_iter().enumerate() {
            let line_nl = if offset + start == end && self.lines[end].nl.is_empty() {
                String::new()
            } else {
                nl.clone()
            };
            replacement.push(SrcLine { text, nl: line_nl });
        }
        let added = replacement.len() as isize - (end - start + 1) as isize;
        self.lines.splice(start..=end, replacement);
        self.keys[index].value = Scalar::Str {
            text: value.to_string(),
            style,
        };
        self.keys[index].end_line = (end as isize + added) as usize;
        let shift = added;
        for key in self.keys.iter_mut().skip(index + 1) {
            key.line = (key.line as isize + shift) as usize;
            key.start_line = (key.start_line as isize + shift) as usize;
            key.end_line = (key.end_line as isize + shift) as usize;
        }
        self.original = self.serialize();
        true
    }

    pub fn remove_key(&mut self, name: &str) -> bool {
        let Some(index) = self.keys.iter().position(|key| key.name == name) else {
            return false;
        };
        let start = self.keys[index].start_line;
        let end = self.keys[index].end_line;
        let removed = (end - start + 1) as isize;
        self.lines.drain(start..=end);
        self.keys.remove(index);
        for key in self.keys.iter_mut().skip(index) {
            key.line = key.line.saturating_sub(removed as usize);
            key.start_line = key.start_line.saturating_sub(removed as usize);
            key.end_line = key.end_line.saturating_sub(removed as usize);
        }
        self.original = self.serialize();
        true
    }

    pub fn body_starts_at(&self) -> usize {
        self.close_line.map(|line| line + 2).unwrap_or(1)
    }

    pub fn rename_key(&mut self, from: &str, to: &str) -> bool {
        let Some(index) = self.keys.iter().position(|key| key.name == from) else {
            return false;
        };
        let line = &mut self.lines[self.keys[index].start_line];
        if let Some(rest) = line.text.strip_prefix(from) {
            line.text = format!("{to}{rest}");
        }
        self.keys[index].name = to.to_string();
        self.original = self.serialize();
        true
    }
}

pub fn parse_document(text: &str) -> Document {
    let lines = split_lines(text);
    let mut doc = Document {
        original: text.to_string(),
        keys: Vec::new(),
        body: String::new(),
        opened_on_line_1: false,
        closed: false,
        yaml_error: None,
        late_marker_line: None,
        lines,
        open_line: None,
        close_line: None,
    };
    if doc.lines.is_empty() {
        doc.yaml_error = Some("empty file".to_string());
        return doc;
    }
    let first = doc.lines[0].text.trim_start_matches('\u{feff}');
    if first == "---" {
        doc.opened_on_line_1 = true;
        doc.open_line = Some(0);
    } else if let Some(index) = doc.lines.iter().position(|line| line.text == "---") {
        doc.late_marker_line = Some(index + 1);
        doc.yaml_error = Some("frontmatter does not open on line 1".to_string());
        doc.body = text.to_string();
        return doc;
    } else {
        doc.yaml_error = Some("frontmatter does not open on line 1".to_string());
        doc.body = text.to_string();
        return doc;
    }
    let close = doc
        .lines
        .iter()
        .enumerate()
        .skip(1)
        .find(|(_, line)| line.text == "---");
    let Some((close_at, _)) = close else {
        doc.yaml_error = Some("frontmatter has no closing ---".to_string());
        doc.body = text.to_string();
        return doc;
    };
    doc.closed = true;
    doc.close_line = Some(close_at);
    doc.body = doc.lines[close_at + 1..]
        .iter()
        .map(|line| format!("{}{}", line.text, line.nl))
        .collect();
    if let Err(err) = parse_keys(&mut doc, 1, close_at) {
        doc.yaml_error = Some(err);
        doc.keys.clear();
    }
    doc
}

fn parse_keys(doc: &mut Document, start: usize, end: usize) -> Result<(), String> {
    let mut index = start;
    while index < end {
        let text = doc.lines[index].text.clone();
        if text.trim().is_empty() {
            index += 1;
            continue;
        }
        if text.starts_with('-') || text.starts_with(' ') || text.starts_with('\t') {
            return Err("YAML frontmatter is not a mapping".to_string());
        }
        let Some((name, rest)) = split_key(&text) else {
            return Err(format!("invalid YAML at line {}", index + 1));
        };
        let indicator = rest.trim();
        if is_block_indicator(indicator) {
            let style = block_style(indicator);
            let content_start = index + 1;
            let mut content_end = content_start;
            while content_end < end {
                let next = &doc.lines[content_end].text;
                if next.trim().is_empty() || next.starts_with(' ') || next.starts_with('\t') {
                    content_end += 1;
                    continue;
                }
                break;
            }
            let raw: Vec<String> = doc.lines[content_start..content_end]
                .iter()
                .map(|line| line.text.clone())
                .collect();
            let text_value = unfold_block(&raw, style);
            doc.keys.push(Entry {
                name,
                line: index + 1,
                value: Scalar::Str {
                    text: text_value,
                    style,
                },
                start_line: index,
                end_line: content_end.saturating_sub(1).max(index),
            });
            index = content_end;
            continue;
        }
        if indicator.is_empty() {
            let content_start = index + 1;
            let mut content_end = content_start;
            let mut seq = Vec::new();
            let mut map = Vec::new();
            while content_end < end {
                let next = &doc.lines[content_end].text;
                if next.trim().is_empty() {
                    content_end += 1;
                    continue;
                }
                if split_key(next).is_some() && !next.starts_with(' ') && !next.starts_with('\t') {
                    break;
                }
                if let Some(item) = next.trim().strip_prefix("- ") {
                    seq.push(unquote(item.trim()).to_string());
                } else if let Some((child, value)) = split_key(next.trim()) {
                    map.push((child, unquote(value.trim()).to_string()));
                } else {
                    return Err(format!("invalid YAML at line {}", content_end + 1));
                }
                content_end += 1;
            }
            let value = if !seq.is_empty() && map.is_empty() {
                Scalar::Seq(seq)
            } else if seq.is_empty() {
                Scalar::Map(map)
            } else {
                return Err("mixed nested YAML".to_string());
            };
            doc.keys.push(Entry {
                name,
                line: index + 1,
                value,
                start_line: index,
                end_line: content_end.saturating_sub(1).max(index),
            });
            index = content_end;
            continue;
        }
        let value = parse_inline(indicator)?;
        doc.keys.push(Entry {
            name,
            line: index + 1,
            value,
            start_line: index,
            end_line: index,
        });
        index += 1;
    }
    Ok(())
}

fn parse_inline(raw: &str) -> Result<Scalar, String> {
    if (raw.starts_with('"') && raw.ends_with('"') && raw.len() >= 2)
        || (raw.starts_with('\'') && raw.ends_with('\'') && raw.len() >= 2)
    {
        let style = if raw.starts_with('"') {
            Style::Double
        } else {
            Style::Single
        };
        return Ok(Scalar::Str {
            text: unquote(raw).to_string(),
            style,
        });
    }
    if raw.starts_with('[') && raw.ends_with(']') {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return Ok(Scalar::Seq(Vec::new()));
        }
        let items = inner
            .split(',')
            .map(|part| unquote(part.trim()).to_string())
            .collect();
        return Ok(Scalar::Seq(items));
    }
    if raw.starts_with('{') {
        return Err("flow mappings are not supported".to_string());
    }
    match raw {
        "true" | "True" | "TRUE" | "yes" | "on" => return Ok(Scalar::Bool(true)),
        "false" | "False" | "FALSE" | "no" | "off" => return Ok(Scalar::Bool(false)),
        "~" | "null" | "Null" | "NULL" => return Ok(Scalar::Null),
        _ => {}
    }
    if let Ok(number) = raw.parse::<i64>() {
        return Ok(Scalar::Int(number));
    }
    if raw.contains(':') && !raw.starts_with('"') {
        return Err(format!("ambiguous plain scalar {raw}"));
    }
    Ok(Scalar::Str {
        text: raw.to_string(),
        style: Style::Plain,
    })
}

fn unfold_block(lines: &[String], style: Style) -> String {
    let folded = matches!(style, Style::Fold | Style::FoldStrip | Style::FoldKeep);
    let mut parts = Vec::new();
    for line in lines {
        if line.trim().is_empty() {
            parts.push(String::new());
        } else {
            parts.push(line.trim().to_string());
        }
    }
    let text = if folded {
        let mut out = String::new();
        for (index, part) in parts.iter().enumerate() {
            if part.is_empty() {
                if !out.ends_with('\n') {
                    out.push('\n');
                }
                continue;
            }
            if !out.is_empty() && !out.ends_with('\n') {
                out.push(' ');
            }
            if index > 0 && out.ends_with('\n') && !out.ends_with("\n\n") {
                // A blank line already separated the paragraph.
            }
            out.push_str(part);
        }
        out
    } else {
        parts.join("\n")
    };
    match style {
        Style::FoldStrip | Style::LiteralStrip => text.trim_end_matches(['\n', ' ']).to_string(),
        Style::FoldKeep | Style::LiteralKeep => {
            if text.ends_with('\n') {
                text
            } else {
                format!("{text}\n")
            }
        }
        _ => {
            let trimmed = text.trim_end_matches('\n');
            if trimmed.is_empty() {
                String::new()
            } else {
                format!("{trimmed}\n")
            }
        }
    }
}

fn render_string_key(name: &str, value: &str, style: Style, _nl: &str) -> Vec<String> {
    match style {
        Style::Double => vec![format!(
            "{name}: \"{}\"",
            value.replace('\\', "\\\\").replace('"', "\\\"")
        )],
        Style::Single => vec![format!("{name}: '{}'", value.replace('\'', "''"))],
        Style::Fold
        | Style::FoldStrip
        | Style::FoldKeep
        | Style::Literal
        | Style::LiteralStrip
        | Style::LiteralKeep => {
            let indicator = match style {
                Style::Fold => ">",
                Style::FoldStrip => ">-",
                Style::FoldKeep => ">+",
                Style::Literal => "|",
                Style::LiteralStrip => "|-",
                Style::LiteralKeep => "|+",
                _ => ">",
            };
            let mut lines = vec![format!("{name}: {indicator}")];
            let body = value.trim_end_matches('\n');
            if body.is_empty() {
                lines.push("  ".to_string());
            } else {
                for line in body.split('\n') {
                    lines.push(format!("  {line}"));
                }
            }
            lines
        }
        Style::Plain => vec![format!("{name}: {value}")],
    }
}

fn split_key(text: &str) -> Option<(String, String)> {
    let (name, rest) = text.split_once(':')?;
    if name.is_empty() || name.chars().any(|ch| ch.is_whitespace()) {
        return None;
    }
    if !name.chars().next()?.is_ascii_alphabetic() && !name.starts_with('_') {
        return None;
    }
    if !name
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
    {
        return None;
    }
    Some((name.to_string(), rest.trim().to_string()))
}

fn is_block_indicator(value: &str) -> bool {
    matches!(value, ">" | ">-" | ">+" | "|" | "|-" | "|+" | ">2" | "|2")
}

fn block_style(value: &str) -> Style {
    match value {
        ">-" => Style::FoldStrip,
        ">+" => Style::FoldKeep,
        "|" => Style::Literal,
        "|-" => Style::LiteralStrip,
        "|+" => Style::LiteralKeep,
        _ if value.starts_with('|') => Style::Literal,
        _ => Style::Fold,
    }
}

fn unquote(value: &str) -> &str {
    let bytes = value.as_bytes();
    if value.len() >= 2
        && ((bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

fn split_lines(text: &str) -> Vec<SrcLine> {
    let mut lines = Vec::new();
    let mut rest = text;
    if rest.is_empty() {
        return lines;
    }
    while !rest.is_empty() {
        if let Some(index) = rest.find('\n') {
            let mut end = index;
            let mut nl = "\n";
            if end > 0 && rest.as_bytes()[end - 1] == b'\r' {
                end -= 1;
                nl = "\r\n";
            }
            lines.push(SrcLine {
                text: rest[..end].to_string(),
                nl: nl.to_string(),
            });
            rest = &rest[index + 1..];
        } else {
            lines.push(SrcLine {
                text: rest.to_string(),
                nl: String::new(),
            });
            break;
        }
    }
    lines
}

/// Approximate token count used by the body gauge and the description budget.
/// Four characters count as one token, rounded up.
pub fn estimate_tokens(text: &str) -> usize {
    let count = text.chars().count();
    if count == 0 {
        0
    } else {
        count.div_ceil(4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_preserves_block_scalars_and_quotes() {
        let samples = [
            "---\nname: demo\ndescription: \"Find papers\"\n---\nHello\n",
            "---\nname: demo\ndescription: >\n  Find papers\n  quickly.\n---\nBody\n",
            "---\nname: demo\ndescription: |\n  line one\n  line two\n---\n",
            "---\nname: demo\ndescription: >-\n  folded\n---\n",
            "---\nname: demo\ndescription: |-\n  literal\n---\n",
        ];
        for sample in samples {
            let doc = parse_document(sample);
            assert!(
                doc.yaml_error.is_none(),
                "{sample:?} {}",
                doc.yaml_error.unwrap_or_default()
            );
            assert_eq!(doc.serialize(), sample);
        }
    }

    #[test]
    fn editing_description_touches_only_that_key() {
        let original = "---\nname: demo\ndescription: Hello\nlicense: MIT\n---\nBody\n";
        let mut doc = parse_document(original);
        assert!(doc.set_string("description", "World"));
        let next = doc.serialize();
        assert_eq!(
            next,
            "---\nname: demo\ndescription: World\nlicense: MIT\n---\nBody\n"
        );
        assert_eq!(doc.string("description"), Some("World"));
        let mut quoted = parse_document("---\nname: demo\ndescription: \"Hello\"\n---\n");
        assert!(quoted.set_string("description", "World"));
        assert_eq!(
            quoted.serialize(),
            "---\nname: demo\ndescription: \"World\"\n---\n"
        );
    }
}
