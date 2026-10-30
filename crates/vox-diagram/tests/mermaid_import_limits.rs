//! Importer DoS bounds: each 60 KiB adversarial block finishes within 10 s on a fresh thread
//! with the default stack and never panics. The timeout is a hang detector, not a benchmark.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use vox_diagram::limits::MAX_SOURCE_BYTES;
use vox_diagram::mermaid::{ImportOpts, ImportReport, import};

fn bounded(block: String) -> ImportReport {
    assert!(block.len() <= MAX_SOURCE_BYTES, "{} bytes", block.len());
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let _ = tx.send(import(&block, ImportOpts::default()));
    });
    match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(r) => r,
        Err(e) => panic!("import did not finish (hang or crash): {e}"),
    }
}

fn constructs(r: &ImportReport, name: &str) -> usize {
    r.unsupported.iter().filter(|u| u.construct == name).count()
}

#[test]
fn adversarial_fan_out_line() {
    // A dangling `&` before the arrow is a plain statement error; a well-formed fan-out of
    // the same size hits the expansion cap before anything is materialised.
    let r = bounded(format!("flowchart TD\n{}--> b\n", "a & ".repeat(15_000)));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "statement"), 1);
    let r = bounded(format!("flowchart TD\n{}a --> b\n", "a & ".repeat(14_000)));
    assert_eq!(constructs(&r, "expansion-cap"), 1);
}

#[test]
fn adversarial_ten_thousand_arrow_chain() {
    let r = bounded(format!("flowchart TD\na{}\n", " --> a".repeat(10_000)));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "expansion-cap"), 1);
}

#[test]
fn adversarial_four_thousand_nested_subgraphs() {
    let block: String = (0..4_000).map(|i| format!("subgraph s{i}\n")).collect();
    let r = bounded(format!("flowchart TD\n{block}"));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "limit-depth"), 1);
    assert_eq!(constructs(&r, "unclosed-subgraph"), 4_000);
}

#[test]
fn adversarial_bracket_quote_dash_soup_line() {
    let soup = "[\"-(|)>{@&:;`".repeat(4_600);
    let r = bounded(format!("flowchart TD\n{soup}\n"));
    assert_eq!(r.source, None);
    assert!(!r.unsupported.is_empty());
}

#[test]
fn adversarial_ten_thousand_edge_lines() {
    let r = bounded(format!("flowchart TD\n{}", "A-->B\n".repeat(10_000)));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "limit-edges"), 1);
}

#[test]
fn adversarial_comment_soup() {
    let r = bounded(format!("flowchart TD\n{}", "%% x\n".repeat(11_000)));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "comment"), 11_000);
}

#[test]
fn adversarial_state_and_sequence_blocks() {
    let r = bounded(format!(
        "stateDiagram-v2\n{}",
        "[*] --> A: x\n".repeat(4_000)
    ));
    assert_eq!(r.source, None);
    assert_eq!(constructs(&r, "limit-edges"), 1);
    let r = bounded(format!("stateDiagram-v2\n{}", "%% x\n".repeat(11_000)));
    assert_eq!(constructs(&r, "comment"), 11_000);
    let r = bounded(format!("sequenceDiagram\n{}", "A->>B: x\n".repeat(6_000)));
    assert_eq!((&r.sequence, constructs(&r, "limit-edges")), (&None, 1));
    let r = bounded(format!(
        "sequenceDiagram\n{}",
        "participant P\n".repeat(4_000)
    ));
    assert_eq!(constructs(&r, "redeclared-participant"), 3_999);
    let soup = "#a&\"-(|)>{@:`<".repeat(4_000);
    let r = bounded(format!("sequenceDiagram\nA->>B: {soup}\n"));
    assert!(!r.unsupported.is_empty() && r.sequence.is_none());
    let r = bounded(format!("stateDiagram-v2\nA --> B: {soup}\n"));
    assert!(!r.unsupported.is_empty() && r.source.is_none());
}
