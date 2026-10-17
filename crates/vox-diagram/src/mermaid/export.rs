//! Graph to Mermaid `flowchart` text: header, node declarations, then every edge.

use std::collections::{BTreeMap, BTreeSet};

use super::escape::{keeps_id, label};
use crate::ir::{Direction, GraphIr, Ir, Shape};

/// Export `ir` as Mermaid text, one statement per line (4 spaces per nesting level).
///
/// Total over any `Ir`: an edge endpoint that is not a node is emitted as an implicit Mermaid
/// node, and containers are the subgraph slice of plan 84-02 (flat graphs only for now).
/// Mermaid's default configuration rejects more than 500 edges.
pub fn export(ir: &Ir) -> String {
    match ir {
        Ir::Graph(g) => graph(g),
    }
}

fn header(d: Direction) -> &'static str {
    match d {
        Direction::Down => "flowchart TD",
        Direction::Right => "flowchart LR",
        Direction::Up => "flowchart BT",
        Direction::Left => "flowchart RL",
    }
}

fn brackets(s: Shape) -> (&'static str, &'static str) {
    match s {
        Shape::Rect => ("[\"", "\"]"),
        Shape::Rounded => ("(\"", "\")"),
        Shape::Diamond => ("{\"", "\"}"),
        Shape::Cylinder => ("[(\"", "\")]"),
        Shape::Start => ("((\"", "\"))"),
        Shape::End => ("(((\"", "\")))"),
    }
}

/// Hands out Mermaid ids: `local_id` when safe and untaken, else `n<index>`.
struct Ids {
    taken: BTreeSet<String>,
    by_path: BTreeMap<String, String>,
}

impl Ids {
    fn assign(&mut self, path: &str, local: &str, index: usize) -> String {
        let id = if keeps_id(local) && !self.taken.contains(local) {
            local.to_owned()
        } else {
            format!("n{index}")
        };
        self.taken.insert(id.clone());
        self.by_path
            .entry(path.to_owned())
            .or_insert_with(|| id.clone());
        id
    }
}

fn graph(g: &GraphIr) -> String {
    let mut ids = Ids {
        taken: BTreeSet::new(),
        by_path: BTreeMap::new(),
    };
    let mut out = String::new();
    out.push_str(header(g.direction));
    out.push('\n');
    for (i, n) in g.nodes.iter().enumerate() {
        let id = ids.assign(&n.id, &n.local_id, i);
        let (open, close) = brackets(n.shape);
        out.push_str(&format!("    {id}{open}{}{close}\n", label(&n.label)));
    }
    let mut next = g.nodes.len() + g.containers.len();
    for e in &g.edges {
        let mut end = |path: &str| {
            if let Some(id) = ids.by_path.get(path) {
                return id.clone();
            }
            next += 1;
            ids.assign(path, path, next - 1)
        };
        let (from, to) = (end(&e.from), end(&e.to));
        out.push_str(&match e.label.as_deref() {
            Some(l) if !l.is_empty() => format!("    {from} -->|\"{}\"| {to}\n", label(l)),
            _ => format!("    {from} --> {to}\n"),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check;

    fn export_src(src: &str) -> String {
        let Some(ir) = check(src).ir else {
            panic!("source does not check:\n{src}");
        };
        export(&ir)
    }

    #[test]
    fn all_six_shapes_print_their_brackets() {
        let src = "graph {\n    node a: \"A\"\n    node b: \"B\" { shape: rounded }\n    node c: \"C\" { shape: diamond }\n    node d: \"D\" { shape: cylinder }\n    node e: \"E\" { shape: start }\n    node f: \"F\" { shape: end }\n}\n";
        let want = "flowchart TD\n    a[\"A\"]\n    b(\"B\")\n    c{\"C\"}\n    d[(\"D\")]\n    e((\"E\"))\n    f(((\"F\")))\n";
        assert_eq!(export_src(src), want);
    }

    #[test]
    fn direction_maps_to_the_header() {
        let src = "graph {\n    direction: right\n    node a: \"A\"\n}\n";
        assert!(export_src(src).starts_with("flowchart LR\n"));
    }

    #[test]
    fn edge_labels_print_and_empty_is_omitted() {
        let labelled = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b: \"go\"\n}\n";
        assert!(export_src(labelled).ends_with("    a -->|\"go\"| b\n"));
        let Some(Ir::Graph(mut g)) = check(labelled).ir else {
            panic!("does not check");
        };
        g.edges[0].label = Some(String::new());
        assert!(export(&Ir::Graph(g)).ends_with("    a --> b\n"));
    }

    #[test]
    fn unsafe_and_duplicate_ids_fall_back_to_index() {
        let src = "graph {\n    node end: \"E\"\n    node o: \"O\"\n    node kb: \"K\"\n    container c: \"C\" {\n        node kb: \"K2\"\n    }\n}\n";
        let out = export_src(src);
        assert!(out.contains("    n0[\"E\"]\n"), "{out}");
        assert!(out.contains("    n1[\"O\"]\n"), "{out}");
        assert!(out.contains("    kb[\"K\"]\n"), "{out}");
        // Second `kb` is a duplicate of the first in Mermaid's single namespace.
        assert!(out.contains("    n3[\"K2\"]\n"), "{out}");
    }

    #[test]
    fn dangling_endpoint_is_an_implicit_node() {
        let Some(Ir::Graph(mut g)) = check("graph {\n    node a: \"A\"\n}\n").ir else {
            panic!("does not check");
        };
        g.edges.push(crate::ir::IrEdge {
            from: "a".into(),
            to: "ghost".into(),
            label: None,
            span: Default::default(),
        });
        assert_eq!(
            export(&Ir::Graph(g)),
            "flowchart TD\n    a[\"A\"]\n    a --> ghost\n"
        );
    }
}
