//! The sequence DSL end to end: check, IR, canonical format, Mermaid export.

mod support;

use serde_json::json;
use vox_diagram::ast::DiagramKind;
use vox_diagram::diag::Severity;
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{export, export_sequence};
use vox_diagram::{check, fmt, parse};

const MIN: &str = "sequence {\n    actor a: \"A\"\n    actor b: \"B\"\n    a -> b: \"hi\"\n}\n";

fn ir_of(src: &str) -> Ir {
    let Some(ir) = check(src).ir else {
        panic!("source does not check: {src:?}");
    };
    ir
}

#[test]
fn tracer_minimal_sequence_checks() {
    let r = check(MIN);
    assert!(r.valid && r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let Some(ir) = r.ir else {
        panic!("no ir");
    };
    assert!(matches!(ir, Ir::Sequence(_)));
    let Ok(v) = serde_json::to_value(&ir) else {
        panic!("ir does not serialise");
    };
    assert_eq!(
        support::strip_spans(v),
        json!({"kind": "sequence",
            "actors": [{"id": "a", "label": "A"}, {"id": "b", "label": "B"}],
            "events": [{"type": "message", "from": "a", "to": "b", "text": "hi", "dashed": false}]})
    );
    assert_eq!(
        (
            r.stats.nodes,
            r.stats.edges,
            r.stats.containers,
            r.stats.max_depth
        ),
        (2, 1, 0, 0)
    );
    let Some(doc) = parse(MIN).ast else {
        panic!("no ast");
    };
    assert_eq!(doc.kind, DiagramKind::Sequence);
}

#[test]
fn tracer_minimal_sequence_formats_canonically() {
    assert_eq!(fmt(MIN).as_deref(), Some(MIN));
    let noisy = "sequence {\n  actor a:\"A\"\n  actor b :  \"B\"\n  a->b:\"hi\"   \n}";
    assert_eq!(fmt(noisy).as_deref(), Some(MIN));
}

#[test]
fn tracer_exports_to_mermaid() {
    let ir = ir_of(MIN);
    assert_eq!(
        export(&ir),
        "sequenceDiagram\n    participant p0 as A\n    participant p1 as B\n    p0->>p1: hi\n"
    );
    let Ir::Sequence(s) = &ir else {
        panic!("expected sequence ir");
    };
    assert_eq!(export(&ir), export_sequence(s));
}

#[test]
fn tracer_undeclared_actor_is_reported() {
    let src = "sequence {\n    actor a: \"A\"\n    a -> c\n}\n";
    let r = check(src);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, "diagram/undeclared-actor");
    assert_eq!(d.severity, Severity::Error);
    assert_eq!(d.span.line, 3);
    assert_eq!(&src[d.span.start as usize..d.span.end as usize], "c");
    assert!(!r.valid && r.ir.is_none());
}

#[test]
fn tracer_duplicate_actor_is_reported() {
    let src = "sequence {\n    actor a: \"A\"\n    actor a: \"B\"\n}\n";
    let r = check(src);
    assert_eq!(r.diagnostics.len(), 1, "{:?}", r.diagnostics);
    let d = &r.diagnostics[0];
    assert_eq!(d.code, "diagram/duplicate-id");
    assert_eq!((d.span.line, d.related.len()), (3, 1));
    assert_eq!(d.related[0].span.line, 2);
    assert!(r.ir.is_none());
}

#[test]
fn tracer_message_without_text() {
    let src = "sequence {\n    actor a: \"A\"\n    actor b: \"B\"\n    a -> b\n}\n";
    let Ir::Sequence(s) = ir_of(src) else {
        panic!("expected sequence ir");
    };
    let vox_diagram::sequence_ir::SeqEvent::Message { text, .. } = &s.events[0] else {
        panic!("expected a message");
    };
    assert_eq!(text, "");
    assert_eq!(fmt(src).as_deref(), Some(src));
}
