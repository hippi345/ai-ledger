//! Tray mode (spec item 31): a tray icon whose menu shows today's usage.

use chrono::{DateTime, Duration, Local, LocalResult, NaiveTime, SecondsFormat, TimeZone, Utc};
use ledger_core::stats::Totals;

/// The UTC span `[start, end)` of the local calendar day containing `now`,
/// as RFC 3339 strings matching the ledger's `ts_utc` column.
pub fn today_span<Tz: TimeZone>(now: &DateTime<Tz>) -> (String, String) {
    let tz = now.timezone();
    let day = now.date_naive();
    let midnight = |d: chrono::NaiveDate| -> DateTime<Utc> {
        let naive = d.and_time(NaiveTime::MIN);
        match tz.from_local_datetime(&naive) {
            LocalResult::Single(t) | LocalResult::Ambiguous(t, _) => t.with_timezone(&Utc),
            // Midnight skipped by a DST jump: the day starts an hour later.
            LocalResult::None => tz
                .from_local_datetime(&(naive + Duration::hours(1)))
                .earliest()
                .map_or_else(|| naive.and_utc(), |t| t.with_timezone(&Utc)),
        }
    };
    let fmt = |t: DateTime<Utc>| t.to_rfc3339_opts(SecondsFormat::Secs, true);
    let next = day.succ_opt().unwrap_or(day);
    (fmt(midnight(day)), fmt(midnight(next)))
}

/// Today's span in the machine's local time zone.
pub fn today_span_local() -> (String, String) {
    today_span(&Local::now())
}

/// The tray menu's summary line.
pub fn today_label(t: &Totals) -> String {
    format!(
        "Today: {} tokens · ${:.2}",
        group_thousands(t.tokens),
        t.cost_usd
    )
}

fn group_thousands(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    out
}
