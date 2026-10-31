//! Core of AI Ledger: local storage, rollups, and the price table.
//!
//! Everything here runs fully offline; this crate never performs network I/O.

pub mod backup;
pub mod db;
pub mod error;
pub mod ingest;
pub mod logging;
pub mod migrations;
pub mod reader;
pub mod retention;
pub mod settings;

pub use error::{Error, Result};

/// Crate version, surfaced in the app's About box and backups.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_semver_like() {
        assert_eq!(VERSION.split('.').count(), 3);
    }
}
