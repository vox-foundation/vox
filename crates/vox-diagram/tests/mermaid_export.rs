//! Exporter behaviour: subgraph nesting, the global id namespace, totality over hand-built IRs
//! and the hostile goldens the Node oracle judges (`fixtures/export-hostile/g*`).

mod support;

use std::collections::BTreeSet;

use support::roundtrip::{assert_golden, canon_graph, fixtures, sets_json, stems};
use vox_diagram::check;
use vox_diagram::ir::{Direction, GraphIr, Ir, IrContainer, IrEdge, IrNode, Shape};
use vox_diagram::mermaid::export;

fn graph_of(src: &str) -> GraphIr {
    match check(src).ir {
        Some(Ir::Graph(g)) => g,
        other => panic!("source does not check ({other:?}):\n{src}"),
    }
}

const NESTED: &str = "graph {\n    node top: \"Top\"\n    container l1: \"Level One\" {\n        node item: \"Item A\"\n        container l2: \"Level Two\" {\n            container l3: \"Level Three\" {\n                container l4: \"Level Four\" {\n                    node deep: \"Deep\"\n                }\n            }\n        }\n    }\n    container sib: \"Sibling\" {\n        node item: \"Item B\"\n        container hollow: \"Hollow\" {\n        }\n    }\n    top -> l1.item: \"enter\"\n    l1.item -> l1.l2.l3.l4.deep\n    l1.l2.l3.l4.deep -> l1.l2.l3.l4.deep\n    sib.item -> top\n}\n";

#[test]
fn export_nests_containers() {
    let src = "graph {\n    container backend: \"Backend\" {\n        node api: \"API\"\n        container core: \"Core\" {\n            node db: \"DB\" { shape: cylinder }\n        }\n    }\n    node client: \"Client\"\n    client -> backend.api\n    backend.api -> backend.core.db: \"query\"\n}\n";
    let want = "flowchart TD\n    client[\"Client\"]\n    subgraph backend [\"Backend\"]\n        api[\"API\"]\n        subgraph core [\"Core\"]\n            db[(\"DB\")]\n        end\n    end\n    client --> api\n    api -->|\"query\"| db\n";
    assert_eq!(export(&Ir::Graph(graph_of(src))), want);
}

#[test]
fn export_ids_are_global() {
    let src = "graph {\n    container a1: \"One\" {\n        node item: \"I1\"\n    }\n    container a2: \"Two\" {\n        node item: \"I2\"\n    }\n    node a1x: \"X\"\n    container a1x_c: \"C\" {\n    }\n}\n";
    let out = export(&Ir::Graph(graph_of(src)));
    // Nodes get ids first (index order), so the second `item` is n1; containers follow.
    assert!(out.contains("        item[\"I1\"]\n"), "{out}");
    assert!(out.contains("        n1[\"I2\"]\n"), "{out}");
    // A container whose local id collides with an earlier node id falls back too.
    let src = "graph {\n    node a: \"A\"\n    container c: \"C\" {\n        container a: \"Clash\" {\n        }\n    }\n}\n";
    let out = export(&Ir::Graph(graph_of(src)));
    assert!(out.contains("        subgraph n2 [\"Clash\"]\n"), "{out}");
    assert!(out.contains("    a[\"A\"]\n"), "{out}");
}

fn node(id: &str, container: Option<&str>) -> IrNode {
    IrNode {
        id: id.into(),
        local_id: id.into(),
        label: id.into(),
        shape: Shape::Rect,
        container: container.map(str::to_owned),
        span: Default::default(),
    }
}

fn container(id: &str, parent: Option<&str>) -> IrContainer {
    IrContainer {
        id: id.into(),
        local_id: id.into(),
        label: id.into(),
        parent: parent.map(str::to_owned),
        depth: 1,
        span: Default::default(),
    }
}

fn edge(from: &str, to: &str) -> IrEdge {
    IrEdge {
        from: from.into(),
        to: to.into(),
        label: None,
        span: Default::default(),
    }
}

fn ir(nodes: Vec<IrNode>, containers: Vec<IrContainer>, edges: Vec<IrEdge>) -> Ir {
    Ir::Graph(GraphIr {
        direction: Direction::Down,
        nodes,
        containers,
        edges,
    })
}

#[test]
fn export_is_total_on_degenerate_irs() {
    let g = ir(
        vec![
            node("n_ok", Some("c1")),
            node("lost", Some("nowhere")),
            node("top", None),
        ],
        vec![
            container("selfp", Some("selfp")),
            container("c1", Some("later")),
            container("later", None),
            container("c2", Some("missing")),
            container("c1", None),
        ],
        vec![
            edge("top", "ghost"),
            edge("top", "later"),
            edge("a.b", "a.b"),
        ],
    );
    let out = export(&g);
    let lines = out.lines().count();
    // header + 3 nodes + 2 x 5 containers + 3 edges
    assert_eq!(lines, 1 + 3 + 10 + 3, "{out}");
    assert_eq!(out.matches("subgraph ").count(), 5, "{out}");
    assert_eq!(
        out.lines().filter(|l| l.trim() == "end").count(),
        5,
        "{out}"
    );
    for label in ["n_ok", "lost", "top"] {
        assert_eq!(out.matches(&format!("[\"{label}\"]")).count(), 1, "{out}");
    }
    // The dangling dotted path gets a fresh safe id; the plain one keeps its name.
    assert!(out.contains(" --> ghost\n"), "{out}");
    assert!(out.contains("    n9 --> n9\n"), "{out}");
}

#[test]
fn export_survives_a_deep_chain_without_recursion() {
    let depth = 500;
    let containers = (0..depth)
        .map(|i| {
            let parent = (i > 0).then(|| format!("c{}", i - 1));
            container(&format!("c{i}"), parent.as_deref())
        })
        .collect();
    let out = export(&ir(vec![], containers, vec![]));
    assert_eq!(out.lines().count(), 1 + 2 * depth);
    assert!(out.ends_with("\n    end\n"), "ends with the outermost end");
}

/// Cases whose export is judged by the Node oracle: `(stem, ir)`.
fn cases() -> Vec<(&'static str, Ir)> {
    let mut all = vec![("g00-nested", Ir::Graph(graph_of(NESTED)))];
    all.extend(
        support::hostile::graph_cases()
            .into_iter()
            .map(|(stem, g)| (stem, Ir::Graph(g))),
    );
    all
}

#[test]
fn export_hostile_goldens() {
    let dir = fixtures().join("export-hostile");
    let table = cases();
    for (stem, ir) in &table {
        let Ir::Graph(g) = ir else {
            panic!("expected a graph ir");
        };
        assert_golden(&dir.join(format!("{stem}.export.mmd")), &export(ir));
        assert_golden(
            &dir.join(format!("{stem}.sets.json")),
            &sets_json(&canon_graph(g)),
        );
    }
    let known: BTreeSet<&str> = table.iter().map(|(s, _)| *s).collect();
    for ext in ["mmd", "json"] {
        for stem in stems(&dir, ext) {
            let base = stem.trim_end_matches(".export").trim_end_matches(".sets");
            assert!(
                !base.starts_with('g') || known.contains(base),
                "orphan golden {stem}.{ext}"
            );
        }
    }
}
