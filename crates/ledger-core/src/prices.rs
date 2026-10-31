//! The bundled price table and its loader (spec §6).
//!
//! `prices.json` ships inside the binary and is versioned. On startup
//! [`sync_bundled`] replaces the `bundled` rows when the shipped version
//! differs from the one last loaded; `user` rows are never touched. Prices
//! only change with app releases: there is no network fetch.

use rusqlite::Connection;
use serde::Deserialize;

use crate::{settings, Error, Result};

/// The shipped price table.
pub const BUNDLED_JSON: &str = include_str!("../prices.json");

/// Settings key recording which bundled version is loaded.
pub const LOADED_VERSION_KEY: &str = "prices.bundled_version";

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct PriceFile {
    pub version: String,
    #[serde(default)]
    pub note: String,
    pub models: Vec<ModelPrice>,
}

/// USD per million tokens for one model from a date.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ModelPrice {
    pub model: String,
    #[serde(default)]
    pub aliases: Vec<String>,
    pub effective_from: String,
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: f64,
    #[serde(default)]
    pub cache_write: f64,
    /// Rate for extended (1-hour) cache writes, when the vendor offers them.
    #[serde(default)]
    pub cache_write_1h: Option<f64>,
    /// Rate for 12-hour cache writes (OpenAI's extended prompt cache).
    #[serde(default)]
    pub cache_write_12h: Option<f64>,
    /// Higher rates for long prompts, e.g. Gemini 2.5 Pro above 200k tokens.
    #[serde(default)]
    pub long_context: Vec<LongContextRate>,
    /// Fields whose values are best estimates rather than read from a vendor
    /// page, e.g. `["effective_from"]` when the launch date is approximate.
    #[serde(default)]
    pub estimated: Vec<String>,
}

/// Field names that may appear in [`ModelPrice::estimated`].
pub const ESTIMABLE_FIELDS: &[&str] = &[
    "effective_from",
    "input",
    "output",
    "cache_read",
    "cache_write",
    "cache_write_1h",
    "cache_write_12h",
    "long_context",
];

impl ModelPrice {
    /// Whether `field` is marked as an estimate.
    pub fn is_estimated(&self, field: &str) -> bool {
        self.estimated.iter().any(|f| f == field)
    }
}

/// Rates applying when the prompt exceeds `above_prompt_tokens`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct LongContextRate {
    pub above_prompt_tokens: u64,
    pub input: f64,
    pub output: f64,
    #[serde(default)]
    pub cache_read: f64,
    #[serde(default)]
    pub cache_write: f64,
    #[serde(default)]
    pub cache_write_12h: Option<f64>,
}

/// Parses and validates a price file.
pub fn parse(json: &str) -> Result<PriceFile> {
    let f: PriceFile = serde_json::from_str(json)?;
    if f.version.trim().is_empty() {
        return Err(Error::InvalidInput("price file has no version".into()));
    }
    let mut names = std::collections::HashSet::new();
    for m in &f.models {
        let mut rates = vec![m.input, m.output, m.cache_read, m.cache_write];
        rates.extend(m.cache_write_1h);
        rates.extend(m.cache_write_12h);
        for t in &m.long_context {
            if t.above_prompt_tokens == 0 {
                return Err(Error::InvalidInput(format!(
                    "bad tier threshold for {}",
                    m.model
                )));
            }
            rates.extend([t.input, t.output, t.cache_read, t.cache_write]);
            rates.extend(t.cache_write_12h);
        }
        if rates.iter().any(|r| !r.is_finite() || *r < 0.0) {
            return Err(Error::InvalidInput(format!("bad rate for {}", m.model)));
        }
        if let Some(f) = m
            .estimated
            .iter()
            .find(|f| !ESTIMABLE_FIELDS.contains(&f.as_str()))
        {
            return Err(Error::InvalidInput(format!(
                "unknown estimated field {f} for {}",
                m.model
            )));
        }
        if m.effective_from.len() != 10 {
            return Err(Error::InvalidInput(format!(
                "bad effective_from for {}",
                m.model
            )));
        }
        for n in std::iter::once(&m.model).chain(&m.aliases) {
            if !names.insert((n.to_ascii_lowercase(), m.effective_from.clone())) {
                return Err(Error::InvalidInput(format!("duplicate model or alias {n}")));
            }
        }
    }
    Ok(f)
}

