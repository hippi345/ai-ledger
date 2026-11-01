//! Property tests (spec §9): every rollup grain sums to the raw events, and
//! incremental refreshes end up identical to a full rebuild.

#![allow(clippy::unwrap_used)]

use ledger_core::ingest::{upsert_events, NewEvent};
use ledger_core::reader::{SourceCandidate, Tier};
use ledger_core::rollup::{self, Grain};
use ledger_core::{db, sources};
use proptest::prelude::*;
use rusqlite::Connection;

fn event() -> impl Strategy<Value = (u32, u8, u8, u32, u32)> {
    // (minutes into a ~100-day span, model, project, input, output)
    (0u32..144_000, 0u8..3, 0u8..3, 0u32..10_000, 0u32..10_000)
}

fn to_event(i: usize, (min, m, p, input, output): (u32, u8, u8, u32, u32)) -> NewEvent {
    let t = 1_790_000_000 + i64::from(min) * 60;
    NewEvent {
        ts_utc: chrono_like(t),
        app: "cursor".into(),
        model: (m > 0).then(|| format!("m{m}")),
        project: (p > 0).then(|| format!("p{p}")),
        input_tokens: input.into(),
        output_tokens: output.into(),
        requests: 1,
        raw_hash: format!("h{i}"),
        ..Default::default()
    }
}

/// Unix seconds to RFC 3339 UTC via SQLite, so no date crate is needed.
fn chrono_like(t: i64) -> String {
    Connection::open_in_memory()
        .unwrap()
        .query_row(
            "SELECT strftime('%Y-%m-%dT%H:%M:%SZ', ?1, 'unixepoch')",
            [t],
            |r| r.get(0),
        )
        .unwrap()
}

fn ledger() -> (Connection, i64) {
    let c = db::open_in_memory().unwrap();
    let cand = SourceCandidate {
        reader_id: "t".into(),
        label: "T".into(),
        config: serde_json::json!({}),
    };
    let id = sources::add(&c, &cand, Tier::Native).unwrap().id;
    (c, id)
}

fn sums(c: &Connection, table: &str) -> (i64, i64, i64) {
    c.query_row(
        &format!("SELECT COALESCE(SUM(events),0), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0) FROM {table}"),
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .unwrap()
}

fn dump(c: &Connection, table: &str) -> Vec<(String, String, String, String, i64, i64)> {
    let mut st = c
        .prepare(&format!(
            "SELECT bucket, app, model, project, events, input_tokens FROM {table} ORDER BY 1,2,3,4"
        ))
        .unwrap();
    st.query_map([], |r| {
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
    .collect()
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(48))]

    #[test]
    fn every_grain_sums_to_events(raw in prop::collection::vec(event(), 0..60)) {
        let (mut c, s) = ledger();
        let evs: Vec<_> = raw.into_iter().enumerate().map(|(i, e)| to_event(i, e)).collect();
        upsert_events(&mut c, s, &evs).unwrap();
        rollup::rebuild_all(&mut c).unwrap();
        let want: (i64, i64, i64) = c
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0) FROM events",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        for g in Grain::ALL {
            prop_assert_eq!(sums(&c, g.table()), want);
        }
    }

    #[test]
    fn incremental_matches_rebuild(raw in prop::collection::vec(event(), 1..60), split in 1usize..6) {
        let (mut c, s) = ledger();
        let evs: Vec<_> = raw.into_iter().enumerate().map(|(i, e)| to_event(i, e)).collect();
        for chunk in evs.chunks(split) {
            upsert_events(&mut c, s, chunk).unwrap();
            rollup::refresh_for(&mut c, chunk).unwrap();
        }
        let incremental: Vec<_> = Grain::ALL.iter().map(|g| dump(&c, g.table())).collect();
        rollup::rebuild_all(&mut c).unwrap();
        let full: Vec<_> = Grain::ALL.iter().map(|g| dump(&c, g.table())).collect();
        prop_assert_eq!(incremental, full);
    }
}
