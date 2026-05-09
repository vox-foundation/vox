//! `TelemetryConfig`: read once at startup, governs which sinks and categories are active.
//!
//! Resolution order (highest wins):
//!   1. `/etc/vox/telemetry-policy.toml` — org-level hard-off (Phase D)
//!   2. `~/.config/vox/config.toml`        — user preference (Phase D)
//!   3. `VOX_TELEMETRY`                    — master on/off/debug (Phase D)
//!   4. Legacy per-category env vars        — compat shim
//!   5. Default                             — local collection on, remote upload off
//!
//! Phases B–D complete the config loading. In Phase A, only `from_env_legacy` is
//! implemented to keep the existing per-category env-var gates working.

/// Master telemetry configuration.
#[derive(Debug, Clone)]
pub struct TelemetryConfig {
    /// Master switch. When false, `record_event!` is a guaranteed no-op.
    pub enabled: bool,
    /// Whether to attempt remote upload via the spool. Requires Clavis credentials (ADR 023).
    pub remote_upload: bool,
    /// Gather research_metrics (existing categories: benchmark, syntax_k, socrates, etc.).
    pub research_metrics: bool,
    /// Gather per-LLM-call model performance events (Phase B).
    pub model_calls: bool,
    /// Gather agent dispatch + task root summary (Phase C).
    pub agent_orchestration: bool,
    /// Gather build summary events (Phase D).
    pub build: bool,
    /// Gather subsystem error events (Phase D).
    pub errors: bool,
}

impl TelemetryConfig {
    /// Returns the default config: local collection on, all categories on, remote upload off.
    pub fn default_on() -> Self {
        Self {
            enabled: true,
            remote_upload: false,
            research_metrics: true,
            model_calls: true,
            agent_orchestration: true,
            build: true,
            errors: true,
        }
    }

    /// Returns an all-off config (used for tests and when master switch is disabled).
    pub fn all_off() -> Self {
        Self {
            enabled: false,
            remote_upload: false,
            research_metrics: false,
            model_calls: false,
            agent_orchestration: false,
            build: false,
            errors: false,
        }
    }

    /// Load from legacy per-category env vars. The master `VOX_TELEMETRY` switch and
    /// file-based config layers are added in Phase D.
    pub fn from_env_legacy() -> Self {
        let benchmark_on = env_flag("VOX_BENCHMARK_TELEMETRY");
        Self {
            enabled: true,
            remote_upload: false,
            // In Phase A, research_metrics category follows legacy benchmark gate.
            // Phase D replaces this with the master switch.
            research_metrics: benchmark_on.unwrap_or(false),
            model_calls: false,         // activated Phase B
            agent_orchestration: false, // activated Phase C
            build: false,               // activated Phase D
            errors: false,              // activated Phase D
        }
    }
}

fn env_flag(key: &str) -> Option<bool> {
    match std::env::var(key).ok()?.trim() {
        "1" | "true" | "yes" => Some(true),
        "0" | "false" | "no" => Some(false),
        _ => None,
    }
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self::from_env_legacy()
    }
}
