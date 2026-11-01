//! Incremental rollups (spec item 33): day, week and month buckets by app,
//! model and project, rebuilt from `events` for just the buckets a change
//! touches. Buckets are UTC dates; weeks start on Monday.

use rusqlite::Connection;

use crate::Result;

/// A rollup granularity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Grain {
    Day,
    Week,
    Month,
}

impl Grain {
    pub const ALL: [Grain; 3] = [Grain::Day, Grain::Week, Grain::Month];

    pub fn table(self) -> &'static str {
        match self {
            Grain::Day => "rollups_day",
            Grain::Week => "rollups_week",
            Grain::Month => "rollups_month",
        }
    }

    /// The tool/MCP call rollup table for this grain.
    pub fn tools_table(self) -> &'static str {
        match self {
            Grain::Day => "rollups_tools_day",
            Grain::Week => "rollups_tools_week",
            Grain::Month => "rollups_tools_month",
        }
    }

    /// SQL expression giving the bucket start date of timestamp `col`.
    fn bucket_sql(self, col: &str) -> String {
        match self {
            Grain::Day => format!("date({col})"),
            Grain::Week => format!("date({col}, 'weekday 0', '-6 days')"),
            Grain::Month => format!("strftime('%Y-%m-01', {col})"),
        }
    }
}

/// The retention watermark: buckets starting before it are frozen, because
/// some of their raw events may have been pruned. `""` when nothing was pruned.
fn frozen_before(conn: &Connection) -> Result<String> {
    crate::settings::get_or(conn, crate::settings::keys::PRUNED_BEFORE, String::new())
}

/// Recomputes every bucket that overlaps `[since_utc, until_utc]` (inclusive
/// RFC 3339 timestamps), at all grains, in one transaction. Buckets frozen by
/// retention are left as they are.
pub fn refresh(conn: &mut Connection, since_utc: &str, until_utc: &str) -> Result<()> {
    let frozen = frozen_before(conn)?;
    let tx = conn.transaction()?;
    for g in Grain::ALL {
        let t = g.table();
        let lo = g.bucket_sql("?1");
        let hi = g.bucket_sql("?2");
        let b = g.bucket_sql("ts_utc");
        tx.execute(
            &format!("DELETE FROM {t} WHERE bucket BETWEEN {lo} AND {hi} AND bucket >= ?3"),
            [since_utc, until_utc, &frozen],
        )?;
        tx.execute(
            &format!(
                "INSERT INTO {t} (bucket, app, model, project, events, requests, input_tokens,
                    output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, estimated_events)
                 SELECT {b}, app, COALESCE(model, ''), COALESCE(project, ''), COUNT(*),
                    SUM(requests), SUM(input_tokens), SUM(output_tokens), SUM(cache_read_tokens),
                    SUM(cache_write_tokens + cache_write_1h_tokens + cache_write_12h_tokens),
                    COALESCE(SUM(cost_usd_est), 0.0), SUM(estimated)
                 FROM events WHERE {b} BETWEEN {lo} AND {hi} AND {b} >= ?3
                 GROUP BY 1, 2, 3, 4"
            ),
            [since_utc, until_utc, &frozen],
        )?;
        let tt = g.tools_table();
        let tb = g.bucket_sql("tc.ts_utc");
        tx.execute(
            &format!("DELETE FROM {tt} WHERE bucket BETWEEN {lo} AND {hi} AND bucket >= ?3"),
            [since_utc, until_utc, &frozen],
        )?;
        tx.execute(
            &format!(
                "INSERT INTO {tt} (bucket, app, tool, mcp_server, calls, failures, duration_ms)
                 SELECT {tb}, e.app, tc.tool, COALESCE(tc.mcp_server, ''), COUNT(*),
                    SUM(1 - tc.ok), COALESCE(SUM(tc.duration_ms), 0)
                 FROM tool_calls tc JOIN events e ON e.id = tc.event_id
                 WHERE {tb} BETWEEN {lo} AND {hi} AND {tb} >= ?3
                 GROUP BY 1, 2, 3, 4"
            ),
            [since_utc, until_utc, &frozen],
        )?;
    }
    tx.commit()?;
    Ok(())
}

