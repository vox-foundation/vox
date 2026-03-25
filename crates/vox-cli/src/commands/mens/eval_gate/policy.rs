//! Eval gate policy types and YAML loading.

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

/// Eval gate policy schema (from eval-gates.yaml).
#[derive(Debug, Clone, Deserialize)]
pub struct EvalGatePolicy {
    #[serde(default)]
    #[allow(dead_code)]
    pub version: String,
    #[serde(default)]
    #[allow(dead_code)]
    pub perplexity: PerplexityGate,
    #[serde(default)]
    pub supervised_ratio: SupervisedRatioGate,
    #[serde(default)]
    pub truncation: TruncationGate,
    #[serde(default)]
    pub throughput: ThroughputGate,
    #[serde(default)]
    pub eval_local: EvalLocalGate,
    /// Per-context thresholds keyed by context name ("target", "meta").
    #[serde(default)]
    pub per_context: std::collections::HashMap<String, ContextGateEntry>,
    /// Modal mix gate — warns/blocks when a modality exceeds its ceiling.
    #[serde(default)]
    pub modal_mix: ModalMixGate,
    /// MCP `call_tool` JSON Schema KPI (`tool_arg_schema_kpi` snapshot written next to run artifacts).
    #[serde(default)]
    pub mcp_tool_schema: McpToolSchemaGate,
}

/// Gate on optional `mcp_tool_schema_kpi.json` in the run directory (from `vox-mcp` diagnostics).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct McpToolSchemaGate {
    /// Minimum `strict_validity_rate` (0.0–1.0). `0.0` disables the gate unless `block` is true.
    #[serde(default)]
    pub min_strict_validity_rate: f64,
    #[serde(default)]
    pub block: bool,
    /// Basename only (path segments with `..` are ignored; falls back to default file).
    #[serde(default = "default_mcp_tool_schema_metrics_file")]
    pub metrics_file: String,
}

fn default_mcp_tool_schema_metrics_file() -> String {
    "mcp_tool_schema_kpi.json".to_string()
}

pub(super) fn mcp_tool_schema_metrics_path(run_dir: &Path, configured: &str) -> std::path::PathBuf {
    let p = std::path::Path::new(configured);
    let fname = p.file_name().unwrap_or_default();
    if fname.is_empty() {
        run_dir.join(default_mcp_tool_schema_metrics_file())
    } else {
        run_dir.join(fname)
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PerplexityGate {
    #[serde(default = "default_perplexity_regression")]
    #[allow(dead_code)]
    pub max_regression_ratio: f64,
    #[serde(default)]
    #[allow(dead_code)]
    pub block: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct SupervisedRatioGate {
    #[serde(default = "default_min_supervised")]
    pub min_pct: f64,
    #[serde(default)]
    pub block: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct TruncationGate {
    #[serde(default = "default_max_truncation")]
    pub max_fraction: f64,
    #[serde(default)]
    pub block: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ThroughputGate {
    #[serde(default)]
    pub min_tokens_per_sec: std::collections::HashMap<String, f64>,
    #[serde(default)]
    pub block: bool,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct EvalLocalGate {
    #[serde(default = "default_min_parse_rate")]
    pub min_parse_rate: f64,
    #[serde(default = "default_min_coverage")]
    pub min_coverage_pct: f64,
    #[serde(default)]
    pub block: bool,
}

fn default_perplexity_regression() -> f64 {
    1.05
}
fn default_min_supervised() -> f64 {
    0.10
}
fn default_max_truncation() -> f64 {
    0.50
}
fn default_min_parse_rate() -> f64 {
    0.60
}
fn default_min_coverage() -> f64 {
    0.35
}

/// Single per-context threshold entry from the `per_context:` map.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ContextGateEntry {
    /// Minimum fraction of records that must parse as valid Vox (0.0–1.0).
    #[serde(default)]
    pub min_parse_rate: f64,
    /// Minimum scope compliance rate (0.0–1.0).
    #[serde(default)]
    pub min_scope_compliance_rate: f64,
    /// If true, a violation blocks the run.
    #[serde(default)]
    pub block: bool,
}

/// Modal-mix gate: warns/blocks when a modality's record fraction exceeds a ceiling.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ModalMixGate {
    /// Maximum fraction of output records that may be tagged `modal: voice` (0.0–1.0).
    #[serde(default)]
    pub max_voice_fraction: f64,
    /// If true, a violation blocks the run.
    #[serde(default)]
    pub block: bool,
}

/// Load policy from path (default: mens/config/eval-gates.yaml).
pub fn load_policy(path: &Path) -> Result<EvalGatePolicy> {
    let content = std::fs::read_to_string(path)?;
    let policy: EvalGatePolicy = serde_yaml::from_str(&content)?;
    Ok(policy)
}
