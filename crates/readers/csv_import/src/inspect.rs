//! Helpers for the import screen: list a file's columns and preview a
//! mapping before the source is added. Nothing here is stored.

use std::collections::BTreeSet;
use std::path::Path;

use ledger_core::{Error, Result};
use serde_json::Value;

use crate::{parse_csv, parse_json, Config, Format, Mapping};

/// Rows shown as a sample.
pub const SAMPLE_ROWS: usize = 5;
/// Files larger than this are refused (exports are small).
pub const MAX_FILE_BYTES: u64 = 64 * 1024 * 1024;

/// A file's columns (CSV headers or JSON dot paths) and sample values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inspection {
    pub format: Format,
    pub columns: Vec<String>,
    /// Up to [`SAMPLE_ROWS`] rows, one value per column ("" when missing).
    pub sample: Vec<Vec<String>>,
    pub rows: usize,
    /// A starting mapping guessed from column names; the user confirms it.
    pub suggested: Mapping,
}

/// What a mapping would import, without importing it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preview {
    pub events: usize,
    pub skipped: usize,
    pub first_ts: Option<String>,
    pub last_ts: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

fn read(path: &Path) -> Result<Vec<u8>> {
    let md = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    if md.len() > MAX_FILE_BYTES {
        return Err(Error::InvalidInput("file is larger than 64 MB".into()));
    }
    std::fs::read(path).map_err(|e| Error::io(path, e))
}

/// Format from the extension, else from the first non-space byte.
pub fn guess_format(path: &Path, data: &[u8]) -> Format {
    match path
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("json") => Format::Json,
        Some("csv") => Format::Csv,
        _ => match data.iter().find(|b| !b.is_ascii_whitespace()) {
            Some(b'[' | b'{') => Format::Json,
            _ => Format::Csv,
        },
    }
}

fn flatten(prefix: &str, v: &Value, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(o) => {
            for (k, child) in o {
                let p = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten(&p, child, out);
            }
        }
        Value::String(s) => out.push((prefix.to_owned(), s.clone())),
        Value::Null => out.push((prefix.to_owned(), String::new())),
        // Arrays are not addressable by dot paths; skip them.
        Value::Array(_) => {}
        other => out.push((prefix.to_owned(), other.to_string())),
    }
}

fn json_rows(data: &[u8]) -> Result<Vec<Value>> {
    let doc: Value = serde_json::from_slice(data)?;
    match doc {
        Value::Array(a) => Ok(a),
        Value::Object(mut o) => ["data", "rows", "items"]
            .iter()
            .find_map(|k| match o.remove(*k) {
                Some(Value::Array(a)) => Some(a),
                _ => None,
            })
            .ok_or_else(|| Error::InvalidInput("JSON has no data/rows/items array".into())),
        _ => Err(Error::InvalidInput(
            "JSON must be an array or object".into(),
        )),
    }
}

/// Lists the columns of a CSV or JSON export and suggests a mapping.
pub fn inspect(path: &Path) -> Result<Inspection> {
    let data = read(path)?;
    let format = guess_format(path, &data);
    let (columns, sample, rows) = match format {
        Format::Csv => {
            let mut r = csv::ReaderBuilder::new()
                .flexible(true)
                .from_reader(data.as_slice());
            let columns: Vec<String> = r
                .headers()
                .map_err(|e| Error::InvalidInput(format!("CSV headers: {e}")))?
                .iter()
                .map(|h| h.trim().to_owned())
                .collect();
            let mut sample = Vec::new();
            let mut rows = 0;
            for rec in r.records().flatten() {
                rows += 1;
                if sample.len() < SAMPLE_ROWS {
                    sample.push(
                        (0..columns.len())
                            .map(|i| rec.get(i).unwrap_or("").to_owned())
                            .collect(),
                    );
                }
            }
            (columns, sample, rows)
        }
        Format::Json => {
            let all = json_rows(&data)?;
            let flat: Vec<Vec<(String, String)>> = all
                .iter()
                .take(100)
                .map(|row| {
                    let mut f = Vec::new();
                    flatten("", row, &mut f);
                    f
                })
                .collect();
            let mut seen = BTreeSet::new();
            let mut columns = Vec::new();
            for (k, _) in flat.iter().flatten() {
                if seen.insert(k.clone()) {
                    columns.push(k.clone());
                }
            }
            let sample = flat
                .iter()
                .take(SAMPLE_ROWS)
                .map(|f| {
                    columns
                        .iter()
                        .map(|c| {
                            f.iter()
                                .find(|(k, _)| k == c)
                                .map(|(_, v)| v.clone())
                                .unwrap_or_default()
                        })
                        .collect()
                })
                .collect();
            (columns, sample, all.len())
        }
    };
    let suggested = suggest(&columns);
    Ok(Inspection {
        format,
        columns,
        sample,
        rows,
        suggested,
    })
}

fn norm(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphanumeric)
        .collect::<String>()
        .to_ascii_lowercase()
}

