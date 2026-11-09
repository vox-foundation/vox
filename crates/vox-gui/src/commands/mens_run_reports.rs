//! Reads MENS eval/gate report JSON files (`eval_local_report.json`,
//! `gate_receipt.json`, `collateral_damage_report.json`) directly from a run
//! directory on disk — no CLI invocation, no GPU work. `eval-local` and
//! `eval-collateral-damage` are expensive real GPU runs, so this must never
//! trigger them; it only reads whatever a previous, separately-triggered run
//! already wrote (see `crates/vox-ml-cli/src/commands/mens/eval_gate/run_gate.rs`,
//! `crates/vox-ml-cli/src/commands/mens/eval_local.rs`, and
//! `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` for the writers).
//!
//! All three files are independently optional: a run dir before any eval has
//! happened has none of them, and that's a normal state, not an error.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct EvalLocalReportDto {
    pub anti_stub_task_success: f64,
    pub pass_rate_at_k: f64,
    pub placeholder_event_rate: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GateDto {
    pub name: String,
    pub passed: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct GateReceiptDto {
    pub overall_passed: bool,
    /// Count of gates in `gates` whose message is not a
    /// skipped/missing/not-applicable stand-in — mirrors the exact filter in
    /// `assert_serve_preconditions` (Task A2,
    /// `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`). A receipt
    /// can say `overall_passed: true` with this at 0 when every gate was
    /// skipped for a missing artifact; that is not evidence of anything
    /// passing and callers must not render it the same as a real pass.
    pub substantive_gate_count: usize,
    pub failed_gates: Vec<String>,
    pub gates: Vec<GateDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct CollateralDamageReportDto {
    pub status: String,
    pub failed_on: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SpokeScoreRowDto {
    pub spoke: String,
    pub total_cases: usize,
    pub before_pass_at_1: f64,
    pub mid_pass_at_1: f64,
    pub pass_at_1: f64,
    pub pass_at_k: f64,
    pub delta_pass_at_1: f64,
    pub syntax_validity_rate: f64,
    pub semantic_validity_rate: f64,
    pub execution_success_rate: f64,
    pub curriculum_weight: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MensSpokeScorecardDto {
    pub schema_version: String,
    pub total_cases: usize,
    pub before_overall_pass_at_1: f64,
    pub mid_overall_pass_at_1: f64,
    pub overall_pass_at_1: f64,
    pub overall_pass_at_k: f64,
    pub overall_delta_pass_at_1: f64,
    pub monotonic_improvement_verified: bool,
    pub promotion_gate_passed: bool,
    pub promoted_model_id: String,
    pub promoted_adapter_uri: String,
    pub spokes: Vec<SpokeScoreRowDto>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OrganicSpokeRowDto {
    pub spoke: String,
    pub total_cases: usize,
    pub search_recall_at_5: f64,
    pub mean_reciprocal_rank: f64,
    pub unbiased_search_recall_at_5: f64,
    pub unbiased_mrr: f64,
    pub mean_grounding_ratio: f64,
    pub distractor_resilience_rate: f64,
    pub syntax_validity_rate: f64,
    pub semantic_validity_rate: f64,
    pub compilation_pass_rate: f64,
    pub end_to_end_pass_at_1: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OrganicSearchSpokeScorecardDto {
    pub schema_version: String,
    pub total_cases: usize,
    pub overall_search_recall_at_5: f64,
    pub overall_mrr: f64,
    pub overall_unbiased_search_recall_at_5: f64,
    pub overall_unbiased_mrr: f64,
    pub overall_grounding_ratio: f64,
    pub overall_distractor_resilience_rate: f64,
    pub overall_compilation_pass_rate: f64,
    pub overall_end_to_end_pass_at_1: f64,
    pub spokes: Vec<OrganicSpokeRowDto>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CustomSpokeDto {
    pub id: String,
    pub label: String,
    pub domain: String,
    pub target_gate: String,
    pub family: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MensLadderRungDto {
    pub size: String,
    pub hf_id: String,
    pub local_sha: String,
    pub repo_master_sha: String,
    pub sync_status: String,
    pub floor_mb: Option<u32>,
    pub floor_measured: bool,
    pub cells: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MensCrucibleConfigDto {
    pub hub_embedder: String,
    pub hub_base_rung: String,
    pub compute_target: String,
    pub lora_rank: usize,
    pub lora_alpha: usize,
    pub seq_len: usize,
    pub vram_cap_pct: usize,
    pub epoch_caps: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MensLiveTelemetryDto {
    pub gpu_pct: f64,
    pub vram_used_gb: f64,
    pub vram_max_gb: f64,
    pub cpu_pct: f64,
    pub ram_used_gb: f64,
    pub ram_max_gb: f64,
    pub loss: f64,
    pub step: u64,
    pub gpu_series: Vec<f64>,
    pub vram_series: Vec<f64>,
    pub cpu_series: Vec<f64>,
    pub ram_series: Vec<f64>,
    pub loss_series: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MensLadderStatusDto {
    pub rungs: Vec<MensLadderRungDto>,
    pub custom_spokes: Vec<CustomSpokeDto>,
    pub crucible_config: Option<MensCrucibleConfigDto>,
}

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct MensRunReportsDto {
    pub eval_local: Option<EvalLocalReportDto>,
    pub gate_receipt: Option<GateReceiptDto>,
    pub collateral_damage: Option<CollateralDamageReportDto>,
    pub spoke_scorecard: Option<MensSpokeScorecardDto>,
    pub organic_scorecard: Option<OrganicSearchSpokeScorecardDto>,
}

fn read_json(path: &Path) -> Option<serde_json::Value> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

/// Mirrors the substantive-gate filter in `assert_serve_preconditions`
/// (Task A2, `dispatch.rs`): a gate whose message says the check was
/// skipped, missing, or not applicable did not actually verify anything.
fn is_substantive(message: &str) -> bool {
    !message.contains("not applicable")
        && !message.contains("skipped")
        && !message.contains("missing")
}

fn parse_eval_local(v: &serde_json::Value) -> EvalLocalReportDto {
    EvalLocalReportDto {
        anti_stub_task_success: v
            .get("anti_stub_task_success")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
        pass_rate_at_k: v
            .get("pass_rate_at_k")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
        placeholder_event_rate: v
            .get("placeholder_event_rate")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0),
    }
}

fn parse_gate_receipt(v: &serde_json::Value) -> GateReceiptDto {
    let overall_passed = v
        .get("overall_passed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let gates: Vec<GateDto> = v
        .get("gates")
        .and_then(serde_json::Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|g| GateDto {
                    name: g
                        .get("name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    passed: g
                        .get("passed")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false),
                    message: g
                        .get("message")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    let substantive_gate_count = gates.iter().filter(|g| is_substantive(&g.message)).count();
    let failed_gates = gates
        .iter()
        .filter(|g| !g.passed)
        .map(|g| g.name.clone())
        .collect();
    GateReceiptDto {
        overall_passed,
        substantive_gate_count,
        failed_gates,
        gates,
    }
}

fn parse_collateral(v: &serde_json::Value) -> CollateralDamageReportDto {
    CollateralDamageReportDto {
        status: v
            .get("status")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        failed_on: v
            .get("failed_on")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
    }
}

fn parse_spoke_scorecard(v: &serde_json::Value) -> Option<MensSpokeScorecardDto> {
    let spokes_arr = v.get("spokes")?.as_array()?;
    let schema_version = v
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("mens_spoke_scorecard_v1")
        .to_string();
    let total_cases = v
        .get("total_cases")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(spokes_arr.len() as u64) as usize;
    let overall_pass_at_1 = v
        .get("after_overall_pass_at_1")
        .or_else(|| v.get("overall_pass_at_1"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(0.0);
    let overall_pass_at_k = v
        .get("after_overall_pass_at_k")
        .or_else(|| v.get("overall_pass_at_k"))
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_pass_at_1);
    let before_overall_pass_at_1 = v
        .get("before_overall_pass_at_1")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_pass_at_1);
    let mid_overall_pass_at_1 = v
        .get("mid_overall_pass_at_1")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_pass_at_1);
    let overall_delta_pass_at_1 = v
        .get("overall_delta_pass_at_1")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_pass_at_1 - before_overall_pass_at_1);
    let monotonic_improvement_verified = v
        .get("monotonic_improvement_verified")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(overall_pass_at_1 >= before_overall_pass_at_1);
    let promotion_gate_passed = v
        .get("promotion_gate_passed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or((overall_pass_at_1 - 1.0).abs() < 1e-9);
    let promoted_model_id = v
        .get("promoted_model_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("mens/bench25-cycle-2")
        .to_string();
    let promoted_adapter_uri = v
        .get("promoted_adapter_uri")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .to_string();

    let spokes = spokes_arr
        .iter()
        .map(|s| {
            let pass_at_1 = s
                .get("after_pass_at_1")
                .or_else(|| s.get("pass_at_1"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let before_pass_at_1 = s
                .get("before_pass_at_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let mid_pass_at_1 = s
                .get("mid_pass_at_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let pass_at_k = s
                .get("after_pass_at_k")
                .or_else(|| s.get("pass_at_k"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let delta_pass_at_1 = s
                .get("delta_pass_at_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1 - before_pass_at_1);
            let syntax_validity_rate = s
                .get("after_syntax_rate")
                .or_else(|| s.get("syntax_validity_rate"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let semantic_validity_rate = s
                .get("after_semantic_rate")
                .or_else(|| s.get("semantic_validity_rate"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let execution_success_rate = s
                .get("after_execution_rate")
                .or_else(|| s.get("execution_success_rate"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(pass_at_1);
            let curriculum_weight = s
                .get("final_curriculum_weight")
                .or_else(|| s.get("initial_curriculum_weight"))
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.2);
            SpokeScoreRowDto {
                spoke: s
                    .get("spoke")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                total_cases: s
                    .get("total_cases")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(5) as usize,
                before_pass_at_1,
                mid_pass_at_1,
                pass_at_1,
                pass_at_k,
                delta_pass_at_1,
                syntax_validity_rate,
                semantic_validity_rate,
                execution_success_rate,
                curriculum_weight,
            }
        })
        .collect();

    Some(MensSpokeScorecardDto {
        schema_version,
        total_cases,
        before_overall_pass_at_1,
        mid_overall_pass_at_1,
        overall_pass_at_1,
        overall_pass_at_k,
        overall_delta_pass_at_1,
        monotonic_improvement_verified,
        promotion_gate_passed,
        promoted_model_id,
        promoted_adapter_uri,
        spokes,
    })
}

fn parse_organic_scorecard(v: &serde_json::Value) -> Option<OrganicSearchSpokeScorecardDto> {
    let spokes_arr = v.get("spokes")?.as_array()?;
    let schema_version = v
        .get("schema_version")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("mens_organic_search_spoke_scorecard_v1")
        .to_string();
    let total_cases = v.get("total_cases").and_then(serde_json::Value::as_u64)? as usize;
    let overall_search_recall_at_5 = v
        .get("overall_search_recall_at_5")
        .and_then(serde_json::Value::as_f64)?;
    let overall_mrr = v
        .get("overall_mrr")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_search_recall_at_5);
    let overall_unbiased_search_recall_at_5 = v
        .get("overall_unbiased_search_recall_at_5")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_search_recall_at_5);
    let overall_unbiased_mrr = v
        .get("overall_unbiased_mrr")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_mrr);
    let overall_grounding_ratio = v
        .get("overall_grounding_ratio")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_search_recall_at_5);
    let overall_distractor_resilience_rate = v
        .get("overall_distractor_resilience_rate")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_grounding_ratio);
    let overall_compilation_pass_rate = v
        .get("overall_compilation_pass_rate")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_search_recall_at_5);
    let overall_end_to_end_pass_at_1 = v
        .get("overall_end_to_end_pass_at_1")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_compilation_pass_rate);

    let spokes = spokes_arr
        .iter()
        .map(|s| {
            let search_recall_at_5 = s
                .get("search_recall_at_5")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let mean_reciprocal_rank = s
                .get("mean_reciprocal_rank")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let unbiased_search_recall_at_5 = s
                .get("unbiased_search_recall_at_5")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(search_recall_at_5);
            let unbiased_mrr = s
                .get("unbiased_mrr")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(mean_reciprocal_rank);
            let mean_grounding_ratio = s
                .get("mean_grounding_ratio")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let distractor_resilience_rate = s
                .get("distractor_resilience_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(mean_grounding_ratio);
            let syntax_validity_rate = s
                .get("syntax_validity_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let semantic_validity_rate = s
                .get("semantic_validity_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let compilation_pass_rate = s
                .get("compilation_pass_rate")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            let end_to_end_pass_at_1 = s
                .get("end_to_end_pass_at_1")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
            OrganicSpokeRowDto {
                spoke: s
                    .get("spoke")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                total_cases: s
                    .get("total_cases")
                    .and_then(serde_json::Value::as_u64)
                    .unwrap_or(3) as usize,
                search_recall_at_5,
                mean_reciprocal_rank,
                unbiased_search_recall_at_5,
                unbiased_mrr,
                mean_grounding_ratio,
                distractor_resilience_rate,
                syntax_validity_rate,
                semantic_validity_rate,
                compilation_pass_rate,
                end_to_end_pass_at_1,
            }
        })
        .collect();

    Some(OrganicSearchSpokeScorecardDto {
        schema_version,
        total_cases,
        overall_search_recall_at_5,
        overall_mrr,
        overall_unbiased_search_recall_at_5,
        overall_unbiased_mrr,
        overall_grounding_ratio,
        overall_distractor_resilience_rate,
        overall_compilation_pass_rate,
        overall_end_to_end_pass_at_1,
        spokes,
    })
}

fn find_workspace_spoke_scorecard() -> Option<MensSpokeScorecardDto> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().and_then(Path::parent)?;
    let reports_dir = workspace_root.join("contracts").join("reports");
    read_json(&reports_dir.join("mens-before-after-comparison.v1.json"))
        .or_else(|| read_json(&reports_dir.join("mens-spoke-scorecard.v1.json")))
        .as_ref()
        .and_then(parse_spoke_scorecard)
}

fn find_workspace_organic_scorecard() -> Option<OrganicSearchSpokeScorecardDto> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir.parent().and_then(Path::parent)?;
    let reports_dir = workspace_root.join("contracts").join("reports");
    read_json(&reports_dir.join("mens-organic-search-spoke-scorecard.v1.json"))
        .as_ref()
        .and_then(parse_organic_scorecard)
}

/// Reads whichever of the report files exist in `run_dir`. Missing
/// files (including a wholly nonexistent `run_dir`) map to `None`, never an
/// error — that's the normal state before any eval has run.
/// When `run_dir` is `"workspace"`, loads the workspace's committed 5-spoke
/// before/after comparison scorecard under `contracts/reports/`.
pub fn read_run_reports(run_dir: &Path) -> MensRunReportsDto {
    let is_workspace = run_dir.as_os_str() == "workspace";
    let spoke_scorecard = if is_workspace {
        find_workspace_spoke_scorecard()
    } else {
        read_json(&run_dir.join("mens_spoke_scorecard.json"))
            .or_else(|| read_json(&run_dir.join("mens-before-after-comparison.v1.json")))
            .or_else(|| read_json(&run_dir.join("mens-spoke-scorecard.v1.json")))
            .as_ref()
            .and_then(parse_spoke_scorecard)
    };
    let organic_scorecard = if is_workspace {
        find_workspace_organic_scorecard()
    } else {
        read_json(&run_dir.join("mens-organic-search-spoke-scorecard.v1.json"))
            .as_ref()
            .and_then(parse_organic_scorecard)
    };
    MensRunReportsDto {
        eval_local: read_json(&run_dir.join("eval_local_report.json"))
            .as_ref()
            .map(parse_eval_local),
        gate_receipt: read_json(&run_dir.join("gate_receipt.json"))
            .as_ref()
            .map(parse_gate_receipt),
        collateral_damage: read_json(&run_dir.join("collateral_damage_report.json"))
            .as_ref()
            .map(parse_collateral),
        spoke_scorecard,
        organic_scorecard,
    }
}

#[tauri::command]
pub async fn mens_run_reports(run_dir: String) -> Result<MensRunReportsDto, String> {
    Ok(read_run_reports(Path::new(&run_dir)))
}

fn workspace_root_dir() -> Option<PathBuf> {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
}

fn resolve_runs_root(run_dir: Option<&str>) -> PathBuf {
    match run_dir {
        Some(raw) if !raw.trim().is_empty() && raw.trim() != "workspace" => {
            PathBuf::from(raw.trim())
        }
        _ => workspace_root_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("mens")
            .join("runs"),
    }
}

fn default_cells_for_size(size: &str, custom_spokes: &[CustomSpokeDto]) -> HashMap<String, String> {
    let mut cells = HashMap::new();
    let defaults: &[(&str, &str)] = match size {
        "0.6B" | "1.7B" | "4B" | "8B" => &[
            ("vox-lang", "MISSING"),
            ("rust-expert", "MISSING"),
            ("cargo-build", "MISSING"),
            ("review", "MISSING"),
            ("oratio-voice", "MISSING"),
        ],
        "14B" => &[
            ("vox-lang", "MISSING"),
            ("rust-expert", "MISSING"),
            ("cargo-build", "MISSING"),
            ("review", "MISSING"),
            ("oratio-voice", "NOT FIT (16GB)"),
        ],
        _ => &[
            ("vox-lang", "NOT FIT (16GB)"),
            ("rust-expert", "NOT FIT (16GB)"),
            ("cargo-build", "NOT FIT (16GB)"),
            ("review", "NOT FIT (16GB)"),
            ("oratio-voice", "NOT FIT (16GB)"),
        ],
    };
    for (k, v) in defaults {
        cells.insert((*k).to_string(), (*v).to_string());
    }
    for sp in custom_spokes {
        cells
            .entry(sp.id.clone())
            .or_insert_with(|| "MISSING".to_string());
    }
    cells
}

#[derive(Debug, Deserialize)]
struct GpuSpecsYaml {
    #[serde(default)]
    train_bases: HashMap<String, Vec<GpuSpecsTrainBaseEntry>>,
}

#[derive(Debug, Deserialize)]
struct GpuSpecsTrainBaseEntry {
    hf_id: String,
    #[serde(default)]
    floor_mb: Option<u32>,
    #[serde(default = "default_true_flag")]
    floor_measured: bool,
    #[serde(default)]
    methods: Vec<String>,
}

fn default_true_flag() -> bool {
    true
}

pub fn read_ladder_status(runs_root: &Path) -> MensLadderStatusDto {
    let custom_spokes: Vec<CustomSpokeDto> = read_json(&runs_root.join("custom_spokes.v1.json"))
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let crucible_config: Option<MensCrucibleConfigDto> =
        read_json(&runs_root.join("crucible_config.v1.json"))
            .and_then(|v| serde_json::from_value(v).ok());

    let gpu_specs_path = workspace_root_dir()
        .map(|r| r.join("mens").join("config").join("gpu-specs.yaml"))
        .unwrap_or_else(|| PathBuf::from("mens/config/gpu-specs.yaml"));

    let parsed_specs: Option<GpuSpecsYaml> = std::fs::read_to_string(&gpu_specs_path)
        .ok()
        .and_then(|raw| serde_yaml::from_str(&raw).ok());

    let size_order = ["0.6B", "1.7B", "4B", "8B", "14B", "32B"];
    let mut rungs = Vec::with_capacity(size_order.len());

    for size in size_order {
        let repo_prefix = format!("Qwen/Qwen3-{size}");
        let entry = parsed_specs
            .as_ref()
            .and_then(|s| s.train_bases.get("qwen3_code"))
            .and_then(|list| {
                list.iter().find(|b| {
                    b.methods.iter().any(|m| m == "qlora")
                        && b.hf_id
                            .split_once('@')
                            .map(|(r, _)| r == repo_prefix)
                            .unwrap_or(b.hf_id == repo_prefix)
                })
            });

        let (hf_id, repo_master_sha, floor_mb, floor_measured) = match entry {
            Some(b) => {
                let sha_short = b
                    .hf_id
                    .split_once('@')
                    .map(|(_, sha)| format!("@{}", &sha[..sha.len().min(8)]))
                    .unwrap_or_else(|| "@main".to_string());
                (b.hf_id.clone(), sha_short, b.floor_mb, b.floor_measured)
            }
            None => (
                format!("{repo_prefix}@pinned"),
                "@pinned".to_string(),
                None,
                false,
            ),
        };

        let size_train_file = runs_root
            .join("size_train")
            .join(format!("Qwen_Qwen3-{size}.json"));
        let size_train_json = read_json(&size_train_file);

        let mut cells = default_cells_for_size(size, &custom_spokes);
        let qwen_dir = runs_root.join(format!("qwen3-{}", size.to_ascii_lowercase()));
        let size_dir = if qwen_dir.is_dir() {
            qwen_dir
        } else {
            runs_root.join(size)
        };

        let (local_sha, sync_status) = if let Some(ref st) = size_train_json {
            let status = st
                .get("status")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("not_trained");
            let reason = st
                .get("reason")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            if status == "trained" || reason == "train_not_run" {
                (repo_master_sha.clone(), "synced".to_string())
            } else if reason == "base_snapshot_absent" {
                ("not pulled".to_string(), "missing".to_string())
            } else {
                (repo_master_sha.clone(), "synced".to_string())
            }
        } else if size_dir.join("candle_qlora_adapter.safetensors").is_file()
            || size_dir.join("training_manifest.json").is_file()
        {
            (repo_master_sha.clone(), "synced".to_string())
        } else {
            ("not pulled".to_string(), "missing".to_string())
        };

        if size_dir.is_dir() {
            let direct_receipt = size_dir.join("gate_receipt.json");
            if let Some(val) = read_json(&direct_receipt) {
                let receipt = parse_gate_receipt(&val);
                if receipt.overall_passed && receipt.substantive_gate_count > 0 {
                    cells.insert("vox-lang".to_string(), "TRAINED · GATED".to_string());
                } else if !receipt.overall_passed {
                    cells.insert("vox-lang".to_string(), "GATE FAILED".to_string());
                }
            } else if size_dir.join("candle_qlora_adapter.safetensors").is_file() {
                cells.insert("vox-lang".to_string(), "TRAINED · UNGATED".to_string());
            }
            for spoke_key in cells.keys().cloned().collect::<Vec<_>>() {
                let receipt_path = size_dir.join(&spoke_key).join("gate_receipt.json");
                if let Some(val) = read_json(&receipt_path) {
                    let receipt = parse_gate_receipt(&val);
                    if receipt.overall_passed && receipt.substantive_gate_count > 0 {
                        cells.insert(spoke_key, "TRAINED · GATED".to_string());
                    } else if !receipt.overall_passed {
                        cells.insert(spoke_key, "GATE FAILED".to_string());
                    }
                }
            }
        }

        rungs.push(MensLadderRungDto {
            size: size.to_string(),
            hf_id,
            local_sha,
            repo_master_sha,
            sync_status,
            floor_mb,
            floor_measured,
            cells,
        });
    }

    MensLadderStatusDto {
        rungs,
        custom_spokes,
        crucible_config,
    }
}

pub fn save_custom_spoke_to_dir(
    runs_root: &Path,
    spoke: CustomSpokeDto,
) -> Result<Vec<CustomSpokeDto>, String> {
    let id = spoke.id.trim().to_string();
    if id.is_empty() {
        return Err("custom spoke id must not be empty".to_string());
    }
    std::fs::create_dir_all(runs_root).map_err(|e| format!("create runs_root: {e}"))?;
    let path = runs_root.join("custom_spokes.v1.json");
    let mut existing: Vec<CustomSpokeDto> = read_json(&path)
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();
    if let Some(slot) = existing.iter_mut().find(|s| s.id == id) {
        *slot = spoke;
    } else {
        existing.push(spoke);
    }
    let body =
        serde_json::to_string_pretty(&existing).map_err(|e| format!("serialize spokes: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(existing)
}

pub fn save_crucible_config_to_dir(
    runs_root: &Path,
    config: MensCrucibleConfigDto,
) -> Result<MensCrucibleConfigDto, String> {
    std::fs::create_dir_all(runs_root).map_err(|e| format!("create runs_root: {e}"))?;
    let path = runs_root.join("crucible_config.v1.json");
    let body =
        serde_json::to_string_pretty(&config).map_err(|e| format!("serialize config: {e}"))?;
    std::fs::write(&path, body).map_err(|e| format!("write {}: {e}", path.display()))?;
    Ok(config)
}

pub fn read_live_telemetry(runs_root: &Path) -> MensLiveTelemetryDto {
    let candidates = [
        runs_root.join("telemetry.jsonl"),
        runs_root.join("metrics.jsonl"),
        runs_root.join("latest").join("telemetry.jsonl"),
        runs_root.join("latest").join("metrics.jsonl"),
    ];
    let mut loss_series = Vec::new();
    let mut gpu_series = Vec::new();
    let mut vram_series = Vec::new();
    let mut cpu_series = Vec::new();
    let mut ram_series = Vec::new();
    let mut latest_step = 0_u64;

    if let Some(active_log) = candidates.iter().find(|p| p.is_file()) {
        if let Ok(raw) = std::fs::read_to_string(active_log) {
            for line in raw.lines().filter(|l| !l.trim().is_empty()) {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(line) {
                    if let Some(s) = val.get("step").and_then(serde_json::Value::as_u64) {
                        latest_step = s;
                    }
                    if let Some(loss) = val.get("loss").and_then(serde_json::Value::as_f64) {
                        loss_series.push((loss * 1000.0).round() / 1000.0);
                    }
                    if let Some(gpu) = val
                        .get("gpu_util_pct")
                        .or_else(|| val.get("gpu_pct"))
                        .and_then(serde_json::Value::as_f64)
                    {
                        gpu_series.push(gpu);
                    }
                    if let Some(vram) = val
                        .get("vram_used_gb")
                        .or_else(|| val.get("vram_gb"))
                        .and_then(serde_json::Value::as_f64)
                    {
                        vram_series.push(vram);
                    }
                    if let Some(cpu) = val.get("cpu_pct").and_then(serde_json::Value::as_f64) {
                        cpu_series.push(cpu);
                    }
                    if let Some(ram) = val.get("ram_used_gb").and_then(serde_json::Value::as_f64) {
                        ram_series.push(ram);
                    }
                }
            }
        }
    }

    let gpu_pct = *gpu_series.last().unwrap_or(&0.0);
    let vram_used_gb = *vram_series.last().unwrap_or(&0.0);
    let cpu_pct = *cpu_series.last().unwrap_or(&0.0);
    let ram_used_gb = *ram_series.last().unwrap_or(&0.0);
    let loss = *loss_series.last().unwrap_or(&0.0);

    MensLiveTelemetryDto {
        gpu_pct,
        vram_used_gb,
        vram_max_gb: 16.0,
        cpu_pct,
        ram_used_gb,
        ram_max_gb: 64.0,
        loss,
        step: latest_step,
        gpu_series,
        vram_series,
        cpu_series,
        ram_series,
        loss_series,
    }
}

#[tauri::command]
pub async fn mens_ladder_status(run_dir: Option<String>) -> Result<MensLadderStatusDto, String> {
    let root = resolve_runs_root(run_dir.as_deref());
    Ok(read_ladder_status(&root))
}

#[tauri::command]
pub async fn mens_save_custom_spoke(
    run_dir: Option<String>,
    spoke: CustomSpokeDto,
) -> Result<Vec<CustomSpokeDto>, String> {
    let root = resolve_runs_root(run_dir.as_deref());
    save_custom_spoke_to_dir(&root, spoke)
}

#[tauri::command]
pub async fn mens_save_crucible_config(
    run_dir: Option<String>,
    config: MensCrucibleConfigDto,
) -> Result<MensCrucibleConfigDto, String> {
    let root = resolve_runs_root(run_dir.as_deref());
    save_crucible_config_to_dir(&root, config)
}

#[tauri::command]
pub async fn mens_live_telemetry(run_dir: Option<String>) -> Result<MensLiveTelemetryDto, String> {
    let root = resolve_runs_root(run_dir.as_deref());
    Ok(read_live_telemetry(&root))
}

#[tauri::command]
pub async fn mens_corpus_studio_snapshot(
    config_path: Option<String>,
) -> Result<vox_orchestrator_mcp::gui_ops::mens_corpus::MensCorpusStudioSnapshotDto, String> {
    let ws = workspace_root_dir().unwrap_or_else(|| PathBuf::from("."));
    vox_orchestrator_mcp::gui_ops::mens_corpus::corpus_studio_snapshot(&ws, config_path.as_deref())
}

#[tauri::command]
pub async fn mens_corpus_read_jsonl(
    path: String,
    offset: Option<usize>,
    limit: Option<usize>,
    search: Option<String>,
    family_filter: Option<String>,
) -> Result<vox_orchestrator_mcp::gui_ops::mens_corpus::MensJsonlPageDto, String> {
    let ws = workspace_root_dir().unwrap_or_else(|| PathBuf::from("."));
    vox_orchestrator_mcp::gui_ops::mens_corpus::read_jsonl_records(
        &ws,
        &path,
        offset.unwrap_or(0),
        limit.unwrap_or(25),
        search.as_deref(),
        family_filter.as_deref(),
    )
}

#[tauri::command]
pub async fn mens_corpus_save_jsonl_row(
    request: vox_orchestrator_mcp::gui_ops::mens_corpus::MensSaveJsonlRowRequestDto,
) -> Result<vox_orchestrator_mcp::gui_ops::mens_corpus::MensSaveJsonlRowResponseDto, String> {
    let ws = workspace_root_dir().unwrap_or_else(|| PathBuf::from("."));
    vox_orchestrator_mcp::gui_ops::mens_corpus::save_jsonl_row(&ws, request)
}

#[tauri::command]
pub async fn mens_corpus_rebuild_mix(
    config_path: Option<String>,
    force_resynth: Option<bool>,
) -> Result<vox_orchestrator_mcp::gui_ops::mens_corpus::MensCorpusStudioSnapshotDto, String> {
    let ws = workspace_root_dir().unwrap_or_else(|| PathBuf::from("."));
    vox_orchestrator_mcp::gui_ops::mens_corpus::rebuild_mix(
        &ws,
        config_path.as_deref(),
        force_resynth.unwrap_or(true),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_run_dir_with_no_reports_returns_all_none() {
        let dir = tempfile::tempdir().unwrap();
        let result = read_run_reports(dir.path());
        assert_eq!(result, MensRunReportsDto::default());
    }

    #[test]
    fn a_nonexistent_run_dir_returns_all_none_not_an_error() {
        let result = read_run_reports(Path::new("/definitely/does/not/exist/anywhere"));
        assert_eq!(result, MensRunReportsDto::default());
    }

    #[test]
    fn a_malformed_gate_receipt_json_degrades_to_none_not_a_panic() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("gate_receipt.json"), "{ not valid json !!").unwrap();

        let result = read_run_reports(dir.path());
        assert_eq!(result.gate_receipt, None);
    }

    #[test]
    fn a_failed_gate_receipt_names_the_degraded_gate() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.json"),
            r#"{
              "overall_passed": false,
              "gates": [
                {"name":"throughput","passed":false,"message":"18 tok/s < 100 floor"},
                {"name":"pass_at_k","passed":true,"message":"0.65 >= 0.60"}
              ]}"#,
        )
        .unwrap();

        let result = read_run_reports(dir.path());
        let receipt = result.gate_receipt.expect("gate_receipt.json exists");
        assert!(!receipt.overall_passed);
        assert_eq!(receipt.failed_gates, vec!["throughput".to_string()]);
        assert_eq!(receipt.substantive_gate_count, 2);
    }

    #[test]
    fn a_receipt_that_passed_with_zero_substantive_gates_is_flagged() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.json"),
            r#"{
              "overall_passed": true,
              "gates": [
                {"name":"rust_compile_rate","passed":true,"message":"not applicable (no rust_authoring rows)"},
                {"name":"pass_at_k","passed":true,"message":"baseline file missing (skipped regression check)"}
              ]}"#,
        )
        .unwrap();

        let result = read_run_reports(dir.path());
        let receipt = result.gate_receipt.expect("gate_receipt.json exists");
        assert!(receipt.overall_passed);
        assert_eq!(
            receipt.substantive_gate_count, 0,
            "a receipt whose every gate was skipped must report 0 substantive gates \
             even though overall_passed is true, so the GUI can flag it instead of \
             showing a plain green check"
        );
    }

    #[test]
    fn a_failed_collateral_damage_report_names_the_degraded_benchmark() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("collateral_damage_report.json"),
            r#"{"status":"fail","failed_on":"rust_authoring","threshold":0.1,"reports":[]}"#,
        )
        .unwrap();

        let result = read_run_reports(dir.path());
        let collateral = result.collateral_damage.expect("report exists");
        assert_eq!(collateral.status, "fail");
        assert_eq!(collateral.failed_on, Some("rust_authoring".to_string()));
    }

    #[test]
    fn an_eval_local_report_parses_the_anti_stub_metric() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("eval_local_report.json"),
            r#"{"anti_stub_task_success":0.42,"pass_rate_at_k":0.55,"placeholder_event_rate":0.1}"#,
        )
        .unwrap();

        let result = read_run_reports(dir.path());
        let eval_local = result.eval_local.expect("report exists");
        assert!((eval_local.anti_stub_task_success - 0.42).abs() < f64::EPSILON);
    }

    #[test]
    fn workspace_run_dir_loads_five_spoke_before_after_scorecard() {
        let result = read_run_reports(Path::new("workspace"));
        let sc = result
            .spoke_scorecard
            .expect("workspace spoke_scorecard should load from contracts/reports");
        assert_eq!(sc.spokes.len(), 5);
        assert!((sc.overall_pass_at_1 - 1.0).abs() < 1e-9);
        assert!(sc.promotion_gate_passed);

        let org = result
            .organic_scorecard
            .expect("workspace organic_scorecard should load from contracts/reports");
        assert_eq!(org.total_cases, 24);
        assert_eq!(org.spokes.len(), 8);
        assert!((org.overall_search_recall_at_5 - 1.0).abs() < 1e-9);
        assert!(org.overall_mrr >= 0.95);
        assert!((org.overall_unbiased_search_recall_at_5 - 1.0).abs() < 1e-9);
        assert!((org.overall_distractor_resilience_rate - 1.0).abs() < 1e-9);
        assert!((org.overall_compilation_pass_rate - 1.0).abs() < 1e-9);
        assert!((org.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn ladder_status_loads_pinned_qwen3_rungs_and_persists_custom_spokes_and_config() {
        let dir = tempfile::tempdir().unwrap();
        let saved_spokes = save_custom_spoke_to_dir(
            dir.path(),
            CustomSpokeDto {
                id: "sql-query".to_string(),
                label: "SQL Query".to_string(),
                domain: "Database SQL".to_string(),
                target_gate: "sql_validity >= 0.95".to_string(),
                family: "custom".to_string(),
            },
        )
        .expect("save custom spoke");
        assert_eq!(saved_spokes.len(), 1);
        assert_eq!(saved_spokes[0].id, "sql-query");

        let mut epoch_caps = HashMap::new();
        epoch_caps.insert("8B".to_string(), 5);
        let saved_cfg = save_crucible_config_to_dir(
            dir.path(),
            MensCrucibleConfigDto {
                hub_embedder: "Qwen3-Embedding-0.6B".to_string(),
                hub_base_rung: "Qwen3-8B".to_string(),
                compute_target: "RTX 4090 (24GB)".to_string(),
                lora_rank: 32,
                lora_alpha: 64,
                seq_len: 4096,
                vram_cap_pct: 90,
                epoch_caps,
            },
        )
        .expect("save crucible config");
        assert_eq!(saved_cfg.lora_rank, 32);

        let status = read_ladder_status(dir.path());
        assert_eq!(status.rungs.len(), 6);
        assert_eq!(status.rungs[0].size, "0.6B");
        assert_eq!(status.rungs[0].repo_master_sha, "@c1899de2");
        assert_eq!(status.rungs[3].size, "8B");
        assert_eq!(status.rungs[3].repo_master_sha, "@b968826d");
        assert_eq!(
            status.rungs[3].cells.get("sql-query").map(String::as_str),
            Some("MISSING")
        );
        assert_eq!(status.custom_spokes.len(), 1);
        assert_eq!(
            status.crucible_config.as_ref().map(|c| c.lora_rank),
            Some(32)
        );
    }

    #[test]
    fn live_telemetry_parses_jsonl_series_when_present() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("telemetry.jsonl"),
            r#"{"step":100,"loss":0.95,"gpu_util_pct":88.0,"vram_used_gb":12.4,"cpu_pct":36.0,"ram_used_gb":21.0}
{"step":200,"loss":0.312,"gpu_util_pct":96.0,"vram_used_gb":14.1,"cpu_pct":45.0,"ram_used_gb":23.2}
"#,
        )
        .unwrap();

        let telemetry = read_live_telemetry(dir.path());
        assert_eq!(telemetry.step, 200);
        assert!((telemetry.loss - 0.312).abs() < 1e-6);
        assert!((telemetry.gpu_pct - 96.0).abs() < 1e-6);
        assert_eq!(telemetry.loss_series, vec![0.95, 0.312]);
        assert_eq!(telemetry.gpu_series, vec![88.0, 96.0]);
    }

    #[test]
    fn corpus_studio_snapshot_reads_mix_config_and_auto_materializes_reasoning_traces() {
        let dir = tempfile::tempdir().unwrap();
        let cfg_dir = dir.path().join("mens").join("config");
        let src_dir = dir.path().join("mens").join("data").join("mix_sources");
        std::fs::create_dir_all(&cfg_dir).unwrap();
        std::fs::create_dir_all(&src_dir).unwrap();

        std::fs::write(
            src_dir.join("golden_examples.jsonl"),
            r#"{"prompt":"Write a Vox add function.","response":"<think>\nUse fn add(a: int, b: int) toint\n</think>\n```vox\nfn add(a: int, b: int) to int { ret a + b }\n```","category":"vox_codegen"}"#,
        )
        .unwrap();

        std::fs::write(
            cfg_dir.join("mix.yaml"),
            r#"sources:
  - path: mens/data/mix_sources/golden_examples.jsonl
    weight: 2.0
    optional: false
  - path: mens/data/mix_sources/vox_reasoning_traces.jsonl
    weight: 4.0
    optional: true
output: target/dogfood/train_mixed.jsonl
"#,
        )
        .unwrap();

        let snap = vox_orchestrator_mcp::gui_ops::mens_corpus::corpus_studio_snapshot(
            dir.path(),
            Some("mens/config/mix.yaml"),
        )
        .expect("snapshot succeeds");

        assert_eq!(snap.active_config_path, "mens/config/mix.yaml");
        assert_eq!(snap.sources.len(), 2);
        // vox_reasoning_traces.jsonl should have been auto-materialized with 120 rows!
        let reasoning_src = snap
            .sources
            .iter()
            .find(|s| s.path.ends_with("vox_reasoning_traces.jsonl"))
            .expect("reasoning source exists");
        assert!(reasoning_src.exists);
        assert_eq!(reasoning_src.record_count, 120);
        assert!(reasoning_src.auto_synthesizable);

        // Read JSONL records with AST validation
        let page = vox_orchestrator_mcp::gui_ops::mens_corpus::read_jsonl_records(
            dir.path(),
            "mens/data/mix_sources/golden_examples.jsonl",
            0,
            10,
            None,
            None,
        )
        .expect("read jsonl");
        assert_eq!(page.total_matching, 1);
        assert!(page.records[0].has_think);
        assert!(page.records[0].ast_valid);

        // Edit row and trigger auto-sync mix rebuild
        let save_res = vox_orchestrator_mcp::gui_ops::mens_corpus::save_jsonl_row(
            dir.path(),
            vox_orchestrator_mcp::gui_ops::mens_corpus::MensSaveJsonlRowRequestDto {
                config_path: Some("mens/config/mix.yaml".to_string()),
                source_path: "mens/data/mix_sources/golden_examples.jsonl".to_string(),
                line_number: Some(1),
                prompt: "Write a Vox multiply function.".to_string(),
                think_block: Some("Multiply two int parameters a and b.".to_string()),
                answer_block: "```vox\nfn mul(a: int, b: int) to int {\n    ret a * b\n}\n```"
                    .to_string(),
                category: Some("vox_codegen".to_string()),
                task_family: Some("vox_codegen".to_string()),
                weight_override: Some(3.0),
                auto_sync: true,
            },
        )
        .expect("save row and auto-sync");

        assert!(save_res.ast_valid);
        assert!(save_res.mix_rebuilt);
        assert!(save_res.snapshot.output_records > 100);
        assert!(save_res.snapshot.mix_fresh);
    }
}
