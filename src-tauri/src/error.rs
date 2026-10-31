//! The error shape every command returns to the UI.

use serde::Serialize;
use specta::Type;

/// `{ code, message }`; `code` is stable for the UI to switch on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
pub struct IpcError {
    pub code: String,
    pub message: String,
}

impl From<ledger_core::Error> for IpcError {
    fn from(e: ledger_core::Error) -> Self {
        let p = ledger_core::error::ErrorPayload::from(&e);
        Self {
            code: p.code,
            message: p.message,
        }
    }
}

/// Result type for commands.
pub type IpcResult<T> = Result<T, IpcError>;
