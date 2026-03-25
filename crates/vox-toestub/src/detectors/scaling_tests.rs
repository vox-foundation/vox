use super::ScalingSurfacesDetector;
use crate::rules::{DetectionRule, SourceFile};
use std::path::PathBuf;

#[test]
fn detects_blocking_fs_in_async() {
    let d = ScalingSurfacesDetector::new();
    let code = r#"
pub async fn bad() -> std::io::Result<String> {
    std::fs::read_to_string("x.txt")
}
"#;
    let f = SourceFile::new(PathBuf::from("crates/demo/src/lib.rs"), code.to_string());
    let findings = d.detect(&f);
    assert!(
        findings
            .iter()
            .any(|x| x.rule_id == "scaling/blocking-in-async"),
        "{findings:?}"
    );
}

#[test]
fn skips_test_module() {
    let d = ScalingSurfacesDetector::new();
    let code = r#"
#[cfg(test)]
mod tests {
    pub async fn t() {
        let _ = std::fs::read_to_string("x.txt");
    }
}
"#;
    let f = SourceFile::new(PathBuf::from("crates/demo/src/lib.rs"), code.to_string());
    let findings = d.detect(&f);
    assert!(!findings.iter().any(|x| x.rule_id == "scaling/blocking-in-async"));
}

#[test]
fn detects_read_in_loop_heuristic() {
    let d = ScalingSurfacesDetector::new();
    let code = r#"
pub fn scan(paths: &[std::path::PathBuf]) {
    for p in paths {
 let _ = std::fs::read_to_string(p);
    }
}
"#;
    let f = SourceFile::new(PathBuf::from("crates/demo/src/lib.rs"), code.to_string());
    let findings = d.detect(&f);
    assert!(
        findings
            .iter()
            .any(|x| x.rule_id == "scaling/cache-miss-hot-read"),
        "{findings:?}"
    );
}

#[test]
fn detects_large_vec_capacity() {
    let d = ScalingSurfacesDetector::new();
    let code = r#"pub fn buf() -> Vec<u8> { Vec::with_capacity(250_000) }"#;
    let f = SourceFile::new(PathBuf::from("crates/demo/src/lib.rs"), code.to_string());
    let findings = d.detect(&f);
    assert!(
        findings
            .iter()
            .any(|x| x.rule_id == "scaling/large-in-memory-accumulator"),
        "{findings:?}"
    );
}

#[test]
fn detects_duplicate_env_defaults() {
    let d = ScalingSurfacesDetector::new();
    let code = r#"
fn a() { let _ = std::env::var("X").unwrap_or("same_default"); }
fn b() { let _ = std::env::var("Y").unwrap_or("same_default"); }
"#;
    let f = SourceFile::new(PathBuf::from("crates/demo/src/lib.rs"), code.to_string());
    let findings = d.detect(&f);
    assert!(
        findings
            .iter()
            .any(|x| x.rule_id == "scaling/env-default-duplication"),
        "{findings:?}"
    );
}
