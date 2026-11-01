#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::aggregate::*;
use ai_ledger_lib::state::AppState;

fn state() -> AppState {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    for (bucket, model, input) in [
        ("2026-10-20", "a", 10),
        ("2026-10-27", "a", 30),
        ("2026-10-28", "b", 20),
        ("2026-10-29", "c", 5),
    ] {
        s.db()
            .execute(
                "INSERT INTO rollups_day VALUES (?1, 'cursor', ?2, '', 1, 1, ?3, 0, 0, 0, 0, 0)",
                rusqlite::params![bucket, model, input],
            )
            .unwrap();
    }
    s
}

fn query() -> AggQuery {
    AggQuery {
        since: "2026-10-26".into(),
        until: "2026-11-02".into(),
        grain: AggGrain::Day,
        group_by: vec![AggDim::Model],
        filters: AggFilters::default(),
        metric: AggMetric::Tokens,
        top: Some(1),
        compare: true,
    }
}

#[test]
fn top_n_with_previous_period() {
    let r = run(&state(), &query()).unwrap();
    assert_eq!(r.total, 55.0);
    assert_eq!(r.rows.len(), 2);
    assert_eq!(r.rows[0].keys, vec!["a"]);
    assert_eq!(
        r.rows[1],
        AggRow {
            keys: vec!["Other".into()],
            value: 25.0
        }
    );
    assert_eq!(r.previous_total, Some(10.0));
    assert_eq!(r.change_pct, Some(450.0));
}

#[test]
fn bad_range_is_an_error_when_comparing() {
    let mut q = query();
    q.until = q.since.clone();
    assert!(run(&state(), &q).is_err());
}
