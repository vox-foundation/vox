//! Hand-built hostile `GraphIr` cases for the exporter (`export-hostile/g01..g15`). The labels
//! and ids here are the ones that broke a naive emitter in research (84-RESEARCH).
#![allow(dead_code)]

use vox_diagram::ir::{Direction, GraphIr, IrContainer, IrEdge, IrNode, Shape};

/// Fragments shared by the case table and the property generators.
pub const FRAGMENTS: &[&str] = &[
    "\"",
    "#quot;",
    "&amp;",
    "#35;",
    "&",
    "#",
    "&#60;",
    "<script>alert(1)</script>",
    "<img src=x onerror=alert(1)>",
    "<br>",
    "</end>",
    "`",
    "style: say \"hi\"",
    "classDef x:#f00;",
    "%%{init: {}}%%",
    "100%",
    ";",
    "|",
    "[]",
    "{}",
    "()",
    "-->",
    "---",
    "end",
    "subgraph",
    "click a call alert(1)",
    "\n",
    "\r",
    "\t",
    "\u{2028}",
    "\u{2029}",
    "\u{fb02}\u{b0}",
    "\u{b6}\u{df}",
    "\u{1}",
    "\u{7f}",
    "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467}",
    "\u{65e5}\u{672c}\u{8a9e}",
    "\"] ; click n0 call alert(1) [\"",
];

/// A container: local id, label, parent index into the same list.
pub type Box_ = (&'static str, &'static str, Option<usize>);
/// A node: local id, label, shape, container index.
pub type Node_ = (&'static str, String, Shape, Option<usize>);
/// An edge: from node index, to node index, label.
pub type Edge_ = (usize, usize, Option<String>);

/// Build a `GraphIr` from compact tuples. Paths join local ids with `.`; spans are default.
pub fn build(boxes: &[Box_], nodes: &[Node_], edges: &[Edge_]) -> GraphIr {
    let mut paths: Vec<String> = Vec::new();
    for (local, _, parent) in boxes {
        let path = match parent {
            Some(p) => format!("{}.{local}", paths[*p]),
            None => (*local).to_owned(),
        };
        paths.push(path);
    }
    let containers = boxes
        .iter()
        .enumerate()
        .map(|(k, (local, label, parent))| IrContainer {
            id: paths[k].clone(),
            local_id: (*local).to_owned(),
            label: (*label).to_owned(),
            parent: parent.map(|p| paths[p].clone()),
            depth: 1,
            span: Default::default(),
        })
        .collect();
    let node_paths: Vec<String> = nodes
        .iter()
        .map(|(local, _, _, c)| match c {
            Some(c) => format!("{}.{local}", paths[*c]),
            None => (*local).to_owned(),
        })
        .collect();
    let ir_nodes = nodes
        .iter()
        .enumerate()
        .map(|(i, (local, label, shape, c))| IrNode {
            id: node_paths[i].clone(),
            local_id: (*local).to_owned(),
            label: label.clone(),
            shape: *shape,
            container: c.map(|c| paths[c].clone()),
            span: Default::default(),
        })
        .collect();
    let ir_edges = edges
        .iter()
        .map(|(f, t, label)| IrEdge {
            from: node_paths[*f].clone(),
            to: node_paths[*t].clone(),
            label: label.clone(),
            span: Default::default(),
        })
        .collect();
    GraphIr {
        direction: Direction::Down,
        nodes: ir_nodes,
        containers,
        edges: ir_edges,
    }
}

/// Flat nodes (ids `k<i>`, rect, top level) chained by edges with the given labels.
fn flat(labels: &[&str]) -> GraphIr {
    const IDS: [&str; 12] = [
        "k0", "k1", "k2", "k3", "k4", "k5", "k6", "k7", "k8", "k9", "k10", "k11",
    ];
    let nodes: Vec<Node_> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| (IDS[i], (*l).to_owned(), Shape::Rect, None))
        .collect();
    let edges: Vec<Edge_> = (1..nodes.len()).map(|i| (i - 1, i, None)).collect();
    build(&[], &nodes, &edges)
}

