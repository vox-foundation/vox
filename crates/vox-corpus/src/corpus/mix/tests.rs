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
    let raw = r#"{"task_prompt":"x","tool_name":"t","arguments_json":"{}","result_json":"{}","success":true,"followup_text":"Ready."}"#;
    let out = normalize_training_jsonl_line(raw, Some("tool_trace")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["response"].as_str(), Some("Ready."));
}

#[test]
fn speech_to_code_normalizes_to_training_pair_shape() {
    let raw =
        r#"{"refined_transcript":"add a hello function","vox_code":"fn hello() { }","rating":5}"#;
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
fn speech_to_code_preserves_diagnostics_snapshot() {
    let raw = r#"{"refined_transcript":"fix typo","vox_code":"fn x() { }","diagnostics_snapshot":[{"message":"bad","code":"E001","severity":"error"}]}"#;
    let out = normalize_training_jsonl_line(raw, Some("speech_to_code")).expect("ok");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    let snap = v["diagnostics_snapshot"].as_array().expect("snapshot");
    assert_eq!(snap.len(), 1);
    assert_eq!(snap[0]["code"].as_str(), Some("E001"));
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
        None,
        MixRunOptions {
            strict: true,
            write_report: false,
        },
    )
    .expect_err("strict missing");
    let s = format!("{err:#}");
    assert!(s.contains("strict") || s.contains("missing"), "{s}");
}

#[test]
fn incremental_skip_works() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("mix.yaml");
    let src_path = dir.path().join("src.jsonl");
    let out_path = dir.path().join("out.jsonl");

    std::fs::write(&src_path, r#"{"prompt":"a","response":"b"}"#).unwrap();

    let p_src = src_path.to_string_lossy().replace('\\', "/");
    let p_out = out_path.to_string_lossy().replace('\\', "/");

    std::fs::write(
        &cfg_path,
        format!("sources:\n  - path: \"{p_src}\"\n    weight: 1\noutput: \"{p_out}\"\n"),
    )
    .unwrap();

    // First run — produces report and output
    run_mix_with_options(
        &cfg_path,
        None,
        MixRunOptions {
            strict: true,
            write_report: true,
        },
    )
    .expect("first run");
    assert!(out_path.is_file());
    assert!(dir.path().join("out.mix_report.json").is_file());

    // Measure time for second run (should be skip)
    let start = std::time::Instant::now();
    run_mix_with_options(
        &cfg_path,
        None,
        MixRunOptions {
            strict: true,
            write_report: true,
        },
    )
    .expect("second run");
    let elapsed = start.elapsed();

    // On a fast system, a skip should be < 10ms for a tiny file, but even on CI it should be very fast.
    assert!(
        elapsed.as_millis() < 500,
        "skip took {}ms",
        elapsed.as_millis()
    );
}

#[test]
fn mix_processes_repeats_and_multiple_sources() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("mix.yaml");
    let s1_path = dir.path().join("s1.jsonl");
    let s2_path = dir.path().join("s2.jsonl");
    let out_path = dir.path().join("out.jsonl");

    std::fs::write(
        &s1_path,
        "{\"lane\":\"vox_codegen\",\"prompt\":\"s1\",\"response\":\"r1\"}\n",
    )
    .unwrap();
    std::fs::write(
        &s2_path,
        "{\"lane\":\"vox_codegen\",\"prompt\":\"s2\",\"response\":\"r2\"}\n",
    )
    .unwrap();

    let p1 = s1_path.to_string_lossy().replace('\\', "/");
    let p2 = s2_path.to_string_lossy().replace('\\', "/");
    let p_out = out_path.to_string_lossy().replace('\\', "/");

    std::fs::write(
        &cfg_path,
        format!("sources:\n  - path: \"{p1}\"\n    weight: 2\n  - path: \"{p2}\"\n    weight: 1\noutput: \"{p_out}\"\ninclude_lanes: [\"vox_codegen\"]\n"),
    ).unwrap();

    run_mix_with_options(
        &cfg_path,
        None,
        MixRunOptions {
            strict: true,
            write_report: true,
        },
    )
    .expect("run");

    let mixed = std::fs::read_to_string(&out_path).unwrap();
    let lines: Vec<&str> = mixed.lines().filter(|l| !l.is_empty()).collect();

    // s1 repeated 2 times, s2 once = 3 lines
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("s1"));
    assert!(lines[1].contains("s1"));
    assert!(lines[2].contains("s2"));
}
