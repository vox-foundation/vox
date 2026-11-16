use super::*;
use crate::mens::gate_policy::passing_collateral_report;
use serde_json::json;
use vox_eval::reasoning_bench::SLICE_SIZE;

const PIN: &str = "Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc"; // gitleaks:allow

fn pin_for(size: &str) -> &'static str {
    match size {
        "0.8B" => "Qwen/Qwen3.5-0.8B@2fc06364715b967f1860aea9cf38778875588b17", // gitleaks:allow
        "0.6B" => "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca",   // gitleaks:allow
        "2B" => PIN,
        "4B" => "Qwen/Qwen3.5-4B@851bf6e806efd8d0a36b00ddf55e13ccb7b8cd0a", // gitleaks:allow
        "9B" => "Qwen/Qwen3.5-9B@c202236235762e1c871ad0ccb60c8ee5ba337b9a", // gitleaks:allow
        "27B" => "Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0", // gitleaks:allow
        _ => PIN,
    }
}

/// Write `files` (relative path, JSON) into a fresh run dir, optionally with weights, and load it.
fn load(size: &str, adapter: bool, files: &[(&str, Value)]) -> RunRecords {
    let root = tempfile::tempdir().unwrap();
    let pin = pin_for(size);
    let slug = crate::mens::hub_repo::local_run_dir_for_size(pin)
        .unwrap_or_else(|_| format!("mens/runs/qwen3-{}", size.to_ascii_lowercase()));
    let dir = root
        .path()
        .join(slug.strip_prefix("mens/runs/").unwrap_or(&slug));
    std::fs::create_dir_all(&dir).unwrap();
    if adapter {
        std::fs::write(dir.join("candle_qlora_adapter.safetensors"), b"").unwrap();
    }
    for (rel, v) in files {
        std::fs::write(dir.join(rel), v.to_string()).unwrap();
    }
    RunRecords::load(&dir, size, pin)
}

fn trained_record(size: &str) -> Value {
    json!({"schema": "vox_mens_train_record_v1", "status": "trained", "reason": null, "size": size})
}

fn passing_receipt() -> Value {
    let hash = crate::mens::gate_policy::policy_blake3(None, "mens/config/eval-gates-ladder.yaml")
        .unwrap();
    json!({
        "schema": vox_eval::gate_state::RECEIPT_SCHEMA,
        "policy": "mens/config/eval-gates-ladder.yaml",
        "policy_blake3": hash,
        "overall_passed": true,
        "gates": [
            {"name": "leakage", "passed": true, "block": true, "message": "no overlap detected"},
            {"name": "anti_stub", "passed": true, "block": true, "message": "anti_stub_pass=1.000 (min=0.920)"},
            {"name": "pass_at_k", "passed": true, "block": true, "message": "pass@1=0.327 (min=0.250)"},
            {"name": "review_recurrence", "passed": true, "block": false, "message": "ok"},
            {"name": "collateral_damage", "passed": true, "block": true, "message": "collateral pass"}
        ]
    })
}

#[test]
fn stale_pass_is_needs_re_gate() {
    // Shape of a 2B run with retired 32-item slices and no rule, pins or grader.
    let rec = load(
        "2B",
        true,
        &[
            ("train-record.json", trained_record("2B")),
            ("gate_receipt.json", passing_receipt()),
            (
                "collateral_damage_report.json",
                json!({
                    "status": "pass", "failed_on": null, "threshold": 0.05,
                    "reports": [{"benchmark": "gsm8k_think", "pre": 0.59375, "post": 0.875, "n": 32}],
                    "reasoning_slices": {"slice_size": 32}, "not_run": []
                }),
            ),
        ],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::NeedsReGate);
    assert!(fact.trained);
    let Some(Blocker::Stale {
        refusal,
        recorded_slice_n,
        current_slice_n,
    }) = fact.blocker
    else {
        panic!("expected a Stale blocker");
    };
    assert!(refusal.contains("reasoning_slices.rule"), "{refusal}");
    assert_eq!(recorded_slice_n, Some(32));
    assert_eq!(current_slice_n, SLICE_SIZE as u64);
}

