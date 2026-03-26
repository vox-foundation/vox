//! Nested document types for [`crate::ScalingPolicy`].

use serde::Deserialize;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Thresholds {
    /// Default max file size (bytes) before streaming recommendations apply.
    #[serde(default = "default_max_file_bytes")]
    pub max_file_bytes_hint: u64,
    /// Suggested JSONL / corpus lines per batch.
    #[serde(default = "default_batch_lines")]
    pub corpus_validate_batch_lines: u64,
}

fn default_max_file_bytes() -> u64 {
    64 * 1024 * 1024
}

fn default_batch_lines() -> u64 {
    10_000
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PathLiterals {
    /// Relative path fragments that should use [`crate::DEFAULT_MENS_RUNS_ROOT`] or env/config instead of literals.
    #[serde(default = "default_mens_runs_literals")]
    pub mens_runs_variants: Vec<String>,
    /// Other fragments to flag in TOESTUB scaling scans.
    #[serde(default)]
    pub extra_flag_literals: Vec<String>,
}

fn default_mens_runs_literals() -> Vec<String> {
    vec![
        "mens/runs/v1".to_string(),
        "mens/runs/latest".to_string(),
        "mens/runs/uv_output".to_string(),
        "mens/runs/".to_string(),
        "mens/runs".to_string(),
    ]
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PerCrateOverride {
    pub crate_name: String,
    #[serde(default)]
    pub allow_blocking_fs_in_async: bool,
    #[serde(default)]
    pub notes: String,
}
