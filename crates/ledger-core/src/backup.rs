//! Backup export (`VACUUM INTO` snapshots) and import.

use std::path::Path;

use rusqlite::{Connection, OpenFlags};

use crate::{db, migrations, Error, Result};

/// Writes a consistent, compacted snapshot of the live database to `dest`.
///
/// Uses `VACUUM INTO`, which is safe while other connections are reading or
/// writing. Refuses to overwrite an existing file.
pub fn export_snapshot(conn: &Connection, dest: &Path) -> Result<()> {
    if dest.exists() {
        return Err(Error::InvalidInput(format!(
            "backup target already exists: {}",
            dest.display()
        )));
    }
    let dest_str = dest
        .to_str()
        .ok_or_else(|| Error::InvalidInput("backup path is not valid UTF-8".into()))?;
    conn.execute("VACUUM INTO ?1", [dest_str])?;
    Ok(())
}

/// Checks that `path` is an intact AI Ledger database this build can open.
/// Returns its schema version.
pub fn validate(path: &Path) -> Result<u32> {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    let integrity: String = conn.query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(Error::InvalidInput(format!(
            "backup failed integrity check: {integrity}"
        )));
    }
    let has_table: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
        [],
        |r| r.get(0),
    )?;
    if !has_table {
        return Err(Error::InvalidInput(
            "file is not an AI Ledger database".into(),
        ));
    }
    let version: u32 = conn.query_row(
        "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
        [],
        |r| r.get(0),
    )?;
    if version > migrations::latest_version() {
        return Err(Error::InvalidInput(format!(
            "backup schema v{version} is newer than this build"
        )));
    }
    Ok(version)
}

/// Replaces the ledger in `app_data_dir` with the backup at `src`.
///
/// All connections to the current ledger must be closed first. The backup is
/// validated, copied next to the target, then atomically renamed into place;
/// stale WAL/SHM files are removed. The next [`db::open`] migrates it forward.
pub fn import(src: &Path, app_data_dir: &Path) -> Result<u32> {
    let version = validate(src)?;
    std::fs::create_dir_all(app_data_dir).map_err(|e| Error::io(app_data_dir, e))?;
    let target = db::db_path(app_data_dir);
    let staging = target.with_extension("db.importing");
    std::fs::copy(src, &staging).map_err(|e| Error::io(&staging, e))?;
    for ext in ["db-wal", "db-shm"] {
        let side = target.with_extension(ext);
        if side.exists() {
            std::fs::remove_file(&side).map_err(|e| Error::io(&side, e))?;
        }
    }
    std::fs::rename(&staging, &target).map_err(|e| Error::io(&target, e))?;
    Ok(version)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seeded(dir: &Path) -> Connection {
        let c = db::open(dir).unwrap();
        c.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('cursor', 1)",
            [],
        )
        .unwrap();
        c
    }

    fn sources(c: &Connection) -> i64 {
        c.query_row("SELECT COUNT(*) FROM sources", [], |r| r.get(0))
            .unwrap()
    }

    #[test]
    fn snapshot_roundtrips_through_import() {
        let tmp = tempfile::tempdir().unwrap();
        let live = tmp.path().join("live");
        let snap = tmp.path().join("snap.db");
        let c = seeded(&live);
        export_snapshot(&c, &snap).unwrap();
        assert!(export_snapshot(&c, &snap).is_err(), "never overwrites");
        assert_eq!(validate(&snap).unwrap(), migrations::latest_version());

        c.execute("DELETE FROM sources", []).unwrap();
        drop(c);
        import(&snap, &live).unwrap();
        let c = db::open(&live).unwrap();
        assert_eq!(sources(&c), 1);
    }

    #[test]
    fn rejects_non_ledger_files() {
        let tmp = tempfile::tempdir().unwrap();
        let other = tmp.path().join("other.db");
        Connection::open(&other)
            .unwrap()
            .execute_batch("CREATE TABLE x (y);")
            .unwrap();
        assert!(validate(&other).is_err());
        let junk = tmp.path().join("junk.db");
        std::fs::write(&junk, b"definitely not sqlite, just some bytes here").unwrap();
        assert!(import(&junk, &tmp.path().join("app")).is_err());
        assert!(!db::db_path(&tmp.path().join("app")).exists());
    }
}
