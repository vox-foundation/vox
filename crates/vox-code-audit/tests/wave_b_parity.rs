//! Locks (rule_id, line_num) tuples for the Wave B detector rewrites.
//!
//! Each test constructs a minimal corpus, runs the rewritten detector, and asserts that the
//! same rule IDs fire on the same lines as before the pattern-to-YAML migration.

use std::path::PathBuf;
use vox_code_audit::detectors::ai_laziness::AiLazinessDetector;
use vox_code_audit::detectors::deprecated_usage::DeprecatedUsageDetector;
use vox_code_audit::detectors::stringly_typed_enum::StringlyTypedEnumDetector;
use vox_code_audit::detectors::stub::StubDetector;
use vox_code_audit::detectors::unwrap_call::UnwrapCallDetector;
use vox_code_audit::rules::{DetectionRule, Language, SourceFile};

fn rs(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("src/lib.rs"), code.to_string())
}

fn vox(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("test.vox"), code.to_string())
}

fn py(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("test.py"), code.to_string())
}

fn ts(code: &str) -> SourceFile {
    SourceFile::new(PathBuf::from("test.ts"), code.to_string())
}

fn rule_ids(findings: &[vox_code_audit::rules::Finding]) -> Vec<&str> {
    findings.iter().map(|f| f.rule_id.as_str()).collect()
}

// ── AiLazinessDetector ──────────────────────────────────────────────────────

#[test]
fn ai_laziness_placeholder_return_fires() {
    let d = AiLazinessDetector::new();
    let f = rs(r#"fn x() -> &'static str { return "TODO" }"#);
    let findings_ = d.detect(&f, None);
    let ids = rule_ids(&findings_);
    assert!(ids.contains(&"ai-laziness/placeholder-return"));
}

#[test]
fn ai_laziness_implement_later_fires() {
    let d = AiLazinessDetector::new();
    let f = rs("// implement later\nfn x() {}");
    let findings_ = d.detect(&f, None);
    let ids = rule_ids(&findings_);
    assert!(ids.contains(&"ai-laziness/implement-later-comment"));
}

#[test]
fn ai_laziness_mock_fn_fires_outside_tests() {
    let d = AiLazinessDetector::new();
    let f = rs("pub fn mock_db() -> u32 { 0 }");
    let findings_ = d.detect(&f, None);
    let ids = rule_ids(&findings_);
    assert!(ids.contains(&"ai-laziness/mock-named-fn"));
}

#[test]
fn ai_laziness_mock_fn_silent_in_tests_dir() {
    let d = AiLazinessDetector::new();
    let f = SourceFile::new(
        PathBuf::from("crates/x/tests/helpers.rs"),
        "pub fn mock_db() -> u32 { 0 }".into(),
    );
    assert!(!rule_ids(&d.detect(&f, None)).contains(&"ai-laziness/mock-named-fn"));
}

#[test]
fn ai_laziness_custom_type_default_fires() {
    let d = AiLazinessDetector::new();
    let f = rs("fn x() -> MyConfig { return MyConfig::default() }");
    let findings_ = d.detect(&f, None);
    let ids = rule_ids(&findings_);
    assert!(ids.contains(&"ai-laziness/custom-type-default-return"));
}

#[test]
fn ai_laziness_builtin_default_silent() {
    let d = AiLazinessDetector::new();
    let f = rs("fn x() -> Vec<u8> { return Vec::new() }");
    assert!(!rule_ids(&d.detect(&f, None)).contains(&"ai-laziness/custom-type-default-return"));
}

#[test]
fn ai_laziness_conditional_stub_fires() {
    let d = AiLazinessDetector::new();
    let f = rs(
        "fn x(b: bool) -> Result<()> { if b { return Ok(()); } else { do_real_work(); Ok(()) } }",
    );
    assert!(rule_ids(&d.detect(&f, None)).contains(&"ai-laziness/conditional-stub"));
}

#[test]
fn ai_laziness_assertion_only_body_fires() {
    let d = AiLazinessDetector::new();
    let f = rs("fn x(n: u32) { assert!(n > 0) }");
    assert!(rule_ids(&d.detect(&f, None)).contains(&"ai-laziness/assertion-only-body"));
}

#[test]
fn ai_laziness_early_return_only_fires() {
    let d = AiLazinessDetector::new();
    let f = rs("fn x() { return; }");
    assert!(rule_ids(&d.detect(&f, None)).contains(&"ai-laziness/early-return-only"));
}

// ── StubDetector ────────────────────────────────────────────────────────────

#[test]
fn stub_todo_fires() {
    let d = StubDetector::new();
    let f = rs("fn foo() {\n    todo!()\n}");
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "stub/todo");
    assert_eq!(findings[0].line, 2);
}

