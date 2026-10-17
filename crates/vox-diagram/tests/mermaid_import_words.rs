//! Position-aware reserved words (84-REVIEW iteration 2, WR-02 and WR-03). Every expectation was
//! probed against the pinned Mermaid 11.17.2 parser, word by word and position by position: a word
//! is refused exactly where Mermaid rejects (or silently misreads) it, and imports elsewhere.

use vox_diagram::mermaid::{ImportOpts, import};

fn imports(block: &str) -> bool {
    let r = import(block, ImportOpts::default());
    r.source.is_some() || r.sequence.is_some()
}

/// `Some(construct)` of the first refusal, `None` when the block imports.
fn refusal(block: &str) -> Option<String> {
    import(block, ImportOpts::default())
        .unsupported
        .into_iter()
        .next()
        .map(|u| u.construct)
}

/// Each block must import (`want == true`) or be refused; the failing blocks are listed.
fn check(blocks: impl IntoIterator<Item = String>, want: bool) {
    let bad: Vec<String> = blocks.into_iter().filter(|b| imports(b) != want).collect();
    assert!(bad.is_empty(), "want import={want}: {bad:#?}");
}

const FLOW_HEAD: &str = "flowchart TD\n  ";

fn flow(body: &str) -> String {
    format!("{FLOW_HEAD}{body}\n")
}

/// Node-reference positions of a flowchart id (the id is never followed by a shape here).
fn flow_positions(w: &str) -> Vec<String> {
    [
        format!("a --> {w}"),
        format!("{w} --> a"),
        format!("a & {w} --> b"),
        format!("a --> b --> {w}"),
        format!("a -->|t| {w}"),
        format!("a -- t --> {w}"),
    ]
    .iter()
    .map(|b| flow(b))
    .collect()
}

#[test]
fn flowchart_statement_words_are_refused_as_node_ids_alone_or_before_a_dash() {
    let words = [
        "end",
        "style",
        "class",
        "graph",
        "flowchart",
        "subgraph",
        "interpolate",
        "classDef",
        "linkStyle",
        "_self",
        "_blank",
        "_parent",
        "_top",
    ];
    for w in words {
        for id in [w.to_owned(), format!("{w}-x")] {
            let with_shape = [
                flow(&format!("{id}[\"x\"] --> a")),
                flow(&format!("a --> {id}{{x}}")),
            ];
            check(flow_positions(&id).into_iter().chain(with_shape), false);
        }
    }
    // Mermaid's flowchart words are case-sensitive and match only at the start of an id.
    for ok in [
        "End", "STYLE", "Class", "x-end", "endx", "classes", "_selfie", "my-end",
    ] {
        check(flow_positions(ok), true);
    }
}

#[test]
fn flowchart_click_href_call_are_refused_only_without_a_shape() {
    for w in ["click", "href", "call"] {
        // `click --> b` is an EMPTY graph in Mermaid: importing it would invent two nodes and an edge.
        check(flow_positions(w), false);
        check([flow(w), flow(&format!("{w}-- --> a"))], false);
        for with in [
            format!("{w}[\"x\"] --> a"),
            format!("a --> {w}[x]"),
            format!("{w}(x) --> a"),
            format!("a --> {w}{{x}}"),
            format!("a --> {w}((x))"),
        ] {
            check([flow(&with)], true);
        }
        check(flow_positions(&format!("{w}-x")), true);
        check(flow_positions(&format!("{w}1")), true);
    }
}

fn subgraph(id: &str, title: &str) -> String {
    flow(&format!("subgraph {id}{title}\n    a --> b\n  end"))
}

#[test]
fn subgraph_ids_are_refused_only_for_the_words_mermaid_rejects_there() {
    for w in [
        "click",
        "href",
        "call",
        "interpolate",
        "default",
        "_self",
        "_blank",
        "_parent",
        "_top",
    ] {
        for title in ["", " [\"T\"]"] {
            check([subgraph(w, title)], false);
        }
    }
    // Head-before-dash applies to `default` and the link targets, not to `click`.
    for w in ["default-x", "interpolate-x", "_top-x"] {
        check([subgraph(w, "")], false);
    }
    for ok in [
        "click-x",
        "href-x",
        "defaultx",
        "default1",
        "End",
        "style",
        "class",
        "classDef",
        "linkStyle",
        "flowchart",
        "subgraph",
    ] {
        for title in ["", " [\"T\"]"] {
            check([subgraph(ok, title)], true);
        }
    }
    // A DSL keyword stays unusable (the printer would reject the id), whatever Mermaid says.
    for w in ["graph", "node", "direction"] {
        assert_eq!(
            refusal(&subgraph(w, "")).as_deref(),
            Some("reserved-id"),
            "{w}"
        );
    }
}

const STATE_HEAD: &str = "stateDiagram-v2\n  ";

fn state(body: &str) -> String {
    format!("{STATE_HEAD}{body}\n")
}

