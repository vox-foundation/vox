//! The `direction` lexer hazard (84-REVIEW CR-01): Mermaid swallows any flowchart line holding
//! `direction`, whitespace, then `TB|TD|LR|RL|BT`, even inside a quoted label. The fixture
//! (`export-hostile/g16`) and the predicate here are independent of `escape.rs`.
#![allow(dead_code)]

use vox_diagram::ir::{GraphIr, Shape};

use super::hostile::{Box_, Edge_, Node_, build};

const LABELS: [&str; 7] = [
    "p direction LR q",
    "direction TB",
    "xdirection BT",
    "DIRECTION td x",
    "direction\tLR",
    "direction LRX",
    "direction",
];

/// g16: hostile node, edge and container labels, plus near misses that need no escaping.
pub fn graph() -> GraphIr {
    let boxes: [Box_; 1] = [("grp", "my direction RL box", None)];
    let ids = ["d0", "d1", "d2", "d3", "d4", "d5", "d6"];
    let mut nodes: Vec<Node_> = LABELS
        .iter()
        .zip(ids)
        .map(|(l, id)| (id, (*l).to_owned(), Shape::Rect, None))
        .collect();
    nodes.push(("in", "inside direction BT".to_owned(), Shape::Rect, Some(0)));
    let mut edges: Vec<Edge_> = (1..nodes.len()).map(|i| (i - 1, i, None)).collect();
    edges.push((0, 1, Some("go direction TB now".to_owned())));
    build(&boxes, &nodes, &edges)
}

/// True when `s` holds a direction statement (case-insensitive superset).
pub fn trap(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    lower.match_indices("direction").any(|(i, _)| {
        let tail = &lower[i + 9..];
        let rest = tail.trim_start();
        rest.len() < tail.len()
            && ["tb", "td", "lr", "rl", "bt"]
                .iter()
                .any(|t| rest.starts_with(t))
    })
}
