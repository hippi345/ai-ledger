//! Pluggable usage readers, their registry, and per-source isolation.
//!
//! A reader turns some app's local data into [`RawEvent`]s. Readers are
//! registered once in a [`Registry`]. Each source is read through
//! [`Registry::read_source`], which catches errors *and panics* so that one
//! misbehaving reader only marks its own source unhealthy.

use std::collections::BTreeMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::ingest::NewEvent;
use crate::{Error, Result};

/// A usage record produced by a reader (ingested via [`crate::ingest`]).
pub type RawEvent = NewEvent;

/// Fidelity tier of a reader (spec §5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// The app's own on-disk logs or databases.
    Native = 1,
    /// User-pointed exported files (CSV/JSON).
    Export = 2,
    /// Opt-in capture (local proxy, manual entry).
    Capture = 3,
}

impl Tier {
    /// Numeric tier as stored in `sources.tier`.
    pub fn as_i64(self) -> i64 {
        self as i64
    }
}

/// Opaque resume position owned by the reader (e.g. file offsets, row ids).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Cursor(pub String);

/// Configuration of one enabled source.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceCfg {
    pub source_id: i64,
    pub reader_id: String,
    pub config: serde_json::Value,
}

/// A source a reader found on this machine, offered to the user to enable.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SourceCandidate {
    pub reader_id: String,
    pub label: String,
    pub config: serde_json::Value,
}

/// The reader contract (spec §5).
pub trait Reader: Send + Sync {
    /// Stable id, e.g. `"cursor"`.
    fn id(&self) -> &str;
    fn tier(&self) -> Tier;
    /// Finds sources on this machine. Must be cheap and must not fail loudly.
    fn detect(&self) -> Vec<SourceCandidate>;
    /// Files or directories to watch for changes.
    fn watch_paths(&self, cfg: &SourceCfg) -> Vec<PathBuf>;
    /// Reads everything after `cursor`, returning new events and the next cursor.
    fn read_since(
        &self,
        cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> Result<(Vec<RawEvent>, Cursor)>;
}

/// Health of one source after its most recent read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Health {
    Unknown,
    Healthy { events: usize },
    Unhealthy { error: String },
}

/// Static registry of readers plus per-source health.
#[derive(Default)]
pub struct Registry {
    readers: BTreeMap<String, Box<dyn Reader>>,
    health: BTreeMap<i64, Health>,
}

impl Registry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a reader. Duplicate ids are a programming error and rejected.
    pub fn register(&mut self, reader: Box<dyn Reader>) -> Result<()> {
        let id = reader.id().to_owned();
        if self.readers.contains_key(&id) {
            return Err(Error::InvalidInput(format!(
                "reader {id:?} registered twice"
            )));
        }
        self.readers.insert(id, reader);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<&dyn Reader> {
        self.readers.get(id).map(|r| r.as_ref())
    }

    pub fn ids(&self) -> impl Iterator<Item = &str> {
        self.readers.keys().map(String::as_str)
    }

    /// Runs `detect` on every reader; a panicking reader contributes nothing.
    pub fn detect_all(&self) -> Vec<SourceCandidate> {
        let mut out = Vec::new();
        for r in self.readers.values() {
            match catch_unwind(AssertUnwindSafe(|| r.detect())) {
                Ok(found) => out.extend(found),
                Err(_) => tracing::error!(reader = r.id(), "detect panicked"),
            }
        }
        out
    }

    /// Reads one source in isolation and records its health.
    pub fn read_source(
        &mut self,
        cfg: &SourceCfg,
        cursor: Option<Cursor>,
    ) -> Result<(Vec<RawEvent>, Cursor)> {
        let result = match self.readers.get(&cfg.reader_id) {
            None => Err(Error::InvalidInput(format!(
                "unknown reader {:?}",
                cfg.reader_id
            ))),
            Some(r) => match catch_unwind(AssertUnwindSafe(|| r.read_since(cfg, cursor))) {
                Ok(res) => res,
                Err(p) => Err(Error::InvalidInput(format!(
                    "reader panicked: {}",
                    panic_msg(p.as_ref())
                ))),
            },
        };
        let health = match &result {
            Ok((ev, _)) => Health::Healthy { events: ev.len() },
            Err(e) => {
                tracing::warn!(source = cfg.source_id, reader = %cfg.reader_id, error = %e, "source unhealthy");
                Health::Unhealthy {
                    error: e.to_string(),
                }
            }
        };
        self.health.insert(cfg.source_id, health);
        result
    }

    pub fn health(&self, source_id: i64) -> Health {
        self.health
            .get(&source_id)
            .cloned()
            .unwrap_or(Health::Unknown)
    }
}

