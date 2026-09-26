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
fn mix_emits_once_with_weight_stamp_by_default() {
    // Default semantics (post-2026-05-23 fix): weight is stamped onto each
    // row as `mix_weight` and each row is emitted exactly once. Previously
    // weight=2 produced 2 literal copies, which destroyed uniqueness for
    // SFT corpora (audit measured 99.2% duplication on 6× weight).
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

    // New semantics: each source emits once → 2 lines, not 3.
    assert_eq!(lines.len(), 2, "expected emit-once: got {lines:#?}");
    let l0: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    let l1: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(l0["prompt"].as_str(), Some("s1"));
    assert_eq!(l1["prompt"].as_str(), Some("s2"));
    // weight=2 is stamped; weight=1 (default) is omitted to keep diffs minimal.
    assert_eq!(l0["mix_weight"].as_f64(), Some(2.0));
    assert!(
        l1.get("mix_weight").is_none() || l1["mix_weight"].as_f64() == Some(1.0),
        "weight=1 should be no-op or 1.0, got {}",
        l1["mix_weight"]
    );
}

#[test]
fn mix_physical_repeats_opt_in_restores_legacy_duplication() {
    // Setting `physical_repeats: true` per-source restores the pre-fix
    // behavior: weight=N emits each row N times. Kept as an escape hatch
    // for downstream consumers that don't honor `mix_weight`.
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg_path = dir.path().join("mix.yaml");
    let s1_path = dir.path().join("s1.jsonl");
    let out_path = dir.path().join("out.jsonl");
    std::fs::write(
        &s1_path,
        "{\"lane\":\"vox_codegen\",\"prompt\":\"s1\",\"response\":\"r1\"}\n",
    )
    .unwrap();
    let p1 = s1_path.to_string_lossy().replace('\\', "/");
    let p_out = out_path.to_string_lossy().replace('\\', "/");
    std::fs::write(
        &cfg_path,
        format!("sources:\n  - path: \"{p1}\"\n    weight: 3\n    physical_repeats: true\noutput: \"{p_out}\"\ninclude_lanes: [\"vox_codegen\"]\n"),
    )
    .unwrap();
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
    let n = mixed.lines().filter(|l| !l.is_empty()).count();
    assert_eq!(n, 3, "physical_repeats:true with weight=3 → 3 lines");
}

#[test]
fn lane_override_promotes_documentation_category_above_generic_codegen() {
    // Regression test for 2026-05-23 audit: validated_mixed.jsonl tags
    // every row `lane: vox_codegen` regardless of its category, so an
    // `include_lanes: [vox_docs_qa]` filter never emitted documentation
    // rows. The fix: when the row's lane is the generic catch-all
    // (`vox_codegen`) AND the category-derived lane is more specific
    // (e.g. `documentation` → `vox_docs_qa`), promote the specific lane.
    let raw = r#"{"lane":"vox_codegen","category":"documentation","prompt":"p","response":"r"}"#;
    let (out, lane) = enrich_lane_metadata(raw).expect("enrich ok");
    assert_eq!(lane, "vox_docs_qa", "specific category should win");
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["lane"].as_str(), Some("vox_docs_qa"));
}

#[test]
fn lane_override_preserves_explicit_non_default_lane() {
    // If the row already has an explicit non-default lane (e.g. an
    // upstream tagger really meant `vox_tooling`), trust it even if the
    // category would also imply something else.
    let raw = r#"{"lane":"vox_tooling","category":"documentation","prompt":"p","response":"r"}"#;
    let (_out, lane) = enrich_lane_metadata(raw).expect("enrich ok");
    assert_eq!(lane, "vox_tooling", "explicit specific lane wins");
}

#[test]
fn stamp_mix_weight_skips_default_weight() {
    let raw = r#"{"prompt":"p","response":"r"}"#;
    // weight=1.0 → no-op, returns the input unchanged.
    assert_eq!(stamp_mix_weight(raw, 1.0).unwrap(), raw);
    // weight=2.5 → adds the field.
    let out = stamp_mix_weight(raw, 2.5).unwrap();
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["mix_weight"].as_f64(), Some(2.5));
}

