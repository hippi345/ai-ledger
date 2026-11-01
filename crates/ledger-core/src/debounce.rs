//! Trailing-edge debouncing for file-change bursts (spec §7: 250 ms).

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

/// Default quiet period before a changed source is re-read.
pub const DEFAULT_WINDOW: Duration = Duration::from_millis(250);

/// Collects change signals per key and releases each key once it has been
/// quiet for `window`. Time is passed in so the logic is testable.
#[derive(Debug)]
pub struct Debouncer<K: Ord + Clone> {
    window: Duration,
    last: BTreeMap<K, Instant>,
}

impl<K: Ord + Clone> Debouncer<K> {
    pub fn new(window: Duration) -> Self {
        Self {
            window,
            last: BTreeMap::new(),
        }
    }

    /// Records a change for `key` at `now`, restarting its quiet period.
    pub fn touch(&mut self, key: K, now: Instant) {
        self.last.insert(key, now);
    }

    /// Removes and returns every key that has been quiet for the window.
    pub fn due(&mut self, now: Instant) -> Vec<K> {
        let ready: Vec<K> = self
            .last
            .iter()
            .filter(|(_, t)| now.saturating_duration_since(**t) >= self.window)
            .map(|(k, _)| k.clone())
            .collect();
        for k in &ready {
            self.last.remove(k);
        }
        ready
    }

    /// How long until the next key is due, or `None` when nothing is pending.
    pub fn next_deadline(&self, now: Instant) -> Option<Duration> {
        self.last
            .values()
            .map(|t| (*t + self.window).saturating_duration_since(now))
            .min()
    }
}

impl<K: Ord + Clone> Default for Debouncer<K> {
    fn default() -> Self {
        Self::new(DEFAULT_WINDOW)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MS: Duration = Duration::from_millis(1);

    #[test]
    fn burst_collapses_into_one_release_after_quiet_period() {
        let t0 = Instant::now();
        let mut d = Debouncer::default();
        d.touch(1, t0);
        d.touch(1, t0 + 100 * MS);
        d.touch(1, t0 + 200 * MS);
        assert!(d.due(t0 + 300 * MS).is_empty());
        assert_eq!(d.next_deadline(t0 + 300 * MS), Some(150 * MS));
        assert_eq!(d.due(t0 + 450 * MS), vec![1]);
        assert!(d.due(t0 + 900 * MS).is_empty());
        assert_eq!(d.next_deadline(t0), None);
    }

    #[test]
    fn keys_are_independent() {
        let t0 = Instant::now();
        let mut d = Debouncer::new(100 * MS);
        d.touch("a", t0);
        d.touch("b", t0 + 80 * MS);
        assert_eq!(d.due(t0 + 100 * MS), vec!["a"]);
        assert_eq!(d.due(t0 + 180 * MS), vec!["b"]);
    }
}
