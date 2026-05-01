//! CLI entry for `vox mens eval-gate`.

use anyhow::Result;
use owo_colors::OwoColorize;
use std::path::PathBuf;

use super::check_run::check_run;

/// Run eval-gate and return exit code (0 = pass, 1 = fail).
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
    if failed_blocking {
        eprintln!("  {} Blocking gates failed.", "✗".red());
        Ok(1)
    } else {
        eprintln!("  {} All blocking gates passed.", "✓".green());
        Ok(0)
    }
}
