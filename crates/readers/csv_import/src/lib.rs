//! Tier 2 reader: user-pointed CSV or JSON usage exports with column mapping.
//!
//! The user picks a file (for example a vendor dashboard export) and maps its
//! columns onto ledger fields. Numbers may contain thousands separators.
//! Timestamps may be ISO-8601, `YYYY-MM-DD`, or epoch seconds/milliseconds.
//! Rows that fail to parse are counted and skipped, never guessed.
//!
//! Each row's `raw_hash` is derived from the file path plus its mapped values,
//! so re-importing the same (or a regenerated, overlapping) export does not
//! double count.

use std::path::{Path, PathBuf};

use ledger_core::cost::ServiceTier;
use ledger_core::ingest::{raw_hash, NewEvent};
use ledger_core::reader::{Cursor, RawEvent, Reader, SourceCandidate, SourceCfg, Tier};
use ledger_core::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub mod inspect;

pub const READER_ID: &str = "csv_import";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Format {
    Csv,
    Json,
}

/// Which source column feeds each ledger field. For JSON, a column is a
/// dot-separated path such as `usage.input`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mapping {
    pub ts: String,
    pub app: Option<String>,
    pub model: Option<String>,
    pub project: Option<String>,
    pub input_tokens: Option<String>,
    pub output_tokens: Option<String>,
    pub cache_read_tokens: Option<String>,
    pub cache_write_tokens: Option<String>,
    pub requests: Option<String>,
    /// Vendor service tier column (e.g. OpenAI's `service_tier`: default,
    /// batch, flex, priority). Unrecognized or empty values stay unset and
    /// bill as Standard; the tier is never guessed.
    pub service_tier: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub path: PathBuf,
    pub format: Format,
    pub mapping: Mapping,
    /// App name used when no `app` column is mapped (e.g. `"openai"`).
    pub default_app: String,
}

/// Result of parsing a whole file.
#[derive(Debug, Default)]
pub struct Parsed {
    pub events: Vec<NewEvent>,
    pub skipped: usize,
}

fn digits(s: &str) -> Option<u64> {
    let t: String = s
        .trim()
        .chars()
        .filter(|c| !matches!(c, ',' | '_' | ' '))
        .collect();
    if t.is_empty() {
        return Some(0);
    }
    t.parse::<u64>().ok().or_else(|| {
        t.parse::<f64>()
            .ok()
            .filter(|f| f.is_finite() && *f >= 0.0)
            .map(|f| f.round() as u64)
    })
}

fn is_iso_date(s: &str) -> bool {
    let b = s.as_bytes();
    b.len() >= 10
        && b[..10].iter().enumerate().all(|(i, c)| {
            if i == 4 || i == 7 {
                *c == b'-'
            } else {
                c.is_ascii_digit()
            }
        })
}

fn iso_from_ms(ms: i64) -> String {
    let secs = ms.div_euclid(1000);
    let (days, sod) = (secs.div_euclid(86_400), secs.rem_euclid(86_400));
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        sod / 3600,
        sod % 3600 / 60,
        sod % 60
    )
}

/// Normalizes a timestamp cell to ISO-8601 UTC.
pub fn parse_ts(s: &str) -> Option<String> {
    let s = s.trim();
    if let Ok(n) = s.parse::<i64>() {
        // Heuristic: values above 10^11 are milliseconds (10^11 s is year 5138).
        return Some(iso_from_ms(if n.abs() >= 100_000_000_000 {
            n
        } else {
            n * 1000
        }));
    }
    if !is_iso_date(s) {
        return None;
    }
    Some(if s.len() == 10 {
        format!("{s}T00:00:00Z")
    } else {
        s.to_owned()
    })
}

fn cell(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

fn json_path<'a>(v: &'a Value, path: &str) -> Option<&'a Value> {
    path.split('.').try_fold(v, |cur, key| cur.get(key))
}

