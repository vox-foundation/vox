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

// CR-02: Mermaid's sequence text rule is `[^#\n;]*`, so a raw `#` that does not open an entity
// truncates the text. The importer must refuse it (state text tolerates a bare `#`).

#[test]
fn sequence_text_with_a_stray_hash_is_refused() {
    for (body, line) in [
        ("participant B as 100#", 2),
        ("participant A as x # y", 2),
        ("A->>B: end #", 2),
        ("A->>B: item #3", 2),
        ("Note over A: note # here", 2),
    ] {
        refused_as(&format!("sequenceDiagram\n  {body}\n"), line, "hash");
    }
}

#[test]
fn sequence_text_with_encoded_hashes_and_state_text_with_a_bare_hash_still_import() {
    let r = run("sequenceDiagram\n  A->>B: item #35;3 and #amp; #59;\n");
    let seq = r.sequence.expect("encoded hashes import");
    assert!(matches!(&seq.events[0], SeqEvent::Message { text, .. } if text == "item #3 and & ;"));
    let s = source("stateDiagram-v2\n  A --> B: item #3\n");
    assert!(s.contains("item #3"), "{s}");
}

// WR-01: Mermaid rewrites every `#\w+;` into an entity, not only all-letter or all-digit ones.
// Anything the importer cannot decode with certainty is refused as `entity`.

#[test]
fn mixed_and_hex_entity_spellings_are_refused() {
    for text in ["x#a1b;c", "#x41;", "#_a;", "#12ab;"] {
        refused_as(
            &format!("flowchart TD\n  A[\"{text}\"] --> B\n"),
            2,
            "entity",
        );
        refused_as(
            &format!("stateDiagram-v2\n  A --> B: {text}\n"),
            2,
            "entity",
        );
        refused_as(&format!("sequenceDiagram\n  A->>B: {text}\n"), 2, "entity");
    }
}

#[test]
fn hash_runs_without_a_terminating_semicolon_stay_literal() {
    let s = source("flowchart TD\n  A[\"C# x #a1b #3 #\"] --> B\n");
    assert!(s.contains("C# x #a1b #3 #"), "{s}");
}

// WR-02: under `drop_styling`, a `:::class` shorthand on a state endpoint is dropped and the
// transition kept; it used to delete the whole transition and its states.

fn dropping(block: &str) -> ImportReport {
    import(block, ImportOpts { drop_styling: true })
}

#[test]
fn state_shorthand_is_dropped_but_the_transition_is_kept() {
    let r = dropping("stateDiagram-v2\n A:::foo --> B\n B --> C\n");
    assert!(r.unsupported.is_empty(), "{r:?}");
    let s = r.source.expect("imports");
    assert_eq!(s, source("stateDiagram-v2\n A --> B\n B --> C\n"));
    assert_eq!(
        r.dropped
            .iter()
            .map(|d| (d.line, d.construct.as_str()))
            .collect::<Vec<_>>(),
        [(2, "styling:shorthand")]
    );
    for block in [
        "stateDiagram-v2\n A --> B:::foo\n",
        "stateDiagram-v2\n A:::foo --> B:::bar\n",
    ] {
        assert_eq!(
            dropping(block).source,
            Some(source("stateDiagram-v2\n A --> B\n"))
        );
    }
    let labelled = dropping("stateDiagram-v2\n A --> B:::foo: lbl\n");
    assert_eq!(
        labelled.source,
        Some(source("stateDiagram-v2\n A --> B: lbl\n"))
    );
}

#[test]
fn state_shorthand_stays_refused_by_default_and_bad_shorthand_is_never_dropped() {
    refused_as("stateDiagram-v2\n A:::foo --> B\n", 2, "styling:shorthand");
    // Mermaid itself rejects `:::` inside a label, so there is nothing to drop.
    for block in [
        "stateDiagram-v2\n A --> B: see x:::y\n",
        "stateDiagram-v2\n A:::foo bar --> B\n",
        "stateDiagram-v2\n A:::: --> B\n",
    ] {
        let r = dropping(block);
        assert!(
            r.source.is_none() && !r.unsupported.is_empty(),
            "{block}: {r:?}"
        );
    }
}
