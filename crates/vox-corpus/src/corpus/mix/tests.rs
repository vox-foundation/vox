use super::*;

#[test]
fn asr_refine_normalizes_to_training_pair_shape() {
    let raw = r#"{"noisy_text":"hello  wrld","corrected_text":"hello world","rating":4}"#;
    let out = normalize_training_jsonl_line(raw, Some("asr_refine")).expect("ok");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let prompt = v["prompt"].as_str().unwrap();
    assert!(prompt.contains("hello  wrld"));
    assert!(prompt.starts_with("Correct the following noisy"));
    assert_eq!(v["response"].as_str(), Some("hello world"));
    assert_eq!(v["rating"].as_u64(), Some(4));
    assert_eq!(v["category"].as_str(), Some("asr_refine"));
}

#[test]
fn passthrough_without_format() {
    let raw = r#"{"prompt":"a","response":"b"}"#;
    let out = normalize_training_jsonl_line(raw, None).unwrap();
    assert_eq!(out, raw);
}

#[test]
fn tool_trace_normalizes_to_training_pair_shape() {
    let raw = r#"{"task_prompt":"Run fmt","tool_name":"shell","arguments_json":"{\"cmd\":\"cargo fmt\"}","result_json":"{\"ok\":true}","success":true}"#;
    let out = normalize_training_jsonl_line(raw, Some("tool_trace")).expect("ok");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(
        v["prompt"]
            .as_str()
            .unwrap()
            .contains("[vox_tool_supervision]")
    );
    assert!(v["prompt"].as_str().unwrap().contains("Run fmt"));
    let resp = v["response"].as_str().unwrap();
    assert!(resp.contains("shell"));
    assert!(resp.contains("cargo fmt"));
    assert_eq!(v["category"].as_str(), Some("tool_trace"));
}

#[test]
fn tool_trace_uses_followup_when_present() {
    let raw = r#"{"task_prompt":"x","tool_name":"t","arguments_json":"{}","result_json":"{}","success":true,"followup_text":"Done."}"#;
    let out = normalize_training_jsonl_line(raw, Some("tool_trace")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["response"].as_str(), Some("Done."));
}

#[test]
fn speech_to_code_normalizes_to_training_pair_shape() {
    let raw = r#"{"refined_transcript":"add a hello function","vox_code":"fn hello() { }","rating":5}"#;
    let out = normalize_training_jsonl_line(raw, Some("speech_to_code")).expect("ok");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let prompt = v["prompt"].as_str().unwrap();
    assert!(prompt.contains("add a hello function"));
    assert!(prompt.starts_with("Given the following spoken"));
    assert_eq!(v["response"].as_str(), Some("fn hello() { }"));
    assert_eq!(v["rating"].as_u64(), Some(5));
    assert_eq!(v["category"].as_str(), Some("speech_to_code"));
}

#[test]
fn strict_rejects_missing_required_source() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("mix.yaml");
    let absent = dir.path().join("nope.jsonl");
    let out_j = dir.path().join("out.jsonl");
    let p_abs = absent.to_string_lossy().replace('\\', "/");
    let p_out = out_j.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &cfg_path,
        format!("sources:\n  - path: \"{p_abs}\"\n    weight: 1\noutput: \"{p_out}\"\n"),
    )
    .unwrap();
    let err = run_mix_with_options(
        &cfg_path,
        MixRunOptions {
            strict: true,
            write_report: false,
        },
    )
    .expect_err("strict missing");
    let s = format!("{err:#}");
    assert!(s.contains("strict") || s.contains("missing"), "{s}");
}
