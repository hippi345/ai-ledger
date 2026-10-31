//! Cost estimation from the local price table (spec §6).
//!
//! cost = Σ tokens × rate / 1e6, using the rate effective at the event's
//! timestamp. Model names are matched case-insensitively: exact model, then
//! alias, then the longest known name that prefixes the event's model (so
//! dated ids like `claude-sonnet-4-5-20250929` resolve). When a user row and
//! a bundled row share the latest effective date, the user row wins.
//! Unknown models get no cost (`NULL`, shown as "—").

use rusqlite::{Connection, OptionalExtension};

use crate::{prices, settings, Result};

/// Rates in USD per million tokens, plus the version they came from.
#[derive(Debug, Clone, PartialEq)]
pub struct Rate {
    pub model: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// `"user"` or the bundled table version.
    pub price_version: String,
}

/// Token counts for one event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
}

/// USD cost of `t` at `r`.
pub fn cost(r: &Rate, t: Tokens) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let f = |n: u64| n as f64;
    (f(t.input) * r.input
        + f(t.output) * r.output
        + f(t.cache_read) * r.cache_read
        + f(t.cache_write) * r.cache_write)
        / 1_000_000.0
}

/// Canonical priced model for `name`, if any.
pub fn resolve_model(conn: &Connection, name: &str) -> Result<Option<String>> {
    let n = name.trim().to_ascii_lowercase();
    if n.is_empty() {
        return Ok(None);
    }
    let found: Option<String> = conn
        .query_row(
            "SELECT model FROM (
                 SELECT model, lower(model) AS k FROM prices
                 UNION SELECT model, alias AS k FROM price_aliases
             )
             WHERE k = ?1 OR (substr(?1, 1, length(k)) = k
                              AND substr(?1, length(k) + 1, 1) IN ('-', '@', ':', '.'))
             ORDER BY length(k) DESC LIMIT 1",
            [&n],
            |r| r.get(0),
        )
        .optional()?;
    Ok(found)
}

/// Rate for `model` effective at `ts_utc`.
pub fn rate_at(conn: &Connection, model: &str, ts_utc: &str) -> Result<Option<Rate>> {
    let Some(canon) = resolve_model(conn, model)? else {
        return Ok(None);
    };
    let bundled_version: Option<String> = settings::get_or(conn, prices::LOADED_VERSION_KEY, None)?;
    let row = conn
        .query_row(
            "SELECT input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, source
             FROM prices WHERE model = ?1 AND effective_from <= ?2
             ORDER BY effective_from DESC, source = 'user' DESC LIMIT 1",
            (&canon, ts_utc),
            |r| {
                Ok((
                    r.get::<_, f64>(0)?,
                    r.get::<_, f64>(1)?,
                    r.get::<_, f64>(2)?,
                    r.get::<_, f64>(3)?,
                    r.get::<_, String>(4)?,
                ))
            },
        )
        .optional()?;
    Ok(
        row.map(|(input, output, cache_read, cache_write, source)| Rate {
            model: canon,
            input,
            output,
            cache_read,
            cache_write,
            price_version: if source == "user" {
                "user".into()
            } else {
                bundled_version.clone().unwrap_or_else(|| "bundled".into())
            },
        }),
    )
}

/// Computes and stores `cost_usd_est` / `price_version` for one event.
pub fn price_event(conn: &Connection, event_id: i64) -> Result<()> {
    let (model, ts, t): (Option<String>, String, Tokens) = conn.query_row(
        "SELECT model, ts_utc, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens
         FROM events WHERE id = ?1",
        [event_id],
        |r| {
            let n = |i| r.get::<_, i64>(i).map(|v| u64::try_from(v).unwrap_or(0));
            Ok((
                r.get(0)?,
                r.get(1)?,
                Tokens {
                    input: n(2)?,
                    output: n(3)?,
                    cache_read: n(4)?,
                    cache_write: n(5)?,
                },
            ))
        },
    )?;
    let rate = match model {
        Some(m) => rate_at(conn, &m, &ts)?,
        None => None,
    };
    conn.execute(
        "UPDATE events SET cost_usd_est = ?2, price_version = ?3 WHERE id = ?1",
        (
            event_id,
            rate.as_ref().map(|r| cost(r, t)),
            rate.map(|r| r.price_version),
        ),
    )?;
    Ok(())
}

