//! The importer's graph model and its scoped DSL printer. Mermaid ids are global, so every
//! node and container id is unique here and a relative dotted path never shadows another id.

use std::fmt::Write as _;

use crate::ir::{Direction, Shape};
use crate::print::{push_quoted, shape_word};

pub(super) struct GNode {
    pub id: String,
    pub label: String,
    pub shape: Shape,
}

pub(super) struct GContainer {
    pub id: String,
    pub title: String,
    pub parent: Option<usize>,
}

pub(super) struct GEdge {
    pub from: usize,
    pub to: usize,
    pub label: Option<String>,
}

/// `nodes` are in global first-mention order; `node_container[i]` is the container owning
/// node `i` (`None` = top level); containers are in open order.
pub(super) struct Graph {
    pub direction: Direction,
    pub nodes: Vec<GNode>,
    pub containers: Vec<GContainer>,
    pub node_container: Vec<Option<usize>>,
    pub edges: Vec<GEdge>,
}

/// Containers from the outermost down to (and including) `c`.
fn chain(g: &Graph, c: Option<usize>) -> Vec<usize> {
    let mut v = Vec::new();
    let mut cur = c;
    while let Some(i) = cur {
        v.push(i);
        cur = g.containers[i].parent;
    }
    v.reverse();
    v
}

/// Per-scope statement lists; scope 0 is the top level and container `i` is scope `i + 1`.
struct Scopes {
    members: Vec<Vec<usize>>,
    children: Vec<Vec<usize>>,
    edges: Vec<Vec<String>>,
}

fn dotted(g: &Graph, path: &[usize], node: usize) -> String {
    let mut s = String::new();
    for &c in path {
        s.push_str(&g.containers[c].id);
        s.push('.');
    }
    s.push_str(&g.nodes[node].id);
    s
}

fn scopes(g: &Graph) -> Scopes {
    let n = g.containers.len() + 1;
    let mut sc = Scopes {
        members: vec![Vec::new(); n],
        children: vec![Vec::new(); n],
        edges: vec![Vec::new(); n],
    };
    for (i, c) in g.node_container.iter().enumerate() {
        sc.members[c.map_or(0, |c| c + 1)].push(i);
    }
    for (i, c) in g.containers.iter().enumerate() {
        sc.children[c.parent.map_or(0, |p| p + 1)].push(i);
    }
    for e in &g.edges {
        let (a, b) = (
            chain(g, g.node_container[e.from]),
            chain(g, g.node_container[e.to]),
        );
        let k = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let scope = if k == 0 { 0 } else { a[k - 1] + 1 };
        let mut line = format!(
            "{} -> {}",
            dotted(g, &a[k..], e.from),
            dotted(g, &b[k..], e.to)
        );
        if let Some(l) = &e.label {
            line.push_str(": ");
            push_quoted(&mut line, l);
        }
        sc.edges[scope].push(line);
    }
    sc
}

fn emit(out: &mut String, g: &Graph, sc: &Scopes, scope: usize, level: usize) {
    let pad = "    ".repeat(level);
    for &i in &sc.members[scope] {
        let n = &g.nodes[i];
        let _ = write!(out, "{pad}node {}: ", n.id);
        push_quoted(out, &n.label);
        if n.shape != Shape::Rect {
            let _ = write!(out, " {{ shape: {} }}", shape_word(n.shape));
        }
        out.push('\n');
    }
    for &c in &sc.children[scope] {
        let _ = write!(out, "{pad}container {}: ", g.containers[c].id);
        push_quoted(out, &g.containers[c].title);
        out.push_str(" {\n");
        emit(out, g, sc, c + 1, level + 1);
        let _ = writeln!(out, "{pad}}}");
    }
    for line in &sc.edges[scope] {
        let _ = writeln!(out, "{pad}{line}");
    }
}

/// Unformatted DSL for `g`: members first, then child containers, then the edges whose
/// deepest common scope this is, endpoints relative to that scope. The caller runs `fmt`.
pub(super) fn to_dsl(g: &Graph) -> String {
    let mut out = String::from("graph {\n");
    match g.direction {
        Direction::Down => {}
        Direction::Right => out.push_str("    direction: right\n"),
        Direction::Up => out.push_str("    direction: up\n"),
        Direction::Left => out.push_str("    direction: left\n"),
    }
    emit(&mut out, g, &scopes(g), 0, 1);
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str) -> GNode {
        GNode {
            id: id.to_owned(),
            label: id.to_uppercase(),
            shape: Shape::Rect,
        }
    }

    #[test]
    fn edge_lands_in_the_deepest_common_scope_with_relative_paths() {
        let g = Graph {
            direction: Direction::Down,
            nodes: vec![node("a"), node("b"), node("c")],
            containers: vec![
                GContainer {
                    id: "p".into(),
                    title: "P".into(),
                    parent: None,
                },
                GContainer {
                    id: "q".into(),
                    title: "Q".into(),
                    parent: Some(0),
                },
            ],
            node_container: vec![Some(0), Some(1), None],
            edges: vec![
                GEdge {
                    from: 0,
                    to: 1,
                    label: None,
                },
                GEdge {
                    from: 2,
                    to: 1,
                    label: Some("x".into()),
                },
            ],
        };
        let want = "graph {\n    node c: \"C\"\n    container p: \"P\" {\n        node a: \"A\"\n        container q: \"Q\" {\n            node b: \"B\"\n        }\n        a -> q.b\n    }\n    c -> p.q.b: \"x\"\n}\n";
        assert_eq!(to_dsl(&g), want);
    }
}
