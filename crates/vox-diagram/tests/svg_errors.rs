//! Renderer refusals on hand-built IR (`Ir` is public, so nothing the DSL guarantees may be
//! trusted) and a smoke render of every hostile hand-built graph.

mod support;

use support::hostile::{Box_, Node_, build, graph_cases};
use vox_diagram::ir::{Direction, GraphIr, Ir, IrContainer, IrEdge, IrNode, Shape};
use vox_diagram::svg::{RenderError, SvgOpts, render};

fn node(id: &str) -> IrNode {
    IrNode {
        id: id.to_owned(),
        local_id: id.to_owned(),
        label: id.to_owned(),
        shape: Shape::Rect,
        container: None,
        span: Default::default(),
    }
}

fn boxed(id: &str, parent: Option<&str>) -> IrContainer {
    IrContainer {
        id: id.to_owned(),
        local_id: id.to_owned(),
        label: id.to_owned(),
        parent: parent.map(str::to_owned),
        // Hand-built IR may claim any depth; the renderer must derive its own.
        depth: 1,
        span: Default::default(),
    }
}

fn graph(nodes: Vec<IrNode>, containers: Vec<IrContainer>, edges: Vec<IrEdge>) -> Ir {
    Ir::Graph(GraphIr {
        direction: Direction::Down,
        nodes,
        containers,
        edges,
    })
}

fn render_err(ir: &Ir) -> RenderError {
    match render(ir, &SvgOpts::default()) {
        Ok(_) => panic!("render unexpectedly succeeded"),
        Err(e) => e,
    }
}

fn is_invalid_ir(ir: &Ir) -> bool {
    matches!(render_err(ir), RenderError::InvalidIr(_))
}

#[test]
fn an_unknown_container_or_parent_id_is_invalid_ir() {
    let mut n = node("a");
    n.container = Some("zz".to_owned());
    assert!(is_invalid_ir(&graph(vec![n], vec![], vec![])));
    let orphan = boxed("c", Some("zz"));
    assert!(is_invalid_ir(&graph(vec![], vec![orphan], vec![])));
}

#[test]
fn a_duplicate_container_id_is_invalid_ir() {
    let twice = vec![boxed("c", None), boxed("c", None)];
    assert!(is_invalid_ir(&graph(vec![], twice, vec![])));
}

#[test]
fn parent_cycles_are_invalid_ir_and_terminate() {
    let two = vec![boxed("a", Some("b")), boxed("b", Some("a"))];
    assert!(is_invalid_ir(&graph(vec![], two, vec![])));
    let selfie = vec![boxed("a", Some("a"))];
    assert!(is_invalid_ir(&graph(vec![], selfie, vec![])));
}

fn levels(n: usize) -> Ir {
    let ids = ["l1", "l2", "l3", "l4", "l5"];
    let boxes: Vec<Box_> = (0..n).map(|i| (ids[i], ids[i], i.checked_sub(1))).collect();
    let leaf: Vec<Node_> = vec![("leaf", "Leaf".to_owned(), Shape::Rect, n.checked_sub(1))];
    Ir::Graph(build(&boxes, &leaf, &[]))
}

#[test]
fn nesting_deeper_than_max_depth_is_invalid_ir() {
    assert_eq!(vox_diagram::limits::MAX_DEPTH, 4);
    assert!(is_invalid_ir(&levels(5)));
    assert!(render(&levels(4), &SvgOpts::default()).is_ok());
}

#[test]
fn labels_over_the_cap_are_invalid_ir() {
    let cap = vox_diagram::limits::MAX_LABEL_CHARS;
    let mut long = node("a");
    long.label = "x".repeat(cap + 1);
    assert!(is_invalid_ir(&graph(vec![long], vec![], vec![])));
    let mut fits = node("a");
    fits.label = "x".repeat(cap);
    assert!(render(&graph(vec![fits], vec![], vec![]), &SvgOpts::default()).is_ok());
    let mut title = boxed("c", None);
    title.label = "x".repeat(cap + 1);
    assert!(is_invalid_ir(&graph(vec![], vec![title], vec![])));
    let edge = IrEdge {
        from: "a".to_owned(),
        to: "b".to_owned(),
        label: Some("x".repeat(cap + 1)),
        span: Default::default(),
    };
    assert!(is_invalid_ir(&graph(
        vec![node("a"), node("b")],
        vec![],
        vec![edge]
    )));
}

#[test]
fn containers_count_toward_the_node_cap() {
    let nodes: Vec<IrNode> = (0..60).map(|i| node(&format!("n{i}"))).collect();
    let boxes: Vec<IrContainer> = (0..41).map(|i| boxed(&format!("c{i}"), None)).collect();
    assert_eq!(
        render_err(&graph(nodes, boxes, vec![])),
        RenderError::TooLarge {
            nodes: 101,
            edges: 0
        }
    );
}

#[test]
fn an_edge_to_a_container_is_invalid_ir() {
    let edge = IrEdge {
        from: "a".to_owned(),
        to: "c".to_owned(),
        label: None,
        span: Default::default(),
    };
    let ir = graph(vec![node("a")], vec![boxed("c", None)], vec![edge]);
    assert!(matches!(
        render_err(&ir),
        RenderError::InvalidIr(m) if m.contains("edge 0")
    ));
}

#[test]
fn hostile_graph_cases_render() {
    for (name, g) in graph_cases() {
        if let Err(e) = render(&Ir::Graph(g), &SvgOpts::default()) {
            panic!("{name} failed to render: {e:?}");
        }
    }
}
