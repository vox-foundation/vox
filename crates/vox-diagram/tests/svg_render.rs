//! SVG renderer: tracer assertions plus the data-driven `cases/*.vdg` -> `*.svg` golden loop.
//! Bless with `UPDATE_GOLDEN=1 cargo nextest run -p vox-diagram -E 'binary(svg_render)'`.

mod support;

use std::collections::BTreeSet;
use std::fs;

use support::roundtrip::{assert_golden, fixtures, stems};
use vox_diagram::ir::{Direction, GraphIr, Ir, IrEdge, IrNode, Shape};
use vox_diagram::svg::{RenderError, SvgOpts, THEME_VARS, render};
use vox_diagram::{check, limits};

const MINIMAL: &str = "graph {\n    node a: \"A\"\n    node b: \"B\"\n    a -> b\n}\n";

fn ir_of(src: &str) -> Ir {
    let Some(ir) = check(src).ir else {
        panic!("source does not check: {src:?}");
    };
    ir
}

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

fn edge(from: &str, to: &str) -> IrEdge {
    IrEdge {
        from: from.to_owned(),
        to: to.to_owned(),
        label: None,
        span: Default::default(),
    }
}

fn graph(nodes: Vec<IrNode>, edges: Vec<IrEdge>) -> Ir {
    Ir::Graph(GraphIr {
        direction: Direction::Down,
        nodes,
        containers: Vec::new(),
        edges,
    })
}

fn render_ok(ir: &Ir, opts: &SvgOpts) -> String {
    match render(ir, opts) {
        Ok(s) => s,
        Err(e) => panic!("render failed: {e}"),
    }
}

#[test]
fn tracer_minimal_graph_renders() {
    let svg = render_ok(&ir_of(MINIMAL), &SvgOpts::default());
    assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 "));
    assert!(svg.contains("role=\"img\""));
    assert!(svg.contains("aria-labelledby=\"vd-title vd-desc\""));
    assert!(svg.contains("<title id=\"vd-title\">Diagram</title>"));
    assert!(svg.contains("<desc id=\"vd-desc\">2 nodes, 1 edge: A \u{2192} B</desc>"));
    assert!(svg.contains("<marker id=\"vd-arrow\""));
    assert!(svg.contains("marker-end=\"url(#vd-arrow)\""));
    assert_eq!(svg.matches("class=\"vd-node\"").count(), 2);
    assert_eq!(svg.matches("class=\"vd-edge\"").count(), 1);
    assert!(svg.ends_with("</svg>\n"));
    let root = svg.lines().next().unwrap_or_default();
    assert!(
        !root.contains("width=") && !root.contains("height="),
        "{root}"
    );
    assert!(!svg.contains("<style") && !svg.contains("foreignObject"));
}

#[test]
fn tracer_every_paint_is_a_theme_variable() {
    let svg = render_ok(&ir_of(MINIMAL), &SvgOpts::default());
    for (name, fallback) in THEME_VARS {
        assert!(name.starts_with("--vd-"), "{name}");
        assert!(!fallback.is_empty());
    }
    assert!(svg.contains("var(--vd-node-fill,#fff)"));
    assert!(svg.contains("var(--vd-edge,#57606a)"));
    assert!(svg.contains("var(--vd-text,#1f2328)"));
    // No literal colour outside a var() fallback.
    let mut rest = svg.as_str();
    while let Some(i) = rest.find('#') {
        let before = &rest[..i];
        assert!(
            before.ends_with(',') || before.ends_with("url(") || before.ends_with("&#"),
            "bare colour near {:?}",
            &rest[i.saturating_sub(20)..(i + 8).min(rest.len())]
        );
        rest = &rest[i + 1..];
    }
}

#[test]
fn tracer_render_is_repeatable() {
    let ir = ir_of(MINIMAL);
    let outs: BTreeSet<String> = (0..20)
        .map(|_| render_ok(&ir, &SvgOpts::default()))
        .collect();
    assert_eq!(outs.len(), 1);
}

