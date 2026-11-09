use serial_test::serial;
use std::path::Path;

use super::check_run::check_run;
use super::policy::EvalGatePolicy;

#[test]
fn pass_at_k_gate_passes_when_above_thresholds() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("benchmark_passatk.json"),
        r#"{"pass_rate_at_1":0.71,"pass_rate_at_k":0.88,"k":4}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"pass_at_k:
  min_pass_rate_at_1: 0.70
  min_pass_rate_at_k: 0.85
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "pass_at_k")
        .expect("gate");
    assert!(g.passed, "{}", g.message);
}

#[test]
fn anti_stub_gate_passes_when_eval_local_report_meets_thresholds() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("eval_local_report.json"),
        r#"{
            "anti_stub_task_success": 0.95,
            "placeholder_event_rate": 0.03,
            "trivial_placeholder_event_rate": 0.02,
            "construct_richness_mean": 0.45
        }"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  max_placeholder_event_rate: 0.08
  max_trivial_placeholder_event_rate: 0.08
  min_construct_richness_mean: 0.20
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "anti_stub")
        .expect("anti_stub gate present");
    assert!(
        g.passed,
        "should pass: all metrics above thresholds. msg={}",
        g.message
    );
    assert!(g.block, "block flag from policy");
}

#[test]
fn anti_stub_gate_fails_blocking_when_pass_rate_below_threshold() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("eval_local_report.json"),
        r#"{
            "anti_stub_task_success": 0.80,
            "placeholder_event_rate": 0.03,
            "trivial_placeholder_event_rate": 0.02,
            "construct_richness_mean": 0.45
        }"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  max_placeholder_event_rate: 0.08
  max_trivial_placeholder_event_rate: 0.08
  min_construct_richness_mean: 0.20
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "anti_stub")
        .expect("anti_stub gate present");
    assert!(!g.passed, "0.80 < 0.92 should fail");
    assert!(g.block, "block: true in policy");
}

#[test]
fn anti_stub_gate_warn_only_when_eval_local_report_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  max_placeholder_event_rate: 0.08
  max_trivial_placeholder_event_rate: 0.08
  min_construct_richness_mean: 0.20
  block: false
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "anti_stub")
        .expect("anti_stub gate present");
    assert!(!g.passed, "missing file → gate fails");
    assert!(!g.block, "block: false → warning, not hard-block");
}

#[test]
fn anti_stub_gate_hard_blocks_when_full_policy_and_eval_local_report_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "anti_stub")
        .expect("anti_stub gate present");
    assert!(!g.passed, "missing file → gate fails");
    assert!(g.block, "block: true → hard-block");
    assert!(
        g.message.contains("eval_local_report.json"),
        "message should name the expected file"
    );
}

#[test]
fn run_eval_gate_writes_gate_receipt_json_on_pass() {
    use super::run_gate::run_eval_gate;
    let dir = tempfile::tempdir().expect("tempdir");

    std::fs::write(
        dir.path().join("eval_local_report.json"),
        r#"{"anti_stub_task_success":0.95,"placeholder_event_rate":0.02,"trivial_placeholder_event_rate":0.01,"construct_richness_mean":0.40}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  max_placeholder_event_rate: 0.08
  max_trivial_placeholder_event_rate: 0.08
  min_construct_richness_mean: 0.20
  block: true
"#,
    )
    .unwrap();

    let code = run_eval_gate(dir.path().to_path_buf(), Some(policy_path)).expect("run");
    assert_eq!(code, 0, "gate should pass");

    let receipt_path = dir.path().join("gate_receipt.json");
    assert!(receipt_path.exists(), "gate_receipt.json must be written");
    let text = std::fs::read_to_string(&receipt_path).unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["schema"], vox_eval::gate_state::RECEIPT_SCHEMA);
    assert_eq!(
        v["source_commit"],
        env!("VOX_GIT_HASH"),
        "a receipt names the commit of the binary that judged it"
    );
    assert!(
        v["overall_passed"].as_bool().unwrap_or(false),
        "overall_passed should be true"
    );
    let gates = v["gates"].as_array().expect("gates array");
    assert!(!gates.is_empty(), "gates list should not be empty");
}

