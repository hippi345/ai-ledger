use ai_ledger_lib::tray::{today_label, today_span};
use chrono::{FixedOffset, TimeZone};
use ledger_core::stats::Totals;

#[test]
fn today_is_the_local_calendar_day_in_utc() {
    let ny = FixedOffset::west_opt(4 * 3600).unwrap();
    let now = ny.with_ymd_and_hms(2026, 10, 31, 23, 54, 0).unwrap();
    assert_eq!(
        today_span(&now),
        (
            "2026-10-31T04:00:00Z".to_owned(),
            "2026-11-01T04:00:00Z".to_owned()
        )
    );
}

#[test]
fn label_groups_tokens_and_rounds_cost() {
    let t = Totals {
        events: 3,
        tokens: 1_234_567,
        cost_usd: 0.425,
    };
    assert_eq!(today_label(&t), "Today: 1,234,567 tokens · $0.42");
    assert_eq!(today_label(&Totals::default()), "Today: 0 tokens · $0.00");
}
