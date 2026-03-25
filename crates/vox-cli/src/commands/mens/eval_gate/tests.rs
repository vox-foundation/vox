use super::check_run::check_run;
use super::policy::EvalGatePolicy;

fn make_policy_with_per_context() -> EvalGatePolicy {
    let yaml = r#"
version: "1"
per_context:
  target:
    min_parse_rate: 0.80
    min_scope_compliance_rate: 0.95
    block: true
  meta:
    min_parse_rate: 0.30
    block: false
modal_mix:
  max_voice_fraction: 0.30
  block: false
"#;
    serde_yaml::from_str(yaml).expect("parse test policy")
}

#[test]
fn per_context_gate_passes_when_above_threshold() {
    let dir = tempfile::tempdir().expect("tempdir");

    // Write eval_results.json with target slice above threshold
    std::fs::write(
        dir.path().join("eval_results.json"),
        r#"{
                "vox_parse_rate": 0.9,
                "context_breakdown": {
                    "target": { "parse_rate": 0.90, "scope_compliance_rate": 0.97 },
                    "meta":   { "parse_rate": 0.35, "scope_compliance_rate": 0.60 }
                }
            }"#,
    )
    .unwrap();

    let results = check_run(dir.path(), &{
        let p = dir.path().join("policy.yaml");
        let yaml = r#"
version: "1"
per_context:
  target:
    min_parse_rate: 0.80
    min_scope_compliance_rate: 0.95
    block: true
  meta:
    min_parse_rate: 0.30
    block: false
"#;
        std::fs::write(&p, yaml).unwrap();
        p
    })
    .expect("check_run");

    let target_gate = results.iter().find(|r| r.name == "per_context[target]");
    assert!(
        target_gate.is_some(),
        "per_context[target] gate should be present"
    );
    assert!(
        target_gate.unwrap().passed,
        "target gate should pass at 90% parse"
    );

    let meta_gate = results.iter().find(|r| r.name == "per_context[meta]");
    assert!(
        meta_gate.is_some(),
        "per_context[meta] gate should be present"
    );
    assert!(
        meta_gate.unwrap().passed,
        "meta gate should pass at 35% parse"
    );
}

#[test]
fn per_context_gate_fails_blocking_when_below_threshold() {
    let dir = tempfile::tempdir().expect("tempdir");
    // target parse_rate below 0.80 threshold
    std::fs::write(
        dir.path().join("eval_results.json"),
        r#"{"vox_parse_rate":0.5,"context_breakdown":{"target":{"parse_rate":0.50,"scope_compliance_rate":0.99}}}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        "version: \"1\"\nper_context:\n  target:\n    min_parse_rate: 0.80\n    block: true\n",
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let gate = results
        .iter()
        .find(|r| r.name == "per_context[target]")
        .expect("gate present");
    assert!(!gate.passed, "target gate should fail below threshold");
    assert!(gate.block, "target gate should be blocking");
}

#[test]
fn modal_mix_gate_passes_when_voice_below_ceiling() {
    let _ = make_policy_with_per_context(); // verify policy parses
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("eval_results.json"),
        r#"{"modal_breakdown":{"text":90,"voice":5}}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        "version: \"1\"\nmodal_mix:\n  max_voice_fraction: 0.30\n  block: false\n",
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let gate = results.iter().find(|r| r.name == "modal_mix[voice]");
    assert!(gate.is_some(), "modal_mix[voice] gate present");
    assert!(
        gate.unwrap().passed,
        "5/95 = 5.3% voice < 30% ceiling should pass"
    );
}

#[test]
fn modal_mix_gate_warns_when_voice_exceeds_ceiling() {
    let dir = tempfile::tempdir().expect("tempdir");
    // 40% voice exceeds 30% ceiling
    std::fs::write(
        dir.path().join("eval_results.json"),
        r#"{"modal_breakdown":{"text":60,"voice":40}}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        "version: \"1\"\nmodal_mix:\n  max_voice_fraction: 0.30\n  block: false\n",
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let gate = results
        .iter()
        .find(|r| r.name == "modal_mix[voice]")
        .expect("gate present");
    assert!(!gate.passed, "40% voice > 30% ceiling should not pass");
    assert!(!gate.block, "warn-only gate should not block");
}

#[test]
fn policy_deserializes_per_context_and_modal_mix() {
    let policy = make_policy_with_per_context();
    assert!(policy.per_context.contains_key("target"));
    assert!(policy.per_context.contains_key("meta"));
    assert_eq!(policy.per_context["target"].min_parse_rate, 0.80);
    assert!(policy.per_context["target"].block);
    assert!(!policy.per_context["meta"].block);
    assert_eq!(policy.modal_mix.max_voice_fraction, 0.30);
    assert!(!policy.modal_mix.block);
}

#[test]
fn mcp_tool_schema_gate_passes_at_threshold() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("mcp_tool_schema_kpi.json"),
        r#"{"validation_enabled":true,"checks_total":100,"strict_passed":99,"strict_failed":1,"schema_compile_skipped":0,"strict_validity_rate":0.99}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.99
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "mcp_tool_schema")
        .expect("gate");
    assert!(g.passed, "{}", g.message);
}

#[test]
fn mcp_tool_schema_gate_fails_below_threshold() {
    let dir = tempfile::tempdir().expect("tempdir");
    std::fs::write(
        dir.path().join("mcp_tool_schema_kpi.json"),
        r#"{"validation_enabled":true,"checks_total":10,"strict_passed":5,"strict_failed":5,"strict_validity_rate":0.5}"#,
    )
    .unwrap();
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.99
  block: true
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    let g = results
        .iter()
        .find(|r| r.name == "mcp_tool_schema")
        .expect("gate");
    assert!(!g.passed);
}

#[test]
fn mcp_tool_schema_gate_skipped_when_inactive() {
    let dir = tempfile::tempdir().expect("tempdir");
    let policy_path = dir.path().join("policy.yaml");
    std::fs::write(
        &policy_path,
        r#"version: "1"
mcp_tool_schema:
  min_strict_validity_rate: 0.0
  block: false
"#,
    )
    .unwrap();
    let results = check_run(dir.path(), &policy_path).expect("check_run");
    assert!(
        !results.iter().any(|r| r.name == "mcp_tool_schema"),
        "inactive gate should not emit a row"
    );
}
