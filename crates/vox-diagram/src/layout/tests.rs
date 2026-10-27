use std::cell::{Cell, RefCell};

use super::*;
use crate::ir::{GraphIr, Ir, IrNode};
use crate::svg::{SvgOpts, render_with};

#[test]
fn quantize_rejects_missing_and_non_finite() {
    assert_eq!(quantize(None), Err(LayoutError::Missing));
    for v in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        1.0e5,
        -1.0e5,
        1.0e7,
    ] {
        assert_eq!(quantize(Some(v)), Err(LayoutError::NonFinite), "{v}");
    }
}

#[test]
fn quantize_rounds_to_half_pixels_without_negative_zero() {
    assert_eq!(quantize(Some(1.24)), Ok(2));
    assert_eq!(quantize(Some(1.25)), Ok(3));
    assert_eq!(quantize(Some(-0.2)), Ok(0));
    assert_eq!(quantize(Some(99_999.0)), Ok(199_998));
}

#[test]
fn guarded_catches_a_panic() {
    let r: Result<(), LayoutError> = guarded(|| panic!("x"));
    assert_eq!(r, Err(LayoutError::EnginePanic));
    assert_eq!(guarded(|| 7), Ok(7));
}

/// Engine double returning a canned result.
struct Canned(Result<RawLayout, LayoutError>);

impl Engine for Canned {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.clone()
    }
}

fn one_node() -> LayoutInput {
    LayoutInput {
        direction: Direction::Down,
        nodes: vec![LNode {
            shape: Shape::Rect,
            lines: vec!["A".into()],
            w: 128,
            h: 76,
            container: None,
        }],
        containers: vec![],
        edges: vec![],
    }
}

fn boxed(cx: Option<f64>) -> RawLayout {
    RawLayout {
        nodes: vec![RawBox {
            cx,
            cy: Some(19.0),
            w: Some(64.0),
            h: Some(38.0),
        }],
        titles: vec![],
        edges: vec![],
    }
}

#[test]
fn lay_out_places_at_the_margin() {
    let engine = Canned(Ok(boxed(Some(100.0))));
    let Ok(p) = lay_out(&engine, &one_node()) else {
        panic!("lay_out failed");
    };
    assert_eq!(
        p.nodes,
        vec![Rect {
            x: 32,
            y: 32,
            w: 128,
            h: 76
        }]
    );
    assert_eq!((p.width, p.height), (192, 140));
}

#[test]
fn lay_out_fails_closed_on_engine_misbehaviour() {
    let nan = Canned(Ok(boxed(Some(f64::NAN))));
    assert_eq!(
        lay_out(&nan, &one_node()),
        Err(RenderError::Layout("non-finite coordinate"))
    );
    let missing = Canned(Ok(boxed(None)));
    assert_eq!(
        lay_out(&missing, &one_node()),
        Err(RenderError::Layout("missing coordinate"))
    );
    let short = Canned(Ok(RawLayout::default()));
    assert_eq!(
        lay_out(&short, &one_node()),
        Err(RenderError::Layout("missing coordinate"))
    );
}

fn two_nodes() -> LayoutInput {
    let n = || LNode {
        shape: Shape::Rect,
        lines: vec!["A".into()],
        w: 128,
        h: 76,
        container: None,
    };
    LayoutInput {
        direction: Direction::Down,
        nodes: vec![n(), n()],
        containers: vec![],
        edges: vec![],
    }
}

/// Raw boxes at the given x centres, each at its input size; edges get a one-point route.
fn raw_at(input: &LayoutInput, cx: impl Fn(usize) -> Option<f64>) -> RawLayout {
    RawLayout {
        nodes: input
            .nodes
            .iter()
            .enumerate()
            .map(|(i, n)| RawBox {
                cx: cx(i),
                cy: Some(19.0),
                w: Some(f64::from(n.w) / 2.0),
                h: Some(f64::from(n.h) / 2.0),
            })
            .collect(),
        titles: vec![],
        edges: input
            .edges
            .iter()
            .map(|e| RawEdge {
                points: vec![(0.0, 0.0)],
                label: e.label.as_ref().map(|_| (Some(0.0), Some(0.0))),
            })
            .collect(),
    }
}

/// Two 64 px nodes 100 px apart: clean.
fn clean(input: &LayoutInput) -> RawLayout {
    raw_at(input, |i| Some(32.0 + 100.0 * i as f64))
}

/// Every node on the same spot: overlapping.
fn stacked(input: &LayoutInput) -> RawLayout {
    raw_at(input, |_| Some(32.0))
}

fn graph_ir() -> Ir {
    Ir::Graph(GraphIr {
        direction: Direction::Down,
        nodes: vec![IrNode {
            id: "a".into(),
            local_id: "a".into(),
            label: "A".into(),
            shape: Shape::Rect,
            container: None,
            span: Default::default(),
        }],
        containers: vec![],
        edges: vec![],
    })
}

struct PanicEngine(Cell<usize>);

impl Engine for PanicEngine {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.set(self.0.get() + 1);
        panic!("boom")
    }
}

struct NanEngine(Cell<usize>);

impl Engine for NanEngine {
    fn solve(&self, i: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.set(self.0.get() + 1);
        Ok(raw_at(i, |n| Some(if n == 0 { f64::NAN } else { 32.0 })))
    }
}

struct HugeEngine(Cell<usize>);

impl Engine for HugeEngine {
    fn solve(&self, i: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.set(self.0.get() + 1);
        Ok(raw_at(i, |n| Some(if n == 0 { 1.0e8 } else { 32.0 })))
    }
}