fn rect(local: &'static str, label: &str, c: Option<usize>) -> Node_ {
    (local, label.to_owned(), Shape::Rect, c)
}

fn chain(nodes: &[Node_]) -> Vec<Edge_> {
    (1..nodes.len()).map(|i| (i - 1, i, None)).collect()
}

/// g01: ids Mermaid treats as keywords, as nodes and as subgraphs.
fn keyword_ids() -> GraphIr {
    let words = [
        "end",
        "End",
        "END",
        "graph",
        "subgraph",
        "style",
        "class",
        "classDef",
        "linkStyle",
        "click",
        "default",
        "interpolate",
        "call",
        "href",
        "direction",
        "flowchart",
    ];
    let mut nodes: Vec<Node_> = words
        .iter()
        .map(|w| rect(w, &format!("kw {w}"), None))
        .collect();
    let boxes: [Box_; 4] = [
        ("click", "Click box", None),
        ("default", "Default box", None),
        ("interpolate", "Interpolate box", Some(0)),
        ("end", "End box", Some(1)),
    ];
    for (i, c) in [Some(0usize), Some(1), Some(2), Some(3)]
        .into_iter()
        .enumerate()
    {
        nodes.push(rect(
            ["in0", "in1", "in2", "in3"][i],
            &format!("inside {i}"),
            c,
        ));
    }
    let mut edges = chain(&nodes);
    edges.push((0, nodes.len() - 1, Some("to the end box".into())));
    build(&boxes, &nodes, &edges)
}

/// g02: ids that would fuse with an adjacent edge arrow (`a---ob`, `a--xb`).
fn edge_letter_ids() -> GraphIr {
    let ids = ["o", "x", "ox", "xo", "oa", "xa", "o1", "X", "O"];
    let nodes: Vec<Node_> = ids
        .iter()
        .map(|i| rect(i, &format!("id {i}"), None))
        .collect();
    let mut edges = chain(&nodes);
    edges.push((1, 0, None));
    edges.push((3, 2, Some("back".into())));
    build(&[], &nodes, &edges)
}

/// g03: ids that collide with the exporter's own `n<k>` fallbacks or are not Mermaid-shaped.
fn pattern_ids() -> GraphIr {
    let boxes: [Box_; 1] = [("grp", "Group", None)];
    let nodes: Vec<Node_> = vec![
        rect("n0", "first n0", None),
        rect("n1", "first n1", None),
        rect("n1", "second n1", Some(0)),
        rect("a-b", "kebab", None),
        rect("_", "underscore", None),
        rect("my_style", "style-ish", None),
        rect("\u{fc}", "umlaut", None),
        rect("n2", "third", None),
    ];
    let edges = chain(&nodes);
    build(&boxes, &nodes, &edges)
}

/// g14: all six shapes, each with a hostile label.
fn shapes() -> GraphIr {
    let kinds = [
        Shape::Rect,
        Shape::Rounded,
        Shape::Diamond,
        Shape::Cylinder,
        Shape::Start,
        Shape::End,
    ];
    let labels = ["a\"b", "<b>x</b>", "#35;", "end", "`tick`", "style: x"];
    let ids = ["s0", "s1", "s2", "s3", "s4", "s5"];
    let nodes: Vec<Node_> = (0..6)
        .map(|i| (ids[i], labels[i].to_owned(), kinds[i], None))
        .collect();
    let edges = chain(&nodes);
    build(&[], &nodes, &edges)
}

/// g15: edge labels drawn from the other cases, plus empty and blank.
fn edge_labels() -> GraphIr {
    let labels: [Option<&str>; 10] = [
        Some("say \"hi\" #quot;"),
        Some("<script>x</script>"),
        Some("`tick`"),
        Some("style: a"),
        Some("%%{init}"),
        Some("a|b;c"),
        Some("two\nlines"),
        Some(""),
        Some("   "),
        None,
    ];
    let ids = [
        "e0", "e1", "e2", "e3", "e4", "e5", "e6", "e7", "e8", "e9", "e10",
    ];
    let nodes: Vec<Node_> = ids.iter().map(|i| rect(i, i, None)).collect();
    let edges: Vec<Edge_> = labels
        .iter()
        .enumerate()
        .map(|(i, l)| (i, i + 1, l.map(str::to_owned)))
        .collect();
    build(&[], &nodes, &edges)
}

