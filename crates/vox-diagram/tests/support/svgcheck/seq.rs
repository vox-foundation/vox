//! Independent geometry oracle for sequence SVGs. It reads only what `svgcheck` read back from
//! the drawn document plus the order of the IR's actors and events, and imports nothing from
//! the renderer. Also the generated-IR strategy used by the no-overlap/no-clip property.
#![allow(dead_code)]

use proptest::prelude::*;
use vox_diagram::limits::MAX_LABEL_CHARS;
use vox_diagram::sequence_ir::{Actor, SeqEvent, SequenceIr};

use super::{Shape, Svg};

/// 8 px in half-pixels: the clearance a label or note keeps from lifelines it does not span.
const CLEAR: i64 = 16;

fn right(s: &Shape) -> i64 {
    s.x + s.w
}

fn bottom(s: &Shape) -> i64 {
    s.y + s.h
}

fn overlaps(a: &Shape, b: &Shape) -> bool {
    a.x < right(b) && b.x < right(a) && a.y < bottom(b) && b.y < bottom(a)
}

/// Whether the text draws any glyph line (the renderer drops blank lines).
fn nonblank(t: &str) -> bool {
    t.chars().any(|c| !matches!(c, ' ' | '\t' | '\r' | '\n'))
}

fn lane(ir: &SequenceIr, id: &str) -> Result<usize, String> {
    ir.actors
        .iter()
        .position(|a| a.id == id)
        .ok_or_else(|| format!("unknown actor {id:?}"))
}

fn inside_canvas(what: &str, s: &Shape, svg: &Svg) -> Result<(), String> {
    if s.x < 0 || s.y < 0 || right(s) > svg.width || bottom(s) > svg.height {
        return Err(format!(
            "{what} {s:?} outside the {}x{} canvas",
            svg.width, svg.height
        ));
    }
    Ok(())
}

fn disjoint(what: &str, a: &Shape, b: &Shape) -> Result<(), String> {
    if overlaps(a, b) {
        return Err(format!("{what}: {a:?} overlaps {b:?}"));
    }
    Ok(())
}

/// One message, checked against its lanes and its label box.
fn message(
    svg: &Svg,
    ir: &SequenceIr,
    (from, to): (&str, &str),
    path: &[(i64, i64)],
    label: Option<&Shape>,
) -> Result<(), String> {
    let (a, b) = (lane(ir, from)?, lane(ir, to)?);
    let (ax, bx) = (svg.lifelines[a].0, svg.lifelines[b].0);
    for &(x, y) in path {
        if x < 0 || y < 0 || x > svg.width || y > svg.height {
            return Err(format!("message point {:?} outside the canvas", (x, y)));
        }
    }
    if a != b {
        let [p, q] = path[..] else {
            return Err(format!("message {path:?} is not two points"));
        };
        if p != (ax, p.1) || q != (bx, p.1) {
            return Err(format!("message {path:?} misses lifelines {ax} and {bx}"));
        }
        if let Some(l) = label
            && (l.x < ax.min(bx) + CLEAR || right(l) > ax.max(bx) - CLEAR)
        {
            return Err(format!("label {l:?} not clear of lifelines {ax} and {bx}"));
        }
        // A multi-hop label must also keep off every lifeline it crosses on the way.
        if let Some(l) = label {
            let crossed = |x: i64| x > ax.min(bx) && x < ax.max(bx);
            if let Some((i, &(x, ..))) = svg
                .lifelines
                .iter()
                .enumerate()
                .find(|&(_, &(x, ..))| crossed(x) && l.x < x + CLEAR && x < right(l) + CLEAR)
            {
                return Err(format!("label {l:?} covers lifeline {i} at {x}"));
            }
        }
        return Ok(());
    }
    let [p, _, _, q] = path[..] else {
        return Err(format!("self-message {path:?} is not four points"));
    };
    if p.0 != ax || q.0 != ax {
        return Err(format!("self-message {path:?} leaves lifeline {ax}"));
    }
    let loop_right = path.iter().map(|p| p.0).max().unwrap_or(ax);
    if let Some(l) = label {
        if l.x < loop_right {
            return Err(format!(
                "self label {l:?} left of its loop edge {loop_right}"
            ));
        }
        if let Some(next) = svg.lifelines.get(a + 1)
            && right(l) > next.0 - CLEAR
        {
            return Err(format!("self label {l:?} too near lifeline {}", next.0));
        }
    }
    Ok(())
}

