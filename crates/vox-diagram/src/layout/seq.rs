//! Sequence geometry. Integer half-pixels only: lane centres, rows, notes and label boxes are
//! sums of measured text sizes and named constants, so the same IR always lays out the same.
//! Arithmetic runs in `i64` and the finished canvas is checked against a size limit before it is
//! narrowed to [`Half`], so no input can overflow.
//!
//! Each gap between adjacent lanes is at least wide enough for the widest message label that
//! crosses it plus [`LANE_CLEAR`] on each side.

use std::collections::BTreeMap;

use super::text::{LINE_H, PAD_X, PAD_Y, measure, node_size, wrap, wrap_at};
use super::{Half, Rect};
use crate::ir::Shape;
use crate::sequence_ir::{SeqEvent, SequenceIr};
use crate::svg::RenderError;
use canvas::{Bx, Raw, finish};

/// Working coordinate: half-pixels in 64 bits.
type W = i64;

/// Pixels to half-pixels.
const fn px(n: W) -> W {
    n * 2
}

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

/// The height of `n` text lines.
fn block(n: usize) -> W {
    count(n) * px(W::from(LINE_H))
}

/// The label box of a message: all lines kept as written, no soft wrap.
fn label_box(text: &str) -> Option<Lbl> {
    let lines = wrap_at(text, usize::MAX);
    if lines.is_empty() {
        return None;
    }
    let (w, h) = (
        widest(&lines) + 2 * LABEL_PAD,
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
                let lines = wrap_at(text, usize::MAX);
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

/// Lane centres: lane 0 at 0, each gap wide enough for its actors and every crossing label.
fn lane_xs(widths: &[W], steps: &[Step]) -> Vec<W> {
    let mut gaps: Vec<W> = widths
        .windows(2)
        .map(|p| p[0] / 2 + p[1] / 2 + ACTOR_GAP)
        .collect();
    for st in steps {
        if let Step::Msg {
            from,
            to,
            label: Some(l),
            ..
        } = st
            && from != to
            && let Some(span) = gaps.get_mut(*from.min(to)..*from.max(to))
        {
            for g in span {
                *g = (*g).max(l.w + 2 * LANE_CLEAR);
            }
        }
    }
    let mut x = 0;
    let mut xs = Vec::with_capacity(widths.len());
    for i in 0..widths.len() {
        xs.push(x);
        x += gaps.get(i).copied().unwrap_or(0);
    }
    xs
}

fn at(xs: &[W], i: usize) -> W {
    xs.get(i).copied().unwrap_or(0)
}

/// A message between two lanes: label box on top, arrow below it.
fn between(l: &Lbl, a: W, b: W, cursor: W) -> (Bx, W) {
    let mid = (a + b) / 2;
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

/// Place `s`. An event naming an actor that is not declared is [`RenderError::InvalidIr`].
pub(crate) fn place(s: &SequenceIr) -> Result<SeqPlaced, RenderError> {
    let mut lanes = BTreeMap::new();
    for (i, a) in s.actors.iter().enumerate() {
        lanes.entry(a.id.as_str()).or_insert(i);
    }
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
                        let (bx, y) = between(l, a, b, cursor);
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
                let w = (text_w + 2 * px(W::from(PAD_X)))
                    .max(NOTE_MIN_W)
                    .max(r - l + 2 * NOTE_OVERHANG);
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
#[cfg(test)]
mod tests;