#[test]
fn run_eval_gate_writes_gate_receipt_json_on_fail() {
    use super::run_gate::run_eval_gate;
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("eval_local_report.json"),
        r#"{"anti_stub_task_success":0.50}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"anti_stub:
  min_pass_rate: 0.92
  block: true
"#,
    )
    .unwrap();
    let code = run_eval_gate(dir.path().to_path_buf(), Some(policy_path)).expect("run");
    assert_eq!(code, 1, "gate should fail");
    let v: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("gate_receipt.json")).unwrap(),
    )
    .unwrap();
    assert!(
        !v["overall_passed"].as_bool().unwrap_or(true),
        "overall_passed false on fail"
    );
}

#[test]
fn pass_at_k_gate_fails_on_regression_drop() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("benchmark_passatk.json"),
        r#"{"pass_rate_at_1":0.50,"pass_rate_at_k":0.70}"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("baseline_passatk.json"),
        r#"{"pass_rate_at_1":0.62,"pass_rate_at_k":0.82}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"pass_at_k:
  max_regression_drop: 0.05
  baseline_file: baseline_passatk.json
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "pass_at_k")
        .expect("gate");
    assert!(!g.passed, "{}", g.message);
}

#[test]
fn pass_at_k_when_k_is_one_collapses_min_k_to_pass_at_1() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("benchmark_passatk.json"),
        r#"{"schema":"vox_mens_benchmark_passatk_v1","n":1,"k":1,"pass_rate_at_1":0.30,"pass_rate_at_k":0.30,"total":52}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"pass_at_k:
  min_pass_rate_at_1: 0.25
  min_pass_rate_at_k: 0.50
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "pass_at_k")
        .expect("pass_at_k gate");
    assert!(
        g.passed,
        "k=1 must not require the k>1 pass@k floor: {}",
        g.message
    );
    assert!(
        g.message.contains("k=1"),
        "message should note the collapse: {}",
        g.message
    );
}

#[test]
fn pass_at_k_when_k_gt_one_still_requires_pass_at_k_floor() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("benchmark_passatk.json"),
        r#"{"schema":"vox_mens_benchmark_passatk_v1","n":5,"k":5,"pass_rate_at_1":0.30,"pass_rate_at_k":0.30,"total":52}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"pass_at_k:
  min_pass_rate_at_1: 0.25
  min_pass_rate_at_k: 0.50
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "pass_at_k")
        .expect("pass_at_k gate");
    assert!(
        !g.passed,
        "k>1 must still enforce min_pass_rate_at_k: {}",
        g.message
    );
}

#[test]
fn pass_at_k_gate_hard_fails_when_baseline_configured_but_missing() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("benchmark_passatk.json"),
        r#"{"pass_rate_at_1":0.90,"pass_rate_at_k":0.95}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"pass_at_k:
  min_pass_rate_at_1: 0.10
  min_pass_rate_at_k: 0.10
  max_regression_drop: 0.05
  baseline_file: baseline_passatk.json
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "pass_at_k")
        .expect("gate");
    assert!(
        !g.passed,
        "a configured-but-missing baseline must hard-fail, not skip: {}",
        g.message
    );
    assert!(
        g.message.to_lowercase().contains("baseline"),
        "message should explain the missing baseline: {}",
        g.message
    );
}

#[test]
fn supervised_ratio_reads_nested_payload() {
    let dir = tempfile::tempdir().unwrap();
    let telem_path = dir.path().join("telemetry.jsonl");
    std::fs::write(
        &telem_path,
        r#"{"event":"step","payload":{"supervised_ratio_pct":2.70}}
"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"supervised_ratio:
  min_pct: 10.0
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).unwrap();
    let row = results
        .iter()
        .find(|r| r.name == "supervised_ratio")
        .expect("supervised_ratio row");
    assert!(!row.passed);
    assert!(row.message.contains("2.7%"), "got: {}", row.message);

    // Test with passing value
    std::fs::write(
        &telem_path,
        r#"{"event":"step","payload":{"supervised_ratio_pct":12.0}}
"#,
    )
    .unwrap();
    let results2 = check_run(dir.path(), &policy_path).unwrap();
    let row2 = results2
        .iter()
        .find(|r| r.name == "supervised_ratio")
        .expect("supervised_ratio row");
    assert!(row2.passed);
    assert!(row2.message.contains("12.0%"), "got: {}", row2.message);

    // Test with no step lines
    std::fs::write(
        &telem_path,
        r#"{"event":"train_start","payload":{}}
"#,
    )
    .unwrap();
    let results3 = check_run(dir.path(), &policy_path).unwrap();
    let row3 = results3
        .iter()
        .find(|r| r.name == "supervised_ratio")
        .expect("supervised_ratio row");
    assert!(!row3.passed);
    assert!(row3.message.contains("missing"), "got: {}", row3.message);
}

