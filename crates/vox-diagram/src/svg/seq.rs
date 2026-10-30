//! Sequence diagram drawing. Every attribute and text goes through the single writer; the
//! label text only ever lands in `<tspan>` content. Draw order: lifelines, actor boxes, notes,
//! message arrows, message labels.

use std::collections::BTreeMap;

use super::writer::{Writer, num};
use super::{RenderError, SvgOpts, cut, open_document, paint, shapes, short};
use crate::ir::Shape;
use crate::layout::seq::{SeqDrawn, SeqPlaced, place};
use crate::layout::{Half, LNode};
use crate::sequence_ir::{SeqEvent, SequenceIr};

/// Dash pattern of a return (`reply`) arrow.
const DASHED: &str = "stroke-dasharray:6 4";

/// Render `s` as an SVG document.
pub(super) fn render_sequence(s: &SequenceIr, opts: &SvgOpts) -> Result<String, RenderError> {
    let placed = place(s)?;
    let mut w = Writer::new();
    let arrow = open_document(&mut w, opts, placed.width, placed.height, &describe(s));
    lifelines(&mut w, &placed);
    actors(&mut w, &placed);
    notes(&mut w, &placed);
    arrows(&mut w, &placed, &arrow);
    arrow_labels(&mut w, &placed);
    w.close("svg");
    Ok(w.finish())
}

fn lifelines(w: &mut Writer, p: &SeqPlaced) {
    let style = format!(
        "fill:none;stroke:{};stroke-width:1;stroke-dasharray:4 4",
        paint("--vd-edge")
    );
    w.open("g", &[("class", "vd-lifelines"), ("style", &style)]);
    for &(x, top, bottom) in &p.lifelines {
        let x = num(x);
        let d = format!("M{x} {}L{x} {}", num(top), num(bottom));
        w.empty("path", &[("class", "vd-lifeline"), ("d", &d)]);
    }
    w.close("g");
}

fn actors(w: &mut Writer, p: &SeqPlaced) {
    w.open("g", &[("class", "vd-nodes")]);
    for (lines, r) in &p.actors {
        let node = LNode {
            shape: Shape::Rect,
            lines: lines.clone(),
            w: r.w,
            h: r.h,
            container: None,
        };
        shapes::write_node(w, &node, *r);
    }
    w.close("g");
}

fn notes(w: &mut Writer, p: &SeqPlaced) {
    let drawn: Vec<_> = p
        .events
        .iter()
        .filter_map(|e| match e {
            SeqDrawn::Note { lines, rect } => Some((lines, *rect)),
            SeqDrawn::Message { .. } => None,
        })
        .collect();
    if drawn.is_empty() {
        return;
    }
    let style = format!(
        "fill:{};stroke:{};stroke-width:1.5",
        paint("--vd-container-fill"),
        paint("--vd-container-stroke")
    );
    w.open("g", &[("class", "vd-notes")]);
    for (lines, r) in drawn {
        let (x, y, bw, bh) = (num(r.x), num(r.y), num(r.w), num(r.h));
        w.empty(
            "rect",
            &[
                ("class", "vd-note"),
                ("x", &x),
                ("y", &y),
                ("width", &bw),
                ("height", &bh),
                ("rx", "3"),
                ("style", &style),
            ],
        );
        shapes::label(
            w,
            lines,
            r,
            ("vd-note-label", "--vd-text"),
            shapes::LABEL_STEP,
        );
    }
    w.close("g");
}

/// Path data from straight segments only: `M` then `L`.
fn straight(points: &[(Half, Half)]) -> String {
    let mut d = String::new();
    for (i, &(x, y)) in points.iter().enumerate() {
        d.push(if i == 0 { 'M' } else { 'L' });
        d.push_str(&format!("{} {}", num(x), num(y)));
    }
    d
}

fn arrows(w: &mut Writer, p: &SeqPlaced, arrow_id: &str) {
    let style = format!("fill:none;stroke:{};stroke-width:1.5", paint("--vd-edge"));
    let marker = format!("url(#{arrow_id})");
    w.open("g", &[("class", "vd-edges"), ("style", &style)]);
    for e in &p.events {
        let SeqDrawn::Message { points, dashed, .. } = e else {
            continue;
        };
        let d = straight(points);
        let mut attrs = vec![
            ("class", "vd-edge"),
            ("d", d.as_str()),
            ("marker-end", marker.as_str()),
        ];
        if *dashed {
            attrs.push(("style", DASHED));
        }
        w.empty("path", &attrs);
    }
    w.close("g");
}

