//! Core of AI Ledger: local storage, rollups, and the price table.
//!
//! Everything here runs fully offline; this crate never performs network I/O.

pub mod aggregate;
pub mod backup;
pub mod cost;
pub mod db;
pub mod debounce;
pub mod delta;
pub mod error;
pub mod ingest;
pub mod logging;
pub mod manual;
pub mod migrations;
pub mod overrides;
pub mod prices;
pub mod reader;
pub mod retention;
pub mod rollup;
pub mod settings;
pub mod sources;
pub mod stats;
pub mod summary;
pub mod sync;
pub mod unknown_models;

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
