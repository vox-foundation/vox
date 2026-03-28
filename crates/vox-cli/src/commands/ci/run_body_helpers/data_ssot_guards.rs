//! Guards for data / telemetry SSOT drift (`vox ci data-ssot-guards`).

use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

/// Run file-based checks that do not require a full workspace `cargo test`.
pub fn run_data_ssot_guards(root: &Path) -> Result<()> {
    let watch = root.join("crates/vox-cli/src/commands/mens/watch_telemetry.rs");
    let watch_txt =
        read_utf8_path_capped(&watch).with_context(|| format!("read {}", watch.display()))?;
    if !watch_txt.contains("eta_seconds_remaining") {
        return Err(anyhow!(
            "{} must parse telemetry payload key `eta_seconds_remaining` (Populi telemetry_schema)",
            watch.display()
        ));
    }
    if !watch_txt.contains("steps_per_sec_ema") {
        return Err(anyhow!(
            "{} must parse `steps_per_sec_ema`",
            watch.display()
        ));
    }

    let policy_doc = root.join("docs/src/architecture/voxdb-connect-policy.md");
    if !policy_doc.is_file() {
        return Err(anyhow!("missing {}", policy_doc.display()));
    }

    let metric_doc = root.join("docs/src/reference/telemetry-metric-contract.md");
    if !metric_doc.is_file() {
        return Err(anyhow!("missing {}", metric_doc.display()));
    }

    let codex_metrics = root.join("crates/vox-db/src/store/ops_codex/codex_metrics_packages.rs");
    let cm = read_utf8_path_capped(&codex_metrics)
        .with_context(|| format!("read {}", codex_metrics.display()))?;
    if cm.contains("COALESCE(metric_value") {
        return Err(anyhow!(
            "{} must not use COALESCE(metric_value — NULL must stay distinct from 0.0",
            codex_metrics.display()
        ));
    }

    println!("data-ssot-guards: OK");
    Ok(())
}
