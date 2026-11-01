//! Price override commands: thin wrappers over `ledger_core::overrides`.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;
use ledger_core::overrides::{self, UserPrice};

/// A user price override, USD per million tokens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PriceOverride {
    pub model: String,
    /// `YYYY-MM-DD` (UTC).
    pub effective_from: String,
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
}

impl From<UserPrice> for PriceOverride {
    fn from(p: UserPrice) -> Self {
        Self {
            model: p.model,
            effective_from: p.effective_from,
            input: p.input,
            output: p.output,
            cache_read: p.cache_read,
            cache_write: p.cache_write,
        }
    }
}

impl From<PriceOverride> for UserPrice {
    fn from(p: PriceOverride) -> Self {
        Self {
            model: p.model,
            effective_from: p.effective_from,
            input: p.input,
            output: p.output,
            cache_read: p.cache_read,
            cache_write: p.cache_write,
        }
    }
}

/// A model seen in events that has no price.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UnknownModelInfo {
    pub model: String,
    pub events: u64,
    pub tokens: u64,
    pub first_seen: String,
    pub last_seen: String,
}

/// Logic callable without a Tauri runtime.
pub mod core {
    use super::*;

    pub fn list(state: &AppState) -> IpcResult<Vec<PriceOverride>> {
        Ok(overrides::list(&state.db())?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub fn unknown(state: &AppState) -> IpcResult<Vec<UnknownModelInfo>> {
        Ok(ledger_core::unknown_models::list(&state.db())?
            .into_iter()
            .map(|u| UnknownModelInfo {
                model: u.model,
                events: u.events,
                tokens: u.tokens,
                first_seen: u.first_seen,
                last_seen: u.last_seen,
            })
            .collect())
    }

    /// Returns how many events were re-priced.
    pub fn set(state: &AppState, p: PriceOverride) -> IpcResult<u32> {
        let n = overrides::set(&mut state.db(), &p.into())?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }

    /// Returns how many events were re-priced.
    pub fn remove(state: &AppState, model: &str, effective_from: &str) -> IpcResult<u32> {
        let n = overrides::remove(&mut state.db(), model, effective_from)?;
        Ok(u32::try_from(n).unwrap_or(u32::MAX))
    }
}

/// Lists user price overrides.
#[tauri::command]
#[specta::specta]
pub fn list_price_overrides(state: State<'_, AppState>) -> IpcResult<Vec<PriceOverride>> {
    core::list(&state)
}

/// Adds or replaces an override and re-prices all events.
#[tauri::command]
#[specta::specta]
pub fn set_price_override(state: State<'_, AppState>, price: PriceOverride) -> IpcResult<u32> {
    core::set(&state, price)
}

/// Removes an override and re-prices all events.
#[tauri::command]
#[specta::specta]
pub fn remove_price_override(
    state: State<'_, AppState>,
    model: String,
    effective_from: String,
) -> IpcResult<u32> {
    core::remove(&state, &model, &effective_from)
}

/// Lists models seen in events that have no price, most-used first.
#[tauri::command]
#[specta::specta]
pub fn list_unknown_models(state: State<'_, AppState>) -> IpcResult<Vec<UnknownModelInfo>> {
    core::unknown(&state)
}
