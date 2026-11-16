//! Per-spoke adapter routing gated on `spoke_gates.json` / gate receipts (`D-06`, `338-06`).

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// Resolve the loadable adapter path for `spoke` inside `run_dir`.
///
/// Enforces that `spoke` has a `Passed` gate state (either recorded in `spoke_gates.json` or
/// computed freshly via `gate_policy::spoke_gate_rows` + `applying_rows`). Returns `Ok(None)` if
/// `spoke` is not recorded as passing when `require_passed` is false, or `Err` if gating fails.
pub fn resolve_spoke_adapter(
    run_dir: &Path,
    spoke: &str,
    workspace_root: Option<&Path>,
) -> Result<PathBuf> {
    let adapter = crate::mens::tensor::adapter_card::locate_adapter(run_dir)
        .or_else(|| {
            let p = run_dir.join("adapter.safetensors");
            p.is_file().then_some(p)
        })
        .ok_or_else(|| anyhow::anyhow!("no adapter safetensors found in {}", run_dir.display()))?;

    let rows = crate::mens::gate_policy::spoke_gate_rows(run_dir, workspace_root);
    let row = rows
        .iter()
        .find(|r| r.spoke == spoke)
        .with_context(|| format!("unknown spoke `{spoke}` in gate rows"))?;

    if row.state != vox_eval::gate_state::GateState::Passed {
        bail!(
            "spoke `{spoke}` in {} is {} ({})",
            run_dir.display(),
            row.state,
            row.reason
        );
    }

    Ok(adapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_spoke_adapter_blocks_unpassed_spoke_and_allows_passed_spoke() {
        let dir = tempfile::tempdir().unwrap();
        let adapter = dir.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&adapter, b"adapter-bytes").unwrap();

        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        // Before any gate_receipt.vox-lang.json exists, resolving vox-lang must refuse.
        let err = resolve_spoke_adapter(dir.path(), "vox-lang", Some(&root)).unwrap_err();
        assert!(err.to_string().contains("not_run"), "{err}");

        // Write passing receipt for vox-lang matching policy_blake3.
        let hash =
            crate::mens::gate_policy::policy_blake3(Some(&root), "mens/config/eval-gates.yaml")
                .unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.vox-lang.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                "policy_blake3": hash,
                "overall_passed": true,
                "gates": [
                    {"name": "leakage", "passed": true, "block": true, "message": "ok"},
                    {"name": "anti_stub", "passed": true, "block": true, "message": "ok"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let resolved = resolve_spoke_adapter(dir.path(), "vox-lang", Some(&root)).unwrap();
        assert_eq!(resolved, adapter);
    }
}
