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
    /// 1-hour cache-write rate; `None` bills those writes at `cache_write`.
    pub cache_write_1h: Option<f64>,
    /// 12-hour cache-write rate; `None` bills those writes at `cache_write`.
    pub cache_write_12h: Option<f64>,
    /// `"user"` or the bundled table version.
    pub price_version: String,
    /// Long-context tiers, ascending by threshold.
    pub tiers: Vec<TierRate>,
}

/// Rates that replace the base rates when the prompt exceeds `above_prompt_tokens`.
#[derive(Debug, Clone, PartialEq)]
pub struct TierRate {
    pub above_prompt_tokens: u64,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    /// Long-context 12-hour cache-write rate; `None` falls back to `cache_write`.
    pub cache_write_12h: Option<f64>,
}

/// Token counts for one event.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tokens {
    pub input: u64,
    pub output: u64,
    pub cache_read: u64,
    pub cache_write: u64,
    pub cache_write_1h: u64,
    pub cache_write_12h: u64,
}

/// USD cost of `t` at `r`.
/// The whole request is billed at the highest tier whose threshold the
/// prompt (input + cache read + cache write) exceeds, as vendors bill it.
pub fn cost(r: &Rate, t: Tokens) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    let f = |n: u64| n as f64;
    let prompt = t
        .input
        .saturating_add(t.cache_read)
        .saturating_add(t.cache_write)
        .saturating_add(t.cache_write_1h)
        .saturating_add(t.cache_write_12h);
    let (i, o, cr, cw, cw12h) = r
        .tiers
        .iter()
        .rfind(|tier| prompt > tier.above_prompt_tokens)
        .map_or(
            (
                r.input,
                r.output,
                r.cache_read,
                r.cache_write,
                r.cache_write_12h.unwrap_or(r.cache_write),
            ),
            |tier| {
                (
                    tier.input,
                    tier.output,
                    tier.cache_read,
                    tier.cache_write,
                    tier.cache_write_12h.unwrap_or(tier.cache_write),
                )
            },
        );
    // Tiers carry no separate 1-hour rate; the base 1-hour rate applies.
    let cw1h = r.cache_write_1h.unwrap_or(cw);
    (f(t.input) * i
        + f(t.output) * o
        + f(t.cache_read) * cr
        + f(t.cache_write) * cw
        + f(t.cache_write_1h) * cw1h
        + f(t.cache_write_12h) * cw12h)
        / 1_000_000.0
}

fn tiers(conn: &Connection, model: &str, from: &str, source: &str) -> Result<Vec<TierRate>> {
    let mut st = conn.prepare_cached(
        "SELECT above_prompt_tokens, input_per_mtok, output_per_mtok, cache_read_per_mtok,
                cache_write_per_mtok, cache_write_12h_per_mtok
         FROM price_tiers WHERE model = ?1 AND effective_from = ?2 AND source = ?3
         ORDER BY above_prompt_tokens",
    )?;
    let rows = st.query_map((model, from, source), |r| {
        Ok(TierRate {
            above_prompt_tokens: u64::try_from(r.get::<_, i64>(0)?).unwrap_or(0),
            input: r.get(1)?,
            output: r.get(2)?,
            cache_read: r.get(3)?,
            cache_write: r.get(4)?,
            cache_write_12h: r.get(5)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
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
            "SELECT input_per_mtok, output_per_mtok, cache_read_per_mtok, cache_write_per_mtok, source,
                    effective_from, cache_write_1h_per_mtok, cache_write_12h_per_mtok
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
                    r.get::<_, String>(5)?,
                    r.get::<_, Option<f64>>(6)?,
                    r.get::<_, Option<f64>>(7)?,
                ))
            },
        )
        .optional()?;
    let Some((
        input,
        output,
        cache_read,
        cache_write,
        source,
        from,
        cache_write_1h,
        cache_write_12h,
    )) = row
    else {
        return Ok(None);
    };
    let tiers = tiers(conn, &canon, &from, &source)?;
    Ok(Some(Rate {
        tiers,
        cache_write_1h,
        cache_write_12h,
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
    }))
}

