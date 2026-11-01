//! The opt-in localhost logging proxy (spec item 21). Off by default: it
//! runs only while `proxy.enabled` is true, and it binds to 127.0.0.1 only
//! (see `reader_proxy::server`).

use std::sync::{Arc, Mutex};

use ledger_core::ingest::{upsert_events, IngestStats, NewEvent};
use ledger_core::settings::{self, keys};
use reader_proxy::server::{self, Handle, HttpsUpstream};
use reader_proxy::{DEFAULT_PORT, READER_ID};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{Emitter, Manager};

use crate::error::IpcResult;
use crate::state::AppState;

/// The running listener, if any, plus the last start error.
#[derive(Default)]
pub struct ProxyState(Mutex<Running>);

#[derive(Default)]
struct Running {
    handle: Option<Handle>,
    error: Option<String>,
}

/// What the Settings panel shows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub enabled: bool,
    pub port: u16,
    pub running: bool,
    pub error: Option<String>,
}

/// The saved choice: (enabled, port). Off unless the user turned it on.
pub fn wanted(state: &AppState) -> (bool, u16) {
    let db = state.db();
    (
        settings::get_or(&db, keys::PROXY_ENABLED, false).unwrap_or(false),
        settings::get_or(&db, keys::PROXY_PORT, DEFAULT_PORT).unwrap_or(DEFAULT_PORT),
    )
}

/// Id of the `proxy` source, created on first use.
pub fn source_id(conn: &Connection) -> ledger_core::Result<i64> {
    let existing: Option<i64> = conn
        .query_row(
            "SELECT id FROM sources WHERE reader_id = ?1",
            [READER_ID],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }
    conn.execute(
        "INSERT INTO sources (reader_id, tier) VALUES (?1, 3)",
        [READER_ID],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Stores one proxied usage event and refreshes its rollups.
pub fn record(state: &AppState, ev: NewEvent) -> ledger_core::Result<(i64, IngestStats)> {
    let mut conn = state.db();
    let src = source_id(&conn)?;
    let evs = [ev];
    let stats = upsert_events(&mut conn, src, &evs)?;
    ledger_core::rollup::refresh_for(&mut conn, &evs)?;
    Ok((src, stats))
}

/// Starts or stops the listener to match the saved settings.
pub fn apply(app: &tauri::AppHandle) {
    let (Some(st), Some(px)) = (app.try_state::<AppState>(), app.try_state::<ProxyState>()) else {
        return;
    };
    let (enabled, port) = wanted(&st);
    let mut run = px.0.lock().unwrap_or_else(|p| p.into_inner());
    let same_port = run.handle.as_ref().map(|h| h.addr().port()) == Some(port);
    if enabled && same_port {
        return;
    }
    // Stop the old listener before binding a new one (dropping stops it).
    run.handle = None;
    run.error = None;
    if !enabled {
        return;
    }
    let handle = app.clone();
    let sink: server::Sink = Arc::new(move |ev| {
        let st = handle.state::<AppState>();
        match record(&st, ev) {
            Ok((id, stats)) => {
                let ev = crate::ipc::IngestedEvent::new(id, stats);
                if ev.changed() {
                    crate::tray::refresh(&handle);
                    let _ = handle.emit(crate::ipc::INGESTED_EVENT, ev);
                }
            }
            Err(e) => tracing::warn!(error = %e, "proxy event not stored"),
        }
    });
    match server::start(
        port,
        Arc::new(HttpsUpstream::default()),
        sink,
        Arc::new(crate::retention::now_utc),
    ) {
        Ok(h) => {
            tracing::info!(port, "logging proxy listening on 127.0.0.1");
            run.handle = Some(h);
        }
        Err(e) => run.error = Some(format!("could not listen on 127.0.0.1:{port}: {e}")),
    }
}

fn status(st: &AppState, px: &ProxyState) -> ProxyStatus {
    let (enabled, port) = wanted(st);
    let run = px.0.lock().unwrap_or_else(|p| p.into_inner());
    ProxyStatus {
        enabled,
        port,
        running: run.handle.is_some(),
        error: run.error.clone(),
    }
}

/// Current proxy settings and whether it is listening.
#[tauri::command]
#[specta::specta]
pub fn proxy_status(
    state: tauri::State<'_, AppState>,
    proxy: tauri::State<'_, ProxyState>,
) -> IpcResult<ProxyStatus> {
    Ok(status(&state, &proxy))
}

/// Turns the proxy on or off and sets its port, then applies it.
#[tauri::command]
#[specta::specta]
pub fn set_proxy(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    proxy: tauri::State<'_, ProxyState>,
    enabled: bool,
    port: u16,
) -> IpcResult<ProxyStatus> {
    if port < 1024 {
        return Err(ledger_core::Error::InvalidInput("port must be 1024 or higher".into()).into());
    }
    {
        let db = state.db();
        settings::set(&db, keys::PROXY_ENABLED, &enabled)?;
        settings::set(&db, keys::PROXY_PORT, &port)?;
    }
    apply(&app);
    Ok(status(&state, &proxy))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        let c = ledger_core::db::open_in_memory().unwrap();
        AppState::from_connection(c)
    }

    #[test]
    fn off_by_default() {
        assert_eq!(wanted(&state()), (false, DEFAULT_PORT));
        assert_eq!(
            status(&state(), &ProxyState::default()),
            ProxyStatus {
                enabled: false,
                port: DEFAULT_PORT,
                running: false,
                error: None
            }
        );
    }

    #[test]
    fn records_into_one_proxy_source() {
        let st = state();
        let ev = |h: &str| NewEvent {
            ts_utc: "2026-11-01T12:00:00Z".into(),
            app: "anthropic".into(),
            model: Some("claude-sonnet-4-5".into()),
            input_tokens: 5,
            output_tokens: 6,
            requests: 1,
            raw_hash: h.into(),
            ..NewEvent::default()
        };
        let (a, s1) = record(&st, ev("h1")).unwrap();
        let (b, s2) = record(&st, ev("h1")).unwrap();
        assert_eq!(a, b);
        assert_eq!((s1.inserted, s2.inserted), (1, 0));
        let n: i64 = st
            .db()
            .query_row("SELECT COUNT(*) FROM events", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, 1);
    }
}
