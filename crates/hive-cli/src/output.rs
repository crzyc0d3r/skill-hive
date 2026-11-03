//! Terminal output helpers: tables, colours and JSON mode.

use comfy_table::{Cell, ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};
use owo_colors::OwoColorize;
use serde::Serialize;

/// Styles that collapse to plain text when colour is disabled (`NO_COLOR`, `--json`, or when
/// stdout is not a terminal).
pub mod st {
    use std::io::IsTerminal;
    use std::sync::OnceLock;

    use owo_colors::Style;

    static ENABLED: OnceLock<bool> = OnceLock::new();

    pub fn enabled() -> bool {
        *ENABLED.get_or_init(|| {
            std::env::var_os("NO_COLOR").is_none()
                && std::env::var_os("HIVE_NO_COLOR").is_none()
                && std::io::stdout().is_terminal()
        })
    }

    fn when(style: Style) -> Style {
        if enabled() { style } else { Style::new() }
    }

    pub fn bold() -> Style {
        when(Style::new().bold())
    }
    pub fn green() -> Style {
        when(Style::new().green())
    }
    pub fn red() -> Style {
        when(Style::new().red())
    }
    pub fn yellow() -> Style {
        when(Style::new().yellow())
    }
    pub fn dim() -> Style {
        when(Style::new().dimmed())
    }
}

pub struct Out {
    pub json: bool,
}

impl Out {
    pub fn emit<T: Serialize>(&self, value: &T, human: impl FnOnce()) -> anyhow::Result<()> {
        if self.json {
            println!("{}", serde_json::to_string_pretty(value)?);
        } else {
            human();
        }
        Ok(())
    }

    pub fn table(&self, headers: &[&str], rows: Vec<Vec<String>>) {
        let mut table = Table::new();
        table
            .load_style(UTF8_FULL_CONDENSED)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(
                headers
                    .iter()
                    .map(|h| Cell::new(h.style(st::bold()).to_string())),
            );
        for row in rows {
            table.add_row(row);
        }
        println!("{table}");
    }

    pub fn ok(&self, message: impl AsRef<str>) {
        if !self.json {
            println!(
                "{} {}",
                "✓".style(st::green()).style(st::bold()),
                message.as_ref()
            );
        }
    }

    pub fn warn(&self, message: impl AsRef<str>) {
        if !self.json {
            eprintln!(
                "{} {}",
                "!".style(st::yellow()).style(st::bold()),
                message.as_ref()
            );
        }
    }

    pub fn note(&self, message: impl AsRef<str>) {
        if !self.json {
            println!("{}", message.as_ref().style(st::dim()));
        }
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    hive_core::util::truncate_chars(s.replace('\n', " ").trim(), max)
}

pub fn percent(f: f64) -> String {
    format!("{:.0}%", f * 100.0)
}

pub fn delta(f: f64) -> String {
    format!("{:+.0}%", f * 100.0)
}
