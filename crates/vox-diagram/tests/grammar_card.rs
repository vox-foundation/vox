//! The grammar card is dogfooded: its example checks clean and is a `fmt` fixed point, every
//! "Do NOT write" example is flagged as Mermaid, and it stays inside its size budget.

use vox_diagram::{GRAMMAR_CARD, check, codes, fmt};

/// The example block: from the `graph {` line to the first column-0 `}`.
fn example() -> String {
    let start = GRAMMAR_CARD
        .find("graph {")
        .unwrap_or_else(|| panic!("no example"));
    let rest = GRAMMAR_CARD.get(start..).unwrap_or_default();
    let end = rest
        .find("\n}")
        .unwrap_or_else(|| panic!("example never closes"));
    format!("{}\n}}\n", rest.get(..end).unwrap_or_default())
}

/// The left-hand side of every line of the "Do NOT write" section.
fn mermaid_examples() -> Vec<String> {
    let section = GRAMMAR_CARD
        .split("Do NOT write")
        .nth(1)
        .unwrap_or_else(|| panic!("no Do NOT section"));
    section
        .lines()
        .filter_map(|l| l.split_once("  => ").map(|(lhs, _)| lhs.trim().to_string()))
        .collect()
}

#[test]
fn grammar_card_example_checks_clean() {
    let src = example();
    let report = check(&src);
    assert!(
        report.valid && report.diagnostics.is_empty(),
        "{:?}",
        report.diagnostics
    );
    assert_eq!(
        fmt(&src).as_deref(),
        Some(src.as_str()),
        "the example is not canonical"
    );
}

#[test]
fn grammar_card_mermaid_lines_are_flagged() {
    let examples = mermaid_examples();
    assert!(examples.len() >= 6, "only {} examples", examples.len());
    for ex in examples {
        let src = if ex.starts_with("graph ") || ex.starts_with("flowchart ") {
            format!("{ex}\n")
        } else {
            format!("graph {{\n    {}\n}}\n", ex.replace(" ... ", "\n    "))
        };
        let report = check(&src);
        let hit = report
            .diagnostics
            .iter()
            .any(|d| d.code == codes::MERMAID_SYNTAX);
        assert!(
            hit,
            "`{ex}` is not flagged as Mermaid: {:?}",
            report.diagnostics
        );
    }
}

#[test]
fn grammar_card_reserved_words_are_rejected() {
    let line = GRAMMAR_CARD
        .lines()
        .find_map(|l| l.split_once("Reserved: "))
        .unwrap_or_else(|| panic!("no Reserved line"))
        .1;
    let words: Vec<&str> = line.trim_end_matches('.').split(' ').collect();
    assert_eq!(words.len(), 6, "{words:?}");
    for w in words {
        let r = check(&format!("graph {{\n    node {w}: \"x\"\n}}\n"));
        assert!(
            r.diagnostics.iter().any(|d| d.code == codes::RESERVED_WORD),
            "{w}"
        );
    }
}

#[test]
fn grammar_card_size_budget() {
    // About 600 tokens at about 4 bytes per token (no tokenizer is available here).
    assert!(GRAMMAR_CARD.len() <= 2_600, "{} bytes", GRAMMAR_CARD.len());
    assert!(!GRAMMAR_CARD.lines().any(|l| l.starts_with("```")));
}
