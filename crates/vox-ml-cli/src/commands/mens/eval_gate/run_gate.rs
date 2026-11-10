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

    let policy_val = super::policy::load_policy_value(&policy_path)?;
    let hash_input = vox_eval::gate_state::policy_hash_input(&policy_val);
    let policy_blake3 = vox_crypto::hex_encode(&vox_crypto::secure_hash(&hash_input));

    let adapter_card_path = run_dir.join("adapter_card.json");
    let adapter_card: Option<serde_json::Value> = if adapter_card_path.is_file() {
        std::fs::read_to_string(&adapter_card_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    } else {
        None
    };
    let adapter_sha256 = adapter_card.as_ref().and_then(|c| {
        c.get("adapter_sha256")
            .and_then(|s| s.as_str())
            .map(ToString::to_string)
    });
    let base_pin = adapter_card.as_ref().and_then(|c| {
        c.get("base_hf_id")
            .or_else(|| c.get("base_model"))
            .and_then(|s| s.as_str())
            .map(ToString::to_string)
    });

    let collateral_path = run_dir.join("collateral_damage_report.json");
    let collateral: Option<serde_json::Value> = if collateral_path.is_file() {
        std::fs::read_to_string(&collateral_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    } else {
        None
    };

    let slice_blake3 = collateral
        .as_ref()
        .and_then(|c| c.get("slice_blake3").cloned())
        .unwrap_or(serde_json::json!({}));
    let grader = collateral
        .as_ref()
        .and_then(|c| {
            c.get("grader")
                .and_then(|s| s.as_str())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| vox_eval::qwen_reasoning::GRADER_ID.to_string());

    let decode_blake3 = if let Some(decode_obj) = collateral.as_ref().and_then(|c| c.get("decode"))
    {
        let mut canonical_decode = decode_obj.clone();
        if let Some(map) = canonical_decode.as_object_mut() {
            map.remove("batch_width");
            map.remove("batch_size");
        }
        let canonical_str = vox_eval::gate_state::canonical_json(&canonical_decode);
        Some(vox_crypto::hex_encode(&vox_crypto::secure_hash(
            canonical_str.as_bytes(),
        )))
    } else {
        None
    };

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
        "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
        "timestamp_unix": timestamp,
        // The commit this binary was built from; only meaningful from a clean tree.
        "source_commit": env!("VOX_GIT_HASH"),
        "run_dir": run_dir.display().to_string(),
        "policy": policy_path.display().to_string(),
        "policy_blake3": policy_blake3,
        "spoke": serde_json::Value::Null,
        "overall_passed": overall_passed,
        "gates": results.iter().map(|r| serde_json::json!({
            "name": r.name,
            "passed": r.passed,
            "block": r.block,
            "message": r.message,
        })).collect::<Vec<_>>(),
        "inputs": {
            "adapter_sha256": adapter_sha256,
            "base_pin": base_pin,
            "slice_blake3": slice_blake3,
            "grader": grader,
            "decode_blake3": decode_blake3,
            "suites": {},
            "datasets": []
        }
    });
    if let Ok(json) = serde_json::to_string_pretty(&receipt) {
        if let Err(e) = std::fs::write(&receipt_path, &json) {
            eprintln!("  ⚠ Could not write gate_receipt.json: {e}");
        } else {
            eprintln!("  Receipt: {}", receipt_path.display());
        }
    }

    #[cfg(any(feature = "gpu", feature = "mens-hf-hub"))]
    {
        let root = vox_corpus::training::contract::find_workspace_root();
        if let Some(ref root_dir) = root {
            let vox_lang_policy = root_dir.join("mens/config/eval-gates.yaml");
            if vox_lang_policy.is_file()
                && run_dir.join("eval_local_report.json").is_file()
                && run_dir.join("benchmark_passatk.json").is_file()
                && let (Ok(spoke_results), Ok(spoke_policy_val)) = (
                    check_run(&run_dir, &vox_lang_policy),
                    super::policy::load_policy_value(&vox_lang_policy),
                )
            {
                let spoke_passed = !spoke_results.iter().any(|r| !r.passed && r.block);
                let spoke_hash_input = vox_eval::gate_state::policy_hash_input(&spoke_policy_val);
                let spoke_policy_blake3 =
                    vox_crypto::hex_encode(&vox_crypto::secure_hash(&spoke_hash_input));
                let spoke_receipt = serde_json::json!({
                    "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                    "timestamp_unix": timestamp,
                    "source_commit": env!("VOX_GIT_HASH"),
                    "run_dir": run_dir.display().to_string(),
                    "policy": "mens/config/eval-gates.yaml",
                    "policy_blake3": spoke_policy_blake3,
                    "spoke": "vox-lang",
                    "overall_passed": spoke_passed,
                    "gates": spoke_results.iter().map(|r| serde_json::json!({
                        "name": r.name,
                        "passed": r.passed,
                        "block": r.block,
                        "message": r.message,
                    })).collect::<Vec<_>>(),
                    "inputs": {
                        "adapter_sha256": adapter_sha256,
                        "base_pin": base_pin,
                        "slice_blake3": slice_blake3,
                        "grader": grader,
                        "decode_blake3": decode_blake3,
                        "suites": {},
                        "datasets": []
                    }
                });
                if let Ok(json) = serde_json::to_string_pretty(&spoke_receipt) {
                    let _ = std::fs::write(run_dir.join("gate_receipt.vox-lang.json"), &json);
                }
            }
        }
        let rows = vox_populi::mens::gate_policy::spoke_gate_rows(&run_dir, root.as_deref());
        let spoke_gates = vox_eval::gate_state::spoke_gates_json(
            adapter_sha256.as_deref(),
            base_pin.as_deref(),
            &rows,
        );
        let spoke_gates_path = run_dir.join("spoke_gates.json");
        if let Ok(json) = serde_json::to_string_pretty(&spoke_gates) {
            let _ = std::fs::write(&spoke_gates_path, &json);
        }
        let sync_env = std::env::var(vox_populi::mens::hub_sync::HF_SYNC_ENV).ok();
        let ns_env = std::env::var(vox_populi::mens::hub_repo::NAMESPACE_ENV).ok();
        let skip = vox_populi::mens::hub_sync::explain_auto_publish_skip(
            false,
            sync_env.as_deref(),
            ns_env.as_deref(),
        );
        if let Some(why) = skip {
            if overall_passed {
                eprintln!("  {} {why}", "⚠".yellow());
            }
        } else if !overall_passed {
            eprintln!(
                "  {} HF auto-publish skipped: blocking gates failed",
                "⚠".yellow()
            );
            if let Err(e) =
                vox_populi::mens::hub_sync::refresh_ladder_index_blocking(None, None, false)
            {
                eprintln!("  {} HF index refresh failed: {e}", "⚠".yellow());
            }
        } else {
            match vox_populi::mens::hub_sync::publish_adapter_run_dir_blocking(
                &run_dir, None, false, false,
            ) {
                Ok(vox_populi::mens::hub_sync::PublishOutcome::Uploaded {
                    repo_id,
                    commit_oid,
                    version,
                    ..
                }) => {
                    eprintln!(
                        "  {} Auto-published adapter to Hugging Face Hub `{repo_id}` as {version} ({commit_oid})",
                        "✓".green()
                    );
                }
                Ok(vox_populi::mens::hub_sync::PublishOutcome::SkippedUnchanged {
                    repo_id,
                    ..
                }) => {
                    eprintln!(
                        "  {} HF Hub `{repo_id}` already up-to-date (hash match; skipped upload)",
                        "ℹ".cyan()
                    );
                }
                Err(e) => {
                    eprintln!("  {} HF auto-publish skipped: {e}", "⚠".yellow());
                }
            }
            if let Err(e) =
                vox_populi::mens::hub_sync::refresh_ladder_index_blocking(None, None, false)
            {
                eprintln!("  {} HF index refresh failed: {e}", "⚠".yellow());
            }
        }
    }

    Ok(if overall_passed { 0 } else { 1 })
}

