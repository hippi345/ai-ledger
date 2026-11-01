//! Summary cards (spec §B): one plain-English sentence each for today, this
//! week and this month, built from aggregate output. Periods are UTC days.

use chrono::{Datelike, Days, NaiveDate, Utc};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;
use ledger_core::aggregate::{aggregate, Dim, Filters, Metric, Query};
use ledger_core::rollup::Grain;
use ledger_core::summary::{sentence, PeriodSummary};

/// One summary card.
#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct SummaryCard {
    pub label: String,
    pub text: String,
}

/// `[since, until)` for today, this week (Monday start) and this month.
pub fn periods(today: NaiveDate) -> Vec<(&'static str, NaiveDate, NaiveDate)> {
    let tomorrow = today.checked_add_days(Days::new(1)).unwrap_or(today);
    let week = today
        .checked_sub_days(Days::new(u64::from(today.weekday().num_days_from_monday())))
        .unwrap_or(today);
    let month = today.with_day(1).unwrap_or(today);
    vec![
        ("Today", today, tomorrow),
        ("This week", week, tomorrow),
        ("This month", month, tomorrow),
    ]
}

fn total(
    conn: &rusqlite::Connection,
    since: &str,
    until: &str,
    metric: Metric,
) -> ledger_core::Result<f64> {
    let q = Query {
        since: since.into(),
        until: until.into(),
        grain: Grain::Day,
        group_by: vec![],
        filters: Filters::default(),
        metric,
    };
    Ok(aggregate(conn, &q)?.iter().map(|r| r.value).sum())
}

fn top(
    conn: &rusqlite::Connection,
    since: &str,
    until: &str,
    dim: Dim,
) -> ledger_core::Result<Option<String>> {
    let q = Query {
        since: since.into(),
        until: until.into(),
        grain: Grain::Day,
        group_by: vec![dim],
        filters: Filters::default(),
        metric: Metric::Tokens,
    };
    let rows = ledger_core::aggregate::top_n(aggregate(conn, &q)?, 1);
    Ok(rows
        .into_iter()
        .find(|r| r.value > 0.0)
        .and_then(|r| r.keys.into_iter().next())
        .filter(|k| !k.is_empty()))
}

/// Core logic for a given UTC date (tests).
pub fn cards_for(state: &AppState, today: NaiveDate) -> IpcResult<Vec<SummaryCard>> {
    let conn = state.db();
    let mut out = Vec::new();
    for (label, since, until) in periods(today) {
        let (s, u) = (since.to_string(), until.to_string());
        let tokens = total(&conn, &s, &u, Metric::Tokens)?;
        let (ps, pu) = ledger_core::delta::previous_range(&s, &u)?;
        let prev = total(&conn, &ps, &pu, Metric::Tokens)?;
        let app = top(&conn, &s, &u, Dim::App)?;
        let model = top(&conn, &s, &u, Dim::Model)?;
        let top: Vec<&str> = [app.as_deref(), model.as_deref()]
            .into_iter()
            .flatten()
            .collect();
        let text = sentence(&PeriodSummary {
            label,
            tokens,
            cost_usd: total(&conn, &s, &u, Metric::CostUsd)?,
            top,
            change_pct: ledger_core::delta::delta(tokens, prev).pct,
        });
        out.push(SummaryCard {
            label: label.into(),
            text,
        });
    }
    Ok(out)
}

/// Today / this week / this month summary sentences.
#[tauri::command]
#[specta::specta]
pub fn summary_cards(state: State<'_, AppState>) -> IpcResult<Vec<SummaryCard>> {
    cards_for(&state, Utc::now().date_naive())
}
