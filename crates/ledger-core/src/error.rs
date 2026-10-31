//! Error type shared by the core crate.

use std::path::PathBuf;

/// Result alias using [`Error`].
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors raised by the ledger core.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Filesystem failure, with the path involved when known.
    #[error("I/O error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    /// JSON (de)serialization failure.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    /// SQLite failure.
    #[error("database error: {0}")]
    Db(#[from] rusqlite::Error),
    /// A migration could not be applied.
    #[error("migration {version} failed: {message}")]
    Migration { version: u32, message: String },
    /// A reader failed or panicked while reading its source.
    #[error("reader error: {0}")]
    Reader(String),
    /// A caller passed a value the core cannot accept.
    #[error("invalid input: {0}")]
    InvalidInput(String),
}

impl Error {
    /// Wraps an I/O error together with the path it happened on.
    pub fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Short, stable code for the frontend to switch on.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Io { .. } => "io",
            Self::Json(_) => "json",
            Self::Db(_) => "db",
            Self::Migration { .. } => "migration",
            Self::Reader(_) => "reader",
            Self::InvalidInput(_) => "invalid_input",
        }
    }
}

/// Serializable error shape sent over IPC: `{ "code": ..., "message": ... }`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl From<&Error> for ErrorPayload {
    fn from(e: &Error) -> Self {
        Self {
            code: e.code().to_owned(),
            message: e.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_mentions_path() {
        let e = Error::io("/tmp/x.db", std::io::Error::other("boom"));
        assert_eq!(e.code(), "io");
        assert!(e.to_string().contains("/tmp/x.db"));
    }

    #[test]
    fn payload_serializes_code_and_message() {
        let e = Error::InvalidInput("bad range".into());
        let json = serde_json::to_value(ErrorPayload::from(&e)).unwrap();
        assert_eq!(json["code"], "invalid_input");
        assert_eq!(json["message"], "invalid input: bad range");
    }

    #[test]
    fn json_errors_convert() {
        let e: Error = serde_json::from_str::<u32>("x").unwrap_err().into();
        assert_eq!(e.code(), "json");
    }
}