#[test]
fn no_collateral_is_not_evaluated() {
    // Shape of mens/runs/qwen3.5-9b: trained, receipt written with missing gates, no collateral report.
    let rec = load(
        "9B",
        true,
        &[
            ("train-record.json", trained_record("9B")),
            (
                "gate_receipt.json",
                json!({
                    "policy": "mens/config/eval-gates-ladder.yaml", "overall_passed": false,
                    "gates": [
                        {"name": "leakage", "passed": true, "block": true, "message": "no overlap detected"},
                        {"name": "anti_stub", "passed": false, "block": true, "message": "eval_local_report.json missing — run `vox mens eval-local` first"},
                        {"name": "collateral_damage", "passed": false, "block": true, "message": "collateral_damage_report.json missing"}
                    ]
                }),
            ),
        ],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::NotEvaluated);
    assert!(fact.trained);
    let Some(Blocker::Missing(m)) = fact.blocker else {
        panic!("expected a Missing blocker");
    };
    assert!(
        m.contains(&"collateral_damage_report.json".to_string()),
        "{m:?}"
    );
    assert!(m.contains(&"anti_stub".to_string()), "{m:?}");
}

#[test]
fn no_adapter_is_not_trained() {
    // Shape of mens/runs/qwen3.8-27b: only a not_trained train-record.
    let rec = load(
        "27B",
        false,
        &[(
            "train-record.json",
            json!({"status": "not_trained", "reason": "above 9B staging (v7.7)", "size": "27B"}),
        )],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::NotEvaluated);
    assert!(!fact.trained);
    assert_eq!(
        fact.blocker,
        Some(Blocker::NoAdapter {
            record_reason: Some("above 9B staging (v7.7)".to_string())
        })
    );
}

#[test]
fn trained_record_without_weights_is_not_published() {
    let rec = load("4B", false, &[("train-record.json", trained_record("4B"))]);
    let fact = gate_of(&rec);
    assert!(!fact.trained);
    assert_eq!(fact.blocker, Some(Blocker::NotPublished));
}

#[test]
fn retired_rule_fail_reads_failed_and_quotes_refusal() {
    // Shape of mens/runs/qwen3-0.6b: collateral failed on gsm8k_think under the 32-item rule.
    let mut receipt = passing_receipt();
    receipt["overall_passed"] = json!(false);
    receipt["gates"][4] = json!({"name": "collateral_damage", "passed": false, "block": true,
        "message": "collateral_damage_report.json missing — run `vox mens eval-collateral-damage`"});
    let rec = load(
        "0.6B",
        true,
        &[
            ("train-record.json", trained_record("0.6B")),
            ("gate_receipt.json", receipt),
            (
                "collateral_damage_report.json",
                json!({
                    "status": "fail", "failed_on": "gsm8k_think", "threshold": 0.05,
                    "reports": [{"benchmark": "gsm8k_think", "pre": 0.5, "post": 0.46875, "n": 32}],
                    "reasoning_slices": {"slice_size": 32},
                    "not_run": [{"benchmark": "math500_think", "status": "not_run", "reason": "fail_fast_after_gsm8k_think"}]
                }),
            ),
        ],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::Failed);
    assert_eq!(
        fact.blocker,
        Some(Blocker::Failed {
            refusal: Some("the collateral report status is `fail` on `gsm8k_think`".to_string()),
            gates: vec![],
        })
    );
}

#[test]
fn current_pass_is_passed() {
    let req = load_requirement(None).unwrap();
    let rec = load(
        "2B",
        true,
        &[
            ("train-record.json", trained_record("2B")),
            ("gate_receipt.json", passing_receipt()),
            (
                "collateral_damage_report.json",
                passing_collateral_report(&req),
            ),
        ],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::Passed);
    assert!(fact.trained);
    assert_eq!(fact.blocker, None);
}

#[test]
fn missing_requirement_fails_closed() {
    let req = load_requirement(None).unwrap();
    let mut rec = load(
        "2B",
        true,
        &[
            ("train-record.json", trained_record("2B")),
            ("gate_receipt.json", passing_receipt()),
            (
                "collateral_damage_report.json",
                passing_collateral_report(&req),
            ),
        ],
    );
    rec.collateral_req = None;
    assert_eq!(gate_of(&rec).state, GateState::NeedsReGate);
}

