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
}

/// Parses and validates a price file.
pub fn parse(json: &str) -> Result<PriceFile> {
    let f: PriceFile = serde_json::from_str(json)?;
    if f.version.trim().is_empty() {
        return Err(Error::InvalidInput("price file has no version".into()));
    }
    let mut names = std::collections::HashSet::new();
    for m in &f.models {
        let rates = [m.input, m.output, m.cache_read, m.cache_write];
        if rates.iter().any(|r| !r.is_finite() || *r < 0.0) {
            return Err(Error::InvalidInput(format!("bad rate for {}", m.model)));
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
                 cache_read_per_mtok, cache_write_per_mtok, source)
             VALUES (?1, ?2 || 'T00:00:00Z', ?3, ?4, ?5, ?6, 'bundled')",
            (
                &m.model,
                &m.effective_from,
                m.input,
                m.output,
                m.cache_read,
                m.cache_write,
            ),
        )?;
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
            .find(|m| m.model == model)
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
