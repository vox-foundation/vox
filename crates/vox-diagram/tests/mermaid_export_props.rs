//! Structural properties of `export` over arbitrary `GraphIr` values, including bogus container,
//! parent and endpoint references. The invariants are restated here independently of
//! `escape.rs`; acceptance by real Mermaid is the Node oracle's job (`export-hostile/`).

mod support;

use proptest::prelude::*;
use support::hostile::{FRAGMENTS, payload_problem};
use support::{model_graph, noise, noisy_source};
use vox_diagram::check;
use vox_diagram::ir::{Direction, GraphIr, Ir, IrContainer, IrEdge, IrNode, Shape};
use vox_diagram::mermaid::export;

/// Lowercase Mermaid words an emitted id must never equal.
const KEYWORDS: [&str; 18] = [
    "end",
    "graph",
    "flowchart",
    "subgraph",
    "style",
    "class",
    "classdef",
    "linkstyle",
    "click",
    "default",
    "interpolate",
    "call",
    "href",
    "direction",
    "_self",
    "_blank",
    "_parent",
    "_top",
];

const POOL: [&str; 26] = [
    "end",
    "End",
    "click",
    "default",
    "o",
    "x",
    "ox",
    "n0",
    "n7",
    "a-b",
    "\u{fc}",
    "a",
    "b",
    "c",
    "kb",
    "ok",
    "my_style",
    "graph",
    "ghost",
    "a.b",
    "",
    "k1",
    "_self",
    "_Top",
    "adirection",
    "LR1",
];

fn id() -> impl Strategy<Value = String> {
    prop_oneof![
        prop::sample::select(POOL.to_vec()).prop_map(str::to_owned),
        (0u8..40).prop_map(|d| format!("n{d}")),
    ]
}

fn text() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            "\\PC{0,6}",
            prop::sample::select(FRAGMENTS.to_vec()).prop_map(str::to_owned),
        ],
        0..4,
    )
    .prop_map(|v| v.concat())
}

fn shape() -> impl Strategy<Value = Shape> {
    prop::sample::select(vec![
        Shape::Rect,
        Shape::Rounded,
        Shape::Diamond,
        Shape::Cylinder,
        Shape::Start,
        Shape::End,
    ])
}

fn direction() -> impl Strategy<Value = Direction> {
    prop::sample::select(vec![
        Direction::Down,
        Direction::Right,
        Direction::Up,
        Direction::Left,
    ])
}

fn arbitrary_graph() -> impl Strategy<Value = GraphIr> {
    let node = (id(), id(), text(), shape(), prop::option::of(id())).prop_map(
        |(id, local_id, label, shape, container)| IrNode {
            id,
            local_id,
            label,
            shape,
            container,
            span: Default::default(),
        },
    );
    let container =
        (id(), id(), text(), prop::option::of(id())).prop_map(|(id, local_id, label, parent)| {
            IrContainer {
                id,
                local_id,
                label,
                parent,
                depth: 1,
                span: Default::default(),
            }
        });
    let edge = (id(), id(), prop::option::of(text())).prop_map(|(from, to, label)| IrEdge {
        from,
        to,
        label,
        span: Default::default(),
    });
    (
        direction(),
        prop::collection::vec(node, 0..8),
        prop::collection::vec(container, 0..6),
        prop::collection::vec(edge, 0..8),
    )
        .prop_map(|(direction, nodes, containers, edges)| GraphIr {
            direction,
            nodes,
            containers,
            edges,
        })
}

