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
    /// Standard (5-minute) cache writes.
    pub cache_write_tokens: u64,
    /// Extended (1-hour) cache writes, priced separately.
    pub cache_write_1h_tokens: u64,
    /// 12-hour cache writes (OpenAI's extended cache), priced separately.
    pub cache_write_12h_tokens: u64,
    pub requests: u64,
    /// Vendor service tier, when the source records it.
    pub service_tier: Option<crate::cost::ServiceTier>,
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
    [i64; 7],
    bool,
    Option<String>,
);

fn same(stored: &StoredRow, e: &NewEvent) -> bool {
    let (_, model, project, session, n, est, tier) = stored;
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
                to_i64(e.cache_write_1h_tokens),
                to_i64(e.cache_write_12h_tokens),
            ]
        && *est == e.estimated
        && tier.as_deref() == e.service_tier.map(crate::cost::ServiceTier::as_str)
}

fn set_extras(conn: &Connection, id: i64, e: &NewEvent) -> Result<()> {
    conn.prepare_cached(
        "UPDATE events SET cache_write_1h_tokens = ?2, cache_write_12h_tokens = ?3,
                service_tier = ?4
         WHERE id = ?1",
    )?
    .execute((
        id,
        to_i64(e.cache_write_1h_tokens),
        to_i64(e.cache_write_12h_tokens),
        e.service_tier.map(crate::cost::ServiceTier::as_str),
    ))?;
    Ok(())
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

/// Creates the session row an event refers to, or widens its time span.
fn touch_session(conn: &Connection, e: &NewEvent) -> Result<()> {
    let Some(id) = &e.session_id else {
        return Ok(());
    };
    conn.prepare_cached(
        "INSERT INTO sessions (id, app, project, started, ended) VALUES (?1, ?2, ?3, ?4, ?4)
         ON CONFLICT(id) DO UPDATE SET
             started = MIN(started, excluded.started),
             ended   = MAX(COALESCE(ended, excluded.ended), excluded.ended),
             project = COALESCE(project, excluded.project)",
    )?
    .execute((id, &e.app, &e.project, &e.ts_utc))?;
    Ok(())
}

/// Upserts `events` for `source_id` in one transaction. Sessions referenced by
/// events are created on first sight and their start/end span is widened.
pub fn upsert_events(
    conn: &mut Connection,
    source_id: i64,
    events: &[NewEvent],
) -> Result<IngestStats> {
    // Hash project names on the way in when the privacy setting is on.
    let hashed: Vec<NewEvent>;
    let events = if crate::privacy::hashing_on(conn)? {
        hashed = events
            .iter()
            .cloned()
            .map(|mut e| {
                e.project = e.project.map(|p| crate::privacy::hash_project(&p));
                e
            })
            .collect();
        &hashed[..]
    } else {
        events
    };
    let tx = conn.transaction()?;
    let mut stats = IngestStats::default();
    for e in events {
        let stored: Option<StoredRow> = tx
            .query_row(
                "SELECT id, model, project, session_id, input_tokens, output_tokens,
                        cache_read_tokens, cache_write_tokens, requests, cache_write_1h_tokens,
                        cache_write_12h_tokens, estimated, service_tier
                 FROM events WHERE raw_hash = ?1",
                [&e.raw_hash],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        [
                            r.get(4)?,
                            r.get(5)?,
                            r.get(6)?,
                            r.get(7)?,
                            r.get(8)?,
                            r.get(9)?,
                            r.get(10)?,
                        ],
                        r.get(11)?,
                        r.get(12)?,
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
        touch_session(&tx, e)?;
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
                set_extras(&tx, row.0, e)?;
                tx.execute("DELETE FROM tool_calls WHERE event_id = ?1", [row.0])?;
                insert_tool_calls(&tx, row.0, &e.tool_calls)?;
                crate::cost::price_event(&tx, row.0)?;
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
                let id = tx.last_insert_rowid();
                set_extras(&tx, id, e)?;
                insert_tool_calls(&tx, id, &e.tool_calls)?;
                crate::cost::price_event(&tx, id)?;
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
    fn sessions_are_created_and_widened() {
        let mut c = setup();
        let mut a = ev("1", 1);
        a.session_id = Some("s".into());
        a.ts_utc = "2026-10-31T12:05:00Z".into();
        let mut b = ev("2", 1);
        b.session_id = Some("s".into());
        b.ts_utc = "2026-10-31T12:00:00Z".into();
        b.project = Some("ai-ledger".into());
        upsert_events(&mut c, 1, &[a, b]).unwrap();
        let row: (String, String, Option<String>) = c
            .query_row(
                "SELECT started, ended, project FROM sessions WHERE id='s'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(row.0, "2026-10-31T12:00:00Z");
        assert_eq!(row.1, "2026-10-31T12:05:00Z");
        assert_eq!(row.2.as_deref(), Some("ai-ledger"));
        assert_eq!(count(&c, "sessions"), 1);
    }

    #[test]
    fn one_hour_cache_writes_are_stored_and_compared() {
        let mut c = setup();
        let mut e = ev("1", 1);
        e.cache_write_1h_tokens = 50;
        upsert_events(&mut c, 1, std::slice::from_ref(&e)).unwrap();
        assert_eq!(
            upsert_events(&mut c, 1, std::slice::from_ref(&e))
                .unwrap()
                .unchanged,
            1
        );
        e.cache_write_1h_tokens = 70;
        assert_eq!(upsert_events(&mut c, 1, &[e]).unwrap().updated, 1);
        let v: i64 = c
            .query_row("SELECT cache_write_1h_tokens FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 70);
    }

    #[test]
    fn twelve_hour_cache_writes_are_stored_and_compared() {
        let mut c = setup();
        let mut e = ev("1", 1);
        e.cache_write_12h_tokens = 40;
        upsert_events(&mut c, 1, std::slice::from_ref(&e)).unwrap();
        e.cache_write_12h_tokens = 90;
        assert_eq!(upsert_events(&mut c, 1, &[e]).unwrap().updated, 1);
        let v: i64 = c
            .query_row("SELECT cache_write_12h_tokens FROM events", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(v, 90);
    }

    #[test]
    fn service_tier_is_stored_and_compared() {
        let mut c = setup();
        let mut e = ev("1", 1);
        e.service_tier = Some(crate::cost::ServiceTier::Batch);
        upsert_events(&mut c, 1, std::slice::from_ref(&e)).unwrap();
        assert_eq!(
            upsert_events(&mut c, 1, std::slice::from_ref(&e))
                .unwrap()
                .unchanged,
            1
        );
        e.service_tier = Some(crate::cost::ServiceTier::Fast);
        assert_eq!(upsert_events(&mut c, 1, &[e]).unwrap().updated, 1);
        let v: String = c
            .query_row("SELECT service_tier FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, "fast");
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

    #[test]
    fn hashes_new_project_names_when_enabled() {
        let mut c = db::open_in_memory().unwrap();
        c.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
            .unwrap();
        let ev = |hash: &str| NewEvent {
            ts_utc: "2026-11-01T00:00:00Z".into(),
            app: "cursor".into(),
            project: Some("secret".into()),
            session_id: Some(format!("s-{hash}")),
            raw_hash: hash.into(),
            ..Default::default()
        };
        upsert_events(&mut c, 1, &[ev("a")]).unwrap();
        crate::settings::set(&c, crate::settings::keys::HASH_PROJECT_NAMES, &true).unwrap();
        upsert_events(&mut c, 1, &[ev("b")]).unwrap();
        let p = |h: &str| -> String {
            c.query_row("SELECT project FROM events WHERE raw_hash = ?1", [h], |r| {
                r.get(0)
            })
            .unwrap()
        };
        assert_eq!(p("a"), "secret", "existing rows are untouched");
        assert_eq!(p("b"), crate::privacy::hash_project("secret"));
        let s: String = c
            .query_row("SELECT project FROM sessions WHERE id = 's-b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(s, crate::privacy::hash_project("secret"));
    }
}