fn panic_msg(p: &(dyn std::any::Any + Send)) -> String {
    p.downcast_ref::<&str>()
        .map(|s| (*s).to_owned())
        .or_else(|| p.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Fake {
        id: &'static str,
        mode: u8,
    }

    impl Reader for Fake {
        fn id(&self) -> &str {
            self.id
        }
        fn tier(&self) -> Tier {
            Tier::Native
        }
        fn detect(&self) -> Vec<SourceCandidate> {
            if self.mode == 2 {
                panic!("detect boom");
            }
            vec![SourceCandidate {
                reader_id: self.id.into(),
                label: self.id.into(),
                config: serde_json::json!({}),
            }]
        }
        fn watch_paths(&self, _: &SourceCfg) -> Vec<PathBuf> {
            vec![]
        }
        fn read_since(&self, _: &SourceCfg, c: Option<Cursor>) -> Result<(Vec<RawEvent>, Cursor)> {
            match self.mode {
                0 => Ok((
                    vec![RawEvent::default()],
                    Cursor(format!("{}+1", c.unwrap_or_default().0)),
                )),
                1 => Err(Error::InvalidInput("bad file".into())),
                _ => panic!("read boom"),
            }
        }
    }

    fn cfg(id: i64, reader: &str) -> SourceCfg {
        SourceCfg {
            source_id: id,
            reader_id: reader.into(),
            config: serde_json::json!({}),
        }
    }

    fn registry() -> Registry {
        let mut r = Registry::new();
        r.register(Box::new(Fake { id: "ok", mode: 0 })).unwrap();
        r.register(Box::new(Fake { id: "err", mode: 1 })).unwrap();
        r.register(Box::new(Fake {
            id: "panic",
            mode: 2,
        }))
        .unwrap();
        r
    }

    #[test]
    fn duplicate_ids_rejected() {
        let mut r = registry();
        assert!(r.register(Box::new(Fake { id: "ok", mode: 0 })).is_err());
        assert_eq!(r.ids().collect::<Vec<_>>(), ["err", "ok", "panic"]);
    }

    #[test]
    fn failures_are_isolated_per_source() {
        let mut r = registry();
        let (ev, cur) = r
            .read_source(&cfg(1, "ok"), Some(Cursor("a".into())))
            .unwrap();
        assert_eq!((ev.len(), cur.0.as_str()), (1, "a+1"));
        assert!(r.read_source(&cfg(2, "err"), None).is_err());
        let e = r.read_source(&cfg(3, "panic"), None).unwrap_err();
        assert!(e.to_string().contains("read boom"));
        assert!(r.read_source(&cfg(4, "missing"), None).is_err());
        assert_eq!(r.health(1), Health::Healthy { events: 1 });
        assert!(matches!(r.health(2), Health::Unhealthy { .. }));
        assert!(matches!(r.health(3), Health::Unhealthy { .. }));
        assert_eq!(r.health(9), Health::Unknown);
    }

    #[test]
    fn detect_survives_panicking_reader() {
        let found = registry().detect_all();
        let ids: Vec<_> = found.iter().map(|c| c.reader_id.as_str()).collect();
        assert_eq!(ids, ["err", "ok"]);
    }
}
