//! Quick totals for the tray menu and status displays.

use rusqlite::Connection;
use serde::Serialize;

use crate::Result;

/// Usage totals over a time span.
#[derive(Debug, Clone, Copy, Default, PartialEq, Serialize)]
pub struct Totals {
    pub events: u64,
    pub tokens: u64,
    /// Sum of priced events; events without a price add nothing.
    pub cost_usd: f64,
}

/// Totals for events with `since_utc <= ts_utc < until_utc` (RFC 3339 UTC).
pub fn totals_between(conn: &Connection, since_utc: &str, until_utc: &str) -> Result<Totals> {
    let (events, tokens, cost): (i64, i64, f64) = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens
                             + cache_write_1h_tokens + cache_write_12h_tokens), 0),
                COALESCE(SUM(cost_usd_est), 0.0)
         FROM events WHERE ts_utc >= ?1 AND ts_utc < ?2",
        [since_utc, until_utc],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let u = |v: i64| u64::try_from(v).unwrap_or(0);
    Ok(Totals {
        events: u(events),
        tokens: u(tokens),
        cost_usd: cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ingest::{upsert_events, NewEvent};
    use crate::reader::{SourceCandidate, Tier};
    use crate::sources;

    fn ev(ts: &str, input: u64, hash: &str) -> NewEvent {
        NewEvent {
            ts_utc: ts.into(),
            app: "t".into(),
            input_tokens: input,
            output_tokens: 1,
            raw_hash: hash.into(),
            ..Default::default()
        }
    }

    #[test]
    fn sums_only_the_span() {
        let mut c = db::open_in_memory().unwrap();
        let cand = SourceCandidate {
            reader_id: "t".into(),
            label: "T".into(),
            config: serde_json::json!({}),
        };
        let s = sources::add(&c, &cand, Tier::Native).unwrap();
        upsert_events(
            &mut c,
            s.id,
            &[
                ev("2026-10-30T23:59:59Z", 100, "a"),
                ev("2026-10-31T00:00:00Z", 10, "b"),
                ev("2026-10-31T12:00:00Z", 20, "c"),
                ev("2026-11-01T00:00:00Z", 1000, "d"),
            ],
        )
        .unwrap();
        c.execute(
            "UPDATE events SET cost_usd_est = 0.5 WHERE raw_hash = 'b'",
            [],
        )
        .unwrap();
        let t = totals_between(&c, "2026-10-31T00:00:00Z", "2026-11-01T00:00:00Z").unwrap();
        assert_eq!(
            t,
            Totals {
                events: 2,
                tokens: 32,
                cost_usd: 0.5
            }
        );
    }

    #[test]
    fn empty_span_is_zero() {
        let c = db::open_in_memory().unwrap();
        let t = totals_between(&c, "2026-10-31T00:00:00Z", "2026-11-01T00:00:00Z").unwrap();
        assert_eq!(t, Totals::default());
    }
}
