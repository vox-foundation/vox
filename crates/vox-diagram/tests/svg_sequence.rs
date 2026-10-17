//! Sequence rendering through the independent `svgcheck` allowlist oracle.

mod support;

use support::svgcheck::{self, Svg};
use vox_diagram::check;
use vox_diagram::ir::Ir;
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};
use vox_diagram::svg::{RenderError, SvgOpts, render};

const MIN: &str = "sequence {\n    actor a: \"A\"\n    actor b: \"B\"\n    a -> b: \"hi\"\n}\n";

fn ir_of(src: &str) -> Ir {
    let Some(ir) = check(src).ir else {
        panic!("source does not check: {src:?}");
    };
    ir
}

fn drawn(ir: &Ir) -> (String, Svg) {
    let svg = match render(ir, &SvgOpts::default()) {
        Ok(s) => s,
        Err(e) => panic!("render failed: {e}"),
    };
    match svgcheck::check(&svg, "vd") {
        Ok(read) => (svg, read),
        Err(e) => panic!("checker rejected the svg: {e}\n{svg}"),
    }
}

fn actor(id: &str) -> Actor {
    Actor {
        id: id.into(),
        label: id.to_uppercase(),
        span: Default::default(),
    }
}

fn message(from: &str, to: &str, text: &str, dashed: bool) -> SeqEvent {
    SeqEvent::Message {
        from: from.into(),
        to: to.into(),
        text: text.into(),
        dashed,
        span: Default::default(),
    }
}

fn note(over: &[&str], text: &str) -> SeqEvent {
    SeqEvent::Note {
        over: over.iter().map(|s| (*s).into()).collect(),
        text: text.into(),
        span: Default::default(),
    }
}

fn seq(ids: &[&str], events: Vec<SeqEvent>) -> Ir {
    Ir::Sequence(SequenceIr {
        actors: ids.iter().map(|i| actor(i)).collect(),
        events,
    })
}

#[test]
fn tracer_minimal_sequence_renders() {
    let ir = ir_of(MIN);
    let (svg, read) = drawn(&ir);
    assert_eq!(read.nodes.len(), 2);
    assert_eq!(read.lifelines.len(), 2);
    assert_eq!(read.edge_paths.len(), 1);
    assert_eq!(read.edge_paths[0].len(), 2);
    assert!(
        read.texts
            .iter()
            .any(|(c, t)| c == "vd-edge-label" && t == "hi")
    );
    assert!(
        read.texts
            .iter()
            .any(|(c, t)| c == "desc" && t == "2 actors, 1 message: A \u{2192} B")
    );
    let bg = read.edge_labels[0];
    assert!(bg.x > read.lifelines[0].0, "{bg:?} {:?}", read.lifelines);
    assert!(bg.x + bg.w < read.lifelines[1].0, "{bg:?}");
    for _ in 0..20 {
        assert_eq!(render(&ir, &SvgOpts::default()), Ok(svg.clone()));
    }
}

#[test]
fn tracer_gap_fits_crossing_labels() {
    let long = "word ".repeat(14).trim_end().to_owned();
    assert_eq!(long.chars().count(), 69);
    let long = format!("{long}!");
    assert_eq!(long.chars().count(), 70);
    let src = format!(
        "sequence {{\n    actor a: \"A\"\n    actor b: \"B\"\n    actor c: \"C\"\n    a -> c: \"{long}\"\n    a -> b: \"short\"\n}}\n"
    );
    let (_, read) = drawn(&ir_of(&src));
    let xs: Vec<i64> = read.lifelines.iter().map(|l| l.0).collect();
    let (wide, narrow) = (read.edge_labels[0], read.edge_labels[1]);
    let clear = 16; // 8 px in half-pixels
    assert!(wide.x >= xs[0] + clear && wide.x + wide.w <= xs[2] - clear);
    assert!(narrow.x >= xs[0] + clear && narrow.x + narrow.w <= xs[1] - clear);
    assert!(xs[1] - xs[0] >= wide.w + 2 * clear, "{xs:?} {wide:?}");
    assert!(xs[2] - xs[1] >= wide.w + 2 * clear, "{xs:?} {wide:?}");
}

#[test]
fn tracer_every_event_kind_renders() {
    let ir = seq(
        &["a", "b"],
        vec![
            message("b", "a", "ret", true),
            message("a", "a", "retry", false),
            note(&["a"], "one lane"),
            note(&["b", "a"], "two lanes"),
        ],
    );
    let (_, read) = drawn(&ir);
    let dashed = read
        .paints
        .iter()
        .filter(|(p, v)| p == "stroke-dasharray" && v == "6 4")
        .count();
    assert_eq!(dashed, 1);
    assert_eq!(read.edge_paths.len(), 2);
    let own = &read.edge_paths[1];
    assert_eq!(own.len(), 4);
    let ax = read.lifelines[0].0;
    assert!(own.iter().map(|p| p.0).max() > Some(ax));
    assert_eq!(read.notes.len(), 2);
    let both = read.notes[1];
    let (lo, hi) = (ax.min(read.lifelines[1].0), ax.max(read.lifelines[1].0));
    assert!(both.x < lo && both.x + both.w > hi, "{both:?} {lo} {hi}");
}

#[test]
fn tracer_unknown_actor_is_invalid_ir() {
    let to_zz = seq(&["a"], vec![message("a", "zz", "x", false)]);
    let over_zz = seq(&["a"], vec![note(&["zz"], "x")]);
    for ir in [to_zz, over_zz] {
        assert!(
            matches!(
                render(&ir, &SvgOpts::default()),
                Err(RenderError::InvalidIr(_))
            ),
            "{ir:?}"
        );
    }
}