#[test]
fn test_max_lines_hard_cap() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let mut src = NamedTempFile::new().unwrap();
    for i in 0..100 {
        writeln!(
            src,
            r#"{{"prompt":"q{}","response":"a{}","lane":"vox_codegen"}}"#,
            i, i
        )
        .unwrap();
    }

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("mix.yaml");
    let out_path = dir.path().join("out.jsonl");
    let p_src = src.path().to_str().unwrap().replace('\\', "/");
    let p_out = out_path.to_str().unwrap().replace('\\', "/");

    let yaml = format!(
        "sources:\n  - path: \"{p_src}\"\n    weight: 1.0\n    max_lines: 10\noutput: \"{p_out}\"\n"
    );
    std::fs::write(&cfg_path, yaml).unwrap();

    let opts = super::MixRunOptions {
        strict: false,
        write_report: false,
    };
    super::run_mix_with_options(&cfg_path, None, opts).unwrap();

    let emitted = std::fs::read_to_string(out_path).unwrap();
    let count = emitted.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(
        count, 10,
        "max_lines cap should emit exactly 10, got {}",
        count
    );
}

#[test]
fn test_dedup_skips_duplicate_rows() {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let dup = r#"{"prompt":"same question","response":"same answer","lane":"vox_codegen"}"#;

    let mut src_a = NamedTempFile::new().unwrap();
    writeln!(src_a, "{}", dup).unwrap();
    writeln!(
        src_a,
        r#"{{"prompt":"unique a","response":"resp a","lane":"vox_codegen"}}"#
    )
    .unwrap();

    let mut src_b = NamedTempFile::new().unwrap();
    writeln!(src_b, "{}", dup).unwrap();
    writeln!(
        src_b,
        r#"{{"prompt":"unique b","response":"resp b","lane":"vox_codegen"}}"#
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("mix.yaml");
    let out_path = dir.path().join("out.jsonl");
    let p_a = src_a.path().to_str().unwrap().replace('\\', "/");
    let p_b = src_b.path().to_str().unwrap().replace('\\', "/");
    let p_out = out_path.to_str().unwrap().replace('\\', "/");

    let yaml = format!(
        "sources:\n  - path: \"{p_a}\"\n    weight: 1.0\n  - path: \"{p_b}\"\n    weight: 1.0\noutput: \"{p_out}\"\ndedup: true\n"
    );
    std::fs::write(&cfg_path, yaml).unwrap();

    let opts = super::MixRunOptions {
        strict: false,
        write_report: false,
    };
    super::run_mix_with_options(&cfg_path, None, opts).unwrap();

    let emitted = std::fs::read_to_string(out_path).unwrap();
    let count = emitted.lines().filter(|l| !l.trim().is_empty()).count();
    assert_eq!(count, 3, "dedup should emit 3 unique rows, got {}", count);
}

#[test]
fn dedup_key_distinguishes_messages_only_rows() {
    let a = serde_json::json!({"messages":[{"role":"user","content":"a"},{"role":"assistant","content":"1"}]});
    let b = serde_json::json!({"messages":[{"role":"user","content":"b"},{"role":"assistant","content":"2"}]});
    assert_ne!(super::dedup_key(&a), super::dedup_key(&b));
    let p = serde_json::json!({"prompt":"x","response":"y"});
    let q = serde_json::json!({"instruction":"x","output":"y"});
    assert_eq!(
        super::dedup_key(&p),
        super::dedup_key(&q),
        "aliases are the same row"
    );
    let e1 = serde_json::json!({"instruction":"x","input":"e1","output":"y"});
    let e2 = serde_json::json!({"instruction":"x","input":"e2","output":"y"});
    assert_ne!(super::dedup_key(&e1), super::dedup_key(&e2));
}

fn write_rows(path: &std::path::Path, n: usize) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    for i in 0..n {
        writeln!(
            f,
            r#"{{"prompt":"q{i}","response":"a{i}","lane":"vox_codegen"}}"#
        )
        .unwrap();
    }
}

#[test]
fn sample_rate_mix_is_reproducible_byte_for_byte() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.jsonl");
    // Spans more than one 10k-line streaming chunk.
    write_rows(&src, 25_000);
    let cfg_path = dir.path().join("mix.yaml");
    let out_path = dir.path().join("out.jsonl");
    let report_path = dir.path().join("out.mix_report.json");
    let yaml = format!(
        "sources:\n  - path: \"{}\"\n    weight: 2.0\n    sample_rate: 0.3\noutput: \"{}\"\n",
        src.to_str().unwrap().replace('\\', "/"),
        out_path.to_str().unwrap().replace('\\', "/")
    );
    std::fs::write(&cfg_path, yaml).unwrap();

    let run = || {
        super::run_mix_with_options(&cfg_path, None, super::MixRunOptions::default()).unwrap();
        let out = std::fs::read(&out_path).unwrap();
        let report = std::fs::read_to_string(&report_path).unwrap();
        // Remove both so the second run cannot take the incremental-skip path.
        std::fs::remove_file(&out_path).unwrap();
        std::fs::remove_file(&report_path).unwrap();
        (out, report)
    };
    let (out1, report1) = run();
    let (out2, report2) = run();
    assert_eq!(out1, out2, "same inputs must give byte-identical output");
    assert_eq!(
        report1, report2,
        "same inputs must give an identical report"
    );

    let kept = out1
        .split(|b| *b == b'\n')
        .filter(|l| !l.is_empty())
        .count();
    assert!(
        (6_500..8_500).contains(&kept),
        "sample_rate 0.3 of 25k should keep ~7.5k rows, kept {kept}"
    );
    let report: super::MixRunReport = serde_json::from_str(&report1).unwrap();
    assert_eq!(
        report.lane_counts.values().sum::<usize>(),
        report.total_emitted,
        "lane counts must describe emitted rows"
    );
}