#[test]
fn tracer_prefix_and_title() {
    let opts = SvgOpts {
        id_prefix: "d2-x".into(),
        title: Some("A & <B> \"q\"".into()),
    };
    let svg = render_ok(&ir_of(MINIMAL), &opts);
    assert!(svg.contains("id=\"d2-x-title\""));
    assert!(svg.contains("id=\"d2-x-desc\""));
    assert!(svg.contains("id=\"d2-x-arrow\""));
    assert!(svg.contains(">A &amp; &lt;B&gt; \"q\"</title>"), "{svg}");
    assert!(!svg.contains("vd-title"));
}

#[test]
fn tracer_bad_prefix_rejected() {
    let ir = ir_of(MINIMAL);
    let long = "a".repeat(33);
    for bad in ["", "2a", "A", "a_b", "a b", long.as_str()] {
        let opts = SvgOpts {
            id_prefix: bad.into(),
            title: None,
        };
        assert_eq!(
            render(&ir, &opts),
            Err(RenderError::InvalidIdPrefix),
            "{bad:?}"
        );
    }
    for ok in ["a", &"a".repeat(32)] {
        let opts = SvgOpts {
            id_prefix: ok.into(),
            title: None,
        };
        assert!(render(&ir, &opts).is_ok(), "{ok:?}");
    }
}

#[test]
fn tracer_caps() {
    let many: Vec<IrNode> = (0..101).map(|i| node(&format!("n{i}"))).collect();
    assert_eq!(
        render(&graph(many, vec![]), &SvgOpts::default()),
        Err(RenderError::TooLarge {
            nodes: 101,
            edges: 0
        })
    );
    let edges: Vec<IrEdge> = (0..201).map(|_| edge("a", "b")).collect();
    assert_eq!(
        render(
            &graph(vec![node("a"), node("b")], edges),
            &SvgOpts::default()
        ),
        Err(RenderError::TooLarge {
            nodes: 2,
            edges: 201
        })
    );
    assert_eq!(
        (limits::MAX_RENDER_NODES, limits::MAX_RENDER_EDGES),
        (100, 200)
    );
    let chain: Vec<IrNode> = (0..100).map(|i| node(&format!("n{i}"))).collect();
    let links: Vec<IrEdge> = (0..99)
        .map(|i| edge(&format!("n{i}"), &format!("n{}", i + 1)))
        .collect();
    assert!(render(&graph(chain, links), &SvgOpts::default()).is_ok());
}

#[test]
fn tracer_unknown_endpoint_is_invalid_ir() {
    let ir = graph(vec![node("a")], vec![edge("a", "zz")]);
    assert!(matches!(
        render(&ir, &SvgOpts::default()),
        Err(RenderError::InvalidIr(m)) if m.contains("edge 0")
    ));
    let dup = graph(vec![node("a"), node("a")], vec![]);
    assert!(matches!(
        render(&dup, &SvgOpts::default()),
        Err(RenderError::InvalidIr(_))
    ));
}

#[test]
fn degenerate_empty_graph() {
    let svg = render_ok(&graph(vec![], vec![]), &SvgOpts::default());
    assert!(svg.contains("viewBox=\"0 0 32 32\""), "{svg}");
    assert!(svg.contains("<desc id=\"vd-desc\">0 nodes, 0 edges</desc>"));
}

#[test]
fn svg_cases() {
    let dir = fixtures().join("svg/cases");
    let inputs = stems(&dir, "vdg");
    assert!(!inputs.is_empty(), "no svg cases in {}", dir.display());
    for stem in &inputs {
        let src = fs::read_to_string(dir.join(format!("{stem}.vdg"))).unwrap_or_default();
        let report = check(&src);
        let Some(ir) = report.ir else {
            panic!("{stem}.vdg does not check: {:?}", report.diagnostics);
        };
        let svg = render_ok(&ir, &SvgOpts::default());
        assert_golden(&dir.join(format!("{stem}.svg")), &svg);
    }
    for stem in stems(&dir, "svg") {
        assert!(
            inputs.contains(&stem),
            "orphan golden {stem}.svg has no .vdg"
        );
    }
}
