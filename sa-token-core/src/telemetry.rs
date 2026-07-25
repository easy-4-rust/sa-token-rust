//! Stable, privacy-safe telemetry vocabulary for Sa-Token.
//!
//! Token values, login IDs, storage keys, request paths, devices, nonces and
//! arbitrary error strings are deliberately excluded from this schema.

use std::time::Duration;

/// Stable span names emitted by the core runtime.
pub mod spans {
    pub const AUTH: &str = "sa_token.auth";
    pub const PLUGIN_LIFECYCLE: &str = "sa_token.plugin.lifecycle";
}

/// Stable metric names emitted when the `metrics` feature is enabled.
pub mod metrics {
    pub const AUTH_TOTAL: &str = "sa_token_auth_total";
    pub const AUTH_DURATION_SECONDS: &str = "sa_token_auth_duration_seconds";
    pub const PLUGIN_LIFECYCLE_TOTAL: &str = "sa_token_plugin_lifecycle_total";
    pub const STORAGE_DURATION_SECONDS: &str = "sa_token_storage_duration_seconds";
}

/// Bounded authentication outcomes suitable for span fields and metric labels.
pub const AUTH_OUTCOME_ALLOWED: &str = "allowed";
pub const AUTH_OUTCOME_REJECTED: &str = "rejected";
pub const AUTH_OUTCOME_ANONYMOUS: &str = "anonymous";
pub const AUTH_OUTCOME_INVALID: &str = "invalid";

pub(crate) fn record_auth(outcome: &'static str, duration: Duration) {
    #[cfg(feature = "metrics")]
    {
        ::metrics::counter!(metrics::AUTH_TOTAL, "outcome" => outcome).increment(1);
        ::metrics::histogram!(
            metrics::AUTH_DURATION_SECONDS,
            "outcome" => outcome
        )
        .record(duration.as_secs_f64());
    }
    #[cfg(not(feature = "metrics"))]
    let _ = (outcome, duration);
}

pub(crate) fn record_plugin_lifecycle(
    plugin: &'static str,
    action: &'static str,
    outcome: &'static str,
) {
    #[cfg(feature = "metrics")]
    ::metrics::counter!(
        metrics::PLUGIN_LIFECYCLE_TOTAL,
        "plugin" => plugin,
        "action" => action,
        "outcome" => outcome
    )
    .increment(1);
    #[cfg(not(feature = "metrics"))]
    let _ = (plugin, action, outcome);
}
