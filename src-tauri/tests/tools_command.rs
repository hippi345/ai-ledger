#![allow(clippy::unwrap_used)]

use ai_ledger_lib::commands::aggregate::{AggGrain, AggRow};
use ai_ledger_lib::commands::tools::*;
use ai_ledger_lib::state::AppState;

#[test]
fn groups_by_server_with_top_n() {
    let s = AppState::from_connection(ledger_core::db::open_in_memory().unwrap());
    for (tool, mcp, calls) in [
        ("a", "github", 5),
        ("b", "linear", 3),
        ("c", "slack", 1),
        ("d", "", 9),
    ] {
        s.db()
            .execute(
                "INSERT INTO rollups_tools_day VALUES ('2026-11-03', 'cursor', ?1, ?2, ?3, 0, 0)",
                rusqlite::params![tool, mcp, calls],
            )
            .unwrap();
    }
    let q = ToolAggQuery {
        since: "2026-11-01".into(),
        until: "2026-11-08".into(),
        grain: AggGrain::Day,
        group_by: vec![ToolAggDim::McpServer],
        app: vec![],
        mcp_server: vec!["github".into(), "linear".into(), "slack".into()],
        metric: ToolAggMetric::Calls,
        top: Some(1),
    };
    let r = run(&s, &q).unwrap();
    assert_eq!(
        r,
        vec![
            AggRow {
                keys: vec!["github".into()],
                value: 5.0
            },
            AggRow {
                keys: vec!["Other".into()],
                value: 4.0
            }
        ]
    );
}
