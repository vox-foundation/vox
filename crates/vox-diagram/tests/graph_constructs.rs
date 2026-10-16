//! Tracer tests: one minimal `graph` flows source -> report through every layer.

use vox_diagram::diag::Severity;
use vox_diagram::{check, parse};

const MINIMAL: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

#[test]
fn tracer_minimal_graph_report() {
    let report = check(MINIMAL);
    assert_eq!(report.schema_version, 1);
    assert!(report.valid);
    assert!(report.diagnostics.is_empty());
    assert_eq!(report.stats.nodes, 2);
    assert_eq!(report.stats.edges, 1);
    insta::assert_json_snapshot!(report);
}

#[test]
fn tracer_reports_unexpected_token() {
    let report = check("graph {\n    node a \"A\"\n}\n");
    assert!(!report.valid);
    assert!(report.ir.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    let d = &report.diagnostics[0];
    assert_eq!(d.code, "diagram/unexpected-token");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.span.line, 2);
    assert_eq!(d.span.col, 12);
    assert!(d.message.contains("`:`"), "{}", d.message);
    assert!(d.message.contains("\"A\""), "{}", d.message);
}

#[test]
fn tracer_reports_undeclared_node() {
    let src = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n    a -> c\n}\n";
    let report = check(src);
    assert!(!report.valid);
    assert!(report.ir.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    let d = &report.diagnostics[0];
    assert_eq!(d.code, "diagram/undeclared-node");
    assert_eq!((d.span.line, d.span.col), (5, 10));
    assert!(d.fixes.is_empty());
}

#[test]
fn tracer_rejects_oversized_source() {
    let mut big = String::from(MINIMAL);
    big.push_str(&" ".repeat(65_537 - big.len()));
    assert_eq!(big.len(), 65_537);
    let report = check(&big);
    assert!(!report.valid);
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].code, "diagram/limit-source");
    assert_eq!(report.diagnostics[0].span.start, 65_536);
    assert!(parse(&big).ast.is_none());

    // Exactly at the cap is accepted: pad with blank lines inside the block.
    let head = "graph {\n    node a: \"A\"\n";
    let tail = "}\n";
    let pad = "\n".repeat(65_536 - head.len() - tail.len());
    let ok = format!("{head}{pad}{tail}");
    assert_eq!(ok.len(), 65_536);
    let report = check(&ok);
    assert!(report.valid, "{:?}", report.diagnostics);
}

const CANONICAL: &str = include_str!("fixtures/valid/canonical.vdg");

#[test]
fn canonical_example_ir() {
    let report = check(CANONICAL);
    assert!(report.valid, "{:?}", report.diagnostics);
    assert_eq!(report.stats.containers, 1);
    assert_eq!(report.stats.max_depth, 1);
    insta::assert_json_snapshot!(report.ir);
}

#[test]
fn ir_json_is_stable() {
    let first = serde_json::to_string(&check(CANONICAL)).unwrap_or_default();
    assert!(first.contains("backend.gateway"));
    for _ in 0..3 {
        assert_eq!(
            serde_json::to_string(&check(CANONICAL)).unwrap_or_default(),
            first
        );
    }
}

const ALL: &str = include_str!("fixtures/valid/all_constructs.vdg");

#[test]
fn all_constructs_ir() {
    let report = check(ALL);
    assert!(report.valid, "{:?}", report.diagnostics);
    assert_eq!(report.stats.max_depth, 3);
    insta::assert_json_snapshot!(report.ir);
}

fn strip_spans(v: &mut serde_json::Value) {
    match v {
        serde_json::Value::Object(m) => {
            m.remove("span");
            m.values_mut().for_each(strip_spans);
        }
        serde_json::Value::Array(a) => a.iter_mut().for_each(strip_spans),
        _ => {}
    }
}

#[test]
fn crlf_input_yields_same_ir() {
    let crlf = ALL.replace('\n', "\r\n");
    let (lf, crlf) = (check(ALL), check(&crlf));
    assert!(crlf.valid, "{:?}", crlf.diagnostics);
    let mut a = serde_json::to_value(&lf.ir).unwrap_or_default();
    let mut b = serde_json::to_value(&crlf.ir).unwrap_or_default();
    strip_spans(&mut a);
    strip_spans(&mut b);
    assert_eq!(a, b);
}
