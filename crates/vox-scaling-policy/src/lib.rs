//! Scaling policy loaded from the repo SSOT [`contracts/scaling/policy.yaml`](../../contracts/scaling/policy.yaml).
//!
//! Use [`ScalingPolicy::embedded`] for stable defaults without I/O. For local overrides in tests,
//! use [`ScalingPolicy::from_yaml_str`].

use serde::Deserialize;

const EMBEDDED_YAML: &str = include_str!("../../../contracts/scaling/policy.yaml");

/// Full policy document from SSOT YAML.
#[derive(Debug, Clone, Deserialize)]
pub struct ScalingPolicy {
    #[serde(default)]
    pub schema_version: u32,
    /// Human-readable baseline id (e.g. git tag or date).
    #[serde(default)]
    pub baseline_id: String,
    #[serde(default)]
    pub thresholds: Thresholds,
    #[serde(default)]
    pub path_literals: PathLiterals,
    #[serde(default)]
    pub magic_numeric_hints: Vec<u64>,
    #[serde(default)]
    pub per_crate_overrides: Vec<PerCrateOverride>,
}

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
    /// Relative path fragments that should use [`DEFAULT_MENS_RUNS_ROOT`] or env/config instead of literals.
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

impl ScalingPolicy {
    /// Policy embedded at build time from `contracts/scaling/policy.yaml`.
    pub fn embedded() -> Self {
        Self::from_yaml_str(EMBEDDED_YAML).expect("embedded scaling policy YAML must parse")
    }

    /// Parse policy from YAML (for tests or tooling).
    pub fn from_yaml_str(s: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(s)
    }
}

// ---------------------------------------------------------------------------
// Stable path constants (SSOT-backed; keep literals out of call sites)
// ---------------------------------------------------------------------------

/// Canonical default for Mens/Populi run artifact root (repo-relative).
pub const DEFAULT_MENS_RUNS_ROOT: &str = "mens/runs";

/// Latest symlink / pointer directory under [`DEFAULT_MENS_RUNS_ROOT`].
pub const DEFAULT_MENS_RUNS_LATEST: &str = "mens/runs/latest";

/// Default training run layout revision under [`DEFAULT_MENS_RUNS_ROOT`].
pub const DEFAULT_MENS_RUNS_V1: &str = "mens/runs/v1";

/// Default UV / tooling output subdirectory under runs.
pub const DEFAULT_MENS_RUNS_UV_OUTPUT: &str = "mens/runs/uv_output";

/// Default QLoRA run directory basename (repo-relative path prefix).
pub const DEFAULT_MENS_RUNS_QWEN_QLORA: &str = "mens/runs/qwen25_qlora";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_parses() {
        let p = ScalingPolicy::embedded();
        assert!(p.schema_version >= 1);
        assert!(!p.path_literals.mens_runs_variants.is_empty());
    }
}
