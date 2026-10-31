//! Tracing setup. Logs go to stderr only; nothing is sent anywhere.

use tracing_subscriber::{fmt, EnvFilter};

/// Default filter when `AI_LEDGER_LOG` is unset.
pub const DEFAULT_FILTER: &str = "info,ai_ledger=debug,ledger_core=debug";

/// Builds the filter from `AI_LEDGER_LOG`, falling back to [`DEFAULT_FILTER`].
pub fn filter_from(value: Option<&str>) -> EnvFilter {
    value
        .and_then(|v| EnvFilter::try_new(v).ok())
        .unwrap_or_else(|| EnvFilter::new(DEFAULT_FILTER))
}

/// Installs the global subscriber. Safe to call more than once; later calls are no-ops.
pub fn init() {
    let filter = filter_from(std::env::var("AI_LEDGER_LOG").ok().as_deref());
    let _ = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_filter_falls_back_to_default() {
        assert_eq!(
            filter_from(Some("[[[")).to_string(),
            filter_from(None).to_string()
        );
    }

    #[test]
    fn custom_filter_is_used() {
        assert_eq!(filter_from(Some("warn")).to_string(), "warn");
    }

    #[test]
    fn init_is_idempotent() {
        init();
        init();
    }
}
