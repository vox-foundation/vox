//! The canvas stage of the sequence layout: shift everything so its minimum sits at the margin
//! and narrow the working `i64` coordinates to [`Half`], failing closed on an oversized canvas.

use super::{SeqDrawn, SeqPlaced, W, px};
use crate::layout::{Half, MARGIN, Rect};
use crate::svg::RenderError;

/// Largest canvas side accepted, 100 000 px (the same bound as the graph layout).
const MAX_EXTENT: W = px(100_000);

/// A box in working coordinates.
#[derive(Debug, Clone, Copy)]
pub(super) struct Bx {
    pub x: W,
    pub y: W,
    pub w: W,
    pub h: W,
}

/// A drawn event before the canvas shift.
pub(super) enum Raw {
    Msg {
        points: Vec<(W, W)>,
        dashed: bool,
        label: Option<(Vec<String>, Bx)>,
    },
    Note {
        lines: Vec<String>,
        bx: Bx,
    },
}

/// Shift everything so its minimum sits at the margin and narrow to [`Half`].
pub(super) fn finish(
    actors: Vec<(Vec<String>, Bx)>,
    xs: &[W],
    (top, bottom): (W, W),
    raw: Vec<Raw>,
) -> Result<SeqPlaced, RenderError> {
    let boxes = actors
        .iter()
        .map(|(_, b)| *b)
        .chain(raw.iter().flat_map(|r| match r {
            Raw::Msg { label, .. } => label.as_ref().map(|(_, b)| *b),
            Raw::Note { bx, .. } => Some(*bx),
        }));
    let mut pts: Vec<(W, W)> = boxes
        .flat_map(|b| [(b.x, b.y), (b.x + b.w, b.y + b.h)])
        .collect();
    for r in &raw {
        if let Raw::Msg { points, .. } = r {
            pts.extend(points.iter().copied());
        }
    }
    pts.extend(xs.iter().map(|&x| (x, bottom)));
    let fold = |f: fn(&(W, W)) -> W| {
        let it = pts.iter().map(f);
        (it.clone().min().unwrap_or(0), it.max().unwrap_or(0))
    };
    let ((min_x, max_x), (min_y, max_y)) = (fold(|p| p.0), fold(|p| p.1));
    if max_x - min_x > MAX_EXTENT || max_y - min_y > MAX_EXTENT {
        return Err(RenderError::Geometry(
            "sequence diagram exceeds the canvas limit".to_owned(),
        ));
    }
    let (dx, dy) = (W::from(MARGIN) - min_x, W::from(MARGIN) - min_y);
    let narrow = |v: W| {
        Half::try_from(v).map_err(|_| RenderError::Geometry("coordinate out of range".to_owned()))
    };
    let rect = |b: Bx| -> Result<Rect, RenderError> {
        Ok(Rect {
            x: narrow(b.x + dx)?,
            y: narrow(b.y + dy)?,
            w: narrow(b.w)?,
            h: narrow(b.h)?,
        })
    };
    let point = |p: (W, W)| Ok::<_, RenderError>((narrow(p.0 + dx)?, narrow(p.1 + dy)?));
    let mut out = SeqPlaced {
        width: narrow(max_x - min_x + 2 * W::from(MARGIN))?,
        height: narrow(max_y - min_y + 2 * W::from(MARGIN))?,
        actors: Vec::with_capacity(actors.len()),
        lifelines: Vec::with_capacity(xs.len()),
        events: Vec::with_capacity(raw.len()),
    };
    for (lines, b) in actors {
        out.actors.push((lines, rect(b)?));
    }
    for &x in xs {
        out.lifelines
            .push((narrow(x + dx)?, narrow(top + dy)?, narrow(bottom + dy)?));
    }
    for r in raw {
        out.events.push(match r {
            Raw::Msg {
                points,
                dashed,
                label,
            } => SeqDrawn::Message {
                points: points.into_iter().map(point).collect::<Result<_, _>>()?,
                dashed,
                label: label
                    .map(|(lines, b)| Ok::<_, RenderError>((lines, rect(b)?)))
                    .transpose()?,
            },
            Raw::Note { lines, bx } => SeqDrawn::Note {
                lines,
                rect: rect(bx)?,
            },
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn boxed(x: W, y: W, w: W, h: W) -> Bx {
        Bx { x, y, w, h }
    }

    #[test]
    fn the_minimum_lands_on_the_margin() {
        let actors = vec![(vec!["A".to_owned()], boxed(-50, 0, 100, 40))];
        let Ok(p) = finish(actors, &[0], (40, 100), Vec::new()) else {
            panic!("finish failed");
        };
        assert_eq!(p.actors[0].1.x, MARGIN);
        assert_eq!(p.actors[0].1.y, MARGIN);
        assert_eq!(p.lifelines, vec![(MARGIN + 50, MARGIN + 40, MARGIN + 100)]);
        assert_eq!((p.width, p.height), (100 + 2 * MARGIN, 100 + 2 * MARGIN));
    }

    #[test]
    fn an_oversized_canvas_is_rejected() {
        let actors = vec![(Vec::new(), boxed(0, 0, MAX_EXTENT + 1, 10))];
        assert!(matches!(
            finish(actors, &[], (10, 10), Vec::new()),
            Err(RenderError::Geometry(_))
        ));
    }
}
