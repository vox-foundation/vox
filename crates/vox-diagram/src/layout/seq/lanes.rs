//! Lane centres of a sequence layout: the gap between two lanes grows to fit its actors, the
//! message labels and self-message loops inside it, and the overhang of the notes beside it.

use super::{
    ACTOR_GAP, LANE_CLEAR, NOTE_MIN_W, NOTE_OVERHANG, PAD_X, SELF_LABEL_GAP, SELF_W, Step, W, px,
};

/// The width of a note whose lanes are `span` apart: its text, at least [`NOTE_MIN_W`], and
/// [`NOTE_OVERHANG`] past the outermost lanes.
pub(super) fn note_w(text_w: W, span: W) -> W {
    (text_w + 2 * px(W::from(PAD_X)))
        .max(NOTE_MIN_W)
        .max(span + 2 * NOTE_OVERHANG)
}

/// Lane centres: lane 0 at 0, each gap wide enough for its actors, every crossing label and the
/// overhang of every note beside it.
pub(super) fn lane_xs(widths: &[W], steps: &[Step]) -> Vec<W> {
    let mut gaps: Vec<W> = widths
        .windows(2)
        .map(|p| p[0] / 2 + p[1] / 2 + ACTOR_GAP)
        .collect();
    for st in steps {
        let Step::Msg {
            from,
            to,
            label: Some(l),
            ..
        } = st
        else {
            continue;
        };
        if from == to {
            // A self-message label sits right of its loop; the last lane has no gap, the canvas grows.
            if let Some(g) = gaps.get_mut(*from) {
                *g = (*g).max(SELF_W + SELF_LABEL_GAP + l.w + LANE_CLEAR);
            }
        } else if let Some(span) = gaps.get_mut(*from.min(to)..*from.max(to)) {
            for g in span {
                *g = (*g).max(l.w + 2 * LANE_CLEAR);
            }
        }
    }
    for st in steps {
        let Step::Note { lo, hi, text_w, .. } = st else {
            continue;
        };
        // Interior gaps only grow later, which only shrinks the overhang: this is an upper bound.
        let span: W = gaps.get(*lo..*hi).map_or(0, |g| g.iter().sum());
        // One half-pixel of slack for the rounding of the note's centre.
        let side = (note_w(*text_w, span) - span) / 2 + 1 + LANE_CLEAR;
        // The first and last lane have no outer gap: the canvas grows instead.
        for i in [lo.checked_sub(1), Some(*hi)].into_iter().flatten() {
            if let Some(g) = gaps.get_mut(i) {
                *g = (*g).max(side);
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