// ---------------------------------------------------------------------------
// Harness safety eval — mocked executor
// ---------------------------------------------------------------------------

struct MockToolExecutor {
    calls: std::sync::Mutex<Vec<String>>,
}

impl MockToolExecutor {
    fn new() -> Self {
        MockToolExecutor {
            calls: std::sync::Mutex::new(Vec::new()),
        }
    }

    fn dispatch(&self, tool_name: &str, _args: &serde_json::Value) -> Result<String, String> {
        self.calls.lock().unwrap().push(tool_name.to_string());
        const SIDE_EFFECTING: &[&str] = &["write_file", "shell_exec", "git_push"];
        if SIDE_EFFECTING.contains(&tool_name) {
            Err(format!(
                "SAFETY VIOLATION: side-effecting tool '{}' dispatched during eval",
                tool_name
            ))
        } else {
            Ok(format!("mock_result_for_{}", tool_name))
        }
    }

    fn recorded_calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

#[test]
fn safety_eval_no_side_effects_dispatched() {
    let executor = MockToolExecutor::new();
    let read_only_tools = ["read_file", "search_code", "list_branches"];
    for tool in &read_only_tools {
        let args = serde_json::json!({"path": "/tmp/test"});
        let result = executor.dispatch(tool, &args);
        assert!(result.is_ok(), "read-only tool should not fail");
    }

    let calls = executor.recorded_calls();
    assert_eq!(calls.len(), 3);
    const SIDE_EFFECTING: &[&str] = &["write_file", "shell_exec", "git_push"];
    for tool in &calls {
        assert!(!SIDE_EFFECTING.contains(&tool.as_str()));
    }
}

#[test]
fn safety_eval_mock_panics_on_side_effecting_tool() {
    let executor = MockToolExecutor::new();
    let result = executor.dispatch(
        "write_file",
        &serde_json::json!({"path": "/tmp/x", "content": "y"}),
    );
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("SAFETY VIOLATION"));
}

#[test]
fn safety_eval_side_effecting_tools_are_never_called_in_eval_path() {
    let executor = MockToolExecutor::new();
    let eval_path_tools = ["read_file", "search_code", "grep_pattern", "list_directory"];
    let mut violations: Vec<String> = Vec::new();
    for tool in &eval_path_tools {
        match executor.dispatch(tool, &serde_json::Value::Null) {
            Ok(_) => {}
            Err(e) => violations.push(e),
        }
    }
    assert!(violations.is_empty(), "violations: {:?}", violations);
}

// ---------------------------------------------------------------------------
// Leakage precondition
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn check_run_always_includes_a_leakage_gate() {
    let dir = tempfile::tempdir().expect("tempdir");
    let results = check_run(dir.path(), &{
        let p = dir.path().join("policy.yaml");
        std::fs::write(&p, "# empty policy\n").unwrap();
        p
    })
    .expect("check_run");

    let g = results
        .iter()
        .find(|r| r.name == "leakage")
        .expect("leakage gate must always be present");
    assert!(g.block);
    assert!(g.passed);
}

#[test]
#[serial]
fn check_leakage_precondition_reports_not_applicable_outside_workspace() {
    let original_cwd = std::env::current_dir().expect("get cwd");
    let outside = tempfile::tempdir().expect("tempdir outside");
    std::env::set_current_dir(outside.path()).expect("chdir outside");

    let mut results = Vec::new();
    super::check_run::check_leakage_precondition(&mut results);
    std::env::set_current_dir(&original_cwd).expect("restore cwd");

    assert_eq!(results.len(), 1);
    let g = &results[0];
    assert_eq!(g.name, "leakage");
    assert!(g.passed);
    assert!(!g.block);
}

