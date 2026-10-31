//! Typed key/value settings stored as JSON in the `settings` table.
//!
//! Unknown keys are allowed so newer builds can add settings without a
//! migration; reads fall back to the documented default when a key is unset.

use rusqlite::{Connection, OptionalExtension};
use serde::{de::DeserializeOwned, Serialize};

use crate::{Error, Result};

/// Well-known setting keys and their defaults.
pub mod keys {
    /// Days of raw events to keep (0 = keep forever). Default 0.
    pub const RETENTION_DAYS: &str = "retention.raw_event_days";
    /// Hash project names before storing them. Default false.
    pub const HASH_PROJECT_NAMES: &str = "privacy.hash_project_names";
    /// Live ingestion paused. Default false.
    pub const LIVE_PAUSED: &str = "live.paused";
}

fn check_key(key: &str) -> Result<()> {
    let ok = !key.is_empty()
        && key.len() <= 128
        && key
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    if ok {
        Ok(())
    } else {
        Err(Error::InvalidInput(format!("bad setting key {key:?}")))
    }
}

/// Raw JSON value for `key`, if set.
pub fn get_json(conn: &Connection, key: &str) -> Result<Option<serde_json::Value>> {
    check_key(key)?;
    let raw: Option<String> = conn
        .query_row(
            "SELECT value_json FROM settings WHERE key = ?1",
            [key],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|s| serde_json::from_str(&s).map_err(Error::from))
        .transpose()
}

/// Typed value for `key`, or `default` when unset.
pub fn get_or<T: DeserializeOwned>(conn: &Connection, key: &str, default: T) -> Result<T> {
    match get_json(conn, key)? {
        Some(v) => Ok(serde_json::from_value(v)?),
        None => Ok(default),
    }
}

/// Stores `value` under `key`, replacing any previous value.
pub fn set<T: Serialize + ?Sized>(conn: &Connection, key: &str, value: &T) -> Result<()> {
    check_key(key)?;
    let json = serde_json::to_string(value)?;
    conn.execute(
        "INSERT INTO settings (key, value_json) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value_json = excluded.value_json,
             updated_at = strftime('%Y-%m-%dT%H:%M:%fZ','now')",
        (key, json),
    )?;
    Ok(())
}

/// Removes `key`, restoring its default. Returns whether it was set.
pub fn reset(conn: &Connection, key: &str) -> Result<bool> {
    check_key(key)?;
    Ok(conn.execute("DELETE FROM settings WHERE key = ?1", [key])? > 0)
}

/// All stored settings as `(key, value)` pairs, sorted by key.
pub fn all(conn: &Connection) -> Result<Vec<(String, serde_json::Value)>> {
    let mut st = conn.prepare("SELECT key, value_json FROM settings ORDER BY key")?;
    let rows = st.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
    rows.map(|r| {
        let (k, v) = r?;
        Ok((k, serde_json::from_str(&v)?))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn defaults_then_set_then_reset() {
        let c = db::open_in_memory().unwrap();
        assert_eq!(get_or(&c, keys::RETENTION_DAYS, 0u32).unwrap(), 0);
        set(&c, keys::RETENTION_DAYS, &90u32).unwrap();
        set(&c, keys::RETENTION_DAYS, &30u32).unwrap();
        assert_eq!(get_or(&c, keys::RETENTION_DAYS, 0u32).unwrap(), 30);
        assert!(reset(&c, keys::RETENTION_DAYS).unwrap());
        assert!(!reset(&c, keys::RETENTION_DAYS).unwrap());
        assert_eq!(get_or(&c, keys::RETENTION_DAYS, 7u32).unwrap(), 7);
    }

    #[test]
    fn structured_values_roundtrip() {
        let c = db::open_in_memory().unwrap();
        set(&c, "ui.theme", "dark").unwrap();
        set(&c, keys::HASH_PROJECT_NAMES, &true).unwrap();
        let all = all(&c).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].0, keys::HASH_PROJECT_NAMES);
        assert_eq!(all[1].1, serde_json::json!("dark"));
    }

    #[test]
    fn rejects_bad_keys_and_wrong_types() {
        let c = db::open_in_memory().unwrap();
        assert!(set(&c, "", &1).is_err());
        assert!(set(&c, "bad key", &1).is_err());
        set(&c, keys::LIVE_PAUSED, "yes").unwrap();
        assert!(get_or(&c, keys::LIVE_PAUSED, false).is_err());
    }
}
