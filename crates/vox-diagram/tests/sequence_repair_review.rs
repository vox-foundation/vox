//! Regressions from the phase-86 code review (CR-01, WR-01, WR-02): `repair` of Mermaid
//! sequence blocks must neither lose statements nor leave garbage, and its text decoding must
//! agree with the strict importer.

use vox_diagram::ir::Ir;
use vox_diagram::mermaid::{ImportOpts, import};
use vox_diagram::sequence_ir::{SeqEvent, SequenceIr};
use vox_diagram::{check, repair};

/// The repaired document as a valid `SequenceIr`; panics with the repair trace otherwise.
fn repaired(src: &str) -> SequenceIr {
    let r = repair(src, 5);
    assert!(
        r.report.valid,
        "{:?}\n{src}\n=>\n{}",
        r.report.diagnostics, r.text
    );
    assert_eq!(
        repair(&r.text, 5).rounds,
        0,
        "not a fixed point:\n{}",
        r.text
    );
    match r.report.ir {
        Some(Ir::Sequence(ir)) => ir,
        other => panic!("no sequence ir: {other:?}\n{}", r.text),
    }
}

/// Events as `from>to:text` (messages) and `over:text` (notes).
fn events(ir: &SequenceIr) -> Vec<String> {
    ir.events
        .iter()
        .map(|e| match e {
            SeqEvent::Message { from, to, text, .. } => format!("{from}>{to}:{text}"),
            SeqEvent::Note { over, text, .. } => format!("{}:{text}", over.join(",")),
        })
        .collect()
}

fn imported(src: &str) -> SequenceIr {
    import(src, ImportOpts::default())
        .sequence
        .unwrap_or_else(|| panic!("the importer refused:\n{src}"))
}

#[test]
fn cr01_semicolon_ends_an_unquoted_message_label() {
    let src =
        "sequenceDiagram\n  participant a\n  participant b\n  a->>b: x; b->>a: y\n  a->>b: z;\n";
    let ir = repaired(src);
    assert_eq!(events(&ir), ["a>b:x", "b>a:y", "a>b:z"]);
}

#[test]
fn cr01_semicolon_ends_an_unquoted_note_label() {
    let src = "sequenceDiagram\n  participant a\n  participant b\n  Note over a: n; b->>a: y\n";
    assert_eq!(events(&repaired(src)), ["a:n", "b>a:y"]);
}

#[test]
fn cr01_an_entity_semicolon_does_not_end_the_label() {
    let src = "sequenceDiagram\n  participant a\n  a->>a: x#59;y\n";
    assert_eq!(events(&repaired(src)), ["a>a:x#59;y"]);
}

#[test]
fn wr01_an_unterminated_alias_repairs_to_a_valid_document() {
    let src = "sequence {\n    participant a as \"unterminated\n}\n";
    assert!(!check(src).valid);
    let ir = repaired(src);
    assert_eq!(
        ir.actors.first().map(|a| a.label.as_str()),
        Some("unterminated")
    );
}

#[test]
fn wr02_br_decodes_like_the_importer_in_alias_message_and_note() {
    let mer = "sequenceDiagram\n  participant a as Foo<br/>Bar\n  a->>a: line1<br/>line2\n  Note over a: n1<BR>n2\n  a->>a: p<br />q\n";
    let ir = repaired(mer);
    assert_eq!(events(&ir), ["a>a:line1\nline2", "a:n1\nn2", "a>a:p\nq"]);
    let strict = imported(mer);
    assert_eq!(events(&strict), events(&ir));
    assert_eq!(strict.actors[0].label, ir.actors[0].label);
    assert_eq!(ir.actors[0].label, "Foo\nBar");
}
