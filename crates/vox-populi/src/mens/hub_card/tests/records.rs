//! `RunRecords::load` and the compare-file lookup.

use serde_json::{Value, json};

use crate::mens::hub_card::{PROMPT_PROTOCOL, RunRecords};

const PIN_06: &str = "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"; // gitleaks:allow

fn heldout(passed: f64, total: u64, protocol: Option<&str>) -> Value {
    let mut v = json!({
        "max_tokens": 1024, "temperature": 0.0, "total": total,
        "passed_at_1": passed, "pass_rate_at_1": passed / total as f64,
        "category_stats": [
            {"category": "action", "passed_at_1": passed.min(2.0), "total": 2},
            {"category": "function", "passed_at_1": (passed - 2.0).max(0.0), "total": total - 2}
        ]
    });
    if let Some(p) = protocol {
        v["prompt_protocol"] = json!(p);
    }
    v
}

#[test]
fn hub_card_load_reads_records_and_rejects_retired_protocol() {
    let root = tempfile::tempdir().unwrap();
    let run = root.path().join("qwen3-0.6b");
    let ev = run.join("evidence");
    std::fs::create_dir_all(&ev).unwrap();
    std::fs::write(run.join("candle_qlora_adapter.safetensors"), b"w").unwrap();
    let w = |p: std::path::PathBuf, v: Value| std::fs::write(p, v.to_string()).unwrap();
    w(ev.join("heldout_base.json"), heldout(0.0, 52, None));
    w(
        ev.join("heldout_base_nothink.json"),
        heldout(6.0, 52, Some("think128")),
    );
    w(
        ev.join("heldout_adapter_nothink.json"),
        heldout(19.0, 52, Some(PROMPT_PROTOCOL)),
    );
    std::fs::write(run.join("gate_receipt.json"), "{}").unwrap();
    let cmp = root.path().join("reasoning_compare");
    std::fs::create_dir_all(&cmp).unwrap();
    w(
        cmp.join("Qwen3-0.6B-c1899de289a0.json"),
        json!({"status": "compared", "adapter": {"dir": "/x/mens/runs/qwen3-0.6b"}, "slices": {}}),
    );

    let rec = RunRecords::load(&run, "0.6B", PIN_06);
    assert!(rec.has_adapter);
    assert!(rec.heldout_base.is_none(), "wrong protocol is not a score");
    assert!(rec.heldout_adapter.is_some());
    assert!(
        rec.gate_receipt.is_none(),
        "placeholder `{{}}` counts as absent"
    );
    assert!(rec.compare.is_some());
    assert_eq!(
        rec.invalid_evidence,
        vec![
            "evidence/heldout_base_nothink.json",
            "evidence/heldout_base.json"
        ]
    );

    // A compare file for another adapter directory is not this run's evidence.
    w(
        cmp.join("Qwen3-0.6B-c1899de289a0.json"),
        json!({"status": "compared", "adapter": {"dir": "/x/mens/runs/320-smoke-0.6b"}, "slices": {}}),
    );
    assert!(RunRecords::load(&run, "0.6B", PIN_06).compare.is_none());
}

#[tokio::test]
async fn render_real_workspace_hub_cards_and_assets() {
    let root = std::path::Path::new("/Users/brbrainerd/dev/vox-mens-compare");
    if !root.join("mens/runs").is_dir() {
        return;
    }
    let rows = crate::mens::hub_sync::sync_all_sizes(
        Some(root),
        Some("brbrainerd"),
        false,
        false,
        false,
        false,
        None,
    )
    .await
    .unwrap();
    assert_eq!(rows.len(), 6);
    let cards_dir = root.join("mens/runs/hub_cards");
    assert!(
        cards_dir
            .join("vox-mens-index/assets/overview.svg")
            .is_file(),
        "missing overview.svg"
    );
    assert!(
        cards_dir
            .join("vox-mens-qwen3-4b/assets/scorecard.svg")
            .is_file(),
        "missing 4b scorecard.svg"
    );
}
