//! Silent-divergence regressions from 84-REVIEW iteration 2 (WR-04, WR-05, WR-06): text the pinned
//! Mermaid 11.17.2 parser reads differently from the importer, or rejects while the importer
//! accepted it. Each case was run through the oracle before it was written down here.

use vox_diagram::mermaid::{ImportOpts, import};

/// `(line, construct)` pairs of a refused block; asserts nothing was imported.
fn refused(block: &str) -> Vec<(u32, String)> {
    let r = import(block, ImportOpts::default());
    assert_eq!((&r.source, &r.sequence), (&None, &None), "{block}");
    r.unsupported
        .into_iter()
        .map(|u| (u.line, u.construct))
        .collect()
}

fn refused_as(block: &str, line: u32, construct: &str) {
    assert_eq!(refused(block), [(line, construct.to_owned())], "{block:?}");
}

fn imports(block: &str) {
    let r = import(block, ImportOpts::default());
    assert!(r.source.is_some(), "{block:?}: {:?}", r.unsupported);
}

fn flow(body: &str) -> String {
    format!("flowchart TD\n  {body}\n")
}

fn state(body: &str) -> String {
    format!("stateDiagram-v2\n  {body}\n")
}

// WR-04: Mermaid's `direction\s+(lr|rl|tb|td|bt)` lexer rule spans newlines and blank lines, so a
// line that ends in `direction` and a next line that starts with a direction word is one statement.

#[test]
fn a_line_ending_in_direction_before_a_direction_word_is_refused() {
    for (first, next) in [
        ("A --> Direction", "lr --> B"),
        ("A --> DIRECTION", "LR --> B"),
        ("A --> B: go direction", "btn --> C"),
        ("A --> B: go direction", "tbd --> C"),
        ("A --> B: go direction", "rl_x --> C"),
        ("A --> B: go Direction", "BT --> C"),
        ("A --> B: xdirection", "lr --> C"),
        ("A --> B: direction ", "Lr --> C"),
    ] {
        // Blank lines between the two do not help.
        for gap in ["\n  ", "\n\n\n  "] {
            let block = state(&format!("{first}{gap}{next}"));
            refused_as(&block, 2, "direction-in-text");
        }
    }
    // The flowchart lexer has the same rule for a bare id (`xdirection`) at the end of a line.
    refused_as(
        &flow("A --> xdirection\n  LR --> B"),
        2,
        "direction-in-text",
    );
}

#[test]
fn direction_at_the_end_of_a_line_is_fine_when_no_direction_word_follows() {
    for (first, next) in [
        ("A --> B: direction", "x --> C"),
        ("A --> B: direction", "l --> C"),
        ("A --> B: direction", "[*] --> C"),
        ("A --> B: go direction x", "lr --> C"),
        // An unrelated line in between breaks the span.
        ("A --> B: go direction", "C --> D\n  btn --> E"),
    ] {
        imports(&state(&format!("{first}\n  {next}")));
    }
    imports(&state("A --> Direction"));
    // A real `direction` statement inside a subgraph is not text.
    imports(&flow("subgraph s\n    direction TB\n    a --> b\n  end"));
}

// WR-05: Mermaid reads `b&c` as ONE id, and rejects `b &c` and `b& c`. Only ` & ` fans out.

#[test]
fn ampersand_without_spaces_on_both_sides_is_refused() {
    for body in [
        "a-->b&c",
        "a-->b&_",
        "a-->b &c",
        "a-->b& c",
        "a&b-->c",
        "a &b-->c",
        "a& b-->c",
        "a[x]&b-->c",
        "a --> b[x]&c",
        "a-->b&c&d",
        "a --> b & c&d",
    ] {
        refused_as(&flow(body), 2, "statement");
    }
    for body in [
        "a --> b & c",
        "a & b --> c",
        "a[x] & b[y] --> c & d",
        "a-->b & c",
    ] {
        imports(&flow(body));
    }
}

// WR-06: unquoted `-- text -->` ends at the first `--`, and zero-length or blank dash text is a
// Mermaid parse error. Quoted dash text may hold `--`.

#[test]
fn dash_edge_text_holding_dashes_or_empty_is_refused() {
    for text in [
        "a--b",
        "a --- b",
        "a--->b",
        "a -- b",
        "a-- b",
        "a --b",
        "a<--b",
        "a --x b",
        "a -----> b",
        "--",
        "a --. ",
        "a--- ",
        " ",
        "\t",
    ] {
        refused_as(&flow(&format!("A -- {text} --> B")), 2, "edge-label");
    }
    refused_as(&flow("A -- \"\" --> B"), 2, "edge-label");
    refused_as(&flow("A -->|\"\"| B"), 2, "edge-label");
    refused_as(&flow("A -->|| B"), 2, "edge-label");
}

#[test]
fn dash_edge_text_with_single_dashes_and_quoted_text_still_imports() {
    for text in [
        "a-b", "a - b", "a->b", "-a", "a- ", "a-.-b", "a==>b", "\"a--b\"", "\" \"",
    ] {
        imports(&flow(&format!("A -- {text} --> B")));
    }
    imports(&flow("A -->| | B"));
    imports(&flow("A -->|\" \"| B"));
    // The first `-->` closes the text; the rest is the next hop (Mermaid reads it the same way).
    imports(&flow("A -- a-->b --> B"));
}

// Found by differential fuzzing against the oracle (iteration 3): more silent accepts.

#[test]
fn more_than_one_space_after_a_pipe_label_is_refused() {
    for body in [
        "a -->|t|  b",
        "a -->|t| \tb",
        "a -->|t|\t\tb",
        "a -->|t|   b",
        "a -->|t|  b & c",
        "a -->| t |  b",
        "a --> |t|  b",
        "a -->|\"t\"|  b",
    ] {
        refused_as(&flow(body), 2, "edge-label");
    }
    // Zero or one whitespace after the closing pipe, and any amount before the opening one.
    for body in [
        "a -->|t| b",
        "a -->|t|\tb",
        "a -->|t|b",
        "a -->  |t| b",
        "a -->|t| b & c",
    ] {
        imports(&flow(body));
    }
}

#[test]
fn a_bare_o_or_x_id_fused_to_an_arrow_is_refused() {
    // `o--` and `x--` start a circle / cross link in Mermaid's lexer.
    for body in ["o-->b", "x-->b", "b-->o-->c", "a & x-->b"] {
        refused_as(&flow(body), 2, "invalid-id");
    }
    for body in [
        "o --> b",
        "o -->b",
        "ox-->b",
        "xa-->b",
        "a-->o",
        "a-->o ",
        "o[\"x\"]-->b",
    ] {
        imports(&flow(body));
    }
}

#[test]
fn sequence_title_followed_by_a_space_is_a_title_statement_in_any_case() {
    for line in ["Title ->> A: x", "TITLE  ->>  A: x", "title ->> A: x"] {
        refused_as(&format!("sequenceDiagram\n  {line}\n"), 2, "title");
    }
}
