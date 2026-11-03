//! Sequence geometry. Integer half-pixels only: lane centres, rows, notes and label boxes are
//! sums of measured text sizes and named constants, so the same IR always lays out the same.
//! Arithmetic runs in `i64` and the finished canvas is checked against a size limit before it is
//! narrowed to [`Half`], so no input can overflow.
//!
//! Each gap between adjacent lanes is at least wide enough for the widest message label that
//! crosses it plus [`LANE_CLEAR`] on each side, and for the loop and label of a self-message on
//! its left lane. A message label sits inside one gap, so it never covers a lifeline it crosses;
//! the gaps beside a note are at least its overhang plus [`LANE_CLEAR`], so a note never covers
//! a lifeline of an actor it is not over.

use std::collections::BTreeMap;

use super::input::check_label;
use super::text::{LINE_H, PAD_X, PAD_Y, measure, node_size, pad_x, wrap, wrap_at};
use super::{Half, Rect};
use crate::ir::Shape;
use crate::limits::{MAX_RENDER_ACTORS, MAX_RENDER_EVENTS};
use crate::sequence_ir::{SeqEvent, SequenceIr};
use crate::svg::RenderError;
use canvas::{Bx, Raw, finish};
use lanes::{lane_xs, note_w};

/// Working coordinate: half-pixels in 64 bits.
type W = i64;

/// Pixels to half-pixels.
const fn px(n: W) -> W {
    n * 2
}

/// Cells per line of message and note text; longer lines soft-wrap (explicit `\n` is kept).
const MSG_WRAP_CELLS: usize = 60;
/// Minimum clearance between two neighbouring actor boxes.
const ACTOR_GAP: W = px(24);
/// Clearance between a crossing label box and each lifeline it lies between.
const LANE_CLEAR: W = px(8);
/// Padding of a message label box around its text.
const LABEL_PAD: W = px(4);
/// Distance from the actor row to the first event.
const FIRST_ROW: W = px(20);
/// Distance between two events.
const ROW_GAP: W = px(16);
/// Width of a self-message loop.
const SELF_W: W = px(30);
/// Height of a self-message loop.
const SELF_H: W = px(20);
/// Distance from a self-message loop to its label box.
const SELF_LABEL_GAP: W = px(6);
/// Narrowest note.
const NOTE_MIN_W: W = px(80);
/// How far a note extends past the outermost lifeline it spans.
const NOTE_OVERHANG: W = px(16);
/// Arrow height below the label box of a labelled message.
const ARROW_BELOW_LABEL: W = px(4);
/// Arrow height below the cursor of a message without a label.
const ARROW_BARE: W = px(6);

/// A laid-out sequence diagram, shifted so the minimum of everything drawn sits at the margin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct SeqPlaced {
    pub width: Half,
    pub height: Half,
    /// Wrapped label lines and box of each actor, IR order.
    pub actors: Vec<(Vec<String>, Rect)>,
    /// `(x, top, bottom)` of each lifeline, IR order.
    pub lifelines: Vec<(Half, Half, Half)>,
    /// One entry per IR event, IR order.
    pub events: Vec<SeqDrawn>,
}

/// One drawn event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SeqDrawn {
    /// An arrow (two points) or a self-message loop (four points).
    Message {
        points: Vec<(Half, Half)>,
        dashed: bool,
        label: Option<(Vec<String>, Rect)>,
    },
    /// A note box.
    Note { lines: Vec<String>, rect: Rect },
}

/// A wrapped label and the size of its box.
struct Lbl {
    lines: Vec<String>,
    w: W,
    h: W,
}

/// An event with its actors resolved to lane indexes.
enum Step {
    Msg {
        from: usize,
        to: usize,
        dashed: bool,
        label: Option<Lbl>,
    },
    Note {
        lo: usize,
        hi: usize,
        lines: Vec<String>,
        text_w: W,
    },
}

fn count(n: usize) -> W {
    W::try_from(n).unwrap_or(W::MAX / 4)
}

fn widest(lines: &[String]) -> W {
    lines.iter().map(|l| W::from(measure(l))).max().unwrap_or(0)
}

