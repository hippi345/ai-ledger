//! Models seen in events that have no price (spec §6): shown in Settings so
//! the user can add an override or alias. Their events cost "—".

use rusqlite::Connection;

use crate::Result;

/// One unpriced model name and how much it was used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModel {
    pub model: String,
    pub events: u64,
    /// Input + output + cache tokens across those events.
    pub tokens: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Unpriced models, most-used first. An event counts when it names a model
/// but has no price version (no rate matched at its timestamp).
pub fn list(conn: &Connection) -> Result<Vec<UnknownModel>> {
    let mut st = conn.prepare(
        "SELECT model, COUNT(*),
                SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens
                    + cache_write_1h_tokens + cache_write_12h_tokens),
                MIN(ts_utc), MAX(ts_utc)
         FROM events
         WHERE model IS NOT NULL AND trim(model) <> '' AND price_version IS NULL
         GROUP BY model
         ORDER BY 3 DESC, 2 DESC, model",
    )?;
    let n = |v: i64| u64::try_from(v).unwrap_or(0);
    let rows = st.query_map([], |r| {
        Ok(UnknownModel {
            model: r.get(0)?,
            events: n(r.get(1)?),
            tokens: n(r.get(2)?),
            first_seen: r.get(3)?,
            last_seen: r.get(4)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ingest::{raw_hash, upsert_events, NewEvent};
    use crate::overrides::{self, UserPrice};
    use crate::{db, prices};

    fn ev(key: &str, model: Option<&str>, ts: &str, input: u64) -> NewEvent {
        NewEvent {
            ts_utc: ts.into(),
            app: "cursor".into(),
            model: model.map(Into::into),
            input_tokens: input,
            requests: 1,
            raw_hash: raw_hash([key]),
            ..Default::default()
        }
    }

    #[test]
    fn lists_unpriced_models_by_usage() {
        let mut c = db::open_in_memory().unwrap();
        prices::sync_bundled(&mut c).unwrap();
        c.execute("INSERT INTO sources (reader_id, tier) VALUES ('t', 1)", [])
            .unwrap();
        let evs = [
            ev("1", Some("mystery-a"), "2026-10-01T00:00:00Z", 10),
            ev("2", Some("mystery-a"), "2026-10-03T00:00:00Z", 10),
            ev("3", Some("mystery-b"), "2026-10-02T00:00:00Z", 500),
            ev("4", Some("gpt-5"), "2026-10-02T00:00:00Z", 999),
            ev("5", None, "2026-10-02T00:00:00Z", 999),
        ];
        upsert_events(&mut c, 1, &evs).unwrap();
        let l = list(&c).unwrap();
        let names: Vec<_> = l.iter().map(|u| u.model.as_str()).collect();
        assert_eq!(names, ["mystery-b", "mystery-a"]);
        assert_eq!(
            l[1],
            UnknownModel {
                model: "mystery-a".into(),
                events: 2,
                tokens: 20,
                first_seen: "2026-10-01T00:00:00Z".into(),
                last_seen: "2026-10-03T00:00:00Z".into(),
            }
        );
        overrides::set(
            &mut c,
            &UserPrice {
                model: "mystery-b".into(),
                effective_from: "2026-01-01".into(),
                input: 1.0,
                output: 1.0,
                cache_read: 0.0,
                cache_write: 0.0,
            },
        )
        .unwrap();
        let names: Vec<_> = list(&c).unwrap().into_iter().map(|u| u.model).collect();
        assert_eq!(names, ["mystery-a"], "an override removes it from the list");
    }
}
