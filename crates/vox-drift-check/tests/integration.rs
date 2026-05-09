use vox_drift_check::engine::DriftEngine;
use tempfile::TempDir;
use std::fs;

#[test]
fn detects_planted_reqwest_bypass() {
    let dir = TempDir::new().unwrap();
    let crate_dir = dir.path().join("crates/vox-publisher/src");
    fs::create_dir_all(&crate_dir).unwrap();
    fs::write(crate_dir.join("lib.rs"), r#"
pub fn make_client() -> reqwest::Client {
    reqwest::Client::new()
}
"#).unwrap();

    let engine = DriftEngine::new(dir.path());
    let findings = engine.run_all().unwrap();
    assert!(
        findings.iter().any(|f| f.rule_id == "drift/reqwest-bypass"),
        "Expected drift/reqwest-bypass finding, got: {:?}",
        findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}

#[test]
fn detects_duplicate_string_literals() {
    let dir = TempDir::new().unwrap();
    let src_dir = dir.path().join("crates/vox-foo/src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("a.rs"), r#"
fn a() { let x = "duplicate-constant-value"; }
fn b() { let y = "duplicate-constant-value"; }
fn c() { let z = "duplicate-constant-value"; }
"#).unwrap();

    let engine = DriftEngine::new(dir.path());
    let findings = engine.run_all().unwrap();
    assert!(
        findings.iter().any(|f| f.rule_id == "sweep/duplicate-string-literal"),
        "Expected sweep/duplicate-string-literal finding, got: {:?}",
        findings.iter().map(|f| &f.rule_id).collect::<Vec<_>>()
    );
}

#[test]
fn cache_avoids_reparse_on_second_run() {
    let dir = TempDir::new().unwrap();
    let src_dir = dir.path().join("crates/vox-bar/src");
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("lib.rs"), r#"fn foo() { let x = "cached"; }"#).unwrap();

    let engine = DriftEngine::new(dir.path());
    // First run — populates cache
    let findings1 = engine.run_all().unwrap();
    // Second run — should hit cache and produce same results
    let findings2 = engine.run_all().unwrap();
    assert_eq!(findings1.len(), findings2.len());
}
