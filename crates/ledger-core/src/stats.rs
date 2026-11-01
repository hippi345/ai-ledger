//! Quick totals for the tray menu and status displays.

use rusqlite::Connection;
use serde::Serialize;

use crate::aggregate::Filters;
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

/// `" AND ..."` clauses narrowing raw `events` by app/model/project (unknown
/// model/project match `""`), pushing their values onto `params`. Empty when
/// no filter is set.
pub(crate) fn event_filter_sql(
    filters: &Filters,
    params: &mut Vec<rusqlite::types::Value>,
) -> String {
    let mut wh = String::new();
    for (col, vals) in [
        ("app", &filters.app),
        ("model", &filters.model),
        ("project", &filters.project),
    ] {
        if vals.is_empty() {
            continue;
        }
        let marks: Vec<String> = vals
            .iter()
            .map(|v| {
                params.push(v.clone().into());
                format!("?{}", params.len())
            })
            .collect();
        wh.push_str(&format!(
            " AND COALESCE({col}, '') IN ({})",
            marks.join(", ")
        ));
    }
    wh
}

/// One cell of the hour-of-day x day-of-week activity heatmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct HeatCell {
    /// 0 = Monday ... 6 = Sunday.
    pub weekday: u8,
    /// 0..=23, UTC.
    pub hour: u8,
    pub events: u64,
    pub tokens: u64,
}

/// Activity by UTC weekday and hour for events with `since_utc <= ts_utc <
/// until_utc`, narrowed by the same app/model/project filters as aggregates
/// (unknown model/project match `""`). Reads raw events (rollups are
/// day-grained), so it covers what retention still keeps. Only non-empty
/// cells are returned.
pub fn heatmap(
    conn: &Connection,
    since_utc: &str,
    until_utc: &str,
    filters: &Filters,
) -> Result<Vec<HeatCell>> {
    let mut params: Vec<rusqlite::types::Value> =
        vec![since_utc.to_owned().into(), until_utc.to_owned().into()];
    let wh = format!(
        "ts_utc >= ?1 AND ts_utc < ?2{}",
        event_filter_sql(filters, &mut params)
    );
    let mut st = conn.prepare(&format!(
        "SELECT (CAST(strftime('%w', ts_utc) AS INTEGER) + 6) % 7, CAST(strftime('%H', ts_utc) AS INTEGER),
                COUNT(*),
                SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens
                    + cache_write_1h_tokens + cache_write_12h_tokens)
         FROM events WHERE {wh}
         GROUP BY 1, 2 ORDER BY 1, 2"
    ))?;
    let rows = st.query_map(rusqlite::params_from_iter(params), |r| {
        let n = |v: i64| u64::try_from(v).unwrap_or(0);
        Ok(HeatCell {
            weekday: u8::try_from(r.get::<_, i64>(0)?).unwrap_or(0),
            hour: u8::try_from(r.get::<_, i64>(1)?).unwrap_or(0),
            events: n(r.get(2)?),
            tokens: n(r.get(3)?),
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
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

    #[test]
    fn heatmap_buckets_by_weekday_and_hour() {
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
                ev("2026-11-02T09:15:00Z", 10, "a"), // Monday 09
                ev("2026-11-02T09:45:00Z", 20, "b"), // Monday 09
                ev("2026-11-08T23:00:00Z", 5, "c"),  // Sunday 23
                ev("2026-11-09T00:00:00Z", 5, "d"),  // outside
            ],
        )
        .unwrap();
        let h = heatmap(
            &c,
            "2026-11-02T00:00:00Z",
            "2026-11-09T00:00:00Z",
            &Filters::default(),
        )
        .unwrap();
        assert_eq!(
            h,
            vec![
                HeatCell {
                    weekday: 0,
                    hour: 9,
                    events: 2,
                    tokens: 32
                },
                HeatCell {
                    weekday: 6,
                    hour: 23,
                    events: 1,
                    tokens: 6
                }
            ]
        );
    }

    #[test]
    fn heatmap_applies_filters_with_unknown_as_empty() {
        let mut c = db::open_in_memory().unwrap();
        let cand = SourceCandidate {
            reader_id: "t".into(),
            label: "T".into(),
            config: serde_json::json!({}),
        };
        let s = sources::add(&c, &cand, Tier::Native).unwrap();
        let mut a = ev("2026-11-02T09:00:00Z", 10, "a");
        a.model = Some("gpt-5".into());
        let mut b = ev("2026-11-02T10:00:00Z", 20, "b");
        b.app = "other".into();
        upsert_events(&mut c, s.id, &[a, b]).unwrap();
        let span = ("2026-11-02T00:00:00Z", "2026-11-03T00:00:00Z");
        let hours = |f: Filters| -> Vec<u8> {
            heatmap(&c, span.0, span.1, &f)
                .unwrap()
                .iter()
                .map(|h| h.hour)
                .collect()
        };
        let f = |app: &[&str], model: &[&str]| Filters {
            app: app.iter().map(|s| s.to_string()).collect(),
            model: model.iter().map(|s| s.to_string()).collect(),
            project: vec![],
        };
        assert_eq!(hours(f(&[], &["gpt-5"])), vec![9]);
        assert_eq!(hours(f(&["other"], &[])), vec![10]);
        assert_eq!(hours(f(&[], &[""])), vec![10]);
        assert_eq!(hours(f(&["t"], &[""])), Vec::<u8>::new());
    }
}
