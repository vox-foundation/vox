//! REND-05: width-aware sequence lanes. Every assertion reads the drawn SVG back through the
//! independent `svgcheck` allowlist checker and `assert_sequence_geometry`; nothing here asks
//! the renderer how it laid itself out.

mod support;

use support::svgcheck::seq::assert_sequence_geometry;
use support::svgcheck::{self, Svg};
use vox_diagram::check;
use vox_diagram::ir::Ir;
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};
use vox_diagram::svg::{SvgOpts, render};

/// 87 characters of ordinary words.
const SENTENCE: &str =
    "The order service confirms the payment with the billing provider before releasing stock";

/// Half-pixels in 8 px.
const CLEAR: i64 = 16;

fn actor(id: &str, label: &str) -> Actor {
    Actor {
        id: id.into(),
        label: label.into(),
        span: Default::default(),
    }
}

fn message(from: &str, to: &str, text: &str) -> SeqEvent {
    SeqEvent::Message {
        from: from.into(),
        to: to.into(),
        text: text.into(),
        dashed: false,
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

fn seq(ids: &[&str], events: Vec<SeqEvent>) -> SequenceIr {
    SequenceIr {
        actors: ids.iter().map(|i| actor(i, &i.to_uppercase())).collect(),
        events,
    }
}

fn svg_of(s: &SequenceIr) -> String {
    match render(&Ir::Sequence(s.clone()), &SvgOpts::default()) {
        Ok(svg) => svg,
        Err(e) => panic!("render failed: {e}"),
    }
}

/// Render, read back through the checker and run the geometry oracle.
fn drawn(s: &SequenceIr) -> Svg {
    let svg = svg_of(s);
    let read = match svgcheck::check(&svg, "vd") {
        Ok(read) => read,
        Err(e) => panic!("checker rejected the svg: {e}\n{svg}"),
    };
    if let Err(e) = assert_sequence_geometry(&read, s) {
        panic!("geometry: {e}\n{svg}");
    }
    read
}

fn label_lines(read: &Svg) -> Vec<Vec<String>> {
    read.labels
        .iter()
        .filter(|l| l.class == "vd-edge-label")
        .map(|l| l.lines.clone())
        .collect()
}

#[test]
fn tracer_long_message_wraps_and_fits() {
    assert_eq!(SENTENCE.chars().count(), 87);
    let src = format!(
        "sequence {{\n    actor a: \"Client\"\n    actor b: \"Server\"\n    a -> b: \"{SENTENCE}\"\n}}\n"
    );
    let Some(Ir::Sequence(s)) = check(&src).ir else {
        panic!("source does not check");
    };
    let read = drawn(&s);
    let lines = label_lines(&read);
    assert_eq!(lines.len(), 1);
    assert_eq!(lines[0].len(), 2, "{lines:?}");
    assert!(lines[0].iter().all(|l| l.chars().count() <= 60));
    assert_eq!(lines[0].join(" "), SENTENCE);
    let bg = read.edge_labels[0];
    let gap = read.lifelines[1].0 - read.lifelines[0].0;
    assert!(gap >= bg.w + 2 * CLEAR, "{gap} {bg:?}");
}

#[test]
fn explicit_newlines_are_kept_and_long_lines_wrap() {
    let long = ["word"; 26].join(" ");
    assert_eq!(long.chars().count(), 129);
    let word61 = "x".repeat(61);
    let cases: [(&str, Vec<String>); 3] = [
        ("one\ntwo", vec!["one".into(), "two".into()]),
        (
            &long,
            vec![
                ["word"; 12].join(" "),
                ["word"; 12].join(" "),
                "word word".into(),
            ],
        ),
        (&word61, vec!["x".repeat(60), "x".into()]),
    ];
    for (text, want) in cases {
        let read = drawn(&seq(&["a", "b"], vec![message("a", "b", text)]));
        assert_eq!(label_lines(&read), vec![want], "{text:?}");
    }
}

#[test]
fn multi_lane_long_message_fits_between_its_endpoints() {
    let long = ["lane"; 24].join(" ");
    assert_eq!(long.chars().count(), 119);
    let s = seq(
        &["a", "b", "c", "d"],
        vec![message("a", "d", &long), message("b", "c", "short")],
    );
    let read = drawn(&s);
    let wide = read.edge_labels[0];
    for w in read.lifelines.windows(2) {
        assert!(w[1].0 - w[0].0 >= wide.w + 2 * CLEAR, "{wide:?} {w:?}");
    }
}

#[test]
fn oracle_rejects_a_forged_overlap() {
    let s = seq(&["a", "b"], vec![message("a", "b", "hello there")]);
    let svg = svg_of(&s);
    let read = match svgcheck::check(&svg, "vd") {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    assert_eq!(assert_sequence_geometry(&read, &s), Ok(()));
    // Move the label background onto the first lifeline in the document text itself.
    let lx = read.lifelines[0].0;
    let bg = read.edge_labels[0];
    let (old, new) = (half_px(bg.x), half_px(lx - 2));
    let marker = format!("class=\"vd-edge-label-bg\" x=\"{old}\"");
    assert!(svg.contains(&marker), "no {marker} in {svg}");
    let forged = svg.replace(&marker, &format!("class=\"vd-edge-label-bg\" x=\"{new}\""));
    let forged_read = match svgcheck::check(&forged, "vd") {
        Ok(r) => r,
        Err(e) => panic!("{e}"),
    };
    assert!(assert_sequence_geometry(&forged_read, &s).is_err());
}

/// Half-pixels as the writer prints them.
fn half_px(h: i64) -> String {
    if h % 2 == 0 {
        (h / 2).to_string()
    } else {
        format!("{}.5", h / 2)
    }
}
