//! `vox mens eval-gate` — check training/eval metrics against policy thresholds.
//!
//! Reads `mens/config/eval-gates.yaml` and validates run artifacts.
//! Exits non-zero if any blocking gate fails.

use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

/// Default minimum Vox parse rate for legacy `vox train` post-training eval (`VOX_EVAL_MIN_PARSE_RATE`).
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) const LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE: f64 = 0.80;
/// Default minimum construct coverage **fraction** (0.0–1.0) for legacy `vox train` (`VOX_EVAL_MIN_COVERAGE`).
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) const LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE: f64 = 0.60;

/// After legacy local/native training, evaluate `train.jsonl` quality and optionally enforce `VOX_EVAL_STRICT`.
///
/// Writes `eval_results.json` with `construct_coverage_pct` on the **0–100** scale so
/// [`check_run`] / `vox mens eval-gate` agree with `eval_local` policy thresholds.
#[cfg(all(feature = "mens-dei", feature = "gpu"))]
pub(crate) fn run_legacy_train_post_eval_gate(
    data_dir: &Path,
    output_dir: Option<&Path>,
) -> anyhow::Result<()> {
    use owo_colors::OwoColorize;

    let train_jsonl = data_dir.join("train.jsonl");
    if !train_jsonl.exists() {
        return Ok(());
    }

    let min_parse_rate = std::env::var("VOX_EVAL_MIN_PARSE_RATE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_PARSE_RATE);
    let min_coverage = std::env::var("VOX_EVAL_MIN_COVERAGE")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(LEGACY_TRAIN_POST_EVAL_DEFAULT_MIN_COVERAGE);

    let eval_output = output_dir
        .unwrap_or(data_dir)
        .to_path_buf()
        .join("eval_results.json");

    println!("{}", "\n── Post-Training Eval Gate ──".bold());

    let eval_result = crate::commands::corpus::eval_metrics(&train_jsonl);

    let (parse_rate, coverage_frac) = match eval_result {
        Ok(m) => (m.parse_rate, m.coverage_pct),
        Err(e) => {
            eprintln!("{} Eval gate error: {}", "⚠".yellow(), e);
            return Ok(()); // Non-fatal — don't block training on eval errors
        }
    };

    let parse_ok = parse_rate >= min_parse_rate;
    let coverage_ok = coverage_frac >= min_coverage;
    let coverage_pct_display = coverage_frac * 100.0;

    println!(
        "  Vox parse rate:      {:.1}%  (threshold: {:.0}%) {}",
        parse_rate * 100.0,
        min_parse_rate * 100.0,
        if parse_ok {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        }
    );
    println!(
        "  Construct coverage:  {:.1}%  (threshold: {:.0}%) {}",
        coverage_pct_display,
        min_coverage * 100.0,
        if coverage_ok {
            "✓".green().to_string()
        } else {
            "✗".red().to_string()
        }
    );

    let gate_result = serde_json::json!({
        "vox_parse_rate": parse_rate,
        "construct_coverage_pct": coverage_pct_display,
        "min_parse_rate": min_parse_rate,
        "min_coverage": min_coverage,
        "gate_passed": parse_ok && coverage_ok,
        "timestamp": "unknown",
    });

    std::fs::write(&eval_output, serde_json::to_string_pretty(&gate_result)?).ok(); // Write best-effort

    if !parse_ok || !coverage_ok {
        eprintln!(
            "{}",
            "\n⚠ Eval gate FAILED — training data quality below thresholds."
                .red()
                .bold()
        );
        eprintln!("  Review eval_results.json and regenerate corpus before promoting this model.");
        let marker = eval_output
            .parent()
            .unwrap_or(data_dir)
            .join("eval_gate_failed.json");
        std::fs::write(&marker, serde_json::to_string_pretty(&gate_result)?).ok();
        let strict = std::env::var("VOX_EVAL_STRICT")
            .is_ok_and(|v| v == "1" || v.eq_ignore_ascii_case("true"));
        if strict {
            anyhow::bail!(
                "Eval gate FAILED (VOX_EVAL_STRICT=1). Parse rate: {:.1}%, Coverage: {:.1}%",
                parse_rate * 100.0,
                coverage_pct_display
            );
        }
    } else {
        println!(
            "{}",
            "✓ Eval gate PASSED — training data meets quality thresholds."
                .green()
                .bold()
        );
        let marker = eval_output
            .parent()
            .unwrap_or(data_dir)
            .join("eval_gate_failed.json");
        std::fs::remove_file(&marker).ok();
    }

    Ok(())
}

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

fn mcp_tool_schema_metrics_path(run_dir: &Path, configured: &str) -> std::path::PathBuf {
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

/// Gate check result.
#[derive(Debug)]
pub struct GateResult {
    pub name: String,
    pub passed: bool,
    pub message: String,
    pub block: bool,
}

/// Run gate checks against a run directory.
pub fn check_run(run_dir: &Path, policy_path: &Path) -> Result<Vec<GateResult>> {
    let policy = load_policy(policy_path)?;
    let mut results = Vec::new();

    let manifest_path = run_dir.join("manifest.json");
    let manifest: Option<serde_json::Value> = if manifest_path.exists() {
        let content = std::fs::read_to_string(&manifest_path)?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };

    let metrics_path = run_dir.join("metrics.jsonl");
    let metrics_lines: Vec<String> = if metrics_path.exists() {
        std::fs::read_to_string(&metrics_path)?
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    } else {
        Vec::new()
    };

    let avg_tokens_per_sec = if metrics_lines.is_empty() {
        None
    } else {
        let mut sum = 0.0;
        let mut count = 0;
        for line in &metrics_lines {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(line)
                && let Some(t) = v.get("tokens_per_sec").and_then(|x| x.as_f64())
            {
                sum += t;
                count += 1;
            }
        }
        if count > 0 {
            Some(sum / count as f64)
        } else {
            None
        }
    };

    let device_profile = manifest
        .as_ref()
        .and_then(|m| m.get("device_profile"))
        .and_then(|v| v.as_str())
        .unwrap_or("cpu");

    if (policy.supervised_ratio.block || policy.supervised_ratio.min_pct > 0.0)
        && !metrics_lines.is_empty()
    {
        let sup_values: Vec<f64> = metrics_lines
            .iter()
            .filter_map(|l| {
                serde_json::from_str::<serde_json::Value>(l)
                    .ok()
                    .and_then(|v| v.get("supervised_ratio_pct").and_then(|x| x.as_f64()))
            })
            .collect();
        if !sup_values.is_empty() {
            let sup_pct = sup_values.iter().sum::<f64>() / sup_values.len() as f64;
            let passed = sup_pct >= policy.supervised_ratio.min_pct;
            results.push(GateResult {
                name: "supervised_ratio".to_string(),
                passed,
                message: format!(
                    "supervised_ratio {:.1}% {} {:.1}%",
                    sup_pct,
                    if passed { ">=" } else { "<" },
                    policy.supervised_ratio.min_pct
                ),
                block: policy.supervised_ratio.block,
            });
        }
    }

    fn normalize_device_profile(profile: &str) -> String {
        let lower = profile.to_lowercase();
        if lower.contains("cpu") {
            "cpu".to_string()
        } else if lower.contains("4080") {
            "rtx_4080_super".to_string()
        } else if lower.contains("4090") {
            "rtx_4090".to_string()
        } else {
            lower.replace([' ', '-'], "_")
        }
    }

    let device_profile_normalized = normalize_device_profile(device_profile);

    if (policy.truncation.block || policy.truncation.max_fraction < 1.0)
        && manifest
            .as_ref()
            .is_some_and(|m| m.get("truncation_fraction").is_some())
    {
        let frac = manifest
            .as_ref()
            .and_then(|m| m.get("truncation_fraction").and_then(|v| v.as_f64()))
            .unwrap_or(0.0);
        let passed = frac <= policy.truncation.max_fraction;
        results.push(GateResult {
            name: "truncation".to_string(),
            passed,
            message: format!(
                "truncation_fraction {:.2} {} {:.2}",
                frac,
                if passed { "<=" } else { ">" },
                policy.truncation.max_fraction
            ),
            block: policy.truncation.block,
        });
    }

    if let Some(tps) = avg_tokens_per_sec {
        let min_tps = policy
            .throughput
            .min_tokens_per_sec
            .get(&device_profile_normalized)
            .copied()
            .or_else(|| policy.throughput.min_tokens_per_sec.get("cpu").copied())
            .unwrap_or(0.0);
        let passed = tps >= min_tps;
        if min_tps > 0.0 {
            results.push(GateResult {
                name: "throughput".to_string(),
                passed,
                message: format!(
                    "throughput {:.0} tok/s {} {:.0} (profile={})",
                    tps,
                    if passed { ">=" } else { "<" },
                    min_tps,
                    device_profile_normalized
                ),
                block: policy.throughput.block,
            });
        }
    }

    let eval_results_path = run_dir.join("eval_results.json");
    // Parse eval_results.json once and share it across all gates that need it (B2).
    let eval_json: Option<serde_json::Value> = if eval_results_path.exists() {
        let content = std::fs::read_to_string(&eval_results_path)?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };

    if (policy.eval_local.block || policy.eval_local.min_parse_rate > 0.0)
        && let Some(ref eval) = eval_json
    {
        let parse_rate = eval
            .get("vox_parse_rate")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        // construct_coverage_pct is stored as 0–100 in eval_results.json (e.g. 35.0)
        // but min_coverage_pct in the YAML is 0.0–1.0 (e.g. 0.35). Normalise (Q4).
        let coverage_pct_json = eval
            .get("construct_coverage_pct")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let coverage = coverage_pct_json / 100.0;
        let passed = parse_rate >= policy.eval_local.min_parse_rate
            && coverage >= policy.eval_local.min_coverage_pct;
        results.push(GateResult {
            name: "eval_local".to_string(),
            passed,
            message: format!(
                "parse_rate={:.2} coverage={:.1}% (min={:.2} / {:.0}%)",
                parse_rate,
                coverage_pct_json,
                policy.eval_local.min_parse_rate,
                policy.eval_local.min_coverage_pct * 100.0,
            ),
            block: policy.eval_local.block,
        });
    }

    // --- per_context gates ------------------------------------------------------
    // eval_results.json already parsed above — reuse eval_json.
    // For each context key in the policy, check parse_rate and scope_compliance_rate.
    // If the context key is present in policy but absent from JSON, emit a failing
    // GateResult so CI doesn't silently pass (B5).
    if !policy.per_context.is_empty() {
        if let Some(ref eval) = eval_json {
            let breakdown_obj = eval.get("context_breakdown").and_then(|v| v.as_object());
            for (ctx_name, gate) in &policy.per_context {
                let slice = breakdown_obj.and_then(|b| b.get(ctx_name));
                match slice {
                    None => {
                        // Context key configured in policy but not found in eval output.
                        // Emit a failing gate so this doesn't silently pass in CI.
                        results.push(GateResult {
                            name: format!("per_context[{}]", ctx_name),
                            passed: false,
                            message: format!(
                                "context '{}' not found in eval_results.json — \
                                 0 samples or not evaluated",
                                ctx_name
                            ),
                            block: gate.block,
                        });
                    }
                    Some(slice) => {
                        let parse_rate = slice
                            .get("parse_rate")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(0.0);
                        let scope_rate = slice
                            .get("scope_compliance_rate")
                            .and_then(|v| v.as_f64())
                            .unwrap_or(1.0);
                        let parse_ok =
                            gate.min_parse_rate == 0.0 || parse_rate >= gate.min_parse_rate;
                        let scope_ok = gate.min_scope_compliance_rate == 0.0
                            || scope_rate >= gate.min_scope_compliance_rate;
                        let passed = parse_ok && scope_ok;
                        results.push(GateResult {
                            name: format!("per_context[{}]", ctx_name),
                            passed,
                            message: format!(
                                "parse={:.1}% (min={:.0}%) scope={:.1}% (min={:.0}%)",
                                100.0 * parse_rate,
                                100.0 * gate.min_parse_rate,
                                100.0 * scope_rate,
                                100.0 * gate.min_scope_compliance_rate,
                            ),
                            block: gate.block,
                        });
                    }
                }
            }
        } else if !policy.per_context.is_empty() {
            // No eval_results.json at all — emit failing gates for all configured contexts.
            for (ctx_name, gate) in &policy.per_context {
                results.push(GateResult {
                    name: format!("per_context[{}]", ctx_name),
                    passed: false,
                    message: "eval_results.json not found — run `vox mens eval` first".to_string(),
                    block: gate.block,
                });
            }
        }
    }

    // --- modal_mix gate ----------------------------------------------------------
    // Prefers modal_breakdown from eval_json (already parsed). Falls back to
    // mix_report.json if eval_results.json is absent or lacks the field.
    if policy.modal_mix.max_voice_fraction > 0.0 {
        let modal_report: Option<serde_json::Value> = {
            // First try eval_json (already in memory)
            let has_modal = eval_json
                .as_ref()
                .and_then(|v| v.get("modal_breakdown"))
                .is_some();
            if has_modal {
                eval_json.clone()
            } else {
                // Fallback: mix_report.json
                let mix_report_path = run_dir.join("mix_report.json");
                if mix_report_path.exists() {
                    let content = std::fs::read_to_string(&mix_report_path)?;
                    serde_json::from_str(&content).ok()
                } else {
                    None
                }
            }
        };
        if let Some(ref report) = modal_report
            && let Some(breakdown) = report.get("modal_breakdown").and_then(|v| v.as_object())
        {
            let total: u64 = breakdown.values().filter_map(|v| v.as_u64()).sum();
            let voice = breakdown.get("voice").and_then(|v| v.as_u64()).unwrap_or(0);
            if total > 0 {
                let voice_frac = voice as f64 / total as f64;
                let passed = voice_frac <= policy.modal_mix.max_voice_fraction;
                results.push(GateResult {
                    name: "modal_mix[voice]".to_string(),
                    passed,
                    message: format!(
                        "voice fraction {:.1}% {} {:.0}%",
                        100.0 * voice_frac,
                        if passed { "<=" } else { ">" },
                        100.0 * policy.modal_mix.max_voice_fraction,
                    ),
                    block: policy.modal_mix.block,
                });
            }
        }
    }

    // --- mcp_tool_schema gate (MCP tool-arg JSON Schema KPI) -------------------
    let mcp_cfg = &policy.mcp_tool_schema;
    let mcp_active = mcp_cfg.min_strict_validity_rate > 0.0 || mcp_cfg.block;
    if mcp_active {
        let path = mcp_tool_schema_metrics_path(run_dir, &mcp_cfg.metrics_file);
        if !path.exists() {
            results.push(GateResult {
                name: "mcp_tool_schema".to_string(),
                passed: false,
                message: format!(
                    "metrics file missing: {} — save `tool_arg_schema_kpi` from vox-mcp diagnostics here",
                    path.display()
                ),
                block: mcp_cfg.block,
            });
        } else {
            let content = std::fs::read_to_string(&path)?;
            let v: serde_json::Value = serde_json::from_str(&content)
                .map_err(|e| anyhow::anyhow!("{}: invalid JSON ({})", path.display(), e))?;
            let enabled = v
                .get("validation_enabled")
                .and_then(|x| x.as_bool())
                .unwrap_or(true);
            if !enabled {
                results.push(GateResult {
                    name: "mcp_tool_schema".to_string(),
                    passed: true,
                    message:
                        "MCP tool-arg schema validation disabled (VOX_MCP_TOOL_ARG_SCHEMA_VALIDATE)"
                            .to_string(),
                    block: false,
                });
            } else {
                let checks = v.get("checks_total").and_then(|x| x.as_u64()).unwrap_or(0);
                let rate_direct = v.get("strict_validity_rate").and_then(|x| x.as_f64());
                let sp = v.get("strict_passed").and_then(|x| x.as_u64()).unwrap_or(0);
                let sf = v.get("strict_failed").and_then(|x| x.as_u64()).unwrap_or(0);
                let decided = sp + sf;
                let rate_computed = if decided > 0 {
                    Some(sp as f64 / decided as f64)
                } else {
                    None
                };
                let effective_rate = rate_direct.or(rate_computed);
                let (passed, msg) = match effective_rate {
                    None if checks == 0 => (
                        false,
                        "checks_total=0 and no strict_validity_rate / pass-fail counts".to_string(),
                    ),
                    None => (
                        false,
                        "could not derive strict_validity_rate (missing field and pass+fail=0)"
                            .to_string(),
                    ),
                    Some(r) => {
                        let ok = r >= mcp_cfg.min_strict_validity_rate;
                        (
                            ok,
                            format!(
                                "strict_validity_rate={:.4} (min={:.4}) checks_total={}",
                                r, mcp_cfg.min_strict_validity_rate, checks
                            ),
                        )
                    }
                };
                results.push(GateResult {
                    name: "mcp_tool_schema".to_string(),
                    passed,
                    message: msg,
                    block: mcp_cfg.block,
                });
            }
        }
    }
    // -------------------------------------------------------------------------

    Ok(results)
}

/// Run eval-gate and return exit code (0 = pass, 1 = fail).
pub fn run_eval_gate(
    run_dir: std::path::PathBuf,
    policy_path: Option<std::path::PathBuf>,
) -> Result<i32> {
    use owo_colors::OwoColorize;

    let policy_path =
        policy_path.unwrap_or_else(|| std::path::PathBuf::from("mens/config/eval-gates.yaml"));
    if !policy_path.exists() {
        eprintln!(
            "  {} Policy not found: {}",
            "⚠".yellow(),
            policy_path.display()
        );
        return Ok(0);
    }

    eprintln!("{}", "╔══════════════════════════════════════════╗".cyan());
    eprintln!("{}", "║   Vox Mens — Eval Gate Check           ║".cyan());
    eprintln!("{}", "╚══════════════════════════════════════════╝".cyan());
    eprintln!("  Run dir: {}", run_dir.display());
    eprintln!("  Policy: {}", policy_path.display());
    eprintln!();

    let results = check_run(&run_dir, &policy_path)?;
    let mut failed_blocking = false;
    for r in &results {
        let icon = if r.passed {
            "✓".green().to_string()
        } else if r.block {
            "✗".red().to_string()
        } else {
            "⚠".yellow().to_string()
        };
        eprintln!("  {} {}: {}", icon, r.name, r.message);
        if !r.passed && r.block {
            failed_blocking = true;
        }
    }
    eprintln!();
    if failed_blocking {
        eprintln!("  {} Blocking gates failed.", "✗".red());
        Ok(1)
    } else {
        eprintln!("  {} All blocking gates passed.", "✓".green());
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_policy_with_per_context() -> EvalGatePolicy {
        let yaml = r#"
version: "1"
per_context:
  target:
    min_parse_rate: 0.80
    min_scope_compliance_rate: 0.95
    block: true
  meta:
    min_parse_rate: 0.30
    block: false
modal_mix:
  max_voice_fraction: 0.30
  block: false
"#;
        serde_yaml::from_str(yaml).expect("parse test policy")
    }

    #[test]
    fn per_context_gate_passes_when_above_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");

        // Write eval_results.json with target slice above threshold
        std::fs::write(
            dir.path().join("eval_results.json"),
            r#"{
                "vox_parse_rate": 0.9,
                "context_breakdown": {
                    "target": { "parse_rate": 0.90, "scope_compliance_rate": 0.97 },
                    "meta":   { "parse_rate": 0.35, "scope_compliance_rate": 0.60 }
                }
            }"#,
        )
        .unwrap();

        let results = super::check_run(dir.path(), &{
            let p = dir.path().join("policy.yaml");
            let yaml = r#"
version: "1"
per_context:
  target:
    min_parse_rate: 0.80
    min_scope_compliance_rate: 0.95
    block: true
  meta:
    min_parse_rate: 0.30
    block: false
"#;
            std::fs::write(&p, yaml).unwrap();
            p
        })
        .expect("check_run");

        let target_gate = results.iter().find(|r| r.name == "per_context[target]");
        assert!(
            target_gate.is_some(),
            "per_context[target] gate should be present"
        );
        assert!(
            target_gate.unwrap().passed,
            "target gate should pass at 90% parse"
        );

        let meta_gate = results.iter().find(|r| r.name == "per_context[meta]");
        assert!(
            meta_gate.is_some(),
            "per_context[meta] gate should be present"
        );
        assert!(
            meta_gate.unwrap().passed,
            "meta gate should pass at 35% parse"
        );
    }

    #[test]
    fn per_context_gate_fails_blocking_when_below_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        // target parse_rate below 0.80 threshold
        std::fs::write(
            dir.path().join("eval_results.json"),
            r#"{"vox_parse_rate":0.5,"context_breakdown":{"target":{"parse_rate":0.50,"scope_compliance_rate":0.99}}}"#,
        )
        .unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            "version: \"1\"\nper_context:\n  target:\n    min_parse_rate: 0.80\n    block: true\n",
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        let gate = results
            .iter()
            .find(|r| r.name == "per_context[target]")
            .expect("gate present");
        assert!(!gate.passed, "target gate should fail below threshold");
        assert!(gate.block, "target gate should be blocking");
    }

    #[test]
    fn modal_mix_gate_passes_when_voice_below_ceiling() {
        let _ = make_policy_with_per_context(); // verify policy parses
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("eval_results.json"),
            r#"{"modal_breakdown":{"text":90,"voice":5}}"#,
        )
        .unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            "version: \"1\"\nmodal_mix:\n  max_voice_fraction: 0.30\n  block: false\n",
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        let gate = results.iter().find(|r| r.name == "modal_mix[voice]");
        assert!(gate.is_some(), "modal_mix[voice] gate present");
        assert!(
            gate.unwrap().passed,
            "5/95 = 5.3% voice < 30% ceiling should pass"
        );
    }

    #[test]
    fn modal_mix_gate_warns_when_voice_exceeds_ceiling() {
        let dir = tempfile::tempdir().expect("tempdir");
        // 40% voice exceeds 30% ceiling
        std::fs::write(
            dir.path().join("eval_results.json"),
            r#"{"modal_breakdown":{"text":60,"voice":40}}"#,
        )
        .unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            "version: \"1\"\nmodal_mix:\n  max_voice_fraction: 0.30\n  block: false\n",
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        let gate = results
            .iter()
            .find(|r| r.name == "modal_mix[voice]")
            .expect("gate present");
        assert!(!gate.passed, "40% voice > 30% ceiling should not pass");
        assert!(!gate.block, "warn-only gate should not block");
    }

    #[test]
    fn policy_deserializes_per_context_and_modal_mix() {
        let policy = make_policy_with_per_context();
        assert!(policy.per_context.contains_key("target"));
        assert!(policy.per_context.contains_key("meta"));
        assert_eq!(policy.per_context["target"].min_parse_rate, 0.80);
        assert!(policy.per_context["target"].block);
        assert!(!policy.per_context["meta"].block);
        assert_eq!(policy.modal_mix.max_voice_fraction, 0.30);
        assert!(!policy.modal_mix.block);
    }

    #[test]
    fn mcp_tool_schema_gate_passes_at_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("mcp_tool_schema_kpi.json"),
            r#"{"validation_enabled":true,"checks_total":100,"strict_passed":99,"strict_failed":1,"schema_compile_skipped":0,"strict_validity_rate":0.99}"#,
        )
        .unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.99
  block: true
"#,
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        let g = results
            .iter()
            .find(|r| r.name == "mcp_tool_schema")
            .expect("gate");
        assert!(g.passed, "{}", g.message);
    }

    #[test]
    fn mcp_tool_schema_gate_fails_below_threshold() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("mcp_tool_schema_kpi.json"),
            r#"{"validation_enabled":true,"checks_total":10,"strict_passed":5,"strict_failed":5,"strict_validity_rate":0.5}"#,
        )
        .unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.99
  block: true
"#,
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        let g = results
            .iter()
            .find(|r| r.name == "mcp_tool_schema")
            .expect("gate");
        assert!(!g.passed);
    }

    #[test]
    fn mcp_tool_schema_gate_skipped_when_inactive() {
        let dir = tempfile::tempdir().expect("tempdir");
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(
            &policy_path,
            r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.0
  block: false
"#,
        )
        .unwrap();
        let results = check_run(dir.path(), &policy_path).expect("check_run");
        assert!(
            !results.iter().any(|r| r.name == "mcp_tool_schema"),
            "inactive gate should not emit a row"
        );
    }
}