/// Rebuilds all rollups from events (after re-pricing or a migration),
/// except buckets frozen by retention, which raw events can no longer rebuild.
pub fn rebuild_all(conn: &mut Connection) -> Result<()> {
    let frozen = frozen_before(conn)?;
    let span: (Option<String>, Option<String>) =
        conn.query_row("SELECT MIN(ts_utc), MAX(ts_utc) FROM events", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })?;
    {
        let tx = conn.transaction()?;
        for g in Grain::ALL {
            for t in [g.table(), g.tools_table()] {
                tx.execute(&format!("DELETE FROM {t} WHERE bucket >= ?1"), [&frozen])?;
            }
        }
        tx.commit()?;
    }
    match span {
        (Some(lo), Some(hi)) => refresh(conn, &lo, &hi),
        _ => Ok(()),
    }
}

/// Builds rollups once for a ledger that has events (or tool calls) but no
/// rollups for them yet (databases created before those rollups existed).
pub fn ensure_built(conn: &mut Connection) -> Result<()> {
    let needed: bool = conn.query_row(
        "SELECT (EXISTS (SELECT 1 FROM events) AND NOT EXISTS (SELECT 1 FROM rollups_day))
             OR (EXISTS (SELECT 1 FROM tool_calls) AND NOT EXISTS (SELECT 1 FROM rollups_tools_day))",
        [],
        |r| r.get(0),
    )?;
    if needed {
        tracing::info!("building rollups for existing events");
        rebuild_all(conn)?;
    }
    Ok(())
}