struct MissingEngine(Cell<usize>);

impl Engine for MissingEngine {
    fn solve(&self, i: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.0.set(self.0.get() + 1);
        Ok(raw_at(i, |n| if n == 0 { None } else { Some(32.0) }))
    }
}

/// Overlapping on the first rung, clean on every later one.
struct OverlapThenClean(RefCell<Vec<Separations>>);

impl Engine for OverlapThenClean {
    fn solve(&self, i: &LayoutInput, sep: Separations) -> Result<RawLayout, LayoutError> {
        self.0.borrow_mut().push(sep);
        Ok(if self.0.borrow().len() == 1 {
            stacked(i)
        } else {
            clean(i)
        })
    }
}

struct AlwaysOverlap(RefCell<Vec<Separations>>);

impl Engine for AlwaysOverlap {
    fn solve(&self, i: &LayoutInput, sep: Separations) -> Result<RawLayout, LayoutError> {
        self.0.borrow_mut().push(sep);
        Ok(stacked(i))
    }
}

#[test]
fn broken_engines_fail_closed_with_a_single_call() {
    let input = two_nodes();
    let panic_e = PanicEngine(Cell::new(0));
    assert_eq!(
        lay_out(&panic_e, &input),
        Err(RenderError::Layout("engine panic"))
    );
    assert_eq!(panic_e.0.get(), 1);
    let nan = NanEngine(Cell::new(0));
    assert_eq!(
        lay_out(&nan, &input),
        Err(RenderError::Layout("non-finite coordinate"))
    );
    assert_eq!(nan.0.get(), 1);
    let huge = HugeEngine(Cell::new(0));
    assert_eq!(
        lay_out(&huge, &input),
        Err(RenderError::Layout("non-finite coordinate"))
    );
    assert_eq!(huge.0.get(), 1);
    let missing = MissingEngine(Cell::new(0));
    assert_eq!(
        lay_out(&missing, &input),
        Err(RenderError::Layout("missing coordinate"))
    );
    assert_eq!(missing.0.get(), 1);
}

#[test]
fn an_overlap_escalates_to_the_next_rung() {
    let e = OverlapThenClean(RefCell::default());
    assert!(lay_out(&e, &two_nodes()).is_ok());
    assert_eq!(*e.0.borrow(), vec![RUNGS[0], RUNGS[1]]);
}

#[test]
fn an_unfixable_overlap_is_a_geometry_error_after_three_rungs() {
    let e = AlwaysOverlap(RefCell::default());
    let r = lay_out(&e, &two_nodes());
    assert!(
        matches!(&r, Err(RenderError::Geometry(m)) if m.contains("NodeOverlap")),
        "{r:?}"
    );
    assert_eq!(*e.0.borrow(), RUNGS.to_vec());
}

#[test]
fn the_public_path_is_guarded_too() {
    let e = PanicEngine(Cell::new(0));
    assert_eq!(
        render_with(&e, &graph_ir(), &SvgOpts::default()),
        Err(RenderError::Layout("engine panic"))
    );
    let a = AlwaysOverlap(RefCell::default());
    let Ir::Graph(mut g) = graph_ir() else {
        panic!("expected a graph ir");
    };
    let mut second = g.nodes[0].clone();
    second.id = "b".into();
    g.nodes.push(second);
    let two = Ir::Graph(g);
    assert!(matches!(
        render_with(&a, &two, &SvgOpts::default()),
        Err(RenderError::Geometry(_))
    ));
}

/// Three nodes in a column; the edge from the top one to the bottom one runs through the middle one
/// on the first `crossed` rungs and beside it after.
struct EdgeThroughNode {
    crossed: usize,
    calls: Cell<usize>,
}

impl Engine for EdgeThroughNode {
    fn solve(&self, _: &LayoutInput, _: Separations) -> Result<RawLayout, LayoutError> {
        self.calls.set(self.calls.get() + 1);
        let via = if self.calls.get() <= self.crossed {
            100.0
        } else {
            160.0
        };
        let at = |cy| RawBox {
            cx: Some(100.0),
            cy: Some(cy),
            w: Some(64.0),
            h: Some(38.0),
        };
        Ok(RawLayout {
            nodes: vec![at(30.0), at(120.0), at(210.0)],
            titles: vec![],
            edges: vec![RawEdge {
                points: vec![(100.0, 49.0), (via, 120.0), (100.0, 191.0)],
                label: None,
            }],
        })
    }
}

#[test]
fn an_edge_through_a_node_escalates_the_rung_and_never_rejects_the_diagram() {
    let mut input = two_nodes();
    input.nodes.push(input.nodes[0].clone());
    input.edges = vec![LEdge {
        from: 0,
        to: 2,
        label: None,
    }];
    let once = EdgeThroughNode {
        crossed: 1,
        calls: Cell::new(0),
    };
    let Ok(p) = lay_out(&once, &input) else {
        panic!("lay_out failed");
    };
    assert_eq!(once.calls.get(), 2);
    assert_eq!(verify::node_crossing(&input, &p), Ok(()));
    let always = EdgeThroughNode {
        crossed: 3,
        calls: Cell::new(0),
    };
    let Ok(p) = lay_out(&always, &input) else {
        panic!("an edge through a node must not reject the diagram");
    };
    assert_eq!(always.calls.get(), 3);
    assert_eq!(
        verify::node_crossing(&input, &p),
        Err(verify::Violation::EdgeThroughNode(0, 1))
    );
}

mod containers;
mod coords;
mod feeds;
