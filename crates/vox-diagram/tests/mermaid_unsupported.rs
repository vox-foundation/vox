//! Fail-closed catalogue: every construct outside the documented flowchart subset is listed by
//! line under a stable name and the whole block is refused. `drop_styling` is the only switch.

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

fn expect(block: &str, want: &[(u32, &str)]) {
    let want: Vec<(u32, String)> = want.iter().map(|(l, c)| (*l, (*c).to_owned())).collect();
    assert_eq!(refused(block), want, "{block}");
}

/// A `flowchart TD` block whose body lines start at line 2.
fn flow(lines: &[&str]) -> String {
    format!("flowchart TD\n{}\n", lines.join("\n"))
}

/// Every body line is refused with the matching construct, in order, one per line.
fn each<L: AsRef<str>, C: AsRef<str>>(lines: &[L], constructs: &[C]) {
    assert_eq!(lines.len(), constructs.len());
    let lines: Vec<&str> = lines.iter().map(AsRef::as_ref).collect();
    let want: Vec<(u32, &str)> = (2u32..).zip(constructs.iter().map(AsRef::as_ref)).collect();
    expect(&flow(&lines), &want);
}

#[test]
fn unsupported_headers() {
    expect("gantt\n    title Plan\n", &[(1, "header:gantt")]);
    expect("classDiagram\n", &[(1, "header:classDiagram")]);
    expect("flowchart-elk TD\n", &[(1, "header:flowchart-elk")]);
    expect("stateDiagram\n", &[(1, "header:stateDiagram")]);
    expect("flowchart XY\n", &[(1, "header:flowchart")]);
    expect("graph TD;\n    a\n", &[(1, "semicolon")]);
    expect("\n\n  pie\n", &[(3, "header:pie")]);
}

#[test]
fn unsupported_styling_heads() {
    each(
        &[
            "style A fill:#f00",
            "classDef hot fill:#f00",
            "class A hot",
            "linkStyle 0 stroke:red",
            "click A callback",
            "callback A \"tip\"",
            "accTitle: A title",
            "accDescr: Some description",
            "A:::hot --> B",
        ],
        &[
            "styling:style",
            "styling:classDef",
            "styling:class",
            "styling:linkStyle",
            "styling:click",
            "styling:callback",
            "styling:accTitle",
            "styling:accDescr",
            "styling:shorthand",
        ],
    );
}

#[test]
fn unsupported_comments_directives_semicolons() {
    each(
        &[
            "%% a comment",
            "%%{init: {}}%%",
            "A --> B;",
            "A[x];",
            "A --> B %% trailing",
        ],
        &["comment", "directive", "semicolon", "semicolon", "comment"],
    );
}

#[test]
fn unsupported_edge_types() {
    let tokens = [
        "---", "--->", "-.->", "-.-", "==>", "===", "--o", "--x", "<-->", "o--o", "x--x", "~~~",
    ];
    let lines: Vec<String> = tokens.iter().map(|t| format!("A {t} B")).collect();
    let want: Vec<String> = tokens.iter().map(|t| format!("edge-type:{t}")).collect();
    each(&lines, &want);
    each(&["A<-->B", "A--oB"], &["edge-type:<-->", "edge-type:--o"]);
}

#[test]
fn unsupported_edge_label() {
    each(
        &["A -- no end", "A -->|open B", "A -- \"q\" B", "A -->|\"q B"],
        &["edge-label"; 4],
    );
}

#[test]
fn unsupported_shapes() {
    each(
        &[
            "A([x])",
            "A[[x]]",
            "A{{x}}",
            "A>x]",
            "A[/x/]",
            "A[\\x\\]",
            "A@{ shape: x }",
        ],
        &[
            "shape:([",
            "shape:[[",
            "shape:{{",
            "shape:>",
            "shape:[/",
            "shape:[\\",
            "shape:@{",
        ],
    );
}

#[test]
fn unsupported_label_syntax() {
    each(
        &[
            "A[a(b)]",
            "A[a|b]",
            "A[a@b]",
            "A[open",
            "A[\"x\" y]",
            "A -->|a(b)| B",
        ],
        &["label-syntax"; 6],
    );
}

#[test]
fn unsupported_markdown_string() {
    each(&["A[\"`x`\"]", "B -->|\"`y`\"| C"], &["markdown-string"; 2]);
}

