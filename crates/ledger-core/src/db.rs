//! Opening the ledger database.
//!
//! The database lives at `<app data dir>/ledger.db`. The app shell resolves the
//! app data dir with Tauri's path API and passes it in, so this crate stays
//! independent of Tauri.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::{migrations, Error, Result};

/// File name of the ledger database inside the app data dir.
pub const DB_FILE_NAME: &str = "ledger.db";

/// Returns the database path inside `app_data_dir`.
pub fn db_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join(DB_FILE_NAME)
}

/// Applies the pragmas every ledger connection uses.
fn configure(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.busy_timeout(std::time::Duration::from_secs(5))?;
    Ok(())
}

/// Opens (creating if needed) the ledger in `app_data_dir`, enables WAL, and
/// runs pending migrations.
pub fn open(app_data_dir: &Path) -> Result<Connection> {
    std::fs::create_dir_all(app_data_dir).map_err(|e| Error::io(app_data_dir, e))?;
    open_file(&db_path(app_data_dir))
}

/// Opens a ledger at an explicit file path (WAL + migrations).
pub fn open_file(path: &Path) -> Result<Connection> {
    let mut conn = Connection::open(path)?;
    let mode: String = conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
    if !mode.eq_ignore_ascii_case("wal") {
        tracing::warn!(%mode, "SQLite refused WAL mode");
    }
    configure(&conn)?;
    migrations::run(&mut conn)?;
    crate::rollup::ensure_built(&mut conn)?;
    Ok(conn)
}

/// Opens an in-memory ledger with migrations applied (tests and previews).
pub fn open_in_memory() -> Result<Connection> {
    let mut conn = Connection::open_in_memory()?;
    configure(&conn)?;
    migrations::run(&mut conn)?;
    Ok(conn)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn open_creates_dir_and_uses_wal() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nested/app");
        let conn = open(&dir).unwrap();
        assert!(db_path(&dir).exists());
        let mode: String = conn
            .query_row("PRAGMA journal_mode", [], |r| r.get(0))
            .unwrap();
        assert_eq!(mode, "wal");
        let fk: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk, 1);
    }

    #[test]
    fn reopen_is_stable() {
        let tmp = tempfile::tempdir().unwrap();
        drop(open(tmp.path()).unwrap());
        let conn = open(tmp.path()).unwrap();
        assert_eq!(
            migrations::current_version(&conn).unwrap(),
            migrations::latest_version()
        );
    }
}
