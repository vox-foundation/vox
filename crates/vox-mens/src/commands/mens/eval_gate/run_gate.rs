//! CLI entry for `vox mens eval-gate`.

use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use super::check_run::check_run;

/// Run eval-gate and return exit code (0 = pass, 1 = fail).
///
/// Always writes `gate_receipt.json` to `run_dir` so orchestrators and A2A
/// consumers have a machine-readable record of the most recent gate run —
/// whether it passed or failed.
pub fn run_eval_gate(run_dir: PathBuf, policy_path: Option<PathBuf>) -> Result<i32> {
    let policy_path = policy_path.unwrap_or_else(|| PathBuf::from("mens/config/eval-gates.yaml"));
    if !policy_path.exists() {
        // Hard-fail: a missing policy file must never silently pass all gates.
        // Callers that intentionally want to skip gating should use VOX_MENS_SKIP_EVAL
        // or VOX_MENS_FORCE_TRAIN, not a non-existent policy path.
        eprintln!(
            "  {} Policy file not found: {} — gate aborted (fix --policy path)",
            "✗".red(),
            policy_path.display()
        );
        return Ok(1);
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

    let overall_passed = !failed_blocking;
    if overall_passed {
        eprintln!("  {} All blocking gates passed.", "✓".green());
    } else {
        eprintln!("  {} Blocking gates failed.", "✗".red());
    }

    // Write gate_receipt.json so orchestrators and A2A consumers can inspect
    // gate state without re-running the gate.  Always written (pass or fail)
    // so the receipt reflects the most-recent run.
    let receipt_path = run_dir.join("gate_receipt.json");
    let timestamp = {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    };
    let receipt = serde_json::json!({
        "schema": "vox_mens_gate_receipt_v1",
        "timestamp_unix": timestamp,
        "run_dir": run_dir.display().to_string(),
        "policy": policy_path.display().to_string(),
        "overall_passed": overall_passed,
        "gates": results.iter().map(|r| serde_json::json!({
            "name": r.name,
            "passed": r.passed,
            "block": r.block,
            "message": r.message,
        })).collect::<Vec<_>>(),
    });
    if let Ok(json) = serde_json::to_string_pretty(&receipt) {
        if let Err(e) = std::fs::write(&receipt_path, &json) {
            eprintln!("  ⚠ Could not write gate_receipt.json: {e}");
        } else {
            eprintln!("  Receipt: {}", receipt_path.display());
        }
    }

    Ok(if overall_passed { 0 } else { 1 })
}