#[test]
fn stub_unimplemented_fires() {
    let d = StubDetector::new();
    let f = rs("fn bar() -> i32 {\n    unimplemented!()\n}");
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "stub/unimplemented");
}

#[test]
fn stub_python_raise_fires() {
    let d = StubDetector::new();
    let f = py("def foo():\n    raise NotImplementedError\n");
    assert!(rule_ids(&d.detect(&f, None)).contains(&"stub/not-implemented-error"));
}

#[test]
fn stub_python_pass_fires() {
    let d = StubDetector::new();
    let f = py("def foo():\n    pass\n");
    assert!(rule_ids(&d.detect(&f, None)).contains(&"stub/pass"));
}

#[test]
fn stub_ts_throw_not_impl_fires() {
    let d = StubDetector::new();
    let f = ts("function foo() { throw new Error('not implemented'); }");
    assert!(rule_ids(&d.detect(&f, None)).contains(&"stub/throw-not-implemented"));
}

#[test]
fn stub_clean_rust_silent() {
    let d = StubDetector::new();
    let f = rs("fn add(a: i32, b: i32) -> i32 { a + b }");
    assert!(d.detect(&f, None).is_empty());
}

// ── DeprecatedUsageDetector ─────────────────────────────────────────────────

#[test]
fn deprecated_fires() {
    let d = DeprecatedUsageDetector::new();
    let f = vox("@deprecated\nfn old() {}");
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "deprecated-usage");
    assert_eq!(findings[0].line, 1);
}

#[test]
fn jsx_classname_fires_with_vox_suggestion() {
    let d = DeprecatedUsageDetector::new();
    let f = vox("<div className=\"x\">");
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "raw-jsx-leakage");
    assert!(
        findings[0]
            .suggestion
            .as_deref()
            .unwrap_or("")
            .contains("class=")
    );
}

#[test]
fn jsx_two_attrs_on_one_line_each_emit_finding() {
    let d = DeprecatedUsageDetector::new();
    let f = vox("<div className=\"x\" onClick={h}>");
    assert_eq!(d.detect(&f, None).len(), 2);
}

// ── UnwrapCallDetector ───────────────────────────────────────────────────────

#[test]
fn unwrap_fires_in_prod_code() {
    let d = UnwrapCallDetector::new();
    let f = SourceFile {
        path: PathBuf::from("crates/demo/src/lib.rs"),
        language: Language::Rust,
        content: "fn f() { x.unwrap(); }".into(),
        lines: vec!["fn f() { x.unwrap(); }".into()],
    };
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "rust/unwrap-call");
}

#[test]
fn unwrap_silent_in_tests_dir() {
    let d = UnwrapCallDetector::new();
    let f = SourceFile {
        path: PathBuf::from("crates/demo/tests/integration.rs"),
        language: Language::Rust,
        content: "fn f() { x.unwrap(); }".into(),
        lines: vec!["fn f() { x.unwrap(); }".into()],
    };
    assert!(d.detect(&f, None).is_empty());
}

// ── StringlyTypedEnumDetector ────────────────────────────────────────────────

#[test]
fn stringly_typed_fires_on_vox() {
    let d = StringlyTypedEnumDetector::new();
    let f = vox(r#"  frame: String // "gain" | "loss""#);
    let findings = d.detect(&f, None);
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].rule_id, "stringly-typed-enum");
}

#[test]
fn stringly_typed_fires_on_rust() {
    let d = StringlyTypedEnumDetector::new();
    let f = SourceFile::new(
        PathBuf::from("test.rs"),
        r#"  pub role: String, // "user" | "assistant""#.to_string(),
    );
    assert_eq!(d.detect(&f, None).len(), 1);
}

#[test]
fn stringly_typed_silent_on_proper_adt() {
    let d = StringlyTypedEnumDetector::new();
    let f = vox("  frame: Frame");
    assert!(d.detect(&f, None).is_empty());
}

#[test]
fn stringly_typed_ignores_raw_string_literal() {
    let d = StringlyTypedEnumDetector::new();
    let inner = r#"  frame: String // "gain" | "loss"#;
    let line = format!("    let _ = r#\"{inner}\"#;");
    let f = SourceFile::new(PathBuf::from("test.rs"), line);
    assert!(d.detect(&f, None).is_empty());
}