/// Builds an event from a row accessor `get(column) -> Option<String>`.
fn row_event(cfg: &Config, get: impl Fn(&str) -> Option<String>) -> Option<NewEvent> {
    let m = &cfg.mapping;
    let text = |col: &Option<String>| {
        col.as_deref()
            .and_then(&get)
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
    };
    let num = |col: &Option<String>| -> Option<u64> {
        match col.as_deref().and_then(&get) {
            None => Some(0),
            Some(s) => digits(&s),
        }
    };
    let ts = parse_ts(&get(&m.ts)?)?;
    let ev = NewEvent {
        ts_utc: ts,
        app: text(&m.app).unwrap_or_else(|| cfg.default_app.clone()),
        model: text(&m.model),
        project: text(&m.project),
        input_tokens: num(&m.input_tokens)?,
        output_tokens: num(&m.output_tokens)?,
        cache_read_tokens: num(&m.cache_read_tokens)?,
        cache_write_tokens: num(&m.cache_write_tokens)?,
        requests: if m.requests.is_some() {
            num(&m.requests)?
        } else {
            1
        },
        service_tier: text(&m.service_tier).and_then(|s| ServiceTier::parse(&s)),
        ..Default::default()
    };
    let path = cfg.path.to_string_lossy();
    let key = format!(
        "{}|{}|{:?}|{:?}|{}|{}|{}|{}|{}",
        ev.ts_utc,
        ev.app,
        ev.model,
        ev.project,
        ev.input_tokens,
        ev.output_tokens,
        ev.cache_read_tokens,
        ev.cache_write_tokens,
        ev.requests
    );
    Some(NewEvent {
        raw_hash: raw_hash([READER_ID, path.as_ref(), key.as_str()]),
        ..ev
    })
}

/// Parses CSV text (first row = headers).
pub fn parse_csv(cfg: &Config, data: &[u8]) -> Result<Parsed> {
    let mut r = csv::ReaderBuilder::new().flexible(true).from_reader(data);
    let headers: Vec<String> = r
        .headers()
        .map_err(|e| Error::InvalidInput(format!("CSV headers: {e}")))?
        .iter()
        .map(|h| h.trim().to_owned())
        .collect();
    let mut out = Parsed::default();
    for rec in r.records() {
        let Ok(rec) = rec else {
            out.skipped += 1;
            continue;
        };
        let get = |col: &str| {
            headers
                .iter()
                .position(|h| h == col)
                .and_then(|i| rec.get(i))
                .map(str::to_owned)
        };
        match row_event(cfg, get) {
            Some(e) => out.events.push(e),
            None => out.skipped += 1,
        }
    }
    Ok(out)
}

/// Parses a JSON array of objects, or an object wrapping one under `data`/`rows`/`items`.
pub fn parse_json(cfg: &Config, data: &[u8]) -> Result<Parsed> {
    let doc: Value = serde_json::from_slice(data)?;
    let rows = match &doc {
        Value::Array(a) => a,
        Value::Object(o) => ["data", "rows", "items"]
            .iter()
            .find_map(|k| o.get(*k).and_then(Value::as_array))
            .ok_or_else(|| Error::InvalidInput("JSON has no data/rows/items array".into()))?,
        _ => {
            return Err(Error::InvalidInput(
                "JSON must be an array or object".into(),
            ))
        }
    };
    let mut out = Parsed::default();
    for row in rows {
        match row_event(cfg, |col| json_path(row, col).map(cell)) {
            Some(e) => out.events.push(e),
            None => out.skipped += 1,
        }
    }
    Ok(out)
}

/// Reads and parses the configured file.
pub fn parse_file(cfg: &Config) -> Result<Parsed> {
    let data = std::fs::read(&cfg.path).map_err(|e| Error::io(&cfg.path, e))?;
    match cfg.format {
        Format::Csv => parse_csv(cfg, &data),
        Format::Json => parse_json(cfg, &data),
    }
}

/// Tier 2 [`Reader`]. Sources are created by the user, so `detect` finds nothing.
#[derive(Debug, Default, Clone)]
pub struct CsvImportReader;