/// The parsed bundled table.
pub fn bundled() -> Result<PriceFile> {
    parse(BUNDLED_JSON)
}

/// Loads `file` as the bundled rows if its version is not already loaded.
/// Returns `true` when rows were (re)written.
pub fn sync(conn: &mut Connection, file: &PriceFile) -> Result<bool> {
    let loaded: Option<String> = settings::get_or(conn, LOADED_VERSION_KEY, None)?;
    if loaded.as_deref() == Some(file.version.as_str()) {
        return Ok(false);
    }
    let tx = conn.transaction()?;
    tx.execute("DELETE FROM prices WHERE source = 'bundled'", [])?;
    tx.execute("DELETE FROM price_aliases", [])?;
    for m in &file.models {
        tx.execute(
            "INSERT INTO prices (model, effective_from, input_per_mtok, output_per_mtok,
                 cache_read_per_mtok, cache_write_per_mtok, cache_write_1h_per_mtok, cache_write_12h_per_mtok, source)
             VALUES (?1, ?2 || 'T00:00:00Z', ?3, ?4, ?5, ?6, ?7, ?8, 'bundled')",
            (
                &m.model,
                &m.effective_from,
                m.input,
                m.output,
                m.cache_read,
                m.cache_write,
                m.cache_write_1h,
                m.cache_write_12h,
            ),
        )?;
        for t in &m.long_context {
            tx.execute(
                "INSERT INTO price_tiers (model, effective_from, source, above_prompt_tokens,
                     input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok,
                     cache_write_12h_per_mtok)
                 VALUES (?1, ?2 || 'T00:00:00Z', 'bundled', ?3, ?4, ?5, ?6, ?7, ?8)",
                (
                    &m.model,
                    &m.effective_from,
                    i64::try_from(t.above_prompt_tokens).unwrap_or(i64::MAX),
                    t.input,
                    t.output,
                    t.cache_read,
                    t.cache_write,
                    t.cache_write_12h,
                ),
            )?;
        }
        for a in &m.aliases {
            tx.execute(
                "INSERT OR REPLACE INTO price_aliases (alias, model) VALUES (?1, ?2)",
                (a.to_ascii_lowercase(), &m.model),
            )?;
        }
    }
    settings::set(&tx, LOADED_VERSION_KEY, &file.version)?;
    tx.commit()?;
    tracing::info!(version = %file.version, models = file.models.len(), "loaded bundled prices");
    Ok(true)
}

