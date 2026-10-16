//! DGM-04: Mermaid-isms parse leniently into the canonical AST, report `diagram/mermaid-syntax`
//! (an Error, so the IR is withheld) and carry the exact canonical rewrite as `fixes[]` where one
//! exists. One named test per catalogue row (M1-M10).

mod support;

use std::fs;
use std::path::PathBuf;

use support::mermaid::{edits, fixes_of, pair, repaired_graph, wrap};

use vox_diagram::ast::{Entry, ItemKind};
use vox_diagram::diag::Severity;
use vox_diagram::ir::{Direction, Shape};
use vox_diagram::{check, codes, parse, repair};

const MS: &str = codes::MERMAID_SYNTAX;
const CE: &str = "diagram/chained-edge";

#[test]
fn mermaid_m1_long_arrow_fix() {
    let src = wrap("    a --> b\n");
    let report = check(&src);
    assert!(!report.valid && report.ir.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(report.diagnostics[0].severity, Severity::Error);
    assert_eq!(fixes_of(&src), vec![(MS, vec![pair("-->", "->")])]);
    let long = wrap("    a -----> b\n");
    assert_eq!(fixes_of(&long), vec![(MS, vec![pair("----->", "->")])]);
    // The lenient AST holds the edge exactly as the canonical form would.
    let ast = parse(&src).ast.unwrap_or_else(|| panic!("no ast"));
    let edges = ast.body.entries.iter().filter(|e| {
        matches!(e, Entry::Item(i) if matches!(&i.kind, ItemKind::Edge(x)
            if x.from.segments[0].name == "a" && x.to.segments[0].name == "b"))
    });
    assert_eq!(edges.count(), 1);
}

#[test]
fn mermaid_m1_converges_with_declare_fixes() {
    let src = "graph {\n    A --> B\n}\n";
    let r = repair(src, 5);
    assert!(r.report.valid, "{}", r.text);
    // The arrow and both declarations are independent fixes, so one round may suffice.
    assert!(r.rounds <= 2, "{} rounds", r.rounds);
    for want in ["node A: \"A\"", "node B: \"B\"", "A -> B"] {
        assert!(r.text.contains(want), "{want} missing from\n{}", r.text);
    }
}

#[test]
fn mermaid_m1_styled_arrows_have_no_fix() {
    for arrow in ["==>", "-.->", "---", "--x", "--o"] {
        let src = wrap(&format!("    a {arrow} b\n"));
        assert_eq!(fixes_of(&src), vec![(MS, vec![])], "{arrow}");
        assert!(!check(&src).valid, "{arrow}");
    }
}

#[test]
fn mermaid_m2_pipe_label() {
    let src = wrap("    a -->|go now| b\n");
    let want = vec![pair("-->|go now|", "->"), pair("", ": \"go now\"")];
    assert_eq!(fixes_of(&src), vec![(MS, want)]);
    // Quotes are stripped, `<br>` becomes the newline escape, backslash and quote are escaped.
    let q = fixes_of(&wrap("    a -->|\"quoted\"| b\n"));
    assert_eq!(q[0].1[1], pair("", ": \"quoted\""));
    let br = fixes_of(&wrap("    a -->|one<br>two| b\n"));
    assert_eq!(br[0].1[1], pair("", ": \"one\\ntwo\""));
    let esc = fixes_of(&wrap("    a -->|x\\y \"z\"| b\n"));
    assert_eq!(esc[0].1[1], pair("", ": \"x\\\\y \\\"z\\\"\""));
    let g = repaired_graph(&wrap("    a -->|one<br>two| b\n"));
    assert_eq!(g.edges[0].label.as_deref(), Some("one\ntwo"));
}

#[test]
fn mermaid_m2_label_goes_after_a_bracketed_target() {
    let g = repaired_graph("graph {\n    node a: \"A\"\n    a -->|go| B[Lab]\n}\n");
    assert_eq!(g.edges[0].label.as_deref(), Some("go"));
    assert!(g.nodes.iter().any(|n| n.id == "B" && n.label == "Lab"));
}

#[test]
fn mermaid_m2_dash_label() {
    let src = wrap("    a -- go now --> b\n");
    let want = vec![pair("-- go now -->", "->"), pair("", ": \"go now\"")];
    assert_eq!(fixes_of(&src), vec![(MS, want)]);
    let g = repaired_graph(&src);
    assert_eq!(g.edges[0].label.as_deref(), Some("go now"));
    // A pipe or dash label on a styled link has no fix.
    assert_eq!(fixes_of(&wrap("    a ==>|x| b\n")), vec![(MS, vec![])]);
}

#[test]
fn mermaid_m3_statement_bracket_forms() {
    let src = "graph {\n    X[Plain]\n    R(Round)\n    D{Pick}\n    C[(Store)]\n}\n";
    let want = [
        ("X[Plain]", "node X: \"Plain\""),
        ("R(Round)", "node R: \"Round\" { shape: rounded }"),
        ("D{Pick}", "node D: \"Pick\" { shape: diamond }"),
        ("C[(Store)]", "node C: \"Store\" { shape: cylinder }"),
    ];
    let got = fixes_of(src);
    assert_eq!(got.len(), 4);
    for ((code, e), (orig, repl)) in got.iter().zip(want) {
        assert_eq!((*code, e.clone()), (MS, vec![pair(orig, repl)]));
    }
    let g = repaired_graph(src);
    let shapes: Vec<Shape> = g.nodes.iter().map(|n| n.shape).collect();
    assert_eq!(
        shapes,
        [Shape::Rect, Shape::Rounded, Shape::Diamond, Shape::Cylinder]
    );
}

#[test]
fn mermaid_m3_edge_bracket_forms() {
    let src = "graph {\n    A[Start] -> B(End)\n}\n";
    let got = edits(src, MS);
    let decl_a = pair("", "\n    node A: \"Start\"");
    let decl_b = pair("", "\n    node B: \"End\" { shape: rounded }");
    assert_eq!(got[0], vec![pair("A[Start]", "A"), decl_a]);
    assert_eq!(got[1], vec![pair("B(End)", "B"), decl_b]);
    // No `undeclared-node`: the brackets declare the nodes.
    assert!(check(src).diagnostics.iter().all(|d| d.code == MS));
    let g = repaired_graph(src);
    let labels: Vec<&str> = g.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(labels, ["Start", "End"]);
    assert_eq!(
        (g.edges[0].from.as_str(), g.edges[0].to.as_str()),
        ("A", "B")
    );
}

#[test]
fn mermaid_m3_quoted_and_punctuated_labels() {
    let src = "graph {\n    A[\"Quoted\"]\n    P[API: gw (beta) / x.y]\n    L[one<br>two]\n    Q[say \"hi\"]\n}\n";
    let want = [
        "node A: \"Quoted\"",
        "node P: \"API: gw (beta) / x.y\"",
        "node L: \"one\\ntwo\"",
        "node Q: \"say \\\"hi\\\"\"",
    ];
    let got = fixes_of(src);
    for ((_, e), repl) in got.iter().zip(want) {
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].1, repl);
    }
    let g = repaired_graph(src);
    let labels: Vec<&str> = g.nodes.iter().map(|n| n.label.as_str()).collect();
    assert_eq!(
        labels,
        ["Quoted", "API: gw (beta) / x.y", "one\ntwo", "say \"hi\""]
    );
}