#[test]
fn refused_receipt_gate_with_current_collateral_is_failed() {
    let req = load_requirement(None).unwrap();
    let mut receipt = passing_receipt();
    receipt["overall_passed"] = json!(false);
    receipt["gates"][2] = json!({"name": "pass_at_k", "passed": false, "block": true, "message": "pass@1=0.100 (min=0.250)"});
    let rec = load(
        "2B",
        true,
        &[
            ("train-record.json", trained_record("2B")),
            ("gate_receipt.json", receipt),
            (
                "collateral_damage_report.json",
                passing_collateral_report(&req),
            ),
        ],
    );
    let fact = gate_of(&rec);
    assert_eq!(fact.state, GateState::Failed);
    assert_eq!(
        fact.blocker,
        Some(Blocker::Failed {
            refusal: None,
            gates: vec![(
                "pass_at_k".to_string(),
                "pass@1=0.100 (min=0.250)".to_string()
            )],
        })
    );
}

#[test]
fn state_label_and_tag_are_paired() {
    let all = [
        (GateState::Passed, "passed", "gate-passed"),
        (
            GateState::NeedsReGate,
            "needs re-gate",
            "gate-needs-re-gate",
        ),
        (GateState::Failed, "failed", "gate-failed"),
        (
            GateState::NotEvaluated,
            "not evaluated",
            "gate-not-evaluated",
        ),
    ];
    for (s, label, tag) in all {
        assert_eq!((s.label(), s.tag()), (label, tag));
    }
}

mod checkpoints {
    use super::*;
    use crate::mens::hub_card::tests::support::{
        CURRENT_CKPT, LEGACY_CKPT, evidence_run, fixture_root,
    };

    const SIZE: &str = "4B";
    const PIN4: &str = "Qwen/Qwen3-4B@1cfa9a7208912126459214e8b04321603b3df60c";

    fn item_lines(rel: &str) -> usize {
        std::fs::read_to_string(fixture_root().join(rel))
            .unwrap()
            .lines()
            .count()
            - 1
    }

    /// Rewrite the adapter checkpoint of `dir` through `f(header_json, item_lines) -> file text`.
    fn rewrite_adapter_ckpt(dir: &Path, f: impl FnOnce(Value, &[&str]) -> String) {
        let p = dir.join("evidence/heldout_adapter_nothink.ckpt.jsonl");
        let text = std::fs::read_to_string(&p).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let header: Value = serde_json::from_str(lines[0]).unwrap();
        std::fs::write(&p, f(header, &lines[1..])).unwrap();
    }

    #[test]
    fn load_reads_checkpoints_with_current_protocol() {
        let run = evidence_run(CURRENT_CKPT);
        let rec = RunRecords::load(run.path(), SIZE, PIN4);
        assert_eq!(
            rec.heldout_adapter_ckpt.as_ref().map(Vec::len),
            Some(item_lines(CURRENT_CKPT))
        );
        assert_eq!(
            rec.heldout_base_ckpt.as_ref().map(Vec::len),
            Some(item_lines("evidence/heldout_base_nothink.ckpt.jsonl"))
        );

        // A different prompt protocol, a different schema and an empty file all read as absent.
        for bad in ["protocol", "schema", "empty"] {
            let run = evidence_run(CURRENT_CKPT);
            rewrite_adapter_ckpt(run.path(), |mut h, items| {
                match bad {
                    "protocol" => h["inputs"]["prompt_protocol"] = json!("qwen3_think128"),
                    "schema" => h["schema"] = json!("something_else.v1"),
                    _ => return String::new(),
                }
                format!("{h}\n{}\n", items.join("\n"))
            });
            let rec = RunRecords::load(run.path(), SIZE, PIN4);
            assert!(rec.heldout_adapter_ckpt.is_none(), "{bad}");
            assert!(rec.adapter_serving.is_none(), "{bad}");
        }

        // A malformed item line is skipped, the rest kept.
        let run = evidence_run(CURRENT_CKPT);
        rewrite_adapter_ckpt(run.path(), |h, items| {
            format!(
                "{h}\nnot json\n{{\"item\":\"x\",\"response\":\"nope\"}}\n{}\n",
                items.join("\n")
            )
        });
        let rec = RunRecords::load(run.path(), SIZE, PIN4);
        assert_eq!(
            rec.heldout_adapter_ckpt.map(|t| t.len()),
            Some(item_lines(CURRENT_CKPT))
        );
    }