fn is_id_shaped(s: &str) -> bool {
    let mut c = s.chars();
    c.next()
        .is_some_and(|f| f.is_ascii_alphabetic() || f == '_')
        && c.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

fn assert_safe_id(s: &str) -> Result<(), TestCaseError> {
    let lower = s.to_ascii_lowercase();
    prop_assert!(is_id_shaped(s), "id shape: {s:?}");
    prop_assert!(!KEYWORDS.contains(&lower.as_str()), "keyword id {s:?}");
    // A line ending in `direction` and a next line starting `LR` are one statement to Mermaid.
    prop_assert!(!lower.ends_with("direction"), "direction-ending id {s:?}");
    prop_assert!(
        !lower.contains("style") && !lower.contains("classdef"),
        "style-ish id {s:?}"
    );
    prop_assert!(!s.starts_with(['o', 'x']), "edge-letter id {s:?}");
    Ok(())
}

fn assert_safe_payload(p: &str) -> Result<(), TestCaseError> {
    match payload_problem(p, false) {
        None => Ok(()),
        Some(why) => Err(TestCaseError::fail(why)),
    }
}

fn payload<'a>(line: &'a str, open: &str, close: &str) -> &'a str {
    let from = line.find(open).map_or(0, |i| i + open.len());
    let to = line.rfind(close).unwrap_or(line.len());
    line.get(from..to.max(from)).unwrap_or("")
}

fn check_export(g: &GraphIr) -> Result<(), TestCaseError> {
    let out = export(&Ir::Graph(g.clone()));
    prop_assert!(out.ends_with('\n'));
    let lines: Vec<&str> = out.lines().collect();
    // P1: exactly one line per statement.
    prop_assert_eq!(
        lines.len(),
        1 + g.nodes.len() + 2 * g.containers.len() + g.edges.len(),
        "{}",
        out
    );
    let (mut declared, mut subgraphs, mut ends, mut edges) = (Vec::new(), 0, 0, 0);
    for line in &lines[1..] {
        let t = line.trim_start();
        if t == "end" {
            ends += 1;
        } else if let Some(rest) = t.strip_prefix("subgraph ") {
            subgraphs += 1;
            let (head, _) = rest.split_once(" [\"").unwrap_or((rest, ""));
            declared.push(head.to_owned());
            assert_safe_payload(payload(t, " [\"", "\"]"))?;
        } else {
            let run = t
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(t.len());
            let (head, rest) = t.split_at(run);
            assert_safe_id(head)?;
            if rest.starts_with(" --> ") || rest.starts_with(" -->|\"") {
                edges += 1;
                let to = rest.rsplit(' ').next().unwrap_or("");
                assert_safe_id(to)?;
                if rest.starts_with(" -->|\"") {
                    assert_safe_payload(payload(rest, "|\"", "\"|"))?;
                }
            } else {
                prop_assert!(
                    rest.starts_with(['[', '(', '{']),
                    "unclassified line {line:?}"
                );
                declared.push(head.to_owned());
                assert_safe_payload(payload(t, "\"", "\""))?;
            }
        }
    }
    prop_assert_eq!(ends, g.containers.len());
    prop_assert_eq!(subgraphs, g.containers.len());
    prop_assert_eq!(edges, g.edges.len());
    // P3: node and subgraph ids are one namespace, unique and safe.
    for d in &declared {
        assert_safe_id(d)?;
    }
    let mut sorted = declared.clone();
    sorted.sort();
    sorted.dedup();
    prop_assert_eq!(sorted.len(), declared.len(), "duplicate id in {}", out);
    prop_assert_eq!(declared.len(), g.nodes.len() + g.containers.len());
    Ok(())
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    #[test]
    fn export_is_structurally_safe_for_arbitrary_irs(g in arbitrary_graph()) {
        check_export(&g)?;
    }

    #[test]
    fn export_of_checked_documents_is_safe_and_deterministic(
        m in model_graph(), n in noise()
    ) {
        let src = noisy_source(&m, &n);
        let Some(Ir::Graph(g)) = check(&src).ir else {
            return Err(TestCaseError::fail(format!("does not check:\n{src}")));
        };
        check_export(&g)?;
        prop_assert_eq!(export(&Ir::Graph(g.clone())), export(&Ir::Graph(g)));
    }
}
