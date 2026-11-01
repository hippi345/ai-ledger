//! The one query API every page and chart uses (spec §8):
//! `aggregate(range, group_by, filters, metric)` over the rollup tables.

use rusqlite::types::Value;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::rollup::Grain;
use crate::Result;

/// What to sum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Metric {
    /// Input + output + cache read + cache write tokens.
    Tokens,
    CostUsd,
    Requests,
    Events,
}

impl Metric {
    fn sql(self) -> &'static str {
        match self {
            Metric::Tokens => {
                "SUM(input_tokens + output_tokens + cache_read_tokens + cache_write_tokens)"
            }
            Metric::CostUsd => "SUM(cost_usd)",
            Metric::Requests => "SUM(requests)",
            Metric::Events => "SUM(events)",
        }
    }
}

/// A dimension to group by. `Bucket` gives a time series at the query's grain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Dim {
    Bucket,
    App,
    Model,
    Project,
}

impl Dim {
    fn column(self) -> &'static str {
        match self {
            Dim::Bucket => "bucket",
            Dim::App => "app",
            Dim::Model => "model",
            Dim::Project => "project",
        }
    }
}

/// Keep only rows whose dimension value is in the list (empty = no filter).
/// Unknown model/project are matched by `""`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Filters {
    #[serde(default)]
    pub app: Vec<String>,
    #[serde(default)]
    pub model: Vec<String>,
    #[serde(default)]
    pub project: Vec<String>,
}

/// An aggregate query. `since`/`until` are UTC dates (`YYYY-MM-DD`); buckets
/// with `since <= bucket < until` are included.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub since: String,
    pub until: String,
    pub grain: Grain,
    #[serde(default)]
    pub group_by: Vec<Dim>,
    #[serde(default)]
    pub filters: Filters,
    pub metric: Metric,
}

/// One result row: the group's dimension values (in `group_by` order) and the sum.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Row {
    pub keys: Vec<String>,
    pub value: f64,
}

/// Runs `q` against the rollups. Rows are ordered by their keys.
pub fn aggregate(conn: &Connection, q: &Query) -> Result<Vec<Row>> {
    let mut params: Vec<Value> = vec![q.since.clone().into(), q.until.clone().into()];
    let mut wh = String::from("bucket >= ?1 AND bucket < ?2");
    for (col, vals) in [
        ("app", &q.filters.app),
        ("model", &q.filters.model),
        ("project", &q.filters.project),
    ] {
        if vals.is_empty() {
            continue;
        }
        let marks: Vec<String> = vals
            .iter()
            .map(|v| {
                params.push(v.clone().into());
                format!("?{}", params.len())
            })
            .collect();
        wh.push_str(&format!(" AND {col} IN ({})", marks.join(", ")));
    }
    let cols: Vec<&str> = q.group_by.iter().map(|d| d.column()).collect();
    let select = cols.iter().map(|c| format!("{c}, ")).collect::<String>();
    let group = if cols.is_empty() {
        String::new()
    } else {
        format!(" GROUP BY {0} ORDER BY {0}", cols.join(", "))
    };
    let sql = format!(
        "SELECT {select}COALESCE({}, 0) FROM {} WHERE {wh}{group}",
        q.metric.sql(),
        q.grain.table()
    );
    let mut st = conn.prepare(&sql)?;
    let n = cols.len();
    let rows = st.query_map(rusqlite::params_from_iter(params), |r| {
        let keys = (0..n)
            .map(|i| r.get::<_, String>(i))
            .collect::<rusqlite::Result<_>>()?;
        let value: f64 = r.get(n)?;
        Ok(Row { keys, value })
    })?;
    Ok(rows.collect::<rusqlite::Result<_>>()?)
}

/// Key used for the group that collects everything outside the top N.
pub const OTHER: &str = "Other";

