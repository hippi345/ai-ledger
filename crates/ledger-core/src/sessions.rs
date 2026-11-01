//! Sessions (spec §D): a list of sessions with their usage, and one session's
//! timeline of requests and tool calls. Reads raw events, so it covers what
//! retention still keeps.

use rusqlite::Connection;
use serde::Serialize;

use crate::aggregate::Filters;
use crate::stats::event_filter_sql;
use crate::Result;

/// Sessions returned at most by [`list`], newest first.
pub const LIST_LIMIT: u32 = 500;

const TOKENS: &str = "input_tokens + output_tokens + cache_read_tokens + cache_write_tokens
                      + cache_write_1h_tokens + cache_write_12h_tokens";

/// One session's usage within the queried span.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SessionSummary {
    pub id: String,
    pub app: String,
    pub project: Option<String>,
    /// First and last event inside the span (RFC 3339 UTC).
    pub first_ts: String,
    pub last_ts: String,
    pub events: u64,
    pub tokens: u64,
    /// Sum of priced events; None when none of them has a price.
    pub cost_usd: Option<f64>,
    /// Distinct models used in the whole session, comma-separated, sorted.
    pub models: String,
}

/// Sessions with events in `since_utc <= ts_utc < until_utc`, narrowed by
/// app/model/project like aggregates, newest first, at most [`LIST_LIMIT`].
/// Events without a session are not listed.
pub fn list(
    conn: &Connection,
    since_utc: &str,
    until_utc: &str,
    filters: &Filters,
) -> Result<Vec<SessionSummary>> {
    let mut params: Vec<rusqlite::types::Value> =
        vec![since_utc.to_owned().into(), until_utc.to_owned().into()];
    let wh = format!(
        "session_id IS NOT NULL AND ts_utc >= ?1 AND ts_utc < ?2{}",
        event_filter_sql(filters, &mut params)
    );
    let sql = format!(
        "SELECT session_id, MIN(app), MIN(project), MIN(ts_utc), MAX(ts_utc), COUNT(*),
                SUM({TOKENS}), SUM(cost_usd_est),
                (SELECT group_concat(m, ',') FROM (SELECT DISTINCT model AS m FROM events e2
                  WHERE e2.session_id = events.session_id AND model IS NOT NULL ORDER BY m))
         FROM events WHERE {wh}
         GROUP BY session_id ORDER BY MAX(ts_utc) DESC, session_id LIMIT {LIST_LIMIT}"
    );
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map(rusqlite::params_from_iter(params), |r| {
        let n = |v: i64| u64::try_from(v).unwrap_or(0);
        Ok(SessionSummary {
            id: r.get(0)?,
            app: r.get(1)?,
            project: r.get(2)?,
            first_ts: r.get(3)?,
            last_ts: r.get(4)?,
            events: n(r.get(5)?),
            tokens: n(r.get(6)?),
            cost_usd: r.get(7)?,
            models: r.get::<_, Option<String>>(8)?.unwrap_or_default(),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// One timeline entry: a request (`tool` is None) or a tool call.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TimelineEntry {
    pub ts_utc: String,
    pub model: Option<String>,
    /// Request tokens; 0 for tool calls.
    pub tokens: u64,
    pub cost_usd: Option<f64>,
    pub tool: Option<String>,
    pub mcp_server: Option<String>,
    pub duration_ms: Option<u64>,
    /// False for a failed tool call; always true for requests.
    pub ok: bool,
}

/// A session's requests and tool calls in time order (requests before the
/// tool calls they made at the same instant).
pub fn timeline(conn: &Connection, session_id: &str) -> Result<Vec<TimelineEntry>> {
    let sql = format!(
        "SELECT ts_utc, model, {TOKENS}, cost_usd_est, NULL, NULL, NULL, 1, 0 AS k, id
           FROM events WHERE session_id = ?1
         UNION ALL
         SELECT t.ts_utc, e.model, 0, NULL, t.tool, t.mcp_server, t.duration_ms, t.ok, 1, t.id
           FROM tool_calls t JOIN events e ON e.id = t.event_id WHERE e.session_id = ?1
         ORDER BY 1, k, 10"
    );
    let mut st = conn.prepare(&sql)?;
    let rows = st.query_map([session_id], |r| {
        Ok(TimelineEntry {
            ts_utc: r.get(0)?,
            model: r.get(1)?,
            tokens: u64::try_from(r.get::<_, i64>(2)?).unwrap_or(0),
            cost_usd: r.get(3)?,
            tool: r.get(4)?,
            mcp_server: r.get(5)?,
            duration_ms: r
                .get::<_, Option<i64>>(6)?
                .map(|v| u64::try_from(v).unwrap_or(0)),
            ok: r.get::<_, i64>(7)? != 0,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ingest::{upsert_events, NewEvent, NewToolCall};
    use crate::reader::{SourceCandidate, Tier};
    use crate::sources;

    fn ev(ts: &str, session: Option<&str>, model: &str, hash: &str) -> NewEvent {
        NewEvent {
            ts_utc: ts.into(),
            app: "claude_code".into(),
            model: Some(model.into()),
            project: Some("p".into()),
            session_id: session.map(Into::into),
            input_tokens: 10,
            output_tokens: 1,
            raw_hash: hash.into(),
            ..Default::default()
        }
    }

    fn setup() -> Connection {
        let mut c = db::open_in_memory().unwrap();
        let cand = SourceCandidate {
            reader_id: "t".into(),
            label: "T".into(),
            config: serde_json::json!({}),
        };
        let s = sources::add(&c, &cand, Tier::Native).unwrap();
        let mut a = ev("2026-11-01T10:00:00Z", Some("s1"), "opus", "a");
        a.tool_calls = vec![NewToolCall {
            ts_utc: "2026-11-01T10:00:00Z".into(),
            tool: "Read".into(),
            mcp_server: None,
            duration_ms: Some(40),
            ok: false,
        }];
        upsert_events(
            &mut c,
            s.id,
            &[
                a,
                ev("2026-11-01T10:05:00Z", Some("s1"), "haiku", "b"),
                ev("2026-11-01T12:00:00Z", Some("s2"), "opus", "c"),
                ev("2026-11-01T13:00:00Z", None, "opus", "d"),
                ev("2026-11-03T00:00:00Z", Some("s3"), "opus", "e"),
            ],
        )
        .unwrap();
        c.execute(
            "UPDATE events SET cost_usd_est = 0.25 WHERE raw_hash = 'a'",
            [],
        )
        .unwrap();
        c
    }

    const SPAN: (&str, &str) = ("2026-11-01T00:00:00Z", "2026-11-02T00:00:00Z");

    #[test]
    fn lists_sessions_in_span_newest_first() {
        let c = setup();
        let l = list(&c, SPAN.0, SPAN.1, &Filters::default()).unwrap();
        assert_eq!(
            l.iter().map(|s| s.id.as_str()).collect::<Vec<_>>(),
            ["s2", "s1"]
        );
        let s1 = &l[1];
        assert_eq!(s1.events, 2);
        assert_eq!(s1.tokens, 22);
        assert_eq!(s1.cost_usd, Some(0.25));
        assert_eq!(s1.models, "haiku,opus");
        assert_eq!(
            (s1.first_ts.as_str(), s1.last_ts.as_str()),
            ("2026-11-01T10:00:00Z", "2026-11-01T10:05:00Z")
        );
        assert_eq!(l[0].cost_usd, None);
    }

    #[test]
    fn list_applies_filters() {
        let c = setup();
        let f = Filters {
            model: vec!["haiku".into()],
            ..Default::default()
        };
        let l = list(&c, SPAN.0, SPAN.1, &f).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!((l[0].id.as_str(), l[0].events), ("s1", 1));
    }

    #[test]
    fn timeline_interleaves_requests_and_tool_calls() {
        let c = setup();
        let t = timeline(&c, "s1").unwrap();
        assert_eq!(t.len(), 3);
        assert_eq!((t[0].tool.as_deref(), t[0].tokens), (None, 11));
        assert_eq!(t[1].tool.as_deref(), Some("Read"));
        assert!(!t[1].ok);
        assert_eq!(t[1].duration_ms, Some(40));
        assert_eq!(t[2].model.as_deref(), Some("haiku"));
        assert!(timeline(&c, "nope").unwrap().is_empty());
    }
}