#[cfg(test)]
mod tests {
    use super::run_eval_gate;

    #[test]
    fn a_missing_policy_fails_and_writes_no_receipt() {
        let dir = tempfile::tempdir().unwrap();
        let code = run_eval_gate(
            dir.path().to_path_buf(),
            Some(dir.path().join("absent.yaml")),
        )
        .unwrap();
        assert_eq!(code, 1);
        assert!(!dir.path().join("gate_receipt.json").exists());
    }

    #[test]
    fn receipt_v2() {
        let dir = tempfile::tempdir().unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(&policy_path, "{}\n").unwrap();

        let code = run_eval_gate(dir.path().to_path_buf(), Some(policy_path.clone())).unwrap();
        assert_eq!(code, 0);

        let receipt_raw = std::fs::read_to_string(dir.path().join("gate_receipt.json")).unwrap();
        let receipt: serde_json::Value = serde_json::from_str(&receipt_raw).unwrap();

        assert_eq!(receipt["schema"], vox_eval::gate_state::RECEIPT_SCHEMA);
        let policy_val = super::super::policy::load_policy_value(&policy_path).unwrap();
        let hash_input = vox_eval::gate_state::policy_hash_input(&policy_val);
        let expected_hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(&hash_input));
        assert_eq!(receipt["policy_blake3"], expected_hash);
        assert!(receipt.get("inputs").is_some());
    }

    #[test]
    #[cfg(feature = "gpu")]
    fn policy_hash_parity() {
        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        let ladder_rel = "mens/config/eval-gates-ladder.yaml";
        let ladder_path = root.join(ladder_rel);

        let policy_val = super::super::policy::load_policy_value(&ladder_path).unwrap();
        let hash_input = vox_eval::gate_state::policy_hash_input(&policy_val);
        let ml_cli_hash = vox_crypto::hex_encode(&vox_crypto::secure_hash(&hash_input));

        let populi_hash =
            vox_populi::mens::gate_policy::policy_blake3(Some(&root), ladder_rel).unwrap();
        assert_eq!(
            ml_cli_hash, populi_hash,
            "cross-crate policy hash must match"
        );
    }

    #[test]
    fn spoke_gates_schema() {
        let dir = tempfile::tempdir().unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(&policy_path, "{}\n").unwrap();

        let _ = run_eval_gate(dir.path().to_path_buf(), Some(policy_path)).unwrap();

        let spoke_gates_file = dir.path().join("spoke_gates.json");
        assert!(spoke_gates_file.exists());
        let raw = std::fs::read_to_string(&spoke_gates_file).unwrap();
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        let schema_path = root.join("contracts/mens/spoke-gate-state.v1.schema.json");
        let schema_str = std::fs::read_to_string(&schema_path).unwrap();
        let validator =
            vox_jsonschema_util::compile_validator_from_utf8(&schema_str, &schema_path).unwrap();

        assert!(vox_jsonschema_util::validate(&val, &validator, "spoke_gates.json").is_ok());

        // Deliberately invalid: state = "pass" instead of "passed"
        let mut invalid_val = val.clone();
        if let Some(rows) = invalid_val.get_mut("rows").and_then(|r| r.as_array_mut())
            && let Some(first) = rows.get_mut(0)
        {
            first["state"] = serde_json::json!("pass");
        }
        let bad_res =
            vox_jsonschema_util::validate(&invalid_val, &validator, "invalid spoke_gates.json");
        assert!(
            bad_res.is_err(),
            "document with invalid state `pass` must be rejected"
        );
    }

    #[test]
    fn receipt_schema() {
        let dir = tempfile::tempdir().unwrap();
        let policy_path = dir.path().join("policy.yaml");
        std::fs::write(&policy_path, "{}\n").unwrap();

        let _ = run_eval_gate(dir.path().to_path_buf(), Some(policy_path)).unwrap();

        let receipt_file = dir.path().join("gate_receipt.json");
        assert!(receipt_file.exists());
        let raw = std::fs::read_to_string(&receipt_file).unwrap();
        let val: serde_json::Value = serde_json::from_str(&raw).unwrap();

        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        let schema_path = root.join("contracts/mens/gate-receipt.v2.schema.json");
        let schema_str = std::fs::read_to_string(&schema_path).unwrap();
        let validator =
            vox_jsonschema_util::compile_validator_from_utf8(&schema_str, &schema_path).unwrap();

        assert!(vox_jsonschema_util::validate(&val, &validator, "gate_receipt.json").is_ok());

        // Deliberately invalid: removing policy_blake3
        let mut invalid_val = val.clone();
        invalid_val.as_object_mut().unwrap().remove("policy_blake3");
        let bad_res =
            vox_jsonschema_util::validate(&invalid_val, &validator, "invalid gate_receipt.json");
        assert!(
            bad_res.is_err(),
            "receipt without policy_blake3 must be rejected"
        );
    }
}