/// Refreshes the buckets covering `events` (no-op when empty).
pub fn refresh_for(conn: &mut Connection, events: &[crate::ingest::NewEvent]) -> Result<()> {
    let all = || {
        events.iter().flat_map(|e| {
            std::iter::once(e.ts_utc.as_str()).chain(e.tool_calls.iter().map(|t| t.ts_utc.as_str()))
        })
    };
    let lo = all().min();
    let hi = all().max();
    match (lo, hi) {
        (Some(lo), Some(hi)) => refresh(conn, lo, hi),
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ingest::{upsert_events, NewEvent};
    use crate::reader::{SourceCandidate, Tier};
    use crate::sources;

    fn ev(ts: &str, model: &str, input: u64, hash: &str) -> NewEvent {
        NewEvent {
            ts_utc: ts.into(),
            app: "cursor".into(),
            model: Some(model.into()),
            input_tokens: input,
            requests: 1,
            raw_hash: hash.into(),
            ..Default::default()
        }
    }

    fn setup(events: &[NewEvent]) -> Connection {
        let mut c = db::open_in_memory().unwrap();
        let cand = SourceCandidate {
            reader_id: "t".into(),
            label: "T".into(),
            config: serde_json::json!({}),
        };
        let s = sources::add(&c, &cand, Tier::Native).unwrap();
        upsert_events(&mut c, s.id, events).unwrap();
        c
    }

    fn rows(c: &Connection, g: Grain) -> Vec<(String, String, i64, i64)> {
        let mut st = c
            .prepare(&format!(
                "SELECT bucket, model, events, input_tokens FROM {} ORDER BY 1, 2",
                g.table()
            ))
            .unwrap();
        st.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn buckets_by_day_week_and_month() {
        // 2026-10-31 is a Saturday, 2026-11-02 a Monday.
        let mut c = setup(&[
            ev("2026-10-31T10:00:00Z", "a", 1, "1"),
            ev("2026-10-31T11:00:00Z", "a", 2, "2"),
            ev("2026-11-01T09:00:00Z", "b", 4, "3"),
            ev("2026-11-02T09:00:00Z", "a", 8, "4"),
        ]);
        rebuild_all(&mut c).unwrap();
        let s = |v: &str| v.to_owned();
        assert_eq!(
            rows(&c, Grain::Day),
            vec![
                (s("2026-10-31"), s("a"), 2, 3),
                (s("2026-11-01"), s("b"), 1, 4),
                (s("2026-11-02"), s("a"), 1, 8)
            ]
        );
        assert_eq!(
            rows(&c, Grain::Week),
            vec![
                (s("2026-10-26"), s("a"), 2, 3),
                (s("2026-10-26"), s("b"), 1, 4),
                (s("2026-11-02"), s("a"), 1, 8)
            ]
        );
        assert_eq!(
            rows(&c, Grain::Month),
            vec![
                (s("2026-10-01"), s("a"), 2, 3),
                (s("2026-11-01"), s("a"), 1, 8),
                (s("2026-11-01"), s("b"), 1, 4)
            ]
        );
    }

    #[test]
    fn refresh_updates_only_touched_buckets() {
        let mut c = setup(&[ev("2026-10-01T10:00:00Z", "a", 1, "1")]);
        rebuild_all(&mut c).unwrap();
        let s = sources::list(&c).unwrap()[0].id;
        upsert_events(&mut c, s, &[ev("2026-10-05T10:00:00Z", "a", 5, "2")]).unwrap();
        // Corrupt an untouched bucket: refresh must leave it alone.
        c.execute(
            "UPDATE rollups_day SET events = 99 WHERE bucket = '2026-10-01'",
            [],
        )
        .unwrap();
        refresh(&mut c, "2026-10-05T10:00:00Z", "2026-10-05T10:00:00Z").unwrap();
        let day = rows(&c, Grain::Day);
        assert_eq!(day[0].2, 99);
        assert_eq!(day[1], ("2026-10-05".to_owned(), "a".to_owned(), 1, 5));
        // The month bucket spans both days and is recomputed from events.
        assert_eq!(rows(&c, Grain::Month)[0].3, 6);
    }

    #[test]
    fn ensure_built_fills_empty_rollups_once() {
        let mut c = setup(&[ev("2026-10-01T10:00:00Z", "a", 1, "1")]);
        assert!(rows(&c, Grain::Day).is_empty());
        ensure_built(&mut c).unwrap();
        assert_eq!(rows(&c, Grain::Day).len(), 1);
    }

    #[test]
    fn pruned_buckets_stay_frozen() {
        let mut c = setup(&[
            ev("2026-10-01T10:00:00Z", "a", 1, "1"),
            ev("2026-10-20T10:00:00Z", "a", 2, "2"),
        ]);
        rebuild_all(&mut c).unwrap();
        // Prune everything before Oct 10: the October month bucket now has
        // fewer raw events than it summarizes.
        crate::retention::prune_older_than(&c, "2026-10-30T00:00:00Z", 20).unwrap();
        let s = sources::list(&c).unwrap()[0].id;
        let late = [ev("2026-10-25T10:00:00Z", "a", 4, "3")];
        upsert_events(&mut c, s, &late).unwrap();
        refresh_for(&mut c, &late).unwrap();
        rebuild_all(&mut c).unwrap();
        let month = rows(&c, Grain::Month);
        assert_eq!(month[0].3, 1 + 2, "frozen month keeps pruned totals");
        let day = rows(&c, Grain::Day);
        assert_eq!(
            day.iter().map(|r| r.3).sum::<i64>(),
            1 + 2 + 4,
            "pruned day kept, new day added"
        );
    }

    #[test]
    fn tool_calls_roll_up_by_tool_and_server() {
        use crate::ingest::NewToolCall;
        let call = |ts: &str, tool: &str, mcp: Option<&str>, ok: bool| NewToolCall {
            ts_utc: ts.into(),
            tool: tool.into(),
            mcp_server: mcp.map(Into::into),
            duration_ms: Some(10),
            ok,
        };
        let mut e = ev("2026-10-01T10:00:00Z", "a", 1, "1");
        e.tool_calls = vec![
            call("2026-10-01T10:00:01Z", "read_file", None, true),
            call("2026-10-01T10:00:02Z", "read_file", None, false),
            call("2026-10-02T00:00:01Z", "search", Some("github"), true),
        ];
        let mut c = setup(&[]);
        let s = sources::list(&c).unwrap()[0].id;
        let evs = [e];
        upsert_events(&mut c, s, &evs).unwrap();
        refresh_for(&mut c, &evs).unwrap();
        let mut st = c
            .prepare("SELECT bucket, tool, mcp_server, calls, failures, duration_ms FROM rollups_tools_day ORDER BY 1, 2")
            .unwrap();
        let got: Vec<(String, String, String, i64, i64, i64)> = st
            .query_map([], |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                ))
            })
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        let s = |v: &str| v.to_owned();
        assert_eq!(
            got,
            vec![
                (s("2026-10-01"), s("read_file"), s(""), 2, 1, 20),
                (s("2026-10-02"), s("search"), s("github"), 1, 0, 10)
            ]
        );
        let month: i64 = c
            .query_row("SELECT SUM(calls) FROM rollups_tools_month", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(month, 3);
    }

    #[test]
    fn empty_ledger_rebuilds_to_nothing() {
        let mut c = setup(&[]);
        rebuild_all(&mut c).unwrap();
        assert!(rows(&c, Grain::Day).is_empty());
    }
}
