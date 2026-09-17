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

#[derive(Debug, Clone, Serialize, Default, PartialEq)]
pub struct MensRunReportsDto {
    pub eval_local: Option<EvalLocalReportDto>,
    pub gate_receipt: Option<GateReceiptDto>,
    pub collateral_damage: Option<CollateralDamageReportDto>,
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

/// Reads whichever of the three report files exist in `run_dir`. Missing
/// files (including a wholly nonexistent `run_dir`) map to `None`, never an
/// error — that's the normal state before any eval has run.
pub fn read_run_reports(run_dir: &Path) -> MensRunReportsDto {
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
}
