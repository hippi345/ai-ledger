//! Embedded, versioned schema migrations.
//!
//! Each migration is a `(version, sql)` pair applied in order inside a
//! transaction. Applied versions are recorded in `schema_migrations`, so the
//! runner is idempotent. Migrations are append-only: never edit a shipped one.

use rusqlite::{Connection, OptionalExtension};

use crate::{Error, Result};

/// A single schema step.
#[derive(Debug, Clone, Copy)]
pub struct Migration {
    pub version: u32,
    pub name: &'static str,
    pub sql: &'static str,
}

/// All migrations, ordered by version.
pub const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "core_schema",
        sql: include_str!("../migrations/0001_core_schema.sql"),
    },
    Migration {
        version: 2,
        name: "settings",
        sql: include_str!("../migrations/0002_settings.sql"),
    },
    Migration {
        version: 3,
        name: "prices",
        sql: include_str!("../migrations/0003_prices.sql"),
    },
];

/// Highest version known to this build (0 when there are none).
pub fn latest_version() -> u32 {
    MIGRATIONS.last().map_or(0, |m| m.version)
}

fn ensure_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version    INTEGER PRIMARY KEY,
            name       TEXT NOT NULL,
            applied_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ','now'))
        );",
    )?;
    Ok(())
}

/// Highest applied version (0 for a fresh database).
pub fn current_version(conn: &Connection) -> Result<u32> {
    ensure_table(conn)?;
    let v: Option<u32> = conn
        .query_row("SELECT MAX(version) FROM schema_migrations", [], |r| {
            r.get(0)
        })
        .optional()?
        .flatten();
    Ok(v.unwrap_or(0))
}

/// Applies the built-in migrations.
pub fn run(conn: &mut Connection) -> Result<u32> {
    run_with(conn, MIGRATIONS)
}

/// Applies `migrations` above the current version. Returns the number applied.
pub fn run_with(conn: &mut Connection, migrations: &[Migration]) -> Result<u32> {
    let current = current_version(conn)?;
    if let Some(m) = migrations.windows(2).find(|w| w[0].version >= w[1].version) {
        return Err(Error::Migration {
            version: m[1].version,
            message: "migrations are not strictly increasing".into(),
        });
    }
    let newest = migrations.last().map_or(0, |m| m.version);
    if current > newest {
        return Err(Error::Migration {
            version: current,
            message: format!("database is newer than this build (knows up to {newest})"),
        });
    }
    let mut applied = 0;
    for m in migrations.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        tx.execute_batch(m.sql).map_err(|e| Error::Migration {
            version: m.version,
            message: e.to_string(),
        })?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            (m.version, m.name),
        )?;
        tx.commit()?;
        tracing::info!(version = m.version, name = m.name, "applied migration");
        applied += 1;
    }
    Ok(applied)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST: &[Migration] = &[
        Migration {
            version: 1,
            name: "a",
            sql: "CREATE TABLE a (x INTEGER);",
        },
        Migration {
            version: 2,
            name: "b",
            sql: "CREATE TABLE b (y INTEGER);",
        },
    ];

    #[test]
    fn applies_in_order_and_is_idempotent() {
        let mut c = Connection::open_in_memory().unwrap();
        assert_eq!(run_with(&mut c, &TEST[..1]).unwrap(), 1);
        assert_eq!(current_version(&c).unwrap(), 1);
        assert_eq!(run_with(&mut c, TEST).unwrap(), 1);
        assert_eq!(run_with(&mut c, TEST).unwrap(), 0);
        assert_eq!(current_version(&c).unwrap(), 2);
    }

    #[test]
    fn failed_migration_rolls_back() {
        let mut c = Connection::open_in_memory().unwrap();
        let bad = [Migration {
            version: 1,
            name: "bad",
            sql: "CREATE TABLE t (x); NOPE;",
        }];
        let err = run_with(&mut c, &bad).unwrap_err();
        assert_eq!(err.code(), "migration");
        assert_eq!(current_version(&c).unwrap(), 0);
        let n: i64 = c
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name='t'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(n, 0);
    }

    fn tables(c: &Connection) -> Vec<String> {
        let mut st = c
            .prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")
            .unwrap();
        st.query_map([], |r| r.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect()
    }

    #[test]
    fn v1_creates_core_tables() {
        let mut c = Connection::open_in_memory().unwrap();
        run(&mut c).unwrap();
        let t = tables(&c);
        for name in ["events", "sessions", "sources", "tool_calls"] {
            assert!(t.contains(&name.to_string()), "missing {name}");
        }
    }

    #[test]
    fn v1_constraints_hold() {
        let mut c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", "ON").unwrap();
        run(&mut c).unwrap();
        c.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('cursor', 1)",
            [],
        )
        .unwrap();
        let ins = "INSERT INTO events (source_id, ts_utc, app, input_tokens, raw_hash) \
                   VALUES (?1, '2026-10-31T00:00:00Z', 'cursor', ?2, ?3)";
        c.execute(ins, (1, 10, "h1")).unwrap();
        assert!(
            c.execute(ins, (1, 10, "h1")).is_err(),
            "raw_hash must be unique"
        );
        assert!(
            c.execute(ins, (1, -1, "h2")).is_err(),
            "tokens must be >= 0"
        );
        assert!(c.execute(ins, (99, 1, "h3")).is_err(), "source must exist");
        assert!(c
            .execute("INSERT INTO sources (reader_id, tier) VALUES ('x', 4)", [])
            .is_err());
        c.execute(
            "INSERT INTO tool_calls (event_id, ts_utc, tool, mcp_server) \
             VALUES (1, '2026-10-31T00:00:00Z', 'read_file', NULL)",
            [],
        )
        .unwrap();
        c.execute("DELETE FROM sources", []).unwrap();
        let n: i64 = c
            .query_row("SELECT COUNT(*) FROM tool_calls", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 0, "deleting a source cascades to events and tool calls");
    }

    #[test]
    fn rejects_unordered_and_newer_db() {
        let mut c = Connection::open_in_memory().unwrap();
        let unordered = [TEST[1], TEST[0]];
        assert!(run_with(&mut c, &unordered).is_err());
        run_with(&mut c, TEST).unwrap();
        assert!(run_with(&mut c, &TEST[..1]).is_err());
    }
}
