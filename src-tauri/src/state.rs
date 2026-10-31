//! Shared backend state: the open ledger database.

use std::path::Path;
use std::sync::{Mutex, MutexGuard};

use ledger_core::{db, prices, Result};
use rusqlite::Connection;

/// State managed by Tauri and handed to commands.
pub struct AppState {
    conn: Mutex<Connection>,
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
        Self {
            conn: Mutex::new(conn),
        }
    }

    /// Locks the database. A poisoned lock is recovered: SQLite transactions
    /// keep the data consistent even if a previous holder panicked.
    pub fn db(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|p| p.into_inner())
    }
}