#[test]
fn unsupported_html_label() {
    each(
        &[
            "A[a <b>x</b>]",
            "A[\"<img src=x>\"]",
            "A[</div>]",
            "A[<!-- c -->]",
        ],
        &["html-label"; 4],
    );
}

#[test]
fn unsupported_html_entity() {
    each(
        &["A[R&amp;D]", "A[\"&#65;\"]", "A[\"&#x41;\"]"],
        &["html-entity"; 3],
    );
}

#[test]
fn unsupported_entity() {
    each(&["A[\"#bogus;\"]", "A[\"#55296;\"]"], &["entity"; 2]);
}

#[test]
fn unsupported_forbidden_char() {
    each(
        &["A[\"a#13;b\"]", "A[\"a#0;b\"]", "A[\"a\u{202e}b\"]"],
        &["forbidden-char"; 3],
    );
}

#[test]
fn unsupported_empty_label() {
    expect(
        &flow(&[
            "A[]",
            "A[\" \"]",
            "A[\"\"]",
            "A[#32;]",
            "subgraph s []",
            "end",
        ]),
        &[
            (2, "empty-label"),
            (3, "empty-label"),
            (4, "empty-label"),
            (5, "empty-label"),
            (6, "empty-label"),
        ],
    );
}

#[test]
fn unsupported_label_too_long() {
    let long = "x".repeat(201);
    expect(
        &flow(&[
            &format!("A[\"{long}\"]"),
            &format!("subgraph s[\"{long}\"]"),
            "end",
        ]),
        &[(2, "label-too-long"), (3, "label-too-long")],
    );
    let ok = "x".repeat(200);
    assert!(
        import(&flow(&[&format!("A[\"{ok}\"]")]), ImportOpts::default())
            .source
            .is_some()
    );
}

#[test]
fn unsupported_ids() {
    each(
        &[
            "1a --> b",
            "\u{e9}t\u{e9} --> b",
            "node --> b",
            "direction --> b",
            "graph",
        ],
        &[
            "invalid-id",
            "invalid-id",
            "reserved-id",
            "reserved-id",
            "reserved-id",
        ],
    );
}

#[test]
fn unsupported_id_clash() {
    expect(
        &flow(&["subgraph s", "end", "subgraph s", "end", "s[Label]", "s"]),
        &[(4, "id-clash"), (6, "id-clash"), (7, "id-clash")],
    );
}

#[test]
fn unsupported_edge_to_subgraph() {
    expect(
        &flow(&["subgraph s", "a", "end", "a --> s", "s --> a"]),
        &[(5, "edge-to-subgraph"), (6, "edge-to-subgraph")],
    );
}

#[test]
fn unsupported_redeclared_node() {
    expect(
        &flow(&["A[One]", "A[One]", "A[Two]", "A(Two)", "B", "B[Late]"]),
        &[(4, "redeclared-node"), (5, "redeclared-node")],
    );
}

#[test]
fn unsupported_subgraph_title() {
    expect(
        &flow(&[
            "subgraph \"Title\"",
            "a",
            "end",
            "subgraph two words",
            "end",
            "subgraph",
            "end",
        ]),
        &[
            (2, "subgraph-title"),
            (5, "subgraph-title"),
            (7, "subgraph-title"),
        ],
    );
}

#[test]
fn unsupported_subgraph_direction_and_direction() {
    expect(
        &flow(&["subgraph s", "direction LR", "a", "end"]),
        &[(3, "subgraph-direction")],
    );
    expect(
        &flow(&["direction LR", "subgraph s", "direction XY", "a", "end"]),
        &[(2, "direction"), (4, "direction")],
    );
}

#[test]
fn unsupported_stray_end_and_unclosed_subgraph() {
    expect(&flow(&["a --> b", "end"]), &[(3, "stray-end")]);
    expect(
        &flow(&["subgraph a", "subgraph b", "x", "end", "subgraph c"]),
        &[(2, "unclosed-subgraph"), (6, "unclosed-subgraph")],
    );
}

#[test]
fn unsupported_statement() {
    each(
        &["no pe", "A &", "A & B", "A --> ", "(x)"],
        &["statement"; 5],
    );
}

