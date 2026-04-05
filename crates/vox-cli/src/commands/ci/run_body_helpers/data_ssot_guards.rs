//! Guards for data / telemetry SSOT drift (`vox ci data-ssot-guards`).

use anyhow::{Context, Result, anyhow};
use std::path::Path;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

fn metric_type_constants_from_research_contract(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let line = line.trim_start();
        if !line.starts_with("pub const METRIC_TYPE_") {
            continue;
        }
        let Some(rest) = line.strip_prefix("pub const METRIC_TYPE_") else {
            continue;
        };
        // `BENCHMARK_EVENT: &str = "benchmark_event";`
        let Some(eq) = rest.find("= \"") else {
            continue;
        };
        let after = &rest[eq + 3..];
        let Some(end) = after.find('"') else {
            continue;
        };
        out.push(after[..end].to_string());
    }
    out
}

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
    let metric_txt = read_utf8_path_capped(&metric_doc)
        .with_context(|| format!("read {}", metric_doc.display()))?;

    let taxonomy_doc = root.join("docs/src/architecture/telemetry-taxonomy-contracts-ssot.md");
    if !taxonomy_doc.is_file() {
        return Err(anyhow!("missing {}", taxonomy_doc.display()));
    }
    let taxonomy_txt = read_utf8_path_capped(&taxonomy_doc)
        .with_context(|| format!("read {}", taxonomy_doc.display()))?;

    let research_contract = root.join("crates/vox-db/src/research_metrics_contract.rs");
    let rc = read_utf8_path_capped(&research_contract)
        .with_context(|| format!("read {}", research_contract.display()))?;
    let metric_types = metric_type_constants_from_research_contract(&rc);
    if metric_types.is_empty() {
        return Err(anyhow!(
            "{}: expected pub const METRIC_TYPE_* string literals",
            research_contract.display()
        ));
    }
    for mt in &metric_types {
        if !metric_txt.contains(mt.as_str()) && !taxonomy_txt.contains(mt.as_str()) {
            return Err(anyhow!(
                "METRIC_TYPE value `{mt}` must appear in {} or {}",
                metric_doc.display(),
                taxonomy_doc.display()
            ));
        }
    }

    let contracts_index = root.join("contracts/index.yaml");
    let ix = read_utf8_path_capped(&contracts_index)
        .with_context(|| format!("read {}", contracts_index.display()))?;
    for id in [
        "telemetry-completion-run-v1-schema",
        "telemetry-completion-finding-v1-schema",
        "telemetry-completion-detector-snapshot-v1-schema",
    ] {
        if !ix.contains(id) {
            return Err(anyhow!(
                "{} must register `{id}` (completion telemetry JSON Schemas)",
                contracts_index.display()
            ));
        }
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
