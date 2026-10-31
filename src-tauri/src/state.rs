//! Shared backend state: the open ledger database.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use ledger_core::reader::Registry;
use ledger_core::{db, prices, Result};
use rusqlite::Connection;

/// State managed by Tauri and handed to commands.
pub struct AppState {
    conn: Mutex<Connection>,
    registry: Mutex<Registry>,
}

/// The built-in readers.
pub fn default_registry() -> Registry {
    let mut r = Registry::new();
    // Ids are distinct constants, so registration cannot fail.
    let _ = r.register(Box::new(reader_cursor::CursorReader::default()));
    let _ = r.register(Box::new(reader_claude_code::ClaudeCodeReader::default()));
    let _ = r.register(Box::new(reader_csv_import::CsvImportReader));
    r
}

impl AppState {
    /// Opens (creating and migrating) the ledger in `app_data_dir` and loads
    /// the bundled price table if its version changed.
    pub fn open(app_data_dir: &Path) -> Result<Self> {
        let mut conn = db::open(app_data_dir)?;
        prices::sync_bundled(&mut conn)?;
        Ok(Self::from_connection(conn))
    }

    /// Wraps an already-open connection (tests).
    pub fn from_connection(conn: Connection) -> Self {
        Self::with_registry(conn, default_registry())
    }

    /// Wraps a connection with a custom reader registry (tests).
    pub fn with_registry(conn: Connection, registry: Registry) -> Self {
        Self {
            conn: Mutex::new(conn),
            registry: Mutex::new(registry),
        }
    }

    /// Locks the database. A poisoned lock is recovered: SQLite transactions
    /// keep the data consistent even if a previous holder panicked.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }

    /// Locks the reader registry (same poison policy as [`Self::db`]).
    pub fn registry(&self) -> MutexGuard<'_, Registry> {
        self.registry.lock().unwrap_or_else(|p| p.into_inner())
    }
}