/// Re-prices every event (after a price edit or a new bundled table).
/// Returns how many events were re-priced.
pub fn recompute_all(conn: &mut Connection) -> Result<usize> {
    let tx = conn.transaction()?;
    let ids: Vec<i64> = {
        let mut st = tx.prepare("SELECT id FROM events ORDER BY id")?;
        let rows = st.query_map([], |r| r.get(0))?;
        rows.collect::<rusqlite::Result<_>>()?
    };
    for id in &ids {
        price_event(&tx, *id)?;
    }
    tx.commit()?;
    tracing::info!(events = ids.len(), "recomputed costs");
    Ok(ids.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;
    use crate::ingest::{raw_hash, upsert_events, NewEvent};

    fn setup() -> Connection {
        let mut c = db::open_in_memory().unwrap();
        prices::sync_bundled(&mut c).unwrap();
        c.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
            .unwrap();
        c
    }

    #[test]
    fn resolves_exact_alias_and_dated_ids() {
        let c = setup();
        let r = |n| resolve_model(&c, n).unwrap();
        assert_eq!(r("GPT-5").as_deref(), Some("gpt-5"));
        assert_eq!(
            r("gpt-5-mini").as_deref(),
            Some("gpt-5-mini"),
            "longest match wins"
        );
        assert_eq!(r("claude-4-sonnet").as_deref(), Some("claude-sonnet-4-5"));
        assert_eq!(
            r("claude-sonnet-4-5-20250929").as_deref(),
            Some("claude-sonnet-4-5")
        );
        assert_eq!(r("gpt-50"), None, "prefix must end at a separator");
        assert_eq!(r("mystery"), None);
        assert_eq!(r("gpt-5.3-codex").as_deref(), Some("gpt-5.3-codex"));
        assert_eq!(r("gpt-5.3-codex-high").as_deref(), Some("gpt-5.3-codex"));
        assert_eq!(
            r("gpt-5.4-mini").as_deref(),
            Some("gpt-5.4-mini"),
            "not gpt-5.4"
        );
    }

    #[test]
    fn recompute_applies_new_prices_to_history() {
        let mut c = setup();
        let e = NewEvent {
            ts_utc: "2026-10-31T00:00:00Z".into(),
            app: "x".into(),
            model: Some("gpt-5".into()),
            input_tokens: 1_000_000,
            requests: 1,
            raw_hash: raw_hash(["r"]),
            ..Default::default()
        };
        upsert_events(&mut c, 1, &[e]).unwrap();
        c.execute(
            "INSERT INTO prices (model, effective_from, input_per_mtok, output_per_mtok, source)
             VALUES ('gpt-5', '2026-01-01T00:00:00Z', 4, 0, 'user')",
            [],
        )
        .unwrap();
        assert_eq!(recompute_all(&mut c).unwrap(), 1);
        let (cost, v): (f64, String) = c
            .query_row("SELECT cost_usd_est, price_version FROM events", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert!((cost - 4.0).abs() < 1e-9);
        assert_eq!(v, "user");
    }

    #[test]
    fn cost_math() {
        let rate = Rate {
            model: "m".into(),
            input: 3.0,
            output: 15.0,
            cache_read: 0.3,
            cache_write: 3.75,
            price_version: "v".into(),
        };
        let t = Tokens {
            input: 1_000_000,
            output: 100_000,
            cache_read: 2_000_000,
            cache_write: 0,
        };
        assert!((cost(&rate, t) - (3.0 + 1.5 + 0.6)).abs() < 1e-9);
    }

    #[test]
    fn ingest_prices_events_with_effective_dates_and_user_override() {
        let mut c = setup();
        c.execute(
            "INSERT INTO prices (model, effective_from, input_per_mtok, output_per_mtok, source)
             VALUES ('gpt-5', '2026-10-01T00:00:00Z', 2, 20, 'user')",
            [],
        )
        .unwrap();
        let ev = |k: &str, ts: &str, model: Option<&str>| NewEvent {
            ts_utc: ts.into(),
            app: "x".into(),
            model: model.map(Into::into),
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            requests: 1,
            raw_hash: raw_hash([k]),
            ..Default::default()
        };
        upsert_events(
            &mut c,
            1,
            &[
                ev("old", "2026-09-01T00:00:00Z", Some("gpt-5")),
                ev("new", "2026-10-31T00:00:00Z", Some("gpt-5")),
                ev("unk", "2026-10-31T00:00:00Z", Some("mystery")),
                ev("none", "2026-10-31T00:00:00Z", None),
            ],
        )
        .unwrap();
        let get = |k: &str| -> (Option<f64>, Option<String>) {
            c.query_row(
                "SELECT cost_usd_est, price_version FROM events WHERE raw_hash = ?1",
                [raw_hash([k])],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap()
        };
        let (old, v_old) = get("old");
        assert!((old.unwrap() - 11.25).abs() < 1e-9, "bundled 1.25 + 10");
        assert_eq!(
            v_old.as_deref(),
            Some(prices::bundled().unwrap().version.as_str())
        );
        let (new, v_new) = get("new");
        assert!(
            (new.unwrap() - 22.0).abs() < 1e-9,
            "user override from Oct 1"
        );
        assert_eq!(v_new.as_deref(), Some("user"));
        assert_eq!(get("unk"), (None, None));
        assert_eq!(get("none"), (None, None));
    }
}
