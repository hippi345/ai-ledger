//! Project-name hashing (spec §8). With [`settings::keys::HASH_PROJECT_NAMES`]
//! on, new events store a short hash instead of the project name. Existing
//! rows change only through the explicit, irreversible [`hash_existing`].

use rusqlite::Connection;
use sha2::{Digest, Sha256};

use crate::{settings, Result};

const PREFIX: &str = "p-";
const HEX_LEN: usize = 12;

/// `p-` plus the first 12 hex digits of SHA-256(name). Already-hashed names
/// and the empty (unknown) name are returned unchanged, so hashing twice is a no-op.
pub fn hash_project(name: &str) -> String {
    if name.is_empty() || is_hashed(name) {
        return name.to_owned();
    }
    let digest = Sha256::digest(name.as_bytes());
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();
    format!("{PREFIX}{}", &hex[..HEX_LEN])
}

/// Whether `name` already looks like a [`hash_project`] output.
pub fn is_hashed(name: &str) -> bool {
    name.strip_prefix(PREFIX).is_some_and(|h| {
        h.len() == HEX_LEN
            && h.bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    })
}

/// Whether new events should have their project names hashed.
pub fn hashing_on(conn: &Connection) -> Result<bool> {
    settings::get_or(conn, settings::keys::HASH_PROJECT_NAMES, false)
}

/// Tables whose `project` column holds names: raw events, sessions and the
/// usage rollups (rollups are rewritten in place, since buckets before the
/// retention cutoff cannot be rebuilt from raw events).
const TABLES: &[&str] = &[
    "events",
    "sessions",
    "rollups_day",
    "rollups_week",
    "rollups_month",
];

/// Replaces every stored project name with its hash. Irreversible. Returns
/// how many distinct names were hashed.
pub fn hash_existing(conn: &mut Connection) -> Result<usize> {
    let tx = conn.transaction()?;
    let mut names: Vec<String> = Vec::new();
    for t in TABLES {
        let mut st = tx.prepare(&format!(
            "SELECT DISTINCT project FROM {t} WHERE project IS NOT NULL AND project != ''"
        ))?;
        for n in st.query_map([], |r| r.get::<_, String>(0))? {
            names.push(n?);
        }
    }
    names.sort();
    names.dedup();
    names.retain(|n| !is_hashed(n));
    for n in &names {
        let h = hash_project(n);
        for t in TABLES {
            tx.execute(
                &format!("UPDATE {t} SET project = ?1 WHERE project = ?2"),
                (&h, n),
            )?;
        }
    }
    tx.commit()?;
    tracing::info!(names = names.len(), "hashed existing project names");
    Ok(names.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn hash_is_short_stable_and_idempotent() {
        let h = hash_project("ai-ledger");
        assert_eq!(h.len(), 14);
        assert!(is_hashed(&h));
        assert_eq!(hash_project("ai-ledger"), h);
        assert_eq!(hash_project(&h), h);
        assert_ne!(hash_project("other"), h);
        assert_eq!(hash_project(""), "");
        assert!(!is_hashed("p-notahash"));
    }

    #[test]
    fn hash_existing_rewrites_events_sessions_and_rollups() {
        let mut c = db::open_in_memory().unwrap();
        c.execute_batch(
            "INSERT INTO sources (reader_id, tier) VALUES ('t', 1);
             INSERT INTO sessions (id, app, project, started) VALUES ('s', 'cursor', 'secret', '2026-11-01T00:00:00Z');
             INSERT INTO events (source_id, ts_utc, app, project, session_id, raw_hash)
               VALUES (1, '2026-11-01T00:00:00Z', 'cursor', 'secret', 's', 'a'),
                      (1, '2026-11-01T00:00:00Z', 'cursor', NULL, NULL, 'b');
             INSERT INTO rollups_month VALUES ('2026-11-01', 'cursor', '', 'secret', 1, 1, 0, 0, 0, 0, 0, 0);",
        )
        .unwrap();
        assert_eq!(hash_existing(&mut c).unwrap(), 1);
        let h = hash_project("secret");
        for t in TABLES {
            let left: i64 = c
                .query_row(
                    &format!("SELECT COUNT(*) FROM {t} WHERE project = 'secret'"),
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(left, 0, "{t}");
        }
        let ev: String = c
            .query_row("SELECT project FROM events WHERE raw_hash = 'a'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(ev, h);
        let null: Option<String> = c
            .query_row("SELECT project FROM events WHERE raw_hash = 'b'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(null, None);
        assert_eq!(hash_existing(&mut c).unwrap(), 0, "second run is a no-op");
    }
}