/// Guesses a mapping from column names. Each column is used at most once,
/// and a field with no clear match is left unmapped.
pub fn suggest(columns: &[String]) -> Mapping {
    let mut used = BTreeSet::new();
    let mut pick = |names: &[&str]| -> Option<String> {
        for want in names {
            for c in columns {
                let leaf = c.rsplit('.').next().unwrap_or(c);
                if (norm(leaf) == *want || norm(c) == *want) && used.insert(c.clone()) {
                    return Some(c.clone());
                }
            }
        }
        None
    };
    let ts = pick(&[
        "timestamp",
        "time",
        "date",
        "datetime",
        "createdat",
        "created",
        "day",
        "starttime",
    ]);
    let cache_read_tokens = pick(&[
        "cachedinput",
        "cachedinputtokens",
        "cachedtokens",
        "cachereadtokens",
        "cachereadinputtokens",
        "inputcached",
    ]);
    let cache_write_tokens = pick(&[
        "cachewritetokens",
        "cachecreationinputtokens",
        "cachecreationtokens",
        "cachewrite",
    ]);
    Mapping {
        ts: ts.unwrap_or_default(),
        cache_read_tokens,
        cache_write_tokens,
        input_tokens: pick(&[
            "inputtokens",
            "input",
            "prompttokens",
            "ninputtokens",
            "contexttokens",
        ]),
        output_tokens: pick(&[
            "outputtokens",
            "output",
            "completiontokens",
            "noutputtokens",
            "generatedtokens",
        ]),
        requests: pick(&[
            "requests",
            "numrequests",
            "nrequests",
            "requestcount",
            "count",
        ]),
        model: pick(&["model", "modelname", "snapshotid", "modelid"]),
        project: pick(&["project", "projectname", "projectid", "workspace"]),
        app: pick(&["app", "application", "product", "source"]),
        service_tier: pick(&["servicetier", "tier"]),
    }
}

/// Parses the file with `cfg` and summarizes the result.
pub fn preview(cfg: &Config) -> Result<Preview> {
    if cfg.mapping.ts.trim().is_empty() {
        return Err(Error::InvalidInput("choose the timestamp column".into()));
    }
    let data = read(&cfg.path)?;
    let parsed = match cfg.format {
        Format::Csv => parse_csv(cfg, &data)?,
        Format::Json => parse_json(cfg, &data)?,
    };
    let mut p = Preview {
        events: parsed.events.len(),
        skipped: parsed.skipped,
        ..Preview::default()
    };
    for e in &parsed.events {
        p.input_tokens += e.input_tokens;
        p.output_tokens += e.output_tokens;
        if !p.first_ts.as_ref().is_some_and(|t| e.ts_utc >= *t) {
            p.first_ts = Some(e.ts_utc.clone());
        }
        if !p.last_ts.as_ref().is_some_and(|t| e.ts_utc <= *t) {
            p.last_ts = Some(e.ts_utc.clone());
        }
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/csv_import")
            .join(name)
    }

    #[test]
    fn csv_columns_sample_and_suggestion() {
        let i = inspect(&fixture("usage.csv")).unwrap();
        assert_eq!(i.format, Format::Csv);
        assert!(i.columns.contains(&"Input Tokens".to_owned()));
        assert!(!i.sample.is_empty() && i.sample.len() <= SAMPLE_ROWS);
        assert!(i.sample.iter().all(|r| r.len() == i.columns.len()));
        let m = &i.suggested;
        assert_eq!(m.ts, "Date");
        assert_eq!(m.input_tokens.as_deref(), Some("Input Tokens"));
        assert_eq!(m.output_tokens.as_deref(), Some("Output Tokens"));
        assert_eq!(m.cache_read_tokens.as_deref(), Some("Cached Input"));
        assert_eq!(m.model.as_deref(), Some("Model"));
    }

    #[test]
    fn json_dot_paths() {
        let i = inspect(&fixture("usage.json")).unwrap();
        assert_eq!(i.format, Format::Json);
        assert!(i.columns.iter().any(|c| c.contains('.')), "{:?}", i.columns);
        assert!(i.rows > 0);
    }

    #[test]
    fn suggest_leaves_unclear_fields_unmapped() {
        let cols = ["when", "foo", "prompt_tokens", "completion_tokens"].map(String::from);
        let m = suggest(&cols);
        assert_eq!(m.ts, "");
        assert_eq!(m.input_tokens.as_deref(), Some("prompt_tokens"));
        assert_eq!(m.output_tokens.as_deref(), Some("completion_tokens"));
        assert_eq!(m.model, None);
    }

    #[test]
    fn preview_matches_import() {
        let i = inspect(&fixture("usage.csv")).unwrap();
        let cfg = Config {
            path: fixture("usage.csv"),
            format: i.format,
            mapping: i.suggested,
            default_app: "openai".into(),
        };
        let p = preview(&cfg).unwrap();
        let full = crate::parse_file(&cfg).unwrap();
        assert_eq!((p.events, p.skipped), (full.events.len(), full.skipped));
        assert!(p.first_ts <= p.last_ts);
        let mut bad = cfg.clone();
        bad.mapping.ts.clear();
        assert!(preview(&bad).is_err());
    }

    #[test]
    fn format_guess() {
        assert_eq!(guess_format(Path::new("a.JSON"), b""), Format::Json);
        assert_eq!(guess_format(Path::new("a.txt"), b"  [1]"), Format::Json);
        assert_eq!(guess_format(Path::new("a"), b"a,b"), Format::Csv);
    }
}