/// [`pad_x`] on `W` values (saturating at the `Half` range).
fn hpad(tw: W, base: W) -> W {
    let h = |v: W| Half::try_from(v).unwrap_or(if v < 0 { Half::MIN } else { Half::MAX });
    W::from(pad_x(h(tw), h(base)))
}

/// The height of `n` text lines.
fn block(n: usize) -> W {
    count(n) * px(W::from(LINE_H))
}

/// The label box of a message: explicit newlines kept, lines soft-wrapped at [`MSG_WRAP_CELLS`].
fn label_box(text: &str) -> Option<Lbl> {
    let lines = wrap_at(text, MSG_WRAP_CELLS);
    if lines.is_empty() {
        return None;
    }
    let tw = widest(&lines);
    let (w, h) = (
        tw + 2 * hpad(tw, LABEL_PAD),
        block(lines.len()) + 2 * LABEL_PAD,
    );
    Some(Lbl { lines, w, h })
}

fn lane_of(lanes: &BTreeMap<&str, usize>, i: usize, id: &str) -> Result<usize, RenderError> {
    lanes
        .get(id)
        .copied()
        .ok_or_else(|| RenderError::InvalidIr(format!("event {i}: unknown actor")))
}

fn steps(s: &SequenceIr, lanes: &BTreeMap<&str, usize>) -> Result<Vec<Step>, RenderError> {
    let mut out = Vec::with_capacity(s.events.len());
    for (i, e) in s.events.iter().enumerate() {
        out.push(match e {
            SeqEvent::Message {
                from,
                to,
                text,
                dashed,
                ..
            } => Step::Msg {
                from: lane_of(lanes, i, from)?,
                to: lane_of(lanes, i, to)?,
                dashed: *dashed,
                label: label_box(text),
            },
            SeqEvent::Note { over, text, .. } => {
                let mut ends = Vec::with_capacity(over.len());
                for id in over {
                    ends.push(lane_of(lanes, i, id)?);
                }
                let (Some(&lo), Some(&hi)) = (ends.iter().min(), ends.iter().max()) else {
                    return Err(RenderError::InvalidIr(format!(
                        "event {i}: note over no actor"
                    )));
                };
                let lines = wrap_at(text, MSG_WRAP_CELLS);
                Step::Note {
                    lo,
                    hi,
                    text_w: widest(&lines),
                    lines,
                }
            }
        });
    }
    Ok(out)
}

fn at(xs: &[W], i: usize) -> W {
    xs.get(i).copied().unwrap_or(0)
}

/// A message between lanes `from` and `to` (different): label box on top, arrow below it. The
/// box is centred in the gap at the middle of the span, which `lane_xs` made wide enough, so it
/// stays clear of every lifeline including those the arrow crosses.
fn between(l: &Lbl, xs: &[W], (from, to): (usize, usize), cursor: W) -> (Bx, W) {
    let (lo, hi) = (from.min(to), from.max(to));
    let gap = lo + (hi - lo).saturating_sub(1) / 2;
    let mid = (at(xs, gap) + at(xs, gap + 1)) / 2;
    (
        Bx {
            x: mid - l.w / 2,
            y: cursor,
            w: l.w,
            h: l.h,
        },
        cursor + l.h + ARROW_BELOW_LABEL,
    )
}

/// Reject oversized or inconsistent IR before anything is placed, in this order: the render
/// caps, duplicate actor ids, label lengths, note arity. Returns the lane of each actor id.
fn validate(s: &SequenceIr) -> Result<BTreeMap<&str, usize>, RenderError> {
    if s.actors.len() > MAX_RENDER_ACTORS || s.events.len() > MAX_RENDER_EVENTS {
        return Err(RenderError::TooLarge {
            nodes: s.actors.len(),
            edges: s.events.len(),
        });
    }
    let mut lanes = BTreeMap::new();
    for (i, a) in s.actors.iter().enumerate() {
        if lanes.insert(a.id.as_str(), i).is_some() {
            return Err(RenderError::InvalidIr(format!("actor {i}: duplicate id")));
        }
        check_label(&format!("actor {i}"), &a.label)?;
    }
    for (i, e) in s.events.iter().enumerate() {
        let (SeqEvent::Message { text, .. } | SeqEvent::Note { text, .. }) = e;
        check_label(&format!("event {i}"), text)?;
    }
    for (i, e) in s.events.iter().enumerate() {
        if let SeqEvent::Note { over, .. } = e
            && !(1..=2).contains(&over.len())
        {
            return Err(RenderError::InvalidIr(format!(
                "event {i}: a note covers one or two actors"
            )));
        }
    }
    Ok(lanes)
}