    #[test]
    fn adapter_evidence_requires_current_serving() {
        let rec = |ckpt| RunRecords::load(evidence_run(ckpt).path(), SIZE, PIN4);
        assert_eq!(
            adapter_evidence(&rec(CURRENT_CKPT)),
            AdapterEvidence::Current
        );
        assert_eq!(
            adapter_evidence(&rec(LEGACY_CKPT)),
            AdapterEvidence::Stale { recorded: None }
        );

        let run = evidence_run(CURRENT_CKPT);
        rewrite_adapter_ckpt(run.path(), |mut h, items| {
            h["inputs"]["model"]["adapter_serving"] = json!("nf4_merged v1");
            format!("{h}\n{}\n", items.join("\n"))
        });
        assert_eq!(
            adapter_evidence(&RunRecords::load(run.path(), SIZE, PIN4)),
            AdapterEvidence::Stale {
                recorded: Some("nf4_merged v1".to_string())
            }
        );

        // No adapter ckpt file, or a header failing the schema check: the report alone is never accepted.
        let run = evidence_run(CURRENT_CKPT);
        std::fs::remove_file(
            run.path()
                .join("evidence/heldout_adapter_nothink.ckpt.jsonl"),
        )
        .unwrap();
        let loaded = RunRecords::load(run.path(), SIZE, PIN4);
        assert!(loaded.heldout_adapter.is_some(), "report is present");
        assert_eq!(adapter_evidence(&loaded), AdapterEvidence::NoCheckpoint);

        let run = evidence_run(CURRENT_CKPT);
        rewrite_adapter_ckpt(run.path(), |mut h, items| {
            h["schema"] = json!("something_else.v1");
            format!("{h}\n{}\n", items.join("\n"))
        });
        assert_eq!(
            adapter_evidence(&RunRecords::load(run.path(), SIZE, PIN4)),
            AdapterEvidence::NoCheckpoint
        );
    }

    #[test]
    fn load_keeps_legacy_serving_texts() {
        let rec = RunRecords::load(evidence_run(LEGACY_CKPT).path(), SIZE, PIN4);
        assert_eq!(
            rec.heldout_adapter_ckpt.map(|t| t.len()),
            Some(item_lines(LEGACY_CKPT))
        );
        assert_eq!(rec.adapter_serving, None);
    }

    #[test]
    fn has_complete_merged_model_detects_single_and_sharded_recombined_full() {
        let single = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(single.path().join("recombined_full")).unwrap();
        std::fs::write(
            single.path().join("recombined_full/model.safetensors"),
            b"weights",
        )
        .unwrap();
        let rec_single = RunRecords::load(single.path(), "4B", pin_for("4B"));
        assert!(rec_single.has_complete_merged_model);
        assert!(
            rec_single
                .merged_model_files
                .iter()
                .any(|f| f == "model.safetensors")
        );

        let sharded_index_only = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(sharded_index_only.path().join("recombined_full")).unwrap();
        std::fs::write(
            sharded_index_only
                .path()
                .join("recombined_full/model.safetensors.index.json"),
            b"{}",
        )
        .unwrap();
        let rec_index = RunRecords::load(sharded_index_only.path(), "9B", pin_for("9B"));
        assert!(rec_index.has_complete_merged_model);

        let sharded_full = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(sharded_full.path().join("recombined_full")).unwrap();
        std::fs::write(
            sharded_full
                .path()
                .join("recombined_full/model-00001-of-00002.safetensors"),
            b"shard1",
        )
        .unwrap();
        std::fs::write(
            sharded_full
                .path()
                .join("recombined_full/model.safetensors.index.json"),
            b"{}",
        )
        .unwrap();
        let rec_sharded = RunRecords::load(sharded_full.path(), "27B", pin_for("27B"));
        assert!(rec_sharded.has_complete_merged_model);
        assert!(
            rec_sharded
                .merged_model_files
                .iter()
                .any(|f| f == "model-00001-of-00002.safetensors")
        );
        assert!(
            rec_sharded
                .merged_model_files
                .iter()
                .any(|f| f == "model.safetensors.index.json")
        );
    }
}