#[test]
fn mix_seed_changes_the_sample() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.jsonl");
    write_rows(&src, 2_000);
    let run = |seed: u64| {
        let cfg_path = dir.path().join(format!("mix{seed}.yaml"));
        let out_path = dir.path().join(format!("out{seed}.jsonl"));
        let yaml = format!(
            "seed: {seed}\nsources:\n  - path: \"{}\"\n    sample_rate: 0.5\noutput: \"{}\"\n",
            src.to_str().unwrap().replace('\\', "/"),
            out_path.to_str().unwrap().replace('\\', "/")
        );
        std::fs::write(&cfg_path, yaml).unwrap();
        let opts = super::MixRunOptions {
            strict: false,
            write_report: false,
        };
        super::run_mix_with_options(&cfg_path, None, opts).unwrap();
        std::fs::read(&out_path).unwrap()
    };
    assert_ne!(run(1), run(2));
}

#[test]
fn lane_counts_reflect_max_lines_cap() {
    let dir = tempfile::tempdir().unwrap();
    let src = dir.path().join("src.jsonl");
    write_rows(&src, 100);
    let cfg_path = dir.path().join("mix.yaml");
    let out_path = dir.path().join("out.jsonl");
    let yaml = format!(
        "sources:\n  - path: \"{}\"\n    max_lines: 10\noutput: \"{}\"\n",
        src.to_str().unwrap().replace('\\', "/"),
        out_path.to_str().unwrap().replace('\\', "/")
    );
    std::fs::write(&cfg_path, yaml).unwrap();
    super::run_mix_with_options(&cfg_path, None, super::MixRunOptions::default()).unwrap();
    let report: super::MixRunReport = serde_json::from_str(
        &std::fs::read_to_string(dir.path().join("out.mix_report.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(report.lane_counts.get("vox_codegen"), Some(&10));
}

#[test]
fn config_fingerprint_covers_max_lines_dedup_and_seed() {
    let base = "sources:\n  - path: a.jsonl\noutput: out.jsonl\n";
    let fp = |yaml: &str| {
        let cfg: super::MixConfigSchema = serde_yaml::from_str(yaml).unwrap();
        super::calculate_config_fingerprint(&cfg)
    };
    let f0 = fp(base);
    assert_ne!(
        f0,
        fp("sources:\n  - path: a.jsonl\n    max_lines: 5\noutput: out.jsonl\n")
    );
    assert_ne!(f0, fp(&format!("{base}dedup: true\n")));
    assert_ne!(f0, fp(&format!("{base}seed: 7\n")));
}

#[test]
fn mix_config_rejects_output_that_is_also_a_source() {
    let dir = tempfile::tempdir().unwrap();
    let cfg_path = dir.path().join("mix.yaml");
    std::fs::write(
        &cfg_path,
        "sources:\n  - path: target/dogfood/train.jsonl\n  - path: ./target/dogfood/train_mixed.jsonl\noutput: target/dogfood/train_mixed.jsonl\n",
    )
    .unwrap();
    let err = super::MixConfigSchema::load(&cfg_path).unwrap_err();
    assert!(
        format!("{err:#}").contains("also listed as a source"),
        "{err:#}"
    );
}

#[test]
fn shipped_mix_configs_never_read_their_own_output() {
    let ws = crate::training::contract::find_workspace_root().expect("workspace");
    let mut checked = 0;
    for entry in std::fs::read_dir(ws.join("mens/config")).unwrap() {
        let p = entry.unwrap().path();
        let name = p.file_name().unwrap().to_string_lossy().to_string();
        if name.starts_with("mix") && name.ends_with(".yaml") {
            super::MixConfigSchema::load(&p).unwrap_or_else(|e| panic!("{name}: {e:#}"));
            checked += 1;
        }
    }
    assert!(
        checked > 3,
        "expected the shipped mix configs, found {checked}"
    );
}
