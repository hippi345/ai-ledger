//! Tool / MCP call queries: typed IPC mirror of `ledger_core::aggregate_tools`.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::commands::aggregate::{AggGrain, AggRow};
use crate::error::IpcResult;
use crate::state::AppState;
use ledger_core::aggregate::{self as core_agg, ToolDim, ToolMetric};
use ledger_core::rollup::Grain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ToolAggDim {
    Bucket,
    App,
    Tool,
    McpServer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum ToolAggMetric {
    Calls,
    Failures,
    DurationMs,
}

/// A tool/MCP query. Dates are UTC `YYYY-MM-DD`, `since <= bucket < until`.
/// Calls without an MCP server have `mcpServer = ""`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ToolAggQuery {
    pub since: String,
    pub until: String,
    pub grain: AggGrain,
    pub group_by: Vec<ToolAggDim>,
    pub app: Vec<String>,
    pub mcp_server: Vec<String>,
    pub metric: ToolAggMetric,
    /// Keep the N largest groups and fold the rest into "Other" (0 or null = all).
    pub top: Option<u32>,
}

/// Core logic, callable without a Tauri runtime (tests).
pub fn run(state: &AppState, q: &ToolAggQuery) -> IpcResult<Vec<AggRow>> {
    let query = core_agg::ToolQuery {
        since: q.since.clone(),
        until: q.until.clone(),
        grain: match q.grain {
            AggGrain::Day => Grain::Day,
            AggGrain::Week => Grain::Week,
            AggGrain::Month => Grain::Month,
        },
        group_by: q
            .group_by
            .iter()
            .map(|d| match d {
                ToolAggDim::Bucket => ToolDim::Bucket,
                ToolAggDim::App => ToolDim::App,
                ToolAggDim::Tool => ToolDim::Tool,
                ToolAggDim::McpServer => ToolDim::McpServer,
            })
            .collect(),
        app: q.app.clone(),
        mcp_server: q.mcp_server.clone(),
        metric: match q.metric {
            ToolAggMetric::Calls => ToolMetric::Calls,
            ToolAggMetric::Failures => ToolMetric::Failures,
            ToolAggMetric::DurationMs => ToolMetric::DurationMs,
        },
    };
    let rows = core_agg::aggregate_tools(&state.db(), &query)?;
    let rows = match q.top {
        Some(n) if n > 0 => core_agg::top_n(rows, n as usize),
        _ => rows,
    };
    Ok(rows
        .into_iter()
        .map(|r| AggRow {
            keys: r.keys,
            value: r.value,
        })
        .collect())
}

/// Sums tool/MCP calls, failures or duration, grouped and filtered.
#[tauri::command]
#[specta::specta]
pub fn aggregate_tools(state: State<'_, AppState>, query: ToolAggQuery) -> IpcResult<Vec<AggRow>> {
    run(&state, &query)
}