/// Message labels on a `--vd-bg` background, after the arrows; no group without labels.
fn arrow_labels(w: &mut Writer, p: &SeqPlaced) {
    let labels: Vec<_> = p
        .events
        .iter()
        .filter_map(|e| match e {
            SeqDrawn::Message { label, .. } => label.as_ref(),
            SeqDrawn::Note { .. } => None,
        })
        .collect();
    if labels.is_empty() {
        return;
    }
    let bg = format!("fill:{};stroke:none", paint("--vd-bg"));
    w.open("g", &[("class", "vd-edge-labels")]);
    for (lines, r) in labels {
        let (x, y, bw, bh) = (num(r.x), num(r.y), num(r.w), num(r.h));
        w.empty(
            "rect",
            &[
                ("class", "vd-edge-label-bg"),
                ("x", &x),
                ("y", &y),
                ("width", &bw),
                ("height", &bh),
                ("style", &bg),
            ],
        );
        shapes::label(
            w,
            lines,
            *r,
            ("vd-edge-label", "--vd-muted"),
            shapes::LABEL_STEP,
        );
    }
    w.close("g");
}

/// The accessible description: counts, then `from → to` pairs by actor label.
fn describe(s: &SequenceIr) -> String {
    let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let mut by_id: BTreeMap<&str, &str> = BTreeMap::new();
    for a in &s.actors {
        by_id.entry(a.id.as_str()).or_insert(a.label.as_str());
    }
    let label_of = |id: &str| short(by_id.get(id).copied().unwrap_or(id), 24);
    let mut pairs = Vec::new();
    let mut notes = 0;
    for e in &s.events {
        match e {
            SeqEvent::Message { from, to, .. } => {
                pairs.push(format!("{} \u{2192} {}", label_of(from), label_of(to)));
            }
            SeqEvent::Note { .. } => notes += 1,
        }
    }
    let mut d = format!(
        "{}, {}",
        plural(s.actors.len(), "actor"),
        plural(pairs.len(), "message")
    );
    if notes > 0 {
        d.push_str(&format!(", {}", plural(notes, "note")));
    }
    if !pairs.is_empty() {
        d.push_str(": ");
        d.push_str(&pairs.join("; "));
    }
    cut(&d, 300)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sequence_ir::Actor;

    fn actor(id: &str, label: &str) -> Actor {
        Actor {
            id: id.into(),
            label: label.into(),
            span: Default::default(),
        }
    }

    fn message(from: &str, to: &str) -> SeqEvent {
        SeqEvent::Message {
            from: from.into(),
            to: to.into(),
            text: String::new(),
            dashed: false,
            span: Default::default(),
        }
    }

    fn ir(events: Vec<SeqEvent>) -> SequenceIr {
        SequenceIr {
            actors: vec![actor("a", "Alpha  one\nsecond"), actor("b", "B")],
            events,
        }
    }

    #[test]
    fn desc_counts_and_pairs() {
        assert_eq!(describe(&ir(vec![])), "2 actors, 0 messages");
        assert_eq!(
            describe(&ir(vec![message("a", "b")])),
            "2 actors, 1 message: Alpha one \u{2192} B"
        );
        assert_eq!(
            describe(&ir(vec![message("a", "b"), message("b", "b")])),
            "2 actors, 2 messages: Alpha one \u{2192} B; B \u{2192} B"
        );
    }

    #[test]
    fn desc_counts_notes_and_cuts_long_text() {
        let note = SeqEvent::Note {
            over: vec!["a".into()],
            text: "n".into(),
            span: Default::default(),
        };
        assert_eq!(
            describe(&ir(vec![note.clone()])),
            "2 actors, 0 messages, 1 note"
        );
        let many: Vec<SeqEvent> = (0..40).map(|_| message("a", "b")).collect();
        let d = describe(&ir(many));
        assert_eq!(d.chars().count(), 300);
        assert!(d.ends_with('\u{2026}'));
    }

    #[test]
    fn a_dashed_arrow_carries_the_dash_style_and_a_plain_one_does_not() {
        let mut dashed = message("a", "b");
        if let SeqEvent::Message { dashed: d, .. } = &mut dashed {
            *d = true;
        }
        let svg = render_sequence(&ir(vec![dashed, message("b", "a")]), &SvgOpts::default());
        let Ok(svg) = svg else {
            panic!("render failed");
        };
        assert_eq!(svg.matches(DASHED).count(), 1);
        assert_eq!(svg.matches("class=\"vd-edge\"").count(), 2);
    }
}