/// The `g01..g15` case table (`g00-nested` lives in `mermaid_export.rs`, built from DSL).
pub fn graph_cases() -> Vec<(&'static str, GraphIr)> {
    vec![
        ("g01-keyword-ids", keyword_ids()),
        ("g02-edge-letter-ids", edge_letter_ids()),
        ("g03-pattern-ids", pattern_ids()),
        (
            "g04-quotes-entities",
            flat(&["\"", "#quot;", "&amp;", "#35;", "&", "#", "&#60;"]),
        ),
        (
            "g05-html",
            flat(&[
                "<script>alert(1)</script>",
                "<img src=x onerror=alert(1)>",
                "literal <br> tag",
                "</end>",
            ]),
        ),
        (
            "g06-backticks",
            flat(&["`first", "last`", "mid`dle", "`whole label`"]),
        ),
        (
            "g07-colon-guard",
            flat(&[
                "style: say \"hi\"",
                "classDef x:#f00;",
                "style a fill:#f00;",
            ]),
        ),
        ("g08-percent", flat(&["%%{init: {}}%%", "100%", "%%"])),
        (
            "g09-punctuation",
            flat(&[
                ";",
                "|",
                "[]",
                "{}",
                "()",
                "-->",
                "---",
                "end",
                "subgraph",
                "click a call alert(1)",
            ]),
        ),
        (
            "g10-whitespace",
            flat(&[
                " leading",
                "trailing ",
                "tab\there",
                "\nfirst",
                "last\n",
                "mid\nline",
                "cr\rx",
                "",
                "   ",
            ]),
        ),
        (
            "g11-unicode",
            flat(&[
                "a\u{2028}b",
                "a\u{2029}b",
                "\u{fb02}\u{b0}",
                "\u{b6}\u{df}",
                "\u{1f468}\u{200d}\u{1f469}\u{200d}\u{1f467} family",
                "\u{65e5}\u{672c}\u{8a9e}",
                "a\u{1}b",
                "a\u{7f}b",
            ]),
        ),
        (
            "g12-long",
            flat(&[
                "word ".repeat(40).as_str(),
                "l0\nl1\nl2\nl3\nl4\nl5\nl6\nl7\nl8\nl9",
            ]),
        ),
        (
            "g13-injection",
            flat(&[
                "\"] ; click n0 call alert(1) [\"",
                "a --> b",
                "\"]\nend\nsubgraph x[\"",
            ]),
        ),
        ("g14-shapes", shapes()),
        ("g15-edge-labels", edge_labels()),
    ]
}

/// Why `p` is not a safe encoded Mermaid label payload, or `None` when it is. Safe means: no raw
/// `"`, `<`, `>`, `&`, backtick or control character (apart from `<br>`, the newline encoding),
/// every `#` opens an entity, and, when `no_semicolon`, no `;` outside an entity terminator.
pub fn payload_problem(p: &str, no_semicolon: bool) -> Option<String> {
    let bare = p.replace("<br>", "");
    if let Some(c) = bare
        .chars()
        .find(|c| matches!(c, '"' | '<' | '>' | '&' | '`') || c.is_control())
    {
        return Some(format!("raw {c:?} in {p:?}"));
    }
    let mut left = String::new();
    let mut rest = bare.as_str();
    while let Some(at) = rest.find('#') {
        left.push_str(&rest[..at]);
        let tail = &rest[at + 1..];
        let named = ["amp;", "quot;", "lt;", "gt;"]
            .iter()
            .find(|n| tail.starts_with(**n));
        let digits = tail
            .find(';')
            .filter(|e| *e > 0 && tail[..*e].bytes().all(|b| b.is_ascii_digit()));
        let len = match (named, digits) {
            (Some(n), _) => n.len(),
            (None, Some(e)) => e + 1,
            (None, None) => return Some(format!("bare # in {p:?}")),
        };
        rest = &tail[len..];
    }
    left.push_str(rest);
    (no_semicolon && left.contains(';')).then(|| format!("raw ; in {p:?}"))
}