/// Loads the shipped table if needed.
pub fn sync_bundled(conn: &mut Connection) -> Result<bool> {
    sync(conn, &bundled()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn bundled_table_is_valid() {
        let f = bundled().unwrap();
        assert!(f.models.len() >= 5);
        assert!(f.models.iter().all(|m| m.output >= m.input));
    }

    #[test]
    fn sync_is_versioned_and_keeps_user_rows() {
        let mut c = db::open_in_memory().unwrap();
        assert!(sync_bundled(&mut c).unwrap());
        assert!(!sync_bundled(&mut c).unwrap(), "same version is a no-op");
        c.execute(
            "INSERT INTO prices (model, effective_from, input_per_mtok, output_per_mtok, source)
             VALUES ('my-model', '2026-01-01T00:00:00Z', 1, 2, 'user')",
            [],
        )
        .unwrap();
        let mut next = bundled().unwrap();
        next.version = "next".into();
        next.models.truncate(1);
        assert!(sync(&mut c, &next).unwrap());
        let (bundled_rows, user_rows): (i64, i64) = c
            .query_row(
                "SELECT SUM(source='bundled'), SUM(source='user') FROM prices",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!((bundled_rows, user_rows), (1, 1));
    }

    fn find(model: &str) -> ModelPrice {
        bundled()
            .unwrap()
            .models
            .into_iter()
            .filter(|m| m.model == model)
            .min_by(|a, b| a.effective_from.cmp(&b.effective_from))
            .unwrap()
    }

    /// Values from https://ai.google.dev/gemini-api/docs/pricing (checked 2026-10-31).
    #[test]
    fn gemini_cache_read_rates() {
        assert_eq!(find("gemini-2.5-pro").cache_read, 0.125);
        assert_eq!(find("gemini-2.5-flash").cache_read, 0.03);
    }

    /// Values from https://platform.claude.com/docs/en/about-claude/pricing
    /// (checked 2026-10-31); cache_write is the 5-minute rate.
    #[test]
    fn anthropic_current_models() {
        let rates = |m: &str| {
            let p = find(m);
            (p.input, p.output, p.cache_read, p.cache_write)
        };
        assert_eq!(rates("claude-opus-4-8"), (5.0, 25.0, 0.5, 6.25));
        assert_eq!(rates("claude-opus-5-5"), (4.0, 20.0, 0.2, 5.0));
        assert_eq!(rates("claude-sonnet-5-5"), (2.0, 10.0, 0.1, 2.5));
        assert_eq!(rates("claude-haiku-5-5"), (0.1, 0.5, 0.01, 0.125));
        assert_eq!(rates("claude-mythos-5-1"), (10.0, 50.0, 0.25, 12.5));
        assert_eq!(rates("claude-opus-4"), rates("claude-opus-4-1"));
        assert_eq!(rates("claude-haiku-3-5"), (0.8, 4.0, 0.08, 1.0));
    }

    /// Values from https://platform.openai.com/docs/pricing (checked
    /// 2026-10-31); cache_write is the 30-minute write rate where listed.
    #[test]
    fn openai_current_models() {
        let rates = |m: &str| {
            let p = find(m);
            (p.input, p.output, p.cache_read, p.cache_write)
        };
        assert_eq!(rates("gpt-5.3-codex"), (1.75, 14.0, 0.175, 0.0));
        assert_eq!(rates("gpt-6.1-sol"), (2.0, 10.0, 0.1, 2.5));
        assert_eq!(rates("gpt-5.6-luna"), (0.2, 1.2, 0.02, 0.25));
        assert_eq!(rates("gpt-4o-mini"), (0.15, 0.6, 0.075, 0.0));
        assert_eq!(rates("o3"), (2.0, 8.0, 0.5, 0.0));
    }

    /// Values from https://ai.google.dev/gemini-api/docs/pricing (checked 2026-10-31).
    #[test]
    fn google_current_models() {
        let rates = |m: &str| {
            let p = find(m);
            (p.input, p.output, p.cache_read)
        };
        assert_eq!(rates("gemini-4-argon"), (2.0, 10.0, 0.1));
        assert_eq!(rates("gemini-3.8-flash"), rates("gemini-3.6-flash"));
        assert_eq!(rates("gemini-3.1-pro-preview"), (2.0, 12.0, 0.2));
        assert_eq!(rates("gemini-2.5-flash-lite"), (0.1, 0.4, 0.01));
    }

    #[test]
    fn long_context_tiers() {
        let pro = find("gemini-2.5-pro").long_context;
        assert_eq!(pro.len(), 1);
        assert_eq!(
            (
                pro[0].above_prompt_tokens,
                pro[0].input,
                pro[0].output,
                pro[0].cache_read
            ),
            (200_000, 2.5, 15.0, 0.25)
        );
        let haiku = &find("claude-haiku-5-5").long_context[0];
        assert_eq!(
            (
                haiku.above_prompt_tokens,
                haiku.input,
                haiku.output,
                haiku.cache_read,
                haiku.cache_write
            ),
            (100_000, 0.5, 2.5, 0.05, 0.625)
        );
        let bad = r#"{"version":"v","models":[{"model":"m","effective_from":"2026-01-01","input":1,"output":1,
            "long_context":[{"above_prompt_tokens":0,"input":1,"output":1}]}]}"#;
        assert!(parse(bad).is_err());
    }

    /// Anthropic bills 1-hour cache writes at 2x the input rate
    /// (https://platform.claude.com/docs/en/about-claude/pricing, checked 2026-10-31).
    #[test]
    fn anthropic_1h_cache_writes_are_twice_input() {
        let all = bundled().unwrap().models;
        let claude: Vec<_> = all
            .iter()
            .filter(|m| m.model.starts_with("claude-"))
            .collect();
        assert!(!claude.is_empty());
        for m in claude {
            assert_eq!(m.cache_write_1h, Some(m.input * 2.0), "{}", m.model);
        }
        assert!(all
            .iter()
            .filter(|m| !m.model.starts_with("claude-"))
            .all(|m| m.cache_write_1h.is_none()));
    }

    /// OpenAI 12-hour cache writes (https://platform.openai.com/docs/pricing,
    /// read 2026-10-31): 2x the 30-minute write; gpt-5.5/5.4 and older have none.
    #[test]
    fn openai_12h_cache_writes() {
        let all = bundled().unwrap().models;
        let get = |n: &str| all.iter().find(|m| m.model == n).unwrap();
        for (n, r) in [
            ("gpt-6-astra", 25.0),
            ("gpt-5.6-luna", 0.5),
            ("gpt-6-rosalind", 37.5),
        ] {
            let m = get(n);
            assert_eq!(m.cache_write_12h, Some(r), "{n}");
            assert!((r - m.cache_write * 2.0).abs() < 1e-9, "{n}");
        }
        for n in ["gpt-5.5", "gpt-5.4", "gpt-5", "gpt-4o"] {
            assert_eq!(get(n).cache_write_12h, None, "{n}");
            assert_eq!(get(n).cache_write, 0.0, "{n}");
        }
        assert!(all
            .iter()
            .filter(|m| !m.model.starts_with("gpt-"))
            .all(|m| m.cache_write_12h.is_none()));
    }

    #[test]
    fn estimated_fields_are_validated() {
        let ok = r#"{"version":"v","models":[{"model":"m","effective_from":"2025-01-01",
            "input":1,"output":2,"estimated":["effective_from"]}]}"#;
        let m = &parse(ok).unwrap().models[0];
        assert!(m.is_estimated("effective_from"));
        assert!(!m.is_estimated("input"));
        let bad = ok.replace("[\"effective_from\"]", "[\"launch\"]");
        assert!(parse(&bad).is_err());
        let tier = r#"{"version":"v","models":[{"model":"m","effective_from":"2025-01-01",
            "input":1,"output":2,"long_context":[{"above_prompt_tokens":5,"input":1,
            "output":1,"cache_write_12h":-1}]}]}"#;
        assert!(parse(tier).is_err(), "tier 12h rate is validated");
    }

    /// Vendors whose placeholder dates (2025-01-01) were replaced by launch
    /// dates, which are marked as estimates.
    const DATED_VENDORS: &[&str] = &["claude-", "gpt-", "o3"];

    #[test]
    fn launch_dates_replace_placeholders() {
        let all = bundled().unwrap().models;
        for m in all
            .iter()
            .filter(|m| DATED_VENDORS.iter().any(|p| m.model.starts_with(p)))
        {
            assert_ne!(m.effective_from, "2025-01-01", "{}", m.model);
        }
        let opus = all.iter().find(|m| m.model == "claude-opus-4-6").unwrap();
        assert_eq!(opus.effective_from, "2026-02-05");
        assert!(opus.is_estimated("effective_from"));
    }

    #[test]
    fn rejects_bad_files() {
        assert!(parse(r#"{"version":"","models":[]}"#).is_err());
        let neg = r#"{"version":"v","models":[{"model":"m","effective_from":"2026-01-01","input":-1,"output":1}]}"#;
        assert!(parse(neg).is_err());
        let dup = r#"{"version":"v","models":[
            {"model":"m","effective_from":"2026-01-01","input":1,"output":1},
            {"model":"x","aliases":["M"],"effective_from":"2026-01-01","input":1,"output":1}]}"#;
        assert!(parse(dup).is_err());
    }
}
