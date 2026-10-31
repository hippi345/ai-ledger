//! Idempotent event ingest.
//!
//! Every event carries a `raw_hash`: a SHA-256 of the reader's stable identity
//! for that record (for example file path + byte offset + record id). Ingesting
//! the same record twice never double counts. If a reader re-reads a record
//! whose values changed (say, token counts filled in later), the stored row is
//! updated in place and its tool calls are replaced.

use rusqlite::{Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::Result;

/// Hex SHA-256 over the given identity parts, separated by a NUL byte so
/// `["ab","c"]` and `["a","bc"]` hash differently.
pub fn raw_hash<I, P>(parts: I) -> String
where
    I: IntoIterator<Item = P>,
    P: AsRef<[u8]>,
{
    let mut h = Sha256::new();
    for (i, p) in parts.into_iter().enumerate() {
        if i > 0 {
            h.update([0u8]);
        }
        h.update(p.as_ref());
    }
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// A tool or MCP call attached to an event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NewToolCall {
    pub ts_utc: String,
    pub tool: String,
    pub mcp_server: Option<String>,
    pub duration_ms: Option<u64>,
    pub ok: bool,
}

/// A usage event as produced by a reader.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct NewEvent {
    pub ts_utc: String,
    pub app: String,
    pub model: Option<String>,
    pub project: Option<String>,
    pub session_id: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: u64,
    pub cache_write_tokens: u64,
    pub requests: u64,
    pub estimated: bool,
    pub raw_hash: String,
    pub tool_calls: Vec<NewToolCall>,
}

/// Outcome counts for one ingest batch.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IngestStats {
    pub inserted: usize,
    pub updated: usize,
    pub unchanged: usize,
}

fn to_i64(v: u64) -> i64 {
    i64::try_from(v).unwrap_or(i64::MAX)
}

type StoredRow = (
    i64,
    Option<String>,
    Option<String>,
    Option<String>,
    [i64; 5],
    bool,
);

fn same(stored: &StoredRow, e: &NewEvent) -> bool {
    let (_, model, project, session, n, est) = stored;
    *model == e.model
        && *project == e.project
        && *session == e.session_id
        && *n
            == [
                to_i64(e.input_tokens),
                to_i64(e.output_tokens),
                to_i64(e.cache_read_tokens),
                to_i64(e.cache_write_tokens),
                to_i64(e.requests),
            ]
        && *est == e.estimated
}

fn insert_tool_calls(conn: &Connection, event_id: i64, calls: &[NewToolCall]) -> Result<()> {
    let mut st = conn.prepare_cached(
        "INSERT INTO tool_calls (event_id, ts_utc, tool, mcp_server, duration_ms, ok)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
    )?;
    for c in calls {
        st.execute((
            event_id,
            &c.ts_utc,
            &c.tool,
            &c.mcp_server,
            c.duration_ms.map(to_i64),
            c.ok,
        ))?;
    }
    Ok(())
}

