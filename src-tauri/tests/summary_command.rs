#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::summary::{cards_for, periods};
use ai_ledger_lib::state::AppState;
use chrono::NaiveDate;

fn d(s: &str) -> NaiveDate {
    s.parse().unwrap()
}

#[test]
fn periods_start_on_monday_and_the_first() {
    let p = periods(d("2026-11-04")); // a Wednesday
    assert_eq!(p[0], ("Today", d("2026-11-04"), d("2026-11-05")));
    assert_eq!(p[1], ("This week", d("2026-11-02"), d("2026-11-05")));
    assert_eq!(p[2], ("This month", d("2026-11-01"), d("2026-11-05")));
}

#[test]
fn cards_describe_each_period() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    for (bucket, app, model, input, cost) in [
        ("2026-11-03", "cursor", "claude-sonnet", 3_000_000, 10.0),
        ("2026-11-04", "cursor", "gpt-5", 1_000_000, 2.0),
        ("2026-10-27", "cursor", "gpt-5", 1_000_000, 1.0),
    ] {
        s.db()
            .execute(
                "INSERT INTO rollups_day VALUES (?1, ?2, ?3, '', 1, 1, ?4, 0, 0, 0, ?5, 0)",
                rusqlite::params![bucket, app, model, input, cost],
            )
            .unwrap();
    }
    let c = cards_for(&s, d("2026-11-04")).unwrap();
    assert_eq!(
        c[0].text,
        "Today: 1M tokens, ~$2.00, mostly cursor + gpt-5, down 67%."
    );
    assert_eq!(
        c[1].text, "This week: 4M tokens, ~$12, mostly cursor + claude-sonnet.",
        "no usage in the previous 3 days, so no change wording"
    );
    assert_eq!(c[2].label, "This month");
}
