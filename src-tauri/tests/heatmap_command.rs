#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::aggregate::AggFilters;
use ai_ledger_lib::commands::heatmap::run;
use ai_ledger_lib::state::AppState;

#[test]
fn date_range_covers_whole_days() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    s.db()
        .execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
        .unwrap();
    for (i, ts) in [
        "2026-11-02T00:00:00Z",
        "2026-11-02T23:59:59Z",
        "2026-11-03T00:00:00Z",
    ]
    .iter()
    .enumerate()
    {
        s.db()
            .execute(
                "INSERT INTO events (source_id, ts_utc, app, input_tokens, raw_hash) VALUES (1, ?1, 'cursor', 7, ?2)",
                rusqlite::params![ts, i.to_string()],
            )
            .unwrap();
    }
    let cells = run(&s, "2026-11-02", "2026-11-03", &AggFilters::default()).unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(
        (cells[0].weekday, cells[0].hour, cells[0].tokens),
        (0, 0, 7.0)
    );
    assert_eq!((cells[1].weekday, cells[1].hour), (0, 23));
}

#[test]
fn filters_narrow_the_cells() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    s.db()
        .execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
        .unwrap();
    s.db()
        .execute(
            "INSERT INTO events (source_id, ts_utc, app, input_tokens, raw_hash) VALUES
             (1, '2026-11-02T09:00:00Z', 'cursor', 1, 'a'),
             (1, '2026-11-02T10:00:00Z', 'claude_code', 1, 'b')",
            [],
        )
        .unwrap();
    let f = AggFilters {
        app: vec!["claude_code".into()],
        ..Default::default()
    };
    let cells = run(&s, "2026-11-02", "2026-11-03", &f).unwrap();
    assert_eq!(cells.len(), 1);
    assert_eq!(cells[0].hour, 10);
}
