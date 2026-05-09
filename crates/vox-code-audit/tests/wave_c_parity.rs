//! Locks (rule_id, line_num) tuples for the Wave C detector rewrites (secrets, magic_value).

use std::path::PathBuf;
use vox_code_audit::detectors::magic_value::MagicValueDetector;
use vox_code_audit::detectors::secrets::SecretDetector;
use vox_code_audit::rules::{DetectionRule, SourceFile};

fn rs(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("src/lib.rs"), code.to_string())
}

fn py(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("test.py"), code.to_string())
}

// ── SecretDetector ───────────────────────────────────────────────────────────

#[test]
fn secret_aws_key_fires_with_sub_id() {
    let d = SecretDetector::new();
    // Avoid a contiguous match in this file by splitting the key.
    let code = ["let key = \"AKIA", "ABCDEF1234567890\";"].concat();
    let f = rs(&code);
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security/hardcoded-secret/aws-key");
    assert_eq!(findings[0].line, 1);
}

#[test]
fn secret_generic_fires_with_sub_id() {
    let d = SecretDetector::new();
    let code = ["DB_PASSWORD = 'super", "-secret-pass-123'"].concat();
    let f = py(&code);
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "security/hardcoded-secret/generic");
}

#[test]
fn secret_example_key_silent() {
    let d = SecretDetector::new();
    let f = rs(r#"let k = "AKIAIOSFODNN7EXAMPLE";"#);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn secret_synthetic_uniform_aws_silent() {
    let d = SecretDetector::new();
    let f = rs(r#"let key = "AKIAZZZZZZZZZZZZZZ";"#);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn secret_env_var_read_silent() {
    let d = SecretDetector::new();
    let f = rs(r#"let key = std::env::var("MY_API_KEY").unwrap();"#);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn secret_comment_line_silent() {
    let d = SecretDetector::new();
    let code = ["// password: \"super", "-secret-123\""].concat();
    let f = rs(&code);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn secret_trailing_rust_comment_silent() {
    let d = SecretDetector::new();
    let f = rs("fn x() {}\nlet _ = 1; // password: \"aaaaaaaa\" \"bbbbbbbb\"\n");
    assert!(d.detect(&f, None).is_empty());
}

// ── MagicValueDetector ───────────────────────────────────────────────────────

#[test]
fn magic_port_fires_with_sub_id() {
    let d = MagicValueDetector::new();
    let f = rs(r#"let addr = "127.0.0.1:3000";"#);
    let findings = d.detect(&f, None);
    assert!(!findings.is_empty());
    assert_eq!(findings[0].rule_id, "magic-value/port");
    assert_eq!(findings[0].line, 1);
}

#[test]
fn magic_db_conn_fires_with_sub_id() {
    let d = MagicValueDetector::new();
    let f = py(r#"conn = "postgres://user:pass@localhost/db""#);
    let findings = d.detect(&f, None);
    assert!(findings.iter().any(|f| f.rule_id == "magic-value/db-conn"));
}

#[test]
fn magic_const_definition_silent() {
    let d = MagicValueDetector::new();
    let f = rs(r#"pub const DEFAULT_PORT: &str = "127.0.0.1:3000";"#);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn magic_comment_line_silent() {
    let d = MagicValueDetector::new();
    let f = rs(r#"// connect to "127.0.0.1:5432""#);
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn magic_ephemeral_port_zero_silent() {
    let d = MagicValueDetector::new();
    let f = rs(r#"let addr = "127.0.0.1:0"; // OS assigns port"#);
    assert!(d.detect(&f, None).is_empty());
}