/// Upserts `events` for `source_id` in one transaction.
pub fn upsert_events(
    conn: &mut Connection,
    source_id: i64,
    events: &[NewEvent],
) -> Result<IngestStats> {
    let tx = conn.transaction()?;
    let mut stats = IngestStats::default();
    for e in events {
        let stored: Option<StoredRow> = tx
            .query_row(
                "SELECT id, model, project, session_id, input_tokens, output_tokens,
                        cache_read_tokens, cache_write_tokens, requests, estimated
                 FROM events WHERE raw_hash = ?1",
                [&e.raw_hash],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        [r.get(4)?, r.get(5)?, r.get(6)?, r.get(7)?, r.get(8)?],
                        r.get(9)?,
                    ))
                },
            )
            .optional()?;
        let params = (
            &e.ts_utc,
            &e.app,
            &e.model,
            &e.project,
            &e.session_id,
            to_i64(e.input_tokens),
            to_i64(e.output_tokens),
            to_i64(e.cache_read_tokens),
            to_i64(e.cache_write_tokens),
            to_i64(e.requests),
            e.estimated,
            &e.raw_hash,
        );
        match stored {
            Some(row) if same(&row, e) => stats.unchanged += 1,
            Some(row) => {
                tx.execute(
                    "UPDATE events SET ts_utc=?1, app=?2, model=?3, project=?4, session_id=?5,
                        input_tokens=?6, output_tokens=?7, cache_read_tokens=?8,
                        cache_write_tokens=?9, requests=?10, estimated=?11
                     WHERE raw_hash=?12",
                    params,
                )?;
                tx.execute("DELETE FROM tool_calls WHERE event_id = ?1", [row.0])?;
                insert_tool_calls(&tx, row.0, &e.tool_calls)?;
                stats.updated += 1;
            }
            None => {
                tx.execute(
                    "INSERT INTO events (ts_utc, app, model, project, session_id, input_tokens,
                        output_tokens, cache_read_tokens, cache_write_tokens, requests,
                        estimated, raw_hash, source_id)
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                    (
                        params.0, params.1, params.2, params.3, params.4, params.5, params.6,
                        params.7, params.8, params.9, params.10, params.11, source_id,
                    ),
                )?;
                insert_tool_calls(&tx, tx.last_insert_rowid(), &e.tool_calls)?;
                stats.inserted += 1;
            }
        }
    }
    tx.commit()?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> Connection {
        let c = db::open_in_memory().unwrap();
        c.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('test', 1)",
            [],
        )
        .unwrap();
        c
    }

    fn ev(key: &str, input: u64) -> NewEvent {
        NewEvent {
            ts_utc: "2026-10-31T12:00:00Z".into(),
            app: "cursor".into(),
            model: Some("m".into()),
            input_tokens: input,
            requests: 1,
            raw_hash: raw_hash(["file.jsonl", key]),
            tool_calls: vec![NewToolCall {
                ts_utc: "2026-10-31T12:00:01Z".into(),
                tool: "read_file".into(),
                mcp_server: None,
                duration_ms: Some(5),
                ok: true,
            }],
            ..Default::default()
        }
    }

    fn count(c: &Connection, table: &str) -> i64 {
        c.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn hash_is_stable_and_separated() {
        assert_eq!(raw_hash(["a", "b"]), raw_hash(["a", "b"]));
        assert_ne!(raw_hash(["ab", "c"]), raw_hash(["a", "bc"]));
        assert_eq!(raw_hash(["x"]).len(), 64);
    }

    #[test]
    fn reingest_never_double_counts() {
        let mut c = setup();
        let batch = vec![ev("1", 10), ev("2", 20)];
        let s1 = upsert_events(&mut c, 1, &batch).unwrap();
        assert_eq!(
            s1,
            IngestStats {
                inserted: 2,
                updated: 0,
                unchanged: 0
            }
        );
        let s2 = upsert_events(&mut c, 1, &batch).unwrap();
        assert_eq!(
            s2,
            IngestStats {
                inserted: 0,
                updated: 0,
                unchanged: 2
            }
        );
        assert_eq!(count(&c, "events"), 2);
        assert_eq!(count(&c, "tool_calls"), 2);
        let sum: i64 = c
            .query_row("SELECT SUM(input_tokens) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(sum, 30);
    }

    #[test]
    fn changed_record_updates_in_place() {
        let mut c = setup();
        upsert_events(&mut c, 1, &[ev("1", 10)]).unwrap();
        let s = upsert_events(&mut c, 1, &[ev("1", 15)]).unwrap();
        assert_eq!(s.updated, 1);
        assert_eq!(count(&c, "events"), 1);
        assert_eq!(
            count(&c, "tool_calls"),
            1,
            "tool calls are replaced, not duplicated"
        );
        let v: i64 = c
            .query_row("SELECT input_tokens FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 15);
    }

    #[test]
    fn batch_is_atomic() {
        let mut c = setup();
        let mut bad = ev("2", 1);
        bad.ts_utc = "t".into();
        // source 99 does not exist, so the whole batch must roll back.
        assert!(upsert_events(&mut c, 99, &[ev("1", 1), bad]).is_err());
        assert_eq!(count(&c, "events"), 0);
    }
}