impl Reader for CsvImportReader {
    fn id(&self) -> &str {
        READER_ID
    }
    fn tier(&self) -> Tier {
        Tier::Export
    }
    fn detect(&self) -> Vec<SourceCandidate> {
        Vec::new()
    }
    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf> {
        serde_json::from_value::<Config>(cfg.config.clone())
            .map(|c| vec![c.path])
            .unwrap_or_default()
    }
    fn read_since(
        &self,
        cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> Result<(Vec<RawEvent>, Cursor)> {
        let c: Config = serde_json::from_value(cfg.config.clone())?;
        let stamp = file_stamp(&c.path)?;
        if cursor.as_ref().is_some_and(|cur| cur.0 == stamp) {
            return Ok((Vec::new(), Cursor(stamp)));
        }
        let parsed = parse_file(&c)?;
        Ok((parsed.events, Cursor(stamp)))
    }
}

/// `len:mtime_ns` of a file, used to skip unchanged files.
fn file_stamp(path: &Path) -> Result<String> {
    let md = std::fs::metadata(path).map_err(|e| Error::io(path, e))?;
    let mtime = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map_or(0, |d| d.as_nanos());
    Ok(format!("{}:{mtime}", md.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/csv_import")
            .join(name)
    }

    fn csv_cfg() -> Config {
        Config {
            path: fixture("usage.csv"),
            format: Format::Csv,
            default_app: "openai".into(),
            mapping: Mapping {
                ts: "Date".into(),
                model: Some("Model".into()),
                project: Some("Project".into()),
                input_tokens: Some("Input Tokens".into()),
                output_tokens: Some("Output Tokens".into()),
                cache_read_tokens: Some("Cached Input".into()),
                requests: Some("Requests".into()),
                ..Mapping::default()
            },
        }
    }

    #[test]
    fn csv_with_mapping() {
        let p = parse_file(&csv_cfg()).unwrap();
        assert_eq!(p.skipped, 1, "bad date row skipped");
        let e = &p.events;
        assert_eq!(e.len(), 3);
        assert_eq!(
            (e[0].ts_utc.as_str(), e[0].input_tokens, e[0].requests),
            ("2026-10-29T00:00:00Z", 1200, 4)
        );
        assert_eq!(e[0].app, "openai");
        assert_eq!(e[1].project, None, "empty cell is None");
        assert_eq!(e[1].cache_read_tokens, 50);
        assert_eq!(e[2].ts_utc, "2026-10-31T12:00:00Z", "epoch seconds");
        assert_eq!(e[2].cache_read_tokens, 0, "empty number is 0");
    }

    #[test]
    fn json_with_paths() {
        let cfg = Config {
            path: fixture("usage.json"),
            format: Format::Json,
            default_app: "anthropic".into(),
            mapping: Mapping {
                ts: "timestamp".into(),
                model: Some("model".into()),
                input_tokens: Some("usage.in".into()),
                output_tokens: Some("usage.out".into()),
                ..Mapping::default()
            },
        };
        let p = parse_file(&cfg).unwrap();
        assert_eq!(p.events.len(), 2);
        assert_eq!(p.events[0].ts_utc, "2026-10-31T12:00:00Z", "epoch ms");
        assert_eq!((p.events[0].input_tokens, p.events[0].requests), (100, 1));
    }

    #[test]
    fn reimport_is_idempotent_and_cursor_skips_unchanged() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("u.csv");
        std::fs::copy(fixture("usage.csv"), &path).unwrap();
        let src = SourceCfg {
            source_id: 1,
            reader_id: READER_ID.into(),
            config: serde_json::to_value(Config {
                path: path.clone(),
                ..csv_cfg()
            })
            .unwrap(),
        };
        let r = CsvImportReader;
        let (ev, cur) = r.read_since(&src, None).unwrap();
        let (again, _) = r.read_since(&src, Some(cur)).unwrap();
        assert!(again.is_empty());
        let mut db = ledger_core::db::open_in_memory().unwrap();
        db.execute(
            "INSERT INTO sources (reader_id, tier) VALUES ('csv_import', 2)",
            [],
        )
        .unwrap();
        ledger_core::ingest::upsert_events(&mut db, 1, &ev).unwrap();
        let s = ledger_core::ingest::upsert_events(&mut db, 1, &ev).unwrap();
        assert_eq!(s.unchanged, 3);
    }

    #[test]
    fn service_tier_column_is_read_only_when_recognized() {
        let mut cfg = csv_cfg();
        cfg.mapping.service_tier = Some("Tier".into());
        let data = b"Date,Model,Input Tokens,Tier\n\
            2026-10-01,gpt-5.6-sol,10,batch\n\
            2026-10-02,gpt-5.6-sol,10,priority\n\
            2026-10-03,gpt-5.6-sol,10,\n\
            2026-10-04,gpt-5.6-sol,10,mystery\n";
        let tiers: Vec<_> = parse_csv(&cfg, data)
            .unwrap()
            .events
            .into_iter()
            .map(|e| e.service_tier)
            .collect();
        assert_eq!(
            tiers,
            [
                Some(ServiceTier::Batch),
                Some(ServiceTier::Fast),
                None,
                None
            ]
        );
        let unmapped = parse_csv(&csv_cfg(), b"Date,Model\n2026-10-01,m\n").unwrap();
        assert_eq!(unmapped.events[0].service_tier, None);
    }

    #[test]
    fn number_and_time_parsing() {
        assert_eq!(digits(" 1,234 "), Some(1234));
        assert_eq!(digits("12.6"), Some(13));
        assert_eq!(digits("-3"), None);
        assert_eq!(digits("abc"), None);
        assert_eq!(
            parse_ts("2026-10-31T01:02:03+00:00").as_deref(),
            Some("2026-10-31T01:02:03+00:00")
        );
        assert_eq!(parse_ts("31/10/2026"), None);
    }
}