/// One note, checked against its lanes.
fn note(svg: &Svg, ir: &SequenceIr, over: &[String], n: &Shape) -> Result<(), String> {
    let mut xs = Vec::new();
    for id in over {
        xs.push(svg.lifelines[lane(ir, id)?].0);
    }
    let (Some(&lo), Some(&hi)) = (xs.iter().min(), xs.iter().max()) else {
        return Err("note without actors".to_owned());
    };
    if lo == hi {
        if (2 * lo - (2 * n.x + n.w)).abs() > 1 {
            return Err(format!("note {n:?} is not centred on lifeline {lo}"));
        }
    } else if n.x >= lo || right(n) <= hi {
        return Err(format!("note {n:?} does not span lifelines {lo} and {hi}"));
    }
    // No lifeline of an actor the note is not over may run under it.
    let foreign = svg
        .lifelines
        .iter()
        .enumerate()
        .find(|&(_, &(x, ..))| (x < lo || x > hi) && n.x < x + CLEAR && x < right(n) + CLEAR);
    if let Some((i, &(x, ..))) = foreign {
        return Err(format!(
            "note {n:?} covers lifeline {i} at {x} outside {lo}..{hi}"
        ));
    }
    Ok(())
}

/// The whole-diagram checks: counts, canvas, lifelines under their actor boxes, pairwise
/// disjointness.
fn frame(svg: &Svg, ir: &SequenceIr) -> Result<(), String> {
    if svg.nodes.len() != ir.actors.len() || svg.lifelines.len() != ir.actors.len() {
        return Err(format!(
            "{} actors drawn as {} boxes and {} lifelines",
            ir.actors.len(),
            svg.nodes.len(),
            svg.lifelines.len()
        ));
    }
    for (i, (b, &(x, top, bot))) in svg.nodes.iter().zip(&svg.lifelines).enumerate() {
        inside_canvas("actor box", b, svg)?;
        if (2 * x - (2 * b.x + b.w)).abs() > 1 || top != bottom(b) || bot < top || bot > svg.height
        {
            return Err(format!(
                "lifeline {i} {:?} does not hang from {b:?}",
                (x, top, bot)
            ));
        }
    }
    for (i, a) in svg.nodes.iter().enumerate() {
        for b in &svg.nodes[i + 1..] {
            disjoint("actors", a, b)?;
        }
    }
    for (i, a) in svg.notes.iter().enumerate() {
        inside_canvas("note", a, svg)?;
        for b in &svg.notes[i + 1..] {
            disjoint("notes", a, b)?;
        }
    }
    for (i, l) in svg.edge_labels.iter().enumerate() {
        inside_canvas("message label", l, svg)?;
        for other in svg.edge_labels[i + 1..]
            .iter()
            .chain(&svg.nodes)
            .chain(&svg.notes)
        {
            disjoint("label", l, other)?;
        }
    }
    Ok(())
}