/// Computes and stores `cost_usd_est` / `price_version` for one event.
pub fn price_event(conn: &Connection, event_id: i64) -> Result<()> {
    let (model, ts, t): (Option<String>, String, Tokens) = conn.query_row(
        "SELECT model, ts_utc, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens,
                cache_write_1h_tokens, cache_write_12h_tokens
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
                    cache_write_1h: n(6)?,
                    cache_write_12h: n(7)?,
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
    fn long_context_tier_bills_whole_request() {
        let c = setup();
        let r = rate_at(&c, "gemini-2.5-pro", "2026-10-31T00:00:00Z")
            .unwrap()
            .unwrap();
        let short = Tokens {
            input: 200_000,
            output: 1_000_000,
            ..Tokens::default()
        };
        assert!(
            (cost(&r, short) - (0.25 + 10.0)).abs() < 1e-9,
            "at threshold: base rates"
        );
        let long = Tokens {
            input: 300_000,
            output: 1_000_000,
            ..Tokens::default()
        };
        assert!(
            (cost(&r, long) - (0.75 + 15.0)).abs() < 1e-9,
            "above: tier rates"
        );
        let cached = Tokens {
            input: 1,
            cache_read: 250_000,
            ..Tokens::default()
        };
        assert!(
            cost(&r, cached) > 0.25 * 0.25 - 1e-9,
            "cache tokens count toward the prompt"
        );
    }

    /// OpenAI >272k surcharge (https://platform.openai.com/docs/pricing, read
    /// 2026-10-31): input 2x, output 1.5x, cached input and cache writes 2x.
    #[test]
    fn openai_long_context_surcharge() {
        let c = setup();
        let at = "2026-10-31T00:00:00Z";
        for n in [
            "gpt-6-astra",
            "gpt-5.6-terra",
            "gpt-5.5",
            "gpt-5.4",
            "gpt-6-rosalind",
        ] {
            let r = rate_at(&c, n, at).unwrap().unwrap();
            assert_eq!(r.tiers.len(), 1, "{n}");
            let t = &r.tiers[0];
            assert_eq!(t.above_prompt_tokens, 272_000, "{n}");
            assert!((t.input - 2.0 * r.input).abs() < 1e-9, "{n}");
            assert!((t.output - 1.5 * r.output).abs() < 1e-9, "{n}");
            assert!((t.cache_read - 2.0 * r.cache_read).abs() < 1e-9, "{n}");
            assert!((t.cache_write - 2.0 * r.cache_write).abs() < 1e-9, "{n}");
        }
        let r = rate_at(&c, "gpt-6-astra", at).unwrap().unwrap();
        assert_eq!(r.tiers[0].cache_write_12h, Some(50.0));
        let long = Tokens {
            input: 300_000,
            output: 1_000_000,
            ..Tokens::default()
        };
        assert!((cost(&r, long) - (6.0 + 75.0)).abs() < 1e-9);
        let pro = rate_at(&c, "gpt-5.5-pro-2026", at).unwrap().unwrap();
        assert_eq!(pro.model, "gpt-5.5-pro");
        for n in ["gpt-5.4-mini", "gpt-5.3-codex", "gpt-5.2"] {
            assert!(rate_at(&c, n, at).unwrap().unwrap().tiers.is_empty(), "{n}");
        }
    }

    #[test]
    fn one_hour_cache_writes_use_their_own_rate() {
        let c = setup();
        let r = rate_at(&c, "claude-sonnet-4-5", "2026-10-31T00:00:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(r.cache_write_1h, Some(6.0));
        let t = Tokens {
            cache_write: 1_000_000,
            cache_write_1h: 1_000_000,
            ..Tokens::default()
        };
        assert!((cost(&r, t) - (3.75 + 6.0)).abs() < 1e-9);
        let g = rate_at(&c, "gpt-5", "2026-10-31T00:00:00Z")
            .unwrap()
            .unwrap();
        assert_eq!(g.cache_write_1h, None);
    }

    /// Scheduled promo endings from https://ai.google.dev/gemini-api/docs/pricing
    /// (checked 2026-10-31) are bundled as future-dated rows.
    #[test]
    fn future_dated_rows_take_effect_on_their_date() {
        let c = setup();
        let at = |m: &str, ts: &str| {
            let r = rate_at(&c, m, ts).unwrap().unwrap();
            (r.input, r.output, r.cache_read)
        };
        assert_eq!(
            at("gemini-4-argon", "2026-11-30T23:59:59Z"),
            (2.0, 10.0, 0.1)
        );
        assert_eq!(
            at("gemini-4-argon", "2026-12-01T00:00:00Z"),
            (4.0, 20.0, 0.2)
        );
        assert_eq!(
            at("gemini-3.8-flash", "2026-12-31T12:00:00Z"),
            (0.75, 3.75, 0.075)
        );
        assert_eq!(
            at("gemini-3.8-flash", "2027-01-01T00:00:00Z"),
            (1.5, 7.5, 0.15)
        );
        assert_eq!(
            at("gemini-3.6-flash", "2027-02-01T00:00:00Z"),
            (1.5, 7.5, 0.15)
        );
    }

    #[test]
    fn twelve_hour_cache_writes_use_their_own_rates() {
        let rate = Rate {
            model: "m".into(),
            input: 2.0,
            output: 10.0,
            cache_read: 0.2,
            cache_write: 2.5,
            cache_write_1h: None,
            cache_write_12h: Some(5.0),
            price_version: "v".into(),
            tiers: vec![TierRate {
                above_prompt_tokens: 272_000,
                input: 4.0,
                output: 15.0,
                cache_read: 0.4,
                cache_write: 5.0,
                cache_write_12h: Some(10.0),
            }],
        };
        let short = Tokens {
            cache_write_12h: 100_000,
            ..Tokens::default()
        };
        assert!((cost(&rate, short) - 0.5).abs() < 1e-9);
        let long = Tokens {
            cache_write_12h: 300_000,
            ..Tokens::default()
        };
        assert!((cost(&rate, long) - 3.0).abs() < 1e-9, "tier 12h rate");
        let mut flat = rate.clone();
        flat.cache_write_12h = None;
        flat.tiers.clear();
        assert!((cost(&flat, short) - 0.25).abs() < 1e-9, "falls back");
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
            tiers: vec![],
            cache_write_1h: None,
            cache_write_12h: None,
        };
        let t = Tokens {
            input: 1_000_000,
            output: 100_000,
            cache_read: 2_000_000,
            cache_write: 0,
            cache_write_1h: 0,
            cache_write_12h: 0,
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
