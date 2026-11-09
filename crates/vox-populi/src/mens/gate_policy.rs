//! Mens gate policy loader and test fixtures.

use std::path::Path;

const EMBEDDED_LADDER: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../mens/config/eval-gates-ladder.yaml"
));

/// Current collateral requirement from `mens/config/eval-gates-ladder.yaml` or embedded fallback.
pub fn ladder_requirement(workspace_root: Option<&Path>) -> vox_eval::CollateralRequirement {
    let text = workspace_root
        .and_then(|r| std::fs::read_to_string(r.join("mens/config/eval-gates-ladder.yaml")).ok())
        .unwrap_or_else(|| EMBEDDED_LADDER.to_string());
    let yaml: serde_yaml::Value = serde_yaml::from_str(&text).expect("valid yaml for ladder gates");
    let policy = serde_json::to_value(yaml).expect("valid json conversion for ladder gates");
    vox_eval::CollateralRequirement::from_policy_json(&policy)
        .expect("valid collateral requirement")
}

/// Compute canonical BLAKE3 hash for a policy file relative to workspace root (or embedded ladder).
pub fn policy_blake3(workspace_root: Option<&Path>, rel: &str) -> Result<String, String> {
    let text = if rel == "mens/config/eval-gates-ladder.yaml" {
        workspace_root
            .and_then(|r| std::fs::read_to_string(r.join(rel)).ok())
            .unwrap_or_else(|| EMBEDDED_LADDER.to_string())
    } else {
        let root = workspace_root
            .ok_or_else(|| "workspace_root required for non-ladder policy".to_string())?;
        std::fs::read_to_string(root.join(rel))
            .map_err(|e| format!("cannot read policy file {rel}: {e}"))?
    };
    let yaml: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|e| format!("invalid yaml in {rel}: {e}"))?;
    let json_val =
        serde_json::to_value(yaml).map_err(|e| format!("failed converting {rel} to json: {e}"))?;
    let input = vox_eval::gate_state::policy_hash_input(&json_val);
    Ok(vox_crypto::facades::hex_encode(
        &vox_crypto::facades::secure_hash(&input),
    ))
}