/// Place `s`. Oversized or inconsistent IR, including an event naming an actor that is not
/// declared, is rejected as described on [`validate`] and [`RenderError::InvalidIr`].
pub(crate) fn place(s: &SequenceIr) -> Result<SeqPlaced, RenderError> {
    let lanes = validate(s)?;
    let labels: Vec<Vec<String>> = s.actors.iter().map(|a| wrap(&a.label)).collect();
    let sizes: Vec<(Half, Half)> = labels.iter().map(|l| node_size(Shape::Rect, l)).collect();
    let widths: Vec<W> = sizes.iter().map(|&(w, _)| W::from(w)).collect();
    let actor_h = sizes.iter().map(|&(_, h)| W::from(h)).max().unwrap_or(0);
    let steps = steps(s, &lanes)?;
    let xs = lane_xs(&widths, &steps);
    let mut cursor = actor_h + FIRST_ROW;
    let mut raw = Vec::with_capacity(steps.len());
    for st in &steps {
        match st {
            Step::Msg {
                from,
                to,
                dashed,
                label,
            } if from == to => {
                let x = at(&xs, *from);
                let lb = label.as_ref().map(|l| {
                    // Centred on the loop, but never above the cursor (the previous row).
                    let top = (cursor + SELF_H / 2 - l.h / 2).max(cursor);
                    (l, top)
                });
                let y0 = lb.map_or(cursor, |(l, top)| top + l.h / 2 - SELF_H / 2);
                let points = vec![
                    (x, y0),
                    (x + SELF_W, y0),
                    (x + SELF_W, y0 + SELF_H),
                    (x, y0 + SELF_H),
                ];
                let bottom = lb.map_or(0, |(l, top)| top + l.h).max(y0 + SELF_H);
                let label = lb.map(|(l, top)| {
                    let bx = Bx {
                        x: x + SELF_W + SELF_LABEL_GAP,
                        y: top,
                        w: l.w,
                        h: l.h,
                    };
                    (l.lines.clone(), bx)
                });
                cursor = bottom + ROW_GAP;
                raw.push(Raw::Msg {
                    points,
                    dashed: *dashed,
                    label,
                });
            }
            Step::Msg {
                from,
                to,
                dashed,
                label,
            } => {
                let (a, b) = (at(&xs, *from), at(&xs, *to));
                let (bx, y) = match label {
                    Some(l) => {
                        let (bx, y) = between(l, &xs, (*from, *to), cursor);
                        (Some((l.lines.clone(), bx)), y)
                    }
                    None => (None, cursor + ARROW_BARE),
                };
                cursor = y + ROW_GAP;
                raw.push(Raw::Msg {
                    points: vec![(a, y), (b, y)],
                    dashed: *dashed,
                    label: bx,
                });
            }
            Step::Note {
                lo,
                hi,
                lines,
                text_w,
            } => {
                let (l, r) = (at(&xs, *lo), at(&xs, *hi));
                let w = note_w(*text_w, r - l);
                let h = block(lines.len()) + 2 * px(W::from(PAD_Y));
                let bx = Bx {
                    x: (l + r) / 2 - w / 2,
                    y: cursor,
                    w,
                    h,
                };
                cursor += h + ROW_GAP;
                raw.push(Raw::Note {
                    lines: lines.clone(),
                    bx,
                });
            }
        }
    }
    let bottom = cursor.max(actor_h + FIRST_ROW);
    let actor_boxes: Vec<(Vec<String>, Bx)> = labels
        .into_iter()
        .zip(&widths)
        .zip(&xs)
        .map(|((lines, &w), &x)| {
            let bx = Bx {
                x: x - w / 2,
                y: 0,
                w,
                h: actor_h,
            };
            (lines, bx)
        })
        .collect();
    finish(actor_boxes, &xs, (actor_h, bottom), raw)
}

mod canvas;
mod lanes;
#[cfg(test)]
mod tests;
