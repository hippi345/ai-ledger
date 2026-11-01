//! Manual usage entry (Tier 3).
//!
//! Lets the user record usage the readers cannot see (another machine, a web
//! chat). Entries live in a dedicated `manual` source and can be edited by
//! re-submitting with the same `entry_id`, or deleted.

use rusqlite::{Connection, OptionalExtension};

use crate::ingest::{raw_hash, upsert_events, IngestStats, NewEvent};
use crate::{Error, Result};

pub const READER_ID: &str = "manual";

/// A user-entered usage record.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ManualEntry {
    /// Caller-chosen stable id (e.g. a UUID from the UI); identifies the entry for edits.
    pub entry_id: String,
    pub ts_utc: String,
    pub app: String,
    pub model: Option<String>,
    pub project: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub requests: u64,
}

/// Returns the id of the `manual` source, creating it on first use.
pub fn source_id(conn: &Connection) -> Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM sources WHERE reader_id = ?1",
            [READER_ID],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO sources (reader_id, tier) VALUES (?1, 3)",
        [READER_ID],
    )?;
    Ok(conn.last_insert_rowid())
}

fn hash(entry_id: &str) -> String {
    raw_hash([READER_ID, entry_id])
}

fn validate(e: &ManualEntry) -> Result<()> {
    let bad = |m: &str| Err(Error::InvalidInput(m.into()));
    if e.entry_id.trim().is_empty() {
        return bad("entry id is required");
    }
    if e.app.trim().is_empty() {
        return bad("app is required");
    }
    let b = e.ts_utc.as_bytes();
    if b.len() < 20 || b[4] != b'-' || b[10] != b'T' || !e.ts_utc.ends_with('Z') {
        return bad("timestamp must be ISO-8601 UTC, e.g. 2026-10-31T12:00:00Z");
    }
    if e.input_tokens == 0 && e.output_tokens == 0 && e.requests == 0 {
        return bad("entry records no usage");
    }
    Ok(())
}

/// Adds or updates (same `entry_id`) a manual entry.
pub fn save(conn: &mut Connection, e: &ManualEntry) -> Result<IngestStats> {
    validate(e)?;
    let src = source_id(conn)?;
    let ev = NewEvent {
        ts_utc: e.ts_utc.clone(),
        app: e.app.trim().to_owned(),
        model: e.model.clone().filter(|m| !m.trim().is_empty()),
        project: e.project.clone().filter(|p| !p.trim().is_empty()),
        input_tokens: e.input_tokens,
        output_tokens: e.output_tokens,
        requests: e.requests,
        raw_hash: hash(&e.entry_id),
        ..Default::default()
    };
    let ev = [ev];
    let stats = upsert_events(conn, src, &ev)?;
    crate::rollup::refresh_for(conn, &ev)?;
    Ok(stats)
}

/// Deletes a manual entry. Returns whether it existed. Never touches reader data.
pub fn delete(conn: &Connection, entry_id: &str) -> Result<bool> {
    let n = conn.execute(
        "DELETE FROM events WHERE raw_hash = ?1
           AND source_id IN (SELECT id FROM sources WHERE reader_id = ?2)",
        (hash(entry_id), READER_ID),
    )?;
    Ok(n > 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn entry(id: &str, out: u64) -> ManualEntry {
        ManualEntry {
            entry_id: id.into(),
            ts_utc: "2026-10-31T12:00:00Z".into(),
            app: "chatgpt-web".into(),
            model: Some("gpt-5".into()),
            output_tokens: out,
            requests: 1,
            ..Default::default()
        }
    }

    #[test]
    fn save_edit_delete() {
        let mut c = db::open_in_memory().unwrap();
        assert_eq!(save(&mut c, &entry("e1", 10)).unwrap().inserted, 1);
        assert_eq!(save(&mut c, &entry("e1", 25)).unwrap().updated, 1);
        save(&mut c, &entry("e2", 5)).unwrap();
        let (n, out): (i64, i64) = c
            .query_row("SELECT COUNT(*), SUM(output_tokens) FROM events", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!((n, out), (2, 30));
        let tier: i64 = c
            .query_row(
                "SELECT tier FROM sources WHERE reader_id='manual'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(tier, 3);
        assert!(delete(&c, "e1").unwrap());
        assert!(!delete(&c, "e1").unwrap());
        assert_eq!(
            source_id(&c).unwrap(),
            source_id(&c).unwrap(),
            "single manual source"
        );
    }

    #[test]
    fn validation() {
        let mut c = db::open_in_memory().unwrap();
        let mut e = entry("", 1);
        assert!(save(&mut c, &e).is_err());
        e.entry_id = "x".into();
        e.ts_utc = "yesterday".into();
        assert!(save(&mut c, &e).is_err());
        let mut e = entry("x", 0);
        e.requests = 0;
        assert!(save(&mut c, &e).is_err());
        let mut e = entry("x", 1);
        e.app = " ".into();
        assert!(save(&mut c, &e).is_err());
    }
}