// ---------------------------------------------------------------------------
// Collateral damage gate
// ---------------------------------------------------------------------------

fn paired_pass_report() -> serde_json::Value {
    use vox_eval::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
    use vox_eval::reasoning_slice::{eval_reasoning_slice, pooled_accuracy, reasoning_slices_json};

    let flags = |n: usize, f: &dyn Fn(usize) -> bool| -> Vec<(String, bool)> {
        (0..n).map(|i| (format!("item_{i}"), f(i))).collect()
    };
    let base = flags(SLICE_SIZE, &|i| i < 16);
    let adapter = flags(SLICE_SIZE, &|i| i < 15);
    let all_true = flags(SLICE_SIZE, &|_| true);
    let verdicts = SLICE_NAMES
        .iter()
        .map(|s| eval_reasoning_slice(s, (&base, &adapter), (&all_true, &all_true), 0.05).unwrap())
        .collect::<Vec<_>>();
    let pooled = pooled_accuracy(&verdicts, 0.05).unwrap();

    let mut slices_map = serde_json::Map::new();
    for s in SLICE_NAMES {
        slices_map.insert(s.to_string(), serde_json::json!(format!("h-{s}")));
    }

    serde_json::json!({
        "status": "pass",
        "failed_on": null,
        "fail_fast": true,
        "reasoning_slices": reasoning_slices_json(&verdicts, Some(&pooled), 0.05),
        "grader": vox_eval::qwen_reasoning::GRADER_ID,
        "slice_blake3": slices_map,
    })
}

fn collateral_gate(report: &str, policy: &str) -> super::GateResult {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(dir.path().join("collateral_damage_report.json"), report).unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(&policy_path, policy).unwrap();
    check_run(dir.path(), &policy_path)
        .expect("check_run")
        .into_iter()
        .find(|r| r.name == "collateral_damage")
        .expect("collateral_damage gate")
}

fn alpha_policy() -> String {
    let mut s = "collateral_damage:\n  block: true\n  alpha: 0.05\n  slices:\n".to_string();
    for name in vox_eval::reasoning_bench::SLICE_NAMES {
        s.push_str(&format!(
            "    {name}: {{ blake3: h-{name}, prefix_n: 32, prefix_blake3: p-{name} }}\n"
        ));
    }
    s
}

#[test]
fn collateral_damage_gate_passes_a_paired_report_at_the_policy_alpha() {
    let g = collateral_gate(&paired_pass_report().to_string(), &alpha_policy());
    assert!(g.passed, "{}", g.message);
    for word in ["missing", "skipped", "not applicable"] {
        assert!(!g.message.contains(word), "{}", g.message);
    }
}

#[test]
fn collateral_damage_gate_does_not_trust_a_bare_status_pass() {
    let g = collateral_gate(
        r#"{"status":"pass","failed_on":null,"fail_fast":true}"#,
        &alpha_policy(),
    );
    assert!(!g.passed, "{}", g.message);
    assert!(g.message.contains("reasoning_slices"), "{}", g.message);
}

#[test]
fn collateral_damage_gate_recomputes_a_stored_pass_over_a_regression() {
    let mut r = paired_pass_report();
    *r.pointer_mut("/reasoning_slices/benches/vox_think/accuracy/losses")
        .unwrap() = serde_json::json!(6);
    let g = collateral_gate(&r.to_string(), &alpha_policy());
    assert!(!g.passed, "{}", g.message);
    assert!(g.message.contains("vox_think regressed"), "{}", g.message);
}

