//! Regression tests for the phase-84 code review (84-REVIEW): input where the importer used to
//! accept text that the pinned Mermaid parser reads differently, or refuse with a wrong name.

use vox_diagram::mermaid::{ImportOpts, ImportReport, import};
use vox_diagram::sequence_ir::SeqEvent;

fn run(block: &str) -> ImportReport {
    import(block, ImportOpts::default())
}

/// `(line, construct)` pairs of a refused block; asserts nothing was imported.
fn refused(block: &str) -> Vec<(u32, String)> {
    let r = run(block);
    assert_eq!((&r.source, &r.sequence), (&None, &None), "{block}");
    r.unsupported
        .into_iter()
        .map(|u| (u.line, u.construct))
        .collect()
}

fn refused_as(block: &str, line: u32, construct: &str) {
    assert_eq!(refused(block), [(line, construct.to_owned())], "{block}");
}

fn source(block: &str) -> String {
    let r = run(block);
    assert!(r.unsupported.is_empty(), "{block}: {:?}", r.unsupported);
    r.source.unwrap_or_default()
}

// CR-03: `direction <TB|TD|LR|RL|BT>` inside text makes Mermaid swallow the line (flowchart)
// or flip the diagram direction (state), so the importer must not read it as text.

#[test]
fn flowchart_text_holding_a_direction_statement_is_refused() {
    for body in [
        "A[\"go direction LR\"] --> B",
        "A -->|\"direction LR\"| B",
        "A -->|go direction TB now| B",
        "A -- \"x direction RL\" --> B",
        "A -- x direction BT --> B",
        "A[\"xDIRECTION td\"] --> B",
        "subgraph s [\"my direction RL box\"]\nend",
    ] {
        let block = format!("flowchart TD\n  {body}\n");
        let got = refused(&block);
        assert!(
            got.iter().any(|(_, c)| c == "direction-in-text"),
            "{body}: {got:?}"
        );
    }
}

#[test]
fn state_transition_text_holding_a_direction_statement_is_refused() {
    refused_as(
        "stateDiagram-v2\n  A --> B: go direction LR\n",
        2,
        "direction-in-text",
    );
}

#[test]
fn direction_lookalikes_and_encoded_forms_are_still_text() {
    assert!(
        source("flowchart TD\n  A[\"direction\"] --> B[\"direction XY\"]\n")
            .contains("direction XY")
    );
    let s = source("flowchart TD\n  A[\"p #100;irection LR q\"] --> B\n");
    assert!(s.contains("p direction LR q"), "{s}");
}

#[test]
fn sequence_text_may_hold_a_direction_statement() {
    let r = run("sequenceDiagram\n  A->>B: go direction LR\n");
    let seq = r.sequence.expect("sequence imports");
    assert!(matches!(&seq.events[0], SeqEvent::Message { text, .. } if text == "go direction LR"));
}
