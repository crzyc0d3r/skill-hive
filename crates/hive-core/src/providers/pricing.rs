//! Built-in prices from the plan's 2026-10-06 table, overridable per provider.

use rusqlite::{Connection, OptionalExtension};

#[derive(Debug, Clone, Copy)]
pub struct Pricing {
    pub input_per_mtok: f64,
    pub output_per_mtok: f64,
    pub cache_read_per_mtok: f64,
    pub cache_write_per_mtok: f64,
}

impl Pricing {
    pub fn anthropic_defaults(model: &str) -> Option<Self> {
        match model {
            "claude-opus-5-5" => Some(Self::row(4.0, 20.0, 0.40, 5.0)),
            "claude-sonnet-5-5" => Some(Self::row(2.0, 10.0, 0.20, 2.5)),
            "claude-haiku-5-5" => Some(Self::row(0.10, 0.50, 0.01, 0.125)),
            "claude-fable-5-1" => Some(Self::row(10.0, 50.0, 1.0, 12.5)),
            _ => None,
        }
    }

    fn row(input: f64, output: f64, cache_read: f64, cache_write: f64) -> Self {
        Self {
            input_per_mtok: input,
            output_per_mtok: output,
            cache_read_per_mtok: cache_read,
            cache_write_per_mtok: cache_write,
        }
    }
}

pub fn seed_models() -> &'static [&'static str] {
    &[
        "claude-opus-5-5",
        "claude-sonnet-5-5",
        "claude-haiku-5-5",
        "claude-fable-5-1",
    ]
}

pub fn cost_usd(
    pricing: &Pricing,
    input: u64,
    output: u64,
    cache_read: u64,
    cache_write: u64,
) -> f64 {
    (input as f64 * pricing.input_per_mtok
        + output as f64 * pricing.output_per_mtok
        + cache_read as f64 * pricing.cache_read_per_mtok
        + cache_write as f64 * pricing.cache_write_per_mtok)
        / 1_000_000.0
}

pub fn lookup(conn: &Connection, provider_id: &str, model: &str) -> Pricing {
    let specific = read_row(conn, provider_id, model);
    specific
        .or_else(|| read_row(conn, "*", model))
        .or_else(|| Pricing::anthropic_defaults(model))
        .unwrap_or(Pricing::row(0.0, 0.0, 0.0, 0.0))
}

fn read_row(conn: &Connection, provider_id: &str, model: &str) -> Option<Pricing> {
    conn.query_row(
        "SELECT input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok
         FROM model_pricing WHERE provider_config_id = ?1 AND model = ?2",
        (provider_id, model),
        |row| {
            Ok(Pricing {
                input_per_mtok: row.get(0)?,
                output_per_mtok: row.get(1)?,
                cache_read_per_mtok: row.get::<_, Option<f64>>(2)?.unwrap_or(0.0),
                cache_write_per_mtok: row.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
            })
        },
    )
    .optional()
    .ok()
    .flatten()
}