#[test]
fn collateral_damage_gate_refuses_a_pass_scored_on_another_item_set() {
    let mut r = paired_pass_report();
    r["slice_blake3"]["math500_think"] = serde_json::json!("h-the-32-item-slice");
    let g = collateral_gate(&r.to_string(), &alpha_policy());
    assert!(!g.passed, "{}", g.message);
    assert!(
        g.message.contains("math500_think was scored on")
            && g.message.contains("h-the-32-item-slice"),
        "{}",
        g.message
    );
    r.as_object_mut().unwrap().remove("slice_blake3");
    let g = collateral_gate(&r.to_string(), &alpha_policy());
    assert!(!g.passed, "{}", g.message);
    assert!(
        g.message.contains("slice_blake3") || g.message.contains("missing slice"),
        "{}",
        g.message
    );
}

#[test]
fn collateral_damage_gate_refuses_a_pass_when_the_policy_states_no_alpha() {
    let g = collateral_gate(
        &paired_pass_report().to_string(),
        "collateral_damage:\n  block: true\n",
    );
    assert!(!g.passed, "{}", g.message);
    assert!(g.message.contains("alpha"), "{}", g.message);
}

#[test]
fn ladder_policy_loads_and_enables_collateral_block() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../mens/config/eval-gates-ladder.yaml");
    let policy = super::policy::load_policy(&path).expect("load ladder policy");
    assert!(policy.collateral_damage.block);
    assert!(policy.anti_stub.block);
    assert!(policy.pass_at_k.block);
}

// ---------------------------------------------------------------------------
// Inventory & unknown fields tests (SPOKE-02)
// ---------------------------------------------------------------------------

#[test]
fn gate_keys_inventory() {
    // SPOKE-02: Mapping every remaining gate key to its producer test
    const INVENTORY: &[(&str, &str)] = &[
        (
            "pass_at_k",
            "commands::mens::eval_gate::tests::pass_at_k_gate_passes_when_above_thresholds",
        ),
        (
            "anti_stub",
            "commands::mens::eval_gate::tests::anti_stub_gate_passes_when_eval_local_report_meets_thresholds",
        ),
        (
            "supervised_ratio",
            "commands::mens::eval_gate::tests::supervised_ratio_reads_nested_payload",
        ),
        (
            "collateral_damage",
            "commands::mens::eval_gate::tests::collateral_damage_gate_passes_a_paired_report_at_the_policy_alpha",
        ),
    ];

    let check_file = |rel_path: &str, expected_keys: &[&str]| {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../")
            .join(rel_path);
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {rel_path}: {e}"));
        let val: serde_json::Value = serde_yaml::from_str(&content)
            .unwrap_or_else(|e| panic!("failed to parse {rel_path}: {e}"));
        let map = val
            .as_object()
            .unwrap_or_else(|| panic!("expected top-level map in {rel_path}"));
        let file_keys: std::collections::BTreeSet<&str> = map.keys().map(|k| k.as_str()).collect();
        let expected_set: std::collections::BTreeSet<&str> =
            expected_keys.iter().copied().collect();
        assert_eq!(
            file_keys, expected_set,
            "gate keys inventory mismatch for {rel_path}"
        );
        for k in &file_keys {
            assert!(
                INVENTORY.iter().any(|(inv_k, _)| inv_k == k),
                "key '{k}' in {rel_path} not found in GATE_KEYS_INVENTORY table"
            );
        }
    };

    check_file(
        "mens/config/eval-gates-ladder.yaml",
        &[
            "pass_at_k",
            "anti_stub",
            "supervised_ratio",
            "collateral_damage",
        ],
    );
    check_file(
        "mens/config/eval-gates.yaml",
        &["pass_at_k", "anti_stub", "supervised_ratio"],
    );
    check_file("mens/config/eval-gates-rust.yaml", &["supervised_ratio"]);
    check_file("mens/config/eval-gates-agents.yaml", &["supervised_ratio"]);
}

#[test]
fn deleted_gate_key_fails_to_load() {
    let yaml = r#"
truncation:
  max_fraction: 0.1
  block: true
"#;
    let err = serde_yaml::from_str::<EvalGatePolicy>(yaml);
    assert!(
        err.is_err(),
        "truncation: should fail to load with unknown field"
    );
    let err_str = err.unwrap_err().to_string();
    assert!(
        err_str.contains("unknown field `truncation`"),
        "expected unknown field error, got: {err_str}"
    );
}
