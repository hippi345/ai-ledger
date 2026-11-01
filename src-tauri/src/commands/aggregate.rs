//! The aggregate query command: typed IPC mirror of `ledger_core::aggregate`,
//! with optional top-N and previous-period comparison.

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::State;

use crate::error::IpcResult;
use crate::state::AppState;
use ledger_core::aggregate as core_agg;
use ledger_core::rollup::Grain;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AggGrain {
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AggDim {
    Bucket,
    App,
    Model,
    Project,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum AggMetric {
    Tokens,
    CostUsd,
    Requests,
    Events,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
pub struct AggFilters {
    #[serde(default)]
    pub app: Vec<String>,
    #[serde(default)]
    pub model: Vec<String>,
    #[serde(default)]
    pub project: Vec<String>,
}

/// An aggregate request. Dates are UTC `YYYY-MM-DD`, `since <= bucket < until`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AggQuery {
    pub since: String,
    pub until: String,
    pub grain: AggGrain,
    pub group_by: Vec<AggDim>,
    pub filters: AggFilters,
    pub metric: AggMetric,
    /// Keep the N largest groups and fold the rest into "Other" (0 or null = all).
    pub top: Option<u32>,
    /// Also total the previous equal-length period.
    pub compare: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
pub struct AggRow {
    pub keys: Vec<String>,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AggResult {
    pub rows: Vec<AggRow>,
    pub total: f64,
    /// Total for the previous period, when `compare` was set.
    pub previous_total: Option<f64>,
    /// Percent change against it; null when it was zero or not requested.
    pub change_pct: Option<f64>,
}

fn to_core(q: &AggQuery, since: String, until: String) -> core_agg::Query {
    core_agg::Query {
        since,
        until,
        grain: match q.grain {
            AggGrain::Day => Grain::Day,
            AggGrain::Week => Grain::Week,
            AggGrain::Month => Grain::Month,
        },
        group_by: q
            .group_by
            .iter()
            .map(|d| match d {
                AggDim::Bucket => core_agg::Dim::Bucket,
                AggDim::App => core_agg::Dim::App,
                AggDim::Model => core_agg::Dim::Model,
                AggDim::Project => core_agg::Dim::Project,
            })
            .collect(),
        filters: core_agg::Filters {
            app: q.filters.app.clone(),
            model: q.filters.model.clone(),
            project: q.filters.project.clone(),
        },
        metric: match q.metric {
            AggMetric::Tokens => core_agg::Metric::Tokens,
            AggMetric::CostUsd => core_agg::Metric::CostUsd,
            AggMetric::Requests => core_agg::Metric::Requests,
            AggMetric::Events => core_agg::Metric::Events,
        },
    }
}

/// Core logic, callable without a Tauri runtime (tests).
pub fn run(state: &AppState, q: &AggQuery) -> IpcResult<AggResult> {
    let conn = state.db();
    let rows = core_agg::aggregate(&conn, &to_core(q, q.since.clone(), q.until.clone()))?;
    let total = rows.iter().map(|r| r.value).sum();
    let rows = match q.top {
        Some(n) if n > 0 => core_agg::top_n(rows, n as usize),
        _ => rows,
    };
    let previous_total = if q.compare {
        let (ps, pu) = ledger_core::delta::previous_range(&q.since, &q.until)?;
        let mut prev = to_core(q, ps, pu);
        prev.group_by.clear();
        Some(
            core_agg::aggregate(&conn, &prev)?
                .iter()
                .map(|r| r.value)
                .sum(),
        )
    } else {
        None
    };
    let change_pct = previous_total.and_then(|p| ledger_core::delta::delta(total, p).pct);
    Ok(AggResult {
        rows: rows
            .into_iter()
            .map(|r| AggRow {
                keys: r.keys,
                value: r.value,
            })
            .collect(),
        total,
        previous_total,
        change_pct,
    })
}

/// Sums a metric over rollups, grouped and filtered (spec §8).
#[tauri::command]
#[specta::specta]
pub fn aggregate(state: State<'_, AppState>, query: AggQuery) -> IpcResult<AggResult> {
    run(&state, &query)
}
