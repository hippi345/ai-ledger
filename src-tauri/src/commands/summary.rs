//! Summary cards (spec §B): one plain-English sentence each for today, this
//! week and this month, built from aggregate output. Periods are UTC days.

use chrono::{Datelike, Days, NaiveDate, Utc};
use serde::Serialize;
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;
use ledger_core::aggregate::{
    aggregate, aggregate_tools, Dim, Filters, Metric, Query, ToolDim, ToolMetric, ToolQuery,
};
use ledger_core::rollup::Grain;
use ledger_core::summary::{
    biggest_mover, mover_sentence, sentence, top_mcp_sentence, top_tool_sentence, PeriodSummary,
};

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

fn tool_rows(
    conn: &rusqlite::Connection,
    since: &str,
    until: &str,
    by: ToolDim,
    metric: ToolMetric,
) -> ledger_core::Result<Vec<ledger_core::aggregate::Row>> {
    aggregate_tools(
        conn,
        &ToolQuery {
            since: since.into(),
            until: until.into(),
            grain: Grain::Day,
            group_by: vec![by],
            app: vec![],
            mcp_server: vec![],
            tool: vec![],
            metric,
        },
    )
}

/// Top tool, top MCP server and biggest-mover cards for one period.
/// Cards with nothing to say are left out.
fn insight_cards(
    conn: &rusqlite::Connection,
    since: &str,
    until: &str,
) -> ledger_core::Result<Vec<SummaryCard>> {
    let period = "this week";
    let mut out = Vec::new();
    let calls = ledger_core::aggregate::top_n(
        tool_rows(conn, since, until, ToolDim::Tool, ToolMetric::Calls)?,
        1,
    );
    if let Some(top) = calls
        .first()
        .filter(|r| r.keys[0] != ledger_core::aggregate::OTHER)
    {
        let tool = &top.keys[0];
        let failures: f64 = tool_rows(conn, since, until, ToolDim::Tool, ToolMetric::Failures)?
            .iter()
            .filter(|r| &r.keys[0] == tool)
            .map(|r| r.value)
            .sum();
        if let Some(text) = top_tool_sentence(period, tool, top.value, failures) {
            out.push(SummaryCard {
                label: "Top tool".into(),
                text,
            });
        }
    }
    let servers: Vec<_> = tool_rows(conn, since, until, ToolDim::McpServer, ToolMetric::Calls)?
        .into_iter()
        .filter(|r| !r.keys[0].is_empty())
        .collect();
    if let Some(top) = ledger_core::aggregate::top_n(servers, 1).first() {
        if let Some(text) = top_mcp_sentence(period, &top.keys[0], top.value) {
            out.push(SummaryCard {
                label: "Top MCP".into(),
                text,
            });
        }
    }
    let (ps, pu) = ledger_core::delta::previous_range(since, until)?;
    let by_model = |a: &str, b: &str| -> ledger_core::Result<Vec<(String, f64)>> {
        Ok(aggregate(
            conn,
            &Query {
                since: a.into(),
                until: b.into(),
                grain: Grain::Day,
                group_by: vec![Dim::Model],
                filters: Filters::default(),
                metric: Metric::Tokens,
            },
        )?
        .into_iter()
        .filter(|r| !r.keys[0].is_empty())
        .map(|r| (r.keys[0].clone(), r.value))
        .collect())
    };
    let now = by_model(since, until)?;
    let before = by_model(&ps, &pu)?;
    let mut names: Vec<String> = now.iter().chain(&before).map(|r| r.0.clone()).collect();
    names.sort();
    names.dedup();
    let get = |v: &[(String, f64)], n: &str| v.iter().find(|r| r.0 == n).map_or(0.0, |r| r.1);
    let rows: Vec<(String, f64, f64)> = names
        .into_iter()
        .map(|n| {
            let (c, p) = (get(&now, &n), get(&before, &n));
            (n, c, p)
        })
        .collect();
    if let Some((name, cur, prev)) = biggest_mover(&rows) {
        if let Some(text) = mover_sentence(period, name, *cur, *prev) {
            out.push(SummaryCard {
                label: "Biggest mover".into(),
                text,
            });
        }
    }
    Ok(out)
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
        let app = top(&conn, &s, &u, Dim::App)?.map(|a| ledger_core::summary::app_label(&a));
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
    let (_, week, until) = periods(today)[1];
    out.extend(insight_cards(&conn, &week.to_string(), &until.to_string())?);
    Ok(out)
}

/// Today / this week / this month summary sentences, plus top tool, top MCP
/// server and biggest mover for this week when there is something to say.
#[tauri::command]
#[specta::specta]
pub fn summary_cards(state: State<'_, AppState>) -> IpcResult<Vec<SummaryCard>> {
    cards_for(&state, Utc::now().date_naive())
}