#[test]
fn mermaid_m3_unfixable_shapes() {
    let src = "graph {\n    A((Circle))\n    S([Stadium])\n    U[[Sub]]\n    H{{Hex}}\n    F>Flag]\n    O[Open\n}\n";
    let got = fixes_of(src);
    assert_eq!(got.len(), 6, "{got:?}");
    assert!(got.iter().all(|(c, e)| *c == MS && e.is_empty()), "{got:?}");
}

#[test]
fn mermaid_m3_repeat_same_and_different() {
    let src = "graph {\n    X[One] -> Y\n    X[One] -> Y\n    X[Two] -> Y\n}\n";
    let got = edits(src, MS);
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].len(), 2, "first occurrence replaces and declares");
    assert_eq!(
        got[1],
        vec![pair("X[One]", "X")],
        "same label and shape: bare id only"
    );
    assert!(
        got[2].is_empty(),
        "a different label never silently drops text"
    );
    let dups = check(src)
        .diagnostics
        .iter()
        .filter(|d| d.code == codes::DUPLICATE_ID)
        .count();
    assert_eq!(dups, 1);
}

#[test]
fn mermaid_m4_subgraph_forms() {
    let src = "graph {\n    subgraph one\n        node a: \"A\"\n    end\n    subgraph two[\"Second\"]\n        node b: \"B\"\n    end\n    subgraph three[Third One]\n        node c: \"C\"\n    end\n}\n";
    let want = [
        ("subgraph one", "container one: \"one\" {"),
        ("subgraph two[\"Second\"]", "container two: \"Second\" {"),
        (
            "subgraph three[Third One]",
            "container three: \"Third One\" {",
        ),
    ];
    let got = edits(src, MS);
    assert_eq!(got.len(), 3);
    for (e, (orig, repl)) in got.iter().zip(want) {
        assert_eq!(*e, vec![pair(orig, repl), pair("end", "}")]);
    }
    let g = repaired_graph(src);
    assert_eq!(g.containers.len(), 3);
    assert_eq!(g.containers[1].label, "Second");
    assert_eq!(g.nodes[2].id, "three.c");
}

