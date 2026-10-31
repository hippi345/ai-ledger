//! The `sources` table: which readers are enabled and with what config.

use rusqlite::{Connection, OptionalExtension};
use serde::Serialize;

use crate::reader::{Cursor, SourceCandidate, SourceCfg, Tier};
use crate::{Error, Result};

/// A configured source as stored.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Source {
    pub id: i64,
    pub reader_id: String,
    pub tier: i64,
    pub config: serde_json::Value,
    pub enabled: bool,
}

impl Source {
    /// The config handed to the reader.
    pub fn cfg(&self) -> SourceCfg {
        SourceCfg {
            source_id: self.id,
            reader_id: self.reader_id.clone(),
            config: self.config.clone(),
        }
    }
}

fn row(r: &rusqlite::Row<'_>) -> rusqlite::Result<(i64, String, i64, String, bool)> {
    Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
}

fn to_source(t: (i64, String, i64, String, bool)) -> Result<Source> {
    Ok(Source {
        id: t.0,
        reader_id: t.1,
        tier: t.2,
        config: serde_json::from_str(&t.3)?,
        enabled: t.4,
    })
}

/// All sources, by id.
pub fn list(conn: &Connection) -> Result<Vec<Source>> {
    let mut st =
        conn.prepare("SELECT id, reader_id, tier, config_json, enabled FROM sources ORDER BY id")?;
    let rows: Vec<_> = st.query_map([], row)?.collect::<rusqlite::Result<_>>()?;
    rows.into_iter().map(to_source).collect()
}

/// Adds a source for a detected candidate, or returns the existing one with
/// the same reader and config (so re-running detection never duplicates).
pub fn add(conn: &Connection, c: &SourceCandidate, tier: Tier) -> Result<Source> {
    let config = serde_json::to_string(&c.config)?;
    let existing = conn
        .query_row(
            "SELECT id, reader_id, tier, config_json, enabled FROM sources
             WHERE reader_id = ?1 AND config_json = ?2",
            (&c.reader_id, &config),
            row,
        )
        .optional()?;
    if let Some(t) = existing {
        return to_source(t);
    }
    conn.execute(
        "INSERT INTO sources (reader_id, tier, config_json) VALUES (?1, ?2, ?3)",
        (&c.reader_id, tier.as_i64(), &config),
    )?;
    let id = conn.last_insert_rowid();
    to_source(conn.query_row(
        "SELECT id, reader_id, tier, config_json, enabled FROM sources WHERE id = ?1",
        [id],
        row,
    )?)
}

/// Enables or disables a source. Errors if it does not exist.
pub fn set_enabled(conn: &Connection, id: i64, enabled: bool) -> Result<()> {
    match conn.execute(
        "UPDATE sources SET enabled = ?2 WHERE id = ?1",
        (id, enabled),
    )? {
        0 => Err(Error::InvalidInput(format!("no source {id}"))),
        _ => Ok(()),
    }
}

/// Saved resume cursor for a source.
pub fn cursor(conn: &Connection, id: i64) -> Result<Option<Cursor>> {
    let c: Option<Option<String>> = conn
        .query_row("SELECT last_cursor FROM sources WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    Ok(c.flatten().map(Cursor))
}

/// Stores the resume cursor after a successful read.
pub fn save_cursor(conn: &Connection, id: i64, c: &Cursor) -> Result<()> {
    conn.execute(
        "UPDATE sources SET last_cursor = ?2 WHERE id = ?1",
        (id, &c.0),
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn cand(root: &str) -> SourceCandidate {
        SourceCandidate {
            reader_id: "cursor".into(),
            label: "Cursor".into(),
            config: serde_json::json!({ "root": root }),
        }
    }

    #[test]
    fn add_is_deduplicated() {
        let c = db::open_in_memory().unwrap();
        let a = add(&c, &cand("/a"), Tier::Native).unwrap();
        let again = add(&c, &cand("/a"), Tier::Native).unwrap();
        let b = add(&c, &cand("/b"), Tier::Native).unwrap();
        assert_eq!(a, again);
        assert_ne!(a.id, b.id);
        assert!(a.enabled);
        assert_eq!(a.cfg().config["root"], "/a");
        assert_eq!(list(&c).unwrap().len(), 2);
    }

    #[test]
    fn enable_toggle_and_cursor() {
        let c = db::open_in_memory().unwrap();
        let s = add(&c, &cand("/a"), Tier::Native).unwrap();
        set_enabled(&c, s.id, false).unwrap();
        assert!(!list(&c).unwrap()[0].enabled);
        assert!(set_enabled(&c, 99, true).is_err());
        assert_eq!(cursor(&c, s.id).unwrap(), None);
        save_cursor(&c, s.id, &Cursor("42".into())).unwrap();
        assert_eq!(cursor(&c, s.id).unwrap(), Some(Cursor("42".into())));
    }
}
