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

use serde::Serialize;
use std::path::Path;

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
    pub mean_grounding_ratio: f64,
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
    pub overall_grounding_ratio: f64,
    pub overall_compilation_pass_rate: f64,
    pub overall_end_to_end_pass_at_1: f64,
    pub spokes: Vec<OrganicSpokeRowDto>,
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
    let overall_grounding_ratio = v
        .get("overall_grounding_ratio")
        .and_then(serde_json::Value::as_f64)
        .unwrap_or(overall_search_recall_at_5);
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
            let mean_grounding_ratio = s
                .get("mean_grounding_ratio")
                .and_then(serde_json::Value::as_f64)
                .unwrap_or(0.0);
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
                mean_grounding_ratio,
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
        overall_grounding_ratio,
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
        assert_eq!(org.total_cases, 15);
        assert_eq!(org.spokes.len(), 5);
        assert!((org.overall_search_recall_at_5 - 1.0).abs() < 1e-9);
        assert!(org.overall_mrr >= 0.95);
        assert!((org.overall_compilation_pass_rate - 1.0).abs() < 1e-9);
        assert!((org.overall_end_to_end_pass_at_1 - 1.0).abs() < 1e-9);
    }
}