#[test]
fn mermaid_m4_title_only_no_fix() {
    let src = "graph {\n    subgraph My Title\n        node a: \"A\"\n    end\n}\n";
    assert_eq!(fixes_of(src), vec![(MS, vec![])]);
}

#[test]
fn mermaid_m5_headers() {
    let cases = [
        ("graph TD", "graph {\n    direction: down"),
        ("graph TB;", "graph {\n    direction: down"),
        ("flowchart LR", "graph {\n    direction: right"),
        ("flowchart RL", "graph {\n    direction: left"),
        ("graph BT", "graph {\n    direction: up"),
        ("graph", "graph {"),
    ];
    for (header, repl) in cases {
        let src = format!("{header}\n    node a: \"A\"\n");
        assert_eq!(
            fixes_of(&src),
            vec![(MS, vec![pair(header, repl), pair("", "}\n")])],
            "{header}"
        );
        assert!(repair(&src, 5).report.valid, "{header}");
    }
    let bare = "graph LR\n    node a: \"A\"";
    assert_eq!(
        edits(bare, MS)[0][1],
        pair("", "\n}\n"),
        "no trailing newline in the source"
    );
    assert_eq!(
        repaired_graph("graph LR\n    node a: \"A\"\n").direction,
        Direction::Right
    );
}

#[test]
fn mermaid_m6_direction_forms() {
    let cases = [
        ("direction LR", "right"),
        ("direction: LR", "right"),
        ("direction TB", "down"),
        ("direction RL", "left"),
        ("direction BT", "up"),
    ];
    for (text, word) in cases {
        let src = format!("graph {{\n    {text}\n    node a: \"A\"\n}}\n");
        let want = vec![(MS, vec![pair(text, &format!("direction: {word}"))])];
        assert_eq!(fixes_of(&src), want, "{text}");
        assert!(repair(&src, 5).report.valid, "{text}");
    }
}

#[test]
fn mermaid_m7_percent_comment() {
    let src = "graph {\n    %% a note\n    node a: \"A\" %% trailing\n}\n";
    assert_eq!(
        fixes_of(src),
        vec![(MS, vec![pair("%%", "//")]), (MS, vec![pair("%%", "//")])]
    );
    let r = repair(src, 5);
    assert!(r.report.valid);
    assert!(
        r.text.contains("    // a note\n") && r.text.contains("\"A\" // trailing"),
        "{}",
        r.text
    );
}

#[test]
fn mermaid_m7_directive_no_fix() {
    let src = "graph {\n    %%{init: {\"theme\": \"dark\"}}%%\n    node a: \"A\"\n}\n";
    assert_eq!(fixes_of(src), vec![(MS, vec![])]);
}

#[test]
fn mermaid_m8_semicolons() {
    let src = "graph {\n    node a: \"A\";\n    node b: \"B\"; node c: \"C\"\n}\n";
    assert_eq!(
        fixes_of(src),
        vec![(MS, vec![pair(";", "")]), (MS, vec![pair("; ", "\n    ")])]
    );
    assert_eq!(repaired_graph(src).nodes.len(), 3);
}

