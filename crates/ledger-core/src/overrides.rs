//! User price overrides (spec §6): rows with `source = 'user'` that win over
//! bundled rows with the same effective date. Every change re-prices all
//! events so totals stay consistent.

use rusqlite::Connection;

use crate::{cost, Error, Result};

/// A user-entered price, USD per million tokens.
#[derive(Debug, Clone, PartialEq)]
pub struct UserPrice {
    pub model: String,
    /// `YYYY-MM-DD` (UTC).
    pub effective_from: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

fn valid_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() == 10
        && b[4] == b'-'
        && b[7] == b'-'
        && b.iter()
            .enumerate()
            .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit())
}

fn validate(p: &UserPrice) -> Result<()> {
    if p.model.trim().is_empty() {
        return Err(Error::InvalidInput("model is required".into()));
    }
    if !valid_date(&p.effective_from) {
        return Err(Error::InvalidInput(format!(
            "effective_from must be YYYY-MM-DD, got {:?}",
            p.effective_from
        )));
    }
    if [p.input, p.output, p.cache_read, p.cache_write]
        .iter()
        .any(|r| !r.is_finite() || *r < 0.0)
    {
        return Err(Error::InvalidInput(
            "rates must be non-negative numbers".into(),
        ));
    }
    Ok(())
}

/// All user overrides, by model then date.
pub fn list(conn: &Connection) -> Result<Vec<UserPrice>> {
    let mut st = conn.prepare(
        "SELECT model, substr(effective_from, 1, 10), input_per_mtok, output_per_mtok,
                cache_read_per_mtok, cache_write_per_mtok
         FROM prices WHERE source = 'user' ORDER BY model, effective_from",
    )?;
    let rows = st.query_map([], |r| {
        Ok(UserPrice {
            model: r.get(0)?,
            effective_from: r.get(1)?,
            input: r.get(2)?,
            output: r.get(3)?,
            cache_read: r.get(4)?,
            cache_write: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Adds or replaces an override, then re-prices every event.
/// Returns how many events were re-priced.
pub fn set(conn: &mut Connection, p: &UserPrice) -> Result<usize> {
    validate(p)?;
    conn.execute(
        "INSERT INTO prices (model, effective_from, input_per_mtok, output_per_mtok,
             cache_read_per_mtok, cache_write_per_mtok, source)
         VALUES (?1, ?2 || 'T00:00:00Z', ?3, ?4, ?5, ?6, 'user')
         ON CONFLICT (model, effective_from, source) DO UPDATE SET
             input_per_mtok = excluded.input_per_mtok,
             output_per_mtok = excluded.output_per_mtok,
             cache_read_per_mtok = excluded.cache_read_per_mtok,
             cache_write_per_mtok = excluded.cache_write_per_mtok",
        (
            p.model.trim(),
            &p.effective_from,
            p.input,
            p.output,
            p.cache_read,
            p.cache_write,
        ),
    )?;
    cost::recompute_all(conn)
}

/// Removes an override (bundled prices apply again), then re-prices every
/// event. Returns how many events were re-priced; errors if none matched.
pub fn remove(conn: &mut Connection, model: &str, effective_from: &str) -> Result<usize> {
    let n = conn.execute(
        "DELETE FROM prices WHERE source = 'user' AND model = ?1
             AND effective_from = ?2 || 'T00:00:00Z'",
        (model, effective_from),
    )?;
    if n == 0 {
        return Err(Error::InvalidInput(format!(
            "no override for {model} from {effective_from}"
        )));
    }
    cost::recompute_all(conn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{raw_hash, upsert_events, NewEvent};
    use crate::{db, prices};

    fn setup() -> Connection {
        let mut c = db::open_in_memory().unwrap();
        prices::sync_bundled(&mut c).unwrap();
        c.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
            .unwrap();
        let e = NewEvent {
            ts_utc: "2026-10-31T12:00:00Z".into(),
            app: "cursor".into(),
            model: Some("my-model".into()),
            input_tokens: 1_000_000,
            requests: 1,
            raw_hash: raw_hash(["x"]),
            ..Default::default()
        };
        upsert_events(&mut c, 1, &[e]).unwrap();
        c
    }

    fn event_cost(c: &Connection) -> Option<f64> {
        c.query_row("SELECT cost_usd_est FROM events", [], |r| r.get(0))
            .unwrap()
    }

    fn price(input: f64) -> UserPrice {
        UserPrice {
            model: "my-model".into(),
            effective_from: "2026-01-01".into(),
            input,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
        }
    }

    #[test]
    fn set_update_and_remove_reprice_events() {
        let mut c = setup();
        assert_eq!(event_cost(&c), None, "unknown model is unpriced");
        assert_eq!(set(&mut c, &price(2.0)).unwrap(), 1);
        assert_eq!(event_cost(&c), Some(2.0));
        set(&mut c, &price(3.0)).unwrap();
        assert_eq!(event_cost(&c), Some(3.0));
        assert_eq!(list(&c).unwrap(), vec![price(3.0)]);
        remove(&mut c, "my-model", "2026-01-01").unwrap();
        assert_eq!(event_cost(&c), None);
        assert!(list(&c).unwrap().is_empty());
        assert!(remove(&mut c, "my-model", "2026-01-01").is_err());
    }

    #[test]
    fn override_beats_bundled_on_the_same_date() {
        let mut c = setup();
        let b: String = c
            .query_row(
                "SELECT substr(effective_from, 1, 10) FROM prices WHERE model = 'gpt-5'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let p = UserPrice {
            model: "gpt-5".into(),
            effective_from: b,
            ..price(9.0)
        };
        set(&mut c, &p).unwrap();
        let r = cost::rate_at(&c, "gpt-5", "2026-10-31T00:00:00Z")
            .unwrap()
            .unwrap();
        assert_eq!((r.input, r.price_version.as_str()), (9.0, "user"));
    }

    #[test]
    fn rejects_bad_input() {
        let mut c = setup();
        for bad in [
            UserPrice {
                model: " ".into(),
                ..price(1.0)
            },
            UserPrice {
                effective_from: "2026-1-1".into(),
                ..price(1.0)
            },
            price(-1.0),
            price(f64::NAN),
        ] {
            assert!(set(&mut c, &bad).is_err(), "{bad:?}");
        }
        assert!(list(&c).unwrap().is_empty());
    }
}