/// Check the drawn `svg` against the geometry rules for `ir`: everything in the viewBox, no
/// overlapping boxes, labels between their lifelines, notes over their lanes.
pub fn assert_sequence_geometry(svg: &Svg, ir: &SequenceIr) -> Result<(), String> {
    frame(svg, ir)?;
    let (mut msgs, mut labels, mut notes) = (0, 0, 0);
    for e in &ir.events {
        match e {
            SeqEvent::Message { from, to, text, .. } => {
                let label = nonblank(text).then(|| {
                    labels += 1;
                    svg.edge_labels.get(labels - 1)
                });
                let Some(path) = svg.edge_paths.get(msgs) else {
                    return Err(format!("message {msgs} has no arrow"));
                };
                msgs += 1;
                let label = match label {
                    None => None,
                    Some(None) => return Err(format!("message {} has no label box", msgs - 1)),
                    Some(Some(l)) => Some(l),
                };
                message(svg, ir, (from, to), path, label)?;
            }
            SeqEvent::Note { over, .. } => {
                let Some(n) = svg.notes.get(notes) else {
                    return Err(format!("note {notes} was not drawn"));
                };
                notes += 1;
                note(svg, ir, over, n)?;
            }
        }
    }
    if msgs != svg.edge_paths.len() || labels != svg.edge_labels.len() || notes != svg.notes.len() {
        return Err(format!(
            "drew {} arrows, {} labels, {} notes for {msgs}, {labels}, {notes}",
            svg.edge_paths.len(),
            svg.edge_labels.len(),
            svg.notes.len()
        ));
    }
    Ok(())
}

/// A label or note text of up to 130 characters: short words, sentences, CJK, emoji and
/// explicit newlines.
fn text() -> impl Strategy<Value = String> {
    let chunk = prop_oneof![
        3 => Just("ok".to_owned()),
        3 => Just("GET /orders/{id}".to_owned()),
        2 => Just("the quick brown fox jumps over the lazy dog and keeps going".to_owned()),
        2 => Just("\u{6ce8}\u{6587}\u{3092}\u{78ba}\u{8a8d}\u{3057}\u{307e}\u{3059}".to_owned()),
        1 => Just("\u{1f680}\u{2705} launched".to_owned()),
        1 => Just("\n".to_owned()),
        1 => "[a-z]{1,30}",
    ];
    prop::collection::vec(chunk, 0..8).prop_map(|parts| {
        let joined = parts.join(" ");
        let t: String = joined.chars().take(130).collect();
        debug_assert!(t.chars().count() <= MAX_LABEL_CHARS);
        t
    })
}

fn actor_label() -> impl Strategy<Value = String> {
    prop_oneof![
        3 => "[A-Za-z][A-Za-z0-9 ]{0,39}",
        1 => "[\u{4e00}-\u{9fa5}]{1,20}",
    ]
}

#[derive(Debug, Clone)]
enum Ev {
    Msg(usize, usize, String, bool),
    Note(usize, Option<usize>, String),
}

/// Sequences of 1-6 actors and 0-14 events, within every render cap and label limit.
pub fn sequence_ir() -> impl Strategy<Value = SequenceIr> {
    (1usize..=6).prop_flat_map(|n| {
        let ev = prop_oneof![
            3 => (0..n, 0..n, text(), any::<bool>()).prop_map(|(a, b, t, d)| Ev::Msg(a, b, t, d)),
            1 => (0..n, prop::option::of(0..n), text())
                .prop_map(|(a, b, t)| Ev::Note(a, b, format!("n {t}"))),
        ];
        (
            prop::collection::vec(actor_label(), n),
            prop::collection::vec(ev, 0..=14),
        )
            .prop_map(|(labels, events)| build(&labels, events))
    })
}

fn build(labels: &[String], events: Vec<Ev>) -> SequenceIr {
    let id = |i: usize| format!("a{i}");
    SequenceIr {
        actors: labels
            .iter()
            .enumerate()
            .map(|(i, l)| Actor {
                id: id(i),
                label: l.clone(),
                span: Default::default(),
            })
            .collect(),
        events: events
            .into_iter()
            .map(|e| match e {
                Ev::Msg(a, b, text, dashed) => SeqEvent::Message {
                    from: id(a),
                    to: id(b),
                    text,
                    dashed,
                    span: Default::default(),
                },
                Ev::Note(a, b, text) => SeqEvent::Note {
                    over: std::iter::once(a).chain(b).map(id).collect(),
                    text,
                    span: Default::default(),
                },
            })
            .collect(),
    }
}