/// Resolve per-spoke gate rows for a run directory.
///
/// Always evaluates the ladder row (`gate_receipt.json`) with `applies: true`.
/// If `workspace_root` is present, also inspects domain profiles in `mens/config/domain-profiles.yaml`
/// that specify an `eval_gate`, sorted by profile name, each evaluated against `gate_receipt.<profile>.json`
/// with `applies: false`.
pub fn spoke_gate_rows(
    run_dir: &Path,
    workspace_root: Option<&Path>,
) -> Vec<vox_eval::gate_state::SpokeGateRow> {
    let mut rows = Vec::new();

    let adapter_card_path = run_dir.join("adapter_card.json");
    let adapter_sha256 = if adapter_card_path.is_file() {
        std::fs::read_to_string(&adapter_card_path)
            .ok()
            .and_then(|raw| serde_json::from_str::<serde_json::Value>(&raw).ok())
            .and_then(|v| {
                v.get("adapter_sha256")
                    .and_then(|s| s.as_str())
                    .map(ToString::to_string)
            })
    } else {
        None
    };

    // 1. Ladder row
    let ladder_hash = policy_blake3(workspace_root, "mens/config/eval-gates-ladder.yaml")
        .unwrap_or_else(|_| "".to_string());
    let ladder_receipt_file = "gate_receipt.json";
    let ladder_receipt_path = run_dir.join(ladder_receipt_file);
    let ladder_receipt_val: Option<serde_json::Value> = if ladder_receipt_path.is_file() {
        std::fs::read_to_string(&ladder_receipt_path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
    } else {
        None
    };

    let ladder_current = vox_eval::gate_state::CurrentGate {
        policy_blake3: ladder_hash.clone(),
        adapter_sha256: adapter_sha256.clone(),
    };
    let (ladder_state, ladder_reason) =
        vox_eval::gate_state::gate_state(ladder_receipt_val.as_ref(), &ladder_current);

    rows.push(vox_eval::gate_state::SpokeGateRow {
        spoke: "ladder".to_string(),
        policy_file: "mens/config/eval-gates-ladder.yaml".to_string(),
        policy_blake3: ladder_hash,
        receipt_file: if ladder_receipt_val.is_some() {
            Some(ladder_receipt_file.to_string())
        } else {
            None
        },
        state: ladder_state,
        reason: ladder_reason,
        applies: true,
    });

    let Some(root) = workspace_root else {
        return rows;
    };

    // 2. Domain profiles with an eval_gate
    let profiles_path = root.join("mens/config/domain-profiles.yaml");
    if let Ok(raw) = std::fs::read_to_string(&profiles_path) {
        if let Ok(file_val) = serde_yaml::from_str::<serde_json::Value>(&raw) {
            if let Some(profiles_map) = file_val.get("profiles").and_then(|p| p.as_object()) {
                let mut sorted_profiles: Vec<_> = profiles_map.iter().collect();
                sorted_profiles.sort_by_key(|(k, _)| (*k).clone());

                for (name, prof) in sorted_profiles {
                    if let Some(eval_gate_rel) = prof.get("eval_gate").and_then(|v| v.as_str()) {
                        let prof_hash = policy_blake3(Some(root), eval_gate_rel)
                            .unwrap_or_else(|_| "".to_string());
                        let receipt_file = format!("gate_receipt.{name}.json");
                        let receipt_path = run_dir.join(&receipt_file);
                        let receipt_val: Option<serde_json::Value> = if receipt_path.is_file() {
                            std::fs::read_to_string(&receipt_path)
                                .ok()
                                .and_then(|r| serde_json::from_str(&r).ok())
                        } else {
                            None
                        };

                        let current = vox_eval::gate_state::CurrentGate {
                            policy_blake3: prof_hash.clone(),
                            adapter_sha256: adapter_sha256.clone(),
                        };
                        let (state, reason) =
                            vox_eval::gate_state::gate_state(receipt_val.as_ref(), &current);

                        rows.push(vox_eval::gate_state::SpokeGateRow {
                            spoke: name.clone(),
                            policy_file: eval_gate_rel.to_string(),
                            policy_blake3: prof_hash,
                            receipt_file: if receipt_val.is_some() {
                                Some(receipt_file)
                            } else {
                                None
                            },
                            state,
                            reason,
                            applies: false,
                        });
                    }
                }
            }
        }
    }

    rows
}

#[cfg(test)]
pub fn passing_collateral_report(req: &vox_eval::CollateralRequirement) -> serde_json::Value {
    use vox_eval::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
    use vox_eval::reasoning_slice::{
        ReasoningSliceVerdict, eval_reasoning_slice, pooled_accuracy, reasoning_slices_json,
    };

    let flags = |n: usize, f: Box<dyn Fn(usize) -> bool>| -> Vec<(String, bool)> {
        (0..n).map(|i| (format!("item_{i}"), f(i))).collect()
    };
    let base = flags(SLICE_SIZE, Box::new(|i| i < 16));
    let adapter = flags(SLICE_SIZE, Box::new(|i| i < 15));
    let all_true = flags(SLICE_SIZE, Box::new(|_| true));
    let verdicts: Vec<ReasoningSliceVerdict> = SLICE_NAMES
        .iter()
        .map(|s| {
            eval_reasoning_slice(s, (&base, &adapter), (&all_true, &all_true), req.alpha).unwrap()
        })
        .collect();
    let pooled = pooled_accuracy(&verdicts, req.alpha).unwrap();
    let mut slices_map = serde_json::Map::new();
    for (k, v) in &req.slices {
        slices_map.insert(k.clone(), serde_json::Value::String(v.clone()));
    }
    serde_json::json!({
        "status": "pass",
        "failed_on": null,
        "threshold": req.alpha,
        "reports": [],
        "reasoning_slices": reasoning_slices_json(&verdicts, Some(&pooled), req.alpha),
        "slice_blake3": slices_map,
        "grader": req.grader,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spoke_gate_rows_ladder_passed_spokes_not_run() {
        let temp = tempfile::tempdir().unwrap();
        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        let ladder_hash = policy_blake3(Some(&root), "mens/config/eval-gates-ladder.yaml").unwrap();

        std::fs::write(
            temp.path().join("gate_receipt.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                "policy_blake3": ladder_hash,
                "overall_passed": true,
                "gates": [
                    {"name": "throughput", "passed": true, "block": true, "message": "150 tok/s"},
                    {"name": "pass_at_k", "passed": true, "block": true, "message": "0.60 >= 0.50"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let rows = spoke_gate_rows(temp.path(), Some(&root));
        assert!(!rows.is_empty());
        let ladder = &rows[0];
        assert_eq!(ladder.spoke, "ladder");
        assert_eq!(ladder.state, vox_eval::gate_state::GateState::Passed);
        assert!(ladder.applies);

        for row in &rows[1..] {
            assert_eq!(row.state, vox_eval::gate_state::GateState::NotRun);
            assert_eq!(row.reason, "no receipt");
            assert!(!row.applies);
        }
    }

    #[test]
    fn d04_collateral_is_not_spoke_evidence() {
        let temp = tempfile::tempdir().unwrap();
        let root = vox_corpus::training::contract::find_workspace_root().unwrap();
        let req = ladder_requirement(Some(&root));
        let rep = passing_collateral_report(&req);
        std::fs::write(
            temp.path().join("collateral_damage_report.json"),
            serde_json::to_string_pretty(&rep).unwrap(),
        )
        .unwrap();

        let ladder_hash = policy_blake3(Some(&root), "mens/config/eval-gates-ladder.yaml").unwrap();
        std::fs::write(
            temp.path().join("gate_receipt.json"),
            serde_json::to_string_pretty(&serde_json::json!({
                "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
                "policy_blake3": ladder_hash,
                "overall_passed": true,
                "gates": [
                    {"name": "throughput", "passed": true, "block": true, "message": "150 tok/s"},
                    {"name": "pass_at_k", "passed": true, "block": true, "message": "0.60 >= 0.50"}
                ]
            }))
            .unwrap(),
        )
        .unwrap();

        let rows = spoke_gate_rows(temp.path(), Some(&root));
        for row in &rows[1..] {
            assert_eq!(
                row.state,
                vox_eval::gate_state::GateState::NotRun,
                "spoke {} must be not_run despite passing collateral report",
                row.spoke
            );
        }
    }
}