#[test]
fn state_words_are_refused_by_position() {
    // Refused in every position.
    for w in ["click", "Href", "DEFAULT", "root_start", "root_end", "a-b"] {
        check(
            [
                format!("{w} --> A"),
                format!("A --> {w}"),
                format!("A --> {w}: hi"),
            ]
            .map(|b| state(&b)),
            false,
        );
    }
    // Statement words: refused as a source or a bare target, but a target with a label is text.
    for w in [
        "style", "Class", "classDef", "classdef", "NOTE", "scale", "state",
    ] {
        let labelled = [format!("A --> {w}: hi"), format!("A --> {w}:hi")];
        check(labelled.map(|b| state(&b)), true);
        check([state(&format!("{w} --> A"))], false);
        check(
            [state(&format!("A --> {w}")), state(&format!("[*] --> {w}"))],
            false,
        );
    }
    // Only the exact lowercase spelling is the `[*]` id.
    check([state("Root_start --> ROOT_END")], true);
    // The accessibility words are only a problem in front of a label.
    for w in ["accTitle", "ACCDESCR", "Acctitle"] {
        check(
            [
                state(&format!("A --> {w}: hi")),
                state(&format!("A --> {w}:hi")),
            ],
            false,
        );
        check(
            [state(&format!("A --> {w}")), state(&format!("{w} --> A"))],
            true,
        );
    }
}

#[test]
fn state_trailing_colon_is_an_empty_label_even_when_mermaid_reads_a_space() {
    assert_eq!(refusal(&state("A --> B:")).as_deref(), Some("empty-label"));
    // Known limit: Mermaid accepts `A --> B: ` (a space after the colon), but import trims every
    // line first, so the importer cannot tell the two apart and refuses both.
    assert_eq!(
        refusal("stateDiagram-v2\n  A --> B: \n").as_deref(),
        Some("empty-label")
    );
}

const SEQ_HEAD: &str = "sequenceDiagram\n  ";

fn seq(body: &str) -> String {
    format!("{SEQ_HEAD}{body}\n")
}

const SEQ_WORDS: [&str; 33] = [
    "end",
    "End",
    "loop",
    "ALT",
    "opt",
    "par",
    "and",
    "else",
    "critical",
    "option",
    "break",
    "rect",
    "box",
    "actor",
    "participant",
    "autonumber",
    "create",
    "destroy",
    "activate",
    "deactivate",
    "note",
    "Note",
    "over",
    "Over",
    "link",
    "Links",
    "properties",
    "details",
    "loop-x",
    "end-x",
    "Note-a",
    "over-a",
    "Box-1",
];

#[test]
fn sequence_statement_words_are_refused_when_used_but_may_be_declared() {
    for w in SEQ_WORDS {
        check(
            [
                format!("A->>{w}: x"),
                format!("{w}->>A: x"),
                format!("A-->>{w}: x"),
                format!("Note over {w}: x"),
                format!("Note over A,{w}: x"),
            ]
            .map(|b| seq(&b)),
            false,
        );
        // `participant <anything>` is accepted by Mermaid; the word just cannot be used later.
        check(
            [
                seq(&format!("participant {w}")),
                seq(&format!("participant {w} as X")),
            ],
            true,
        );
    }
}

#[test]
fn sequence_title_is_only_a_keyword_as_a_receiver_or_followed_by_a_space() {
    for ok in [
        "Title->>B: hi",
        "TITLE->>B: hi",
        "title-1->>B: hi",
        "title->>B: hi",
        "A->>title-a: x",
        "A->>Title-a: x",
    ] {
        check([seq(ok)], true);
    }
    for bad in [
        "A->>title: x",
        "A->>Title: x",
        "Note over TITLE: x",
        "title ->>B: hi",
        "title B",
    ] {
        check([seq(bad)], false);
    }
    check([seq("participant title-a"), seq("participant Title")], true);
}

#[test]
fn sequence_acc_words_are_refused_as_receivers_and_note_actors_only() {
    for w in ["accTitle", "ACCDESCR", "Acctitle", "accdescr"] {
        check(
            [
                seq(&format!("A->>{w}: x")),
                seq(&format!("Note over {w}: x")),
            ],
            false,
        );
        check(
            [
                seq(&format!("participant {w}")),
                seq(&format!("{w}->>A: x")),
                seq(&format!("A->>{w}-a: x")),
            ],
            true,
        );
    }
}

#[test]
fn sequence_ids_holding_a_cross_arrow_are_refused_when_used() {
    // `-x` is the cross arrow: Mermaid reads `a-x1` as the actor `a`, the arrow `-x`, then `1`.
    for id in [
        "a-x",
        "a-X",
        "a-xy",
        "a-x1",
        "a-x-b",
        "click-x",
        "default-x",
    ] {
        for b in [
            format!("A->>{id}: y"),
            format!("{id}->>A: y"),
            format!("Note over {id}: y"),
        ] {
            assert_eq!(refusal(&seq(&b)).as_deref(), Some("invalid-id"), "{b}");
        }
        check([seq(&format!("participant {id}"))], true);
    }
    check(
        [seq("A->>a-b: y"), seq("A->>a-y: y"), seq("A->>ax: y")],
        true,
    );
}
