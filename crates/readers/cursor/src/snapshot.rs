//! Read-only, copy-on-read access to Cursor's `state.vscdb` files.
//!
//! Cursor keeps these SQLite files open while it runs. Reading them in place
//! risks lock contention with the editor, so each read copies the database
//! (plus its `-wal` file, which may hold recent, uncheckpointed writes) into a
//! private temp dir and opens the copy read-only. The original is never
//! opened for writing and never locked by us.
//!
//! VS Code-style state DBs store key/value rows in `ItemTable`; Cursor also
//! uses a `cursorDiskKV` table with the same `(key, value)` shape.

use std::path::{Path, PathBuf};

use ledger_core::{Error, Result};
use rusqlite::{Connection, OpenFlags, OptionalExtension};

/// Key/value tables we know how to read.
pub const KV_TABLES: &[&str] = &["ItemTable", "cursorDiskKV"];

/// A private read-only copy of a state DB. Deleted on drop.
pub struct Snapshot {
    conn: Connection,
    source: PathBuf,
    _dir: tempfile::TempDir,
}

fn copy_if_exists(from: &Path, to: &Path) -> Result<bool> {
    match std::fs::copy(from, to) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(Error::io(from, e)),
    }
}

impl Snapshot {
    /// Copies `db` (and its WAL, if any) and opens the copy read-only.
    pub fn take(db: &Path) -> Result<Self> {
        let dir = tempfile::Builder::new()
            .prefix("ai-ledger-cursor-")
            .tempdir()
            .map_err(|e| Error::io(std::env::temp_dir(), e))?;
        let copy = dir.path().join("state.vscdb");
        if !copy_if_exists(db, &copy)? {
            return Err(Error::io(db, std::io::ErrorKind::NotFound.into()));
        }
        let mut wal = db.as_os_str().to_owned();
        wal.push("-wal");
        copy_if_exists(Path::new(&wal), &dir.path().join("state.vscdb-wal"))?;
        // Read-write on the *copy* only so SQLite can replay the copied WAL;
        // query_only then forbids any write through this connection.
        let conn = Connection::open_with_flags(
            &copy,
            OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        conn.pragma_update(None, "query_only", "ON")?;
        Ok(Self {
            conn,
            source: db.to_path_buf(),
            _dir: dir,
        })
    }

    /// Path of the original database this snapshot was taken from.
    pub fn source(&self) -> &Path {
        &self.source
    }

    fn has_table(&self, table: &str) -> Result<bool> {
        Ok(self.conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
            [table],
            |r| r.get(0),
        )?)
    }

    fn checked(&self, table: &str) -> Result<bool> {
        if !KV_TABLES.contains(&table) {
            return Err(Error::InvalidInput(format!("unknown kv table {table:?}")));
        }
        self.has_table(table)
    }

    /// Value for `key` in `table`, decoded as UTF-8 text.
    pub fn get(&self, table: &str, key: &str) -> Result<Option<String>> {
        if !self.checked(table)? {
            return Ok(None);
        }
        let v: Option<Vec<u8>> = self
            .conn
            .query_row(
                &format!("SELECT value FROM {table} WHERE key = ?1"),
                [key],
                |r| r.get::<_, rusqlite::types::Value>(0).map(value_bytes),
            )
            .optional()?;
        Ok(v.map(|b| String::from_utf8_lossy(&b).into_owned()))
    }

    /// All `(key, value)` rows in `table` whose key starts with `prefix`, sorted by key.
    pub fn scan_prefix(&self, table: &str, prefix: &str) -> Result<Vec<(String, String)>> {
        if !self.checked(table)? {
            return Ok(Vec::new());
        }
        let mut st = self.conn.prepare(&format!(
            "SELECT key, value FROM {table} WHERE substr(key, 1, length(?1)) = ?1 ORDER BY key"
        ))?;
        let rows = st.query_map([prefix], |r| {
            let k: String = r.get(0)?;
            let v = value_bytes(r.get(1)?);
            Ok((k, String::from_utf8_lossy(&v).into_owned()))
        })?;
        rows.map(|r| r.map_err(Error::from)).collect()
    }
}

fn value_bytes(v: rusqlite::types::Value) -> Vec<u8> {
    use rusqlite::types::Value;
    match v {
        Value::Text(s) => s.into_bytes(),
        Value::Blob(b) => b,
        Value::Integer(i) => i.to_string().into_bytes(),
        Value::Real(f) => f.to_string().into_bytes(),
        Value::Null => Vec::new(),
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// Builds a fake state.vscdb with the given rows, left in WAL mode with an
    /// open writer so some rows live only in the -wal file.
    pub fn fake_db(path: &Path, item: &[(&str, &str)], disk: &[(&str, &[u8])]) -> Connection {
        let c = Connection::open(path).unwrap();
        c.pragma_update(None, "journal_mode", "WAL").unwrap();
        c.pragma_update(None, "wal_autocheckpoint", 0).unwrap();
        c.execute_batch(
            "CREATE TABLE ItemTable (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);
             CREATE TABLE cursorDiskKV (key TEXT UNIQUE ON CONFLICT REPLACE, value BLOB);",
        )
        .unwrap();
        for (k, v) in item {
            c.execute("INSERT INTO ItemTable VALUES (?1, ?2)", (k, v))
                .unwrap();
        }
        for (k, v) in disk {
            c.execute("INSERT INTO cursorDiskKV VALUES (?1, ?2)", (k, v))
                .unwrap();
        }
        c
    }

    #[test]
    fn reads_rows_including_uncheckpointed_wal() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.vscdb");
        let _writer = fake_db(
            &db,
            &[("a", "1")],
            &[
                ("bubble:1", b"{\"x\":1}"),
                ("bubble:2", b"{}"),
                ("other", b""),
            ],
        );
        assert!(tmp.path().join("state.vscdb-wal").exists());
        let s = Snapshot::take(&db).unwrap();
        assert_eq!(s.get("ItemTable", "a").unwrap().as_deref(), Some("1"));
        assert_eq!(s.get("ItemTable", "zzz").unwrap(), None);
        let rows = s.scan_prefix("cursorDiskKV", "bubble:").unwrap();
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], ("bubble:1".into(), "{\"x\":1}".into()));
        assert_eq!(s.source(), db);
    }

    #[test]
    fn snapshot_is_read_only_and_original_untouched() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.vscdb");
        drop(fake_db(&db, &[("a", "1")], &[]));
        let before = std::fs::read(&db).unwrap();
        let s = Snapshot::take(&db).unwrap();
        assert!(s.conn.execute("DELETE FROM ItemTable", []).is_err());
        drop(s);
        assert_eq!(std::fs::read(&db).unwrap(), before);
    }

    #[test]
    fn missing_tables_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let db = tmp.path().join("state.vscdb");
        assert_eq!(Snapshot::take(&db).err().unwrap().code(), "io");
        Connection::open(&db)
            .unwrap()
            .execute_batch("CREATE TABLE t (x);")
            .unwrap();
        let s = Snapshot::take(&db).unwrap();
        assert_eq!(s.get("ItemTable", "a").unwrap(), None);
        assert!(s.get("sqlite_master", "a").is_err(), "only known kv tables");
    }
}