#[test]
fn unsupported_lists_every_line() {
    let block = "flowchart TD\n    classDef hot fill:#f00\n    A --- B\n    %% note\n    C[\"`md`\"]\n    subgraph s\n    d\n";
    expect(
        block,
        &[
            (2, "styling:classDef"),
            (3, "edge-type:---"),
            (4, "comment"),
            (5, "markdown-string"),
            (6, "unclosed-subgraph"),
        ],
    );
}

#[test]
fn drop_styling_drops_and_lists() {
    let block = "flowchart TD\n    style A fill:#f00\n    classDef hot fill:#f00\n    class A hot\n    linkStyle 0 stroke:red\n    click A callback\n    A:::hot --> B\n";
    let r = import(block, ImportOpts { drop_styling: true });
    assert!(r.unsupported.is_empty(), "{:?}", r.unsupported);
    let got: Vec<(u32, &str)> = r
        .dropped
        .iter()
        .map(|d| (d.line, d.construct.as_str()))
        .collect();
    assert_eq!(
        got,
        vec![
            (2, "styling:style"),
            (3, "styling:classDef"),
            (4, "styling:class"),
            (5, "styling:linkStyle"),
            (6, "styling:click"),
            (7, "styling:shorthand"),
        ]
    );
    let src = r.source.unwrap_or_default();
    assert!(
        src.contains("A -> B") && src.contains("node A: \"A\""),
        "{src}"
    );
    assert_eq!(refused(block).len(), 6, "default options fail closed");
}

#[test]
fn drop_styling_does_not_drop_other_constructs() {
    let block = "flowchart TD\n    style A fill:#f00\n    A --- B\n    %% note\n";
    let r = import(block, ImportOpts { drop_styling: true });
    let got: Vec<(u32, &str)> = r
        .unsupported
        .iter()
        .map(|u| (u.line, u.construct.as_str()))
        .collect();
    assert_eq!(got, vec![(3, "edge-type:---"), (4, "comment")]);
    assert_eq!(r.dropped.len(), 1);
    assert_eq!(r.source, None);
}

fn nodes(n: usize) -> String {
    let body: String = (0..n).map(|i| format!("    n{i}\n")).collect();
    format!("flowchart TD\n{body}")
}

fn nested(depth: usize) -> String {
    let mut s = String::from("flowchart TD\n");
    for i in 0..depth {
        s.push_str(&format!("subgraph s{i}\n"));
    }
    s.push_str("x\n");
    s.push_str(&"end\n".repeat(depth));
    s
}

#[test]
fn limit_nodes_500_accepted_501_rejected() {
    assert!(import(&nodes(500), ImportOpts::default()).source.is_some());
    expect(&nodes(501), &[(502, "limit-nodes")]);
}

#[test]
fn limit_edges_1000_accepted_1001_rejected() {
    let edges = |n: usize| format!("flowchart TD\n{}", "    a --> b\n".repeat(n));
    assert!(import(&edges(1000), ImportOpts::default()).source.is_some());
    expect(&edges(1001), &[(1002, "limit-edges")]);
}

#[test]
fn limit_depth_4_accepted_5_rejected() {
    assert!(import(&nested(4), ImportOpts::default()).source.is_some());
    expect(&nested(5), &[(6, "limit-depth")]);
}

#[test]
fn limit_source_gate() {
    let big = format!("flowchart TD\n{}", "    a --> b\n".repeat(6000));
    expect(&big, &[(1, "limit-source")]);
}

#[test]
fn expansion_cap_101_edges_rejected_100_imported() {
    let fan = |n: usize| {
        let ids: Vec<String> = (0..n).map(|i| format!("a{i}")).collect();
        format!("flowchart TD\n    {} --> b\n", ids.join(" & "))
    };
    assert!(import(&fan(100), ImportOpts::default()).source.is_some());
    expect(&fan(101), &[(2, "expansion-cap")]);
    let group = |p: &str, n: usize| {
        (0..n)
            .map(|i| format!("{p}{i}"))
            .collect::<Vec<_>>()
            .join(" & ")
    };
    let chain = format!(
        "flowchart TD\n    {} --> {} --> {}\n",
        group("a", 10),
        group("b", 6),
        group("c", 7)
    );
    expect(&chain, &[(2, "expansion-cap")]);
}