/// The `n` largest rows by value (ties broken by keys), plus one `"Other"`
/// row summing the rest when there is a rest. `n == 0` returns everything
/// sorted.
pub fn top_n(mut rows: Vec<Row>, n: usize) -> Vec<Row> {
    rows.sort_by(|a, b| {
        b.value
            .total_cmp(&a.value)
            .then_with(|| a.keys.cmp(&b.keys))
    });
    if n == 0 || rows.len() <= n {
        return rows;
    }
    let rest: f64 = rows[n..].iter().map(|r| r.value).sum();
    let width = rows[0].keys.len().max(1);
    rows.truncate(n);
    rows.push(Row {
        keys: vec![OTHER.to_owned(); width],
        value: rest,
    });
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn seed() -> Connection {
        let c = db::open_in_memory().unwrap();
        for (bucket, app, model, project, input, cost) in [
            ("2026-10-01", "cursor", "gpt-5", "web", 10, 0.5),
            ("2026-10-01", "cursor", "claude", "", 20, 1.0),
            ("2026-10-02", "claude_code", "claude", "web", 40, 2.0),
            ("2026-10-03", "cursor", "gpt-5", "web", 80, 4.0),
        ] {
            c.execute(
                "INSERT INTO rollups_day VALUES (?1, ?2, ?3, ?4, 1, 1, ?5, 0, 0, 0, ?6, 0)",
                rusqlite::params![bucket, app, model, project, input, cost],
            )
            .unwrap();
        }
        c
    }

    fn q(group_by: Vec<Dim>, metric: Metric) -> Query {
        Query {
            since: "2026-10-01".into(),
            until: "2026-10-03".into(),
            grain: Grain::Day,
            group_by,
            filters: Filters::default(),
            metric,
        }
    }

    fn row(keys: &[&str], value: f64) -> Row {
        Row {
            keys: keys.iter().map(|k| (*k).to_owned()).collect(),
            value,
        }
    }

    #[test]
    fn total_respects_the_half_open_range() {
        let r = aggregate(&seed(), &q(vec![], Metric::Tokens)).unwrap();
        assert_eq!(r, vec![row(&[], 70.0)]);
    }

    #[test]
    fn groups_by_several_dims() {
        let r = aggregate(&seed(), &q(vec![Dim::App, Dim::Model], Metric::CostUsd)).unwrap();
        assert_eq!(
            r,
            vec![
                row(&["claude_code", "claude"], 2.0),
                row(&["cursor", "claude"], 1.0),
                row(&["cursor", "gpt-5"], 0.5)
            ]
        );
    }

    #[test]
    fn time_series_and_filters() {
        let mut query = q(vec![Dim::Bucket], Metric::Events);
        query.until = "2026-10-04".into();
        query.filters.app = vec!["cursor".into()];
        query.filters.project = vec!["web".into()];
        let r = aggregate(&seed(), &query).unwrap();
        assert_eq!(
            r,
            vec![row(&["2026-10-01"], 1.0), row(&["2026-10-03"], 1.0)]
        );
    }

    #[test]
    fn empty_range_totals_zero() {
        let mut query = q(vec![], Metric::Requests);
        query.since = "2027-01-01".into();
        query.until = "2027-02-01".into();
        assert_eq!(aggregate(&seed(), &query).unwrap(), vec![row(&[], 0.0)]);
    }

    #[test]
    fn query_deserializes_from_json() {
        let query: Query = serde_json::from_value(serde_json::json!({
            "since": "2026-10-01", "until": "2026-11-01", "grain": "week",
            "group_by": ["model"], "metric": "cost_usd"
        }))
        .unwrap();
        assert_eq!(query.grain, Grain::Week);
        assert_eq!(query.group_by, vec![Dim::Model]);
    }

    #[test]
    fn top_n_groups_the_rest_as_other() {
        let rows = vec![
            row(&["a"], 1.0),
            row(&["b"], 5.0),
            row(&["c"], 3.0),
            row(&["d"], 3.0),
        ];
        assert_eq!(
            top_n(rows.clone(), 2),
            vec![row(&["b"], 5.0), row(&["c"], 3.0), row(&["Other"], 4.0)]
        );
        assert_eq!(
            top_n(rows.clone(), 4).len(),
            4,
            "no Other when nothing is left"
        );
        assert_eq!(top_n(rows, 0)[0], row(&["b"], 5.0));
        assert!(top_n(vec![], 3).is_empty());
    }
}
