//! Period-over-period deltas (spec item 36): compare a range with the
//! previous range of equal length.

use serde::Serialize;

use crate::{Error, Result};

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's algorithm).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

fn parse(date: &str) -> Result<i64> {
    let bad = || Error::InvalidInput(format!("bad date {date:?}, want YYYY-MM-DD"));
    let mut it = date
        .splitn(3, '-')
        .map(|p| p.parse::<i64>().map_err(|_| bad()));
    let (y, m, d) = (
        it.next().ok_or_else(bad)??,
        it.next().ok_or_else(bad)??,
        it.next().ok_or_else(bad)??,
    );
    if date.len() != 10 || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    let days = days_from_civil(y, m, d);
    if civil_from_days(days) != (y, m, d) {
        return Err(bad());
    }
    Ok(days)
}

fn format(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The range of equal length just before `[since, until)` (UTC dates).
pub fn previous_range(since: &str, until: &str) -> Result<(String, String)> {
    let (a, b) = (parse(since)?, parse(until)?);
    if b <= a {
        return Err(Error::InvalidInput(format!("empty range {since}..{until}")));
    }
    Ok((format(a - (b - a)), since.to_owned()))
}

/// A metric now versus the previous period.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Delta {
    pub current: f64,
    pub previous: f64,
    pub abs: f64,
    /// Percent change; `None` when the previous period was zero.
    pub pct: Option<f64>,
}

pub fn delta(current: f64, previous: f64) -> Delta {
    Delta {
        current,
        previous,
        abs: current - previous,
        pct: (previous != 0.0).then(|| (current - previous) / previous * 100.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previous_range_has_equal_length() {
        let p = |a, b| previous_range(a, b).unwrap();
        assert_eq!(
            p("2026-10-26", "2026-11-02"),
            ("2026-10-19".into(), "2026-10-26".into())
        );
        assert_eq!(
            p("2026-03-01", "2026-03-02"),
            ("2026-02-28".into(), "2026-03-01".into())
        );
        assert_eq!(
            p("2024-03-01", "2024-03-02"),
            ("2024-02-29".into(), "2024-03-01".into())
        );
        assert_eq!(
            p("2027-01-01", "2027-01-31"),
            ("2026-12-02".into(), "2027-01-01".into())
        );
    }

    #[test]
    fn rejects_bad_ranges() {
        assert!(previous_range("2026-11-02", "2026-11-02").is_err());
        assert!(previous_range("2026-02-30", "2026-03-02").is_err());
        assert!(previous_range("26-1-1", "2026-03-02").is_err());
    }

    #[test]
    fn deltas() {
        assert_eq!(delta(130.0, 100.0).pct, Some(30.0));
        assert_eq!(delta(50.0, 100.0).abs, -50.0);
        assert_eq!(delta(5.0, 0.0).pct, None);
    }

    #[test]
    fn civil_round_trip() {
        for d in [-1000, 0, 1, 19_000, 20_757, 100_000] {
            let (y, m, dd) = civil_from_days(d);
            assert_eq!(days_from_civil(y, m, dd), d);
        }
    }
}
