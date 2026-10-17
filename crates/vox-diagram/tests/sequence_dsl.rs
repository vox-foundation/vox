//! The sequence DSL end to end: check, IR, canonical format, Mermaid export.

mod support;

use serde_json::json;
use vox_diagram::ast::DiagramKind;
use vox_diagram::diag::{Diagnostic, Severity};
use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{export, export_sequence};
use vox_diagram::svg::{SvgOpts, render};
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

const ALL: &str = include_str!("fixtures/valid/sequence_all_constructs.vdg");

fn codes(src: &str) -> Vec<&'static str> {
    check(src).diagnostics.iter().map(|d| d.code).collect()
}

fn only(src: &str) -> Diagnostic {
    let r = check(src);
    assert_eq!(r.diagnostics.len(), 1, "{src:?}: {:?}", r.diagnostics);
    assert!(r.ir.is_none());
    r.diagnostics[0].clone()
}

fn with_body(body: &str) -> String {
    format!("sequence {{\n    actor a: \"A\"\n    actor b: \"B\"\n    actor c: \"C\"\n{body}}}\n")
}

#[test]
fn reply_self_and_notes_round_trip() {
    let r = check(ALL);
    assert!(r.valid && r.diagnostics.is_empty(), "{:?}", r.diagnostics);
    let Some(ir) = r.ir else {
        panic!("no ir");
    };
    let Ok(v) = serde_json::to_value(&ir) else {
        panic!("ir does not serialise");
    };
    assert_eq!(
        support::strip_spans(v)["events"],
        json!([
            {"type": "message", "from": "web", "to": "api", "text": "POST /orders", "dashed": false},
            {"type": "message", "from": "api", "to": "db", "text": "", "dashed": false},
            {"type": "message", "from": "db", "to": "api", "text": "id", "dashed": true},
            {"type": "message", "from": "api", "to": "api", "text": "validate", "dashed": false},
            {"type": "note", "over": ["api"], "text": "idempotent by key"},
            {"type": "note", "over": ["web", "db"], "text": "one transaction"}
        ])
    );
    assert_eq!(fmt(ALL).as_deref(), Some(ALL));
    let mm = export(&ir);
    assert!(mm.contains("p2-->>p1: id"), "{mm}");
    assert!(mm.contains("Note over p0,p2: one transaction"), "{mm}");
    let Ok(svg) = render(&ir, &SvgOpts::default()) else {
        panic!("render failed");
    };
    let Ok(read) = support::svgcheck::check(&svg, "vd") else {
        panic!("checker rejected the svg");
    };
    assert_eq!(read.notes.len(), 2);
}

#[test]
fn note_over_three_actors_is_one_error() {
    let d = only(&with_body("    note over a, b, c: \"x\"\n"));
    assert_eq!(d.code, "diagram/unexpected-token");
    assert!(d.message.starts_with("expected `:`"), "{}", d.message);
}

#[test]
fn empty_labels_follow_the_owner_rules() {
    let actor = only("sequence {\n    actor a: \"\"\n}\n");
    assert_eq!(actor.code, "diagram/empty-label");
    assert_eq!(actor.fixes.len(), 1);
    assert_eq!(actor.fixes[0].replacement, "\"a\"");

    let msg_src = with_body("    a -> b: \"\"\n");
    let msg = only(&msg_src);
    assert_eq!(msg.code, "diagram/empty-label");
    let fix = &msg.fixes[0];
    assert_eq!(fix.replacement, "");
    assert_eq!(
        &msg_src[fix.span.start as usize..fix.span.end as usize],
        ": \"\""
    );

    let note = only(&with_body("    note over a: \"\"\n"));
    assert_eq!(note.code, "diagram/empty-label");
    assert!(note.fixes.is_empty());
}

#[test]
fn reply_needs_a_message() {
    for body in ["    reply\n", "    reply \"x\"\n"] {
        let d = only(&with_body(body));
        assert_eq!(d.code, "diagram/unexpected-token", "{body:?}");
        assert!(
            d.message.contains("a message after `reply`"),
            "{}",
            d.message
        );
    }
}

#[test]
fn keywords_are_contextual() {
    let src = "sequence {\n    actor note: \"N\"\n    actor reply: \"R\"\n    actor over: \"O\"\n    actor actor: \"A\"\n    note -> reply: \"x\"\n    reply reply -> note\n    note over note, over: \"y\"\n    actor -> over\n}\n";
    assert_eq!(codes(src), Vec::<&str>::new());
    let Some(ir) = check(src).ir else {
        panic!("no ir");
    };
    let Ok(v) = serde_json::to_value(&ir) else {
        panic!("ir does not serialise");
    };
    let v = support::strip_spans(v);
    let ids: Vec<&str> = v["actors"].as_array().map_or_else(Vec::new, |a| {
        a.iter().filter_map(|x| x["id"].as_str()).collect()
    });
    assert_eq!(ids, ["note", "reply", "over", "actor"]);
    assert_eq!(v["events"][1]["dashed"], json!(true));
    assert_eq!(v["events"][2]["over"], json!(["note", "over"]));
    assert_eq!(v["events"][3]["to"], json!("over"));
    assert_eq!(fmt(src).as_deref(), Some(src));
}