#[test]
fn mermaid_m9_styling_statements() {
    let lines = [
        "style a fill:#f9f,stroke:#333,stroke-width:2px",
        "classDef hot fill:#f96",
        "class a hot",
        "linkStyle 0 stroke:red",
        "click a callback \"tooltip\"",
        "callback a",
        "accTitle: A title",
        "accDescr: A description",
    ];
    let src = wrap(&lines.map(|l| format!("    {l}\n")).concat());
    let got = fixes_of(&src);
    assert_eq!(got.len(), lines.len(), "{got:?}");
    assert!(got.iter().all(|(c, e)| *c == MS && e.is_empty()));
    let msg = &check(&src).diagnostics[0].message;
    assert_eq!(
        msg,
        "styling is theme-controlled; VoxDiagram has no equivalent"
    );
    let suffix = wrap("    a:::hot -> b\n");
    assert_eq!(fixes_of(&suffix), vec![(MS, vec![])]);
}

#[test]
fn mermaid_m9_class_id_in_edge_is_not_styling() {
    let src = "graph {\n    node class: \"C\"\n    node style: \"S\"\n    class -> style\n    style.x -> class\n}\n";
    // `style.x` has no container `style`: the error is the unresolved path, not styling.
    assert!(
        check(src).diagnostics.iter().all(|d| d.code != MS),
        "{:?}",
        check(src).diagnostics
    );
    let ok = "graph {\n    node class: \"C\"\n    node style: \"S\"\n    class -> style\n}\n";
    assert!(check(ok).valid);
    let long = "graph {\n    node class: \"C\"\n    node style: \"S\"\n    class --> style\n}\n";
    assert_eq!(fixes_of(long), vec![(MS, vec![pair("-->", "->")])]);
}

#[test]
fn mermaid_m10_ampersand_fanout() {
    let decls = "    node a: \"A\"\n    node b: \"B\"\n    node c: \"C\"\n";
    let run = |stmt: &str| format!("graph {{\n{decls}    {stmt}\n}}\n");
    let cases = [
        ("a & b -> c", "a & b -> c", "a -> c\n    b -> c"),
        ("a & b --> c", "a & b --> c", "a -> c\n    b -> c"),
        ("a --> b & c", "a --> b & c", "a -> b\n    a -> c"),
        (
            "a & b -->|x| c",
            "a & b -->|x| c",
            "a -> c: \"x\"\n    b -> c: \"x\"",
        ),
    ];
    for (stmt, orig, repl) in cases {
        let src = run(stmt);
        assert_eq!(fixes_of(&src), vec![(MS, vec![pair(orig, repl)])], "{stmt}");
        assert_eq!(repaired_graph(&src).edges.len(), 2, "{stmt}");
    }
}

#[test]
fn mermaid_m10_chained_edge() {
    let decls = "    node a: \"A\"\n    node b: \"B\"\n    node c: \"C\"\n";
    let run = |stmt: &str| format!("graph {{\n{decls}    {stmt}\n}}\n");
    for (stmt, repl) in [
        ("a -> b -> c", "a -> b\n    b -> c"),
        ("a --> b --> c", "a -> b\n    b -> c"),
        ("a -> b -> c: \"x\"", "a -> b\n    b -> c: \"x\""),
    ] {
        let src = run(stmt);
        assert_eq!(fixes_of(&src), vec![(CE, vec![pair(stmt, repl)])], "{stmt}");
        assert_eq!(repaired_graph(&src).edges.len(), 2, "{stmt}");
    }
}

#[test]
fn mermaid_every_diagnostic_is_an_error_without_ir() {
    for body in [
        "    a --> b\n",
        "    a & b -> a\n",
        "    %% c\n",
        "    style a fill:red\n",
    ] {
        let r = check(&wrap(body));
        assert!(!r.valid && r.ir.is_none(), "{body}");
        assert!(
            r.diagnostics.iter().all(|d| d.severity == Severity::Error),
            "{body}"
        );
    }
}

#[test]
fn mermaid_corpus_fixtures_repair() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/mermaid");
    let mut files: Vec<_> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{dir:?}: {e}"))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|x| x.to_str()) == Some("vdg"))
        .collect();
    files.sort();
    assert!(files.len() >= 5, "only {} fixtures", files.len());
    for path in files {
        let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
        let name = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        assert!(!check(&src).valid, "{name} is not Mermaid-flavoured");
        let r = repair(&src, 5);
        assert!(
            r.report.valid,
            "{name} -> {}\n{:?}",
            r.text, r.report.diagnostics
        );
        assert_eq!(
            repair(&r.text, 5).rounds,
            0,
            "{name}: repaired text is not stable"
        );
        insta::assert_snapshot!(name.as_str(), r.text);
    }
}
