//! Hard-invariant geometry check on quantized integer geometry: exact comparisons, no epsilon.
//! A layout that fails is never drawn. A label merely touching a node is fine. An edge label
//! sharing interior area with a node, a title box or another label is hard. An edge through a
//! title box or through a node it does not join is checked separately ([`title_crossing`],
//! [`node_crossing`]): it escalates the rung but does not reject the last one, since dagre does
//! not route around boxes. An edge end on a node corner ([`port_at_corner`]) is a test metric.

use std::fmt;

use super::text::{LINE_H, PAD_X, PAD_Y, measure};
use super::{Half, LNode, LayoutInput, Placed, Rect};
use crate::ir::Shape;

/// A hard invariant a layout broke. Names kinds and indices only, never label text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Violation {
    /// Two node boxes share interior area (touching edges are fine).
    NodeOverlap(usize, usize),
    /// A node's text block, with padding, does not fit its box.
    LabelOverflow(usize),
    /// A node, a container, an edge point or an edge label lies outside the view box.
    OutsideView,
    /// A node is not fully inside the container it belongs to.
    OutsideContainer(usize),
    /// A container is not fully inside its parent container.
    OutsideParent(usize),
    /// Two containers with the same parent share interior area.
    SiblingOverlap(usize, usize),
    /// A node overlaps a container that neither holds it nor encloses it (node, container).
    ForeignOverlap(usize, usize),
    /// A node, or a container that does not hold it, overlaps a container's title box.
    TitleBand(usize),
    /// An edge label box overlaps a container's title box (edge, container): its opaque
    /// background would erase title text.
    LabelInTitleBand(usize, usize),
    /// An edge segment passes through a container's title box (edge, container).
    EdgeThroughTitle(usize, usize),
    /// An edge label box overlaps a node box (edge, node).
    LabelOnNode(usize, usize),
    /// Two edge label boxes share interior area (earlier edge, later edge).
    LabelOnLabel(usize, usize),
    /// An edge segment passes through a node the edge does not join (edge, node).
    EdgeThroughNode(usize, usize),
    /// An edge ends within [`CORNER_CLEAR`] of a corner of its node (edge, node).
    #[cfg(test)]
    PortAtCorner(usize, usize),
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}

/// Whether the text of `n` fits a box of `w` by `h`: padded block for rectangular shapes, the
/// half-extent rule `tw/w + th/h <= 1` for a diamond.
fn fits(n: &LNode, w: Half, h: Half) -> bool {
    let tw = i64::from(n.lines.iter().map(|l| measure(l)).max().unwrap_or(0));
    let th = i64::try_from(n.lines.len()).unwrap_or(0) * i64::from(LINE_H) * 2;
    let (w, h) = (i64::from(w), i64::from(h));
    if n.shape == Shape::Diamond {
        tw * h + th * w <= w * h
    } else {
        w >= tw + 4 * i64::from(PAD_X) && h >= th + 4 * i64::from(PAD_Y)
    }
}

pub(crate) fn overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
}

/// Whether the segment `a`-`b` passes through the interior of `r` (touching a side or a corner is
/// clear). Separating-axis test in exact integer arithmetic: the boxes' extents, then the line
/// through the segment against the four corners.
pub(crate) fn crosses(a: (Half, Half), b: (Half, Half), r: Rect) -> bool {
    let (x0, y0, x1, y1) = (r.x, r.y, r.x + r.w, r.y + r.h);
    if a.0.max(b.0) <= x0 || a.0.min(b.0) >= x1 || a.1.max(b.1) <= y0 || a.1.min(b.1) >= y1 {
        return false;
    }
    let (dx, dy) = (i64::from(b.0 - a.0), i64::from(b.1 - a.1));
    let side = |x: Half, y: Half| dx * i64::from(y - a.1) - dy * i64::from(x - a.0);
    let s = [side(x0, y0), side(x1, y0), side(x0, y1), side(x1, y1)];
    !(s.iter().all(|&v| v >= 0) || s.iter().all(|&v| v <= 0))
}

fn inside(r: Rect, width: Half, height: Half) -> bool {
    r.x >= 0 && r.y >= 0 && r.x + r.w <= width && r.y + r.h <= height
}

/// Whether `inner` lies fully within `outer` (touching sides are fine).
fn contains(outer: Rect, inner: Rect) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && inner.x + inner.w <= outer.x + outer.w
        && inner.y + inner.h <= outer.y + outer.h
}

/// Whether container `c` is `start` or one of its ancestors. The walk is bounded by the container
/// count, so a cyclic hand-built parent chain ends.
pub(crate) fn encloses(input: &LayoutInput, c: usize, start: Option<usize>) -> bool {
    let mut cur = start;
    for _ in 0..=input.containers.len() {
        match cur {
            None => return false,
            Some(k) if k == c => return true,
            Some(k) => cur = input.containers.get(k).and_then(|x| x.parent),
        }
    }
    false
}

/// The containment rules: every child inside its container, siblings apart, nothing foreign
/// inside a container, and nothing on a title box but its own frame and its ancestors. Containers
/// are checked before the view box.
fn containment(input: &LayoutInput, placed: &Placed) -> Result<(), Violation> {
    let rects: Vec<Rect> = placed.containers.iter().map(|c| c.rect).collect();
    for (i, (n, r)) in input.nodes.iter().zip(&placed.nodes).enumerate() {
        if let Some(frame) = n.container.and_then(|k| rects.get(k))
            && !contains(*frame, *r)
        {
            return Err(Violation::OutsideContainer(i));
        }
    }
    for (i, c) in input.containers.iter().enumerate() {
        if let (Some(frame), Some(own)) = (c.parent.and_then(|p| rects.get(p)), rects.get(i))
            && !contains(*frame, *own)
        {
            return Err(Violation::OutsideParent(i));
        }
    }
    for (i, a) in input.containers.iter().enumerate() {
        for (j, b) in input.containers.iter().enumerate().skip(i + 1) {
            let apart = rects
                .get(i)
                .zip(rects.get(j))
                .is_none_or(|(x, y)| !overlap(*x, *y));
            if a.parent == b.parent && !apart {
                return Err(Violation::SiblingOverlap(i, j));
            }
        }
    }
    for (i, (n, r)) in input.nodes.iter().zip(&placed.nodes).enumerate() {
        for (k, frame) in rects.iter().enumerate() {
            if !encloses(input, k, n.container) && overlap(*r, *frame) {
                return Err(Violation::ForeignOverlap(i, k));
            }
        }
    }
    for (k, c) in placed.containers.iter().enumerate() {
        let node_in = placed.nodes.iter().any(|r| overlap(*r, c.title));
        let box_in = rects
            .iter()
            .enumerate()
            .any(|(j, r)| !encloses(input, j, Some(k)) && overlap(*r, c.title));
        if node_in || box_in {
            return Err(Violation::TitleBand(k));
        }
    }
    Ok(())
}

/// Edge-label boxes clear of every title box, node box and later label box. Labels are drawn on an
/// opaque background, so any overlap hides what is underneath.
fn labels(placed: &Placed) -> Result<(), Violation> {
    let boxes: Vec<(usize, Rect)> = placed
        .edges
        .iter()
        .enumerate()
        .filter_map(|(i, e)| Some((i, e.label?)))
        .collect();
    for (n, &(e, l)) in boxes.iter().enumerate() {
        for (k, c) in placed.containers.iter().enumerate() {
            if overlap(l, c.title) {
                return Err(Violation::LabelInTitleBand(e, k));
            }
        }
        if let Some(k) = placed.nodes.iter().position(|r| overlap(l, *r)) {
            return Err(Violation::LabelOnNode(e, k));
        }
        if let Some(&(o, _)) = boxes.iter().skip(n + 1).find(|(_, r)| overlap(l, *r)) {
            return Err(Violation::LabelOnLabel(e, o));
        }
    }
    Ok(())
}

/// The first edge segment through a title box, if any. Kept apart from [`verify`] because dagre
/// routes edges as straight segments between ranks and cannot always avoid a box: the caller
/// prefers a rung without one but draws the last rung anyway.
pub(crate) fn title_crossing(placed: &Placed) -> Result<(), Violation> {
    for (e, edge) in placed.edges.iter().enumerate() {
        for (k, c) in placed.containers.iter().enumerate() {
            if edge.points.windows(2).any(|s| crosses(s[0], s[1], c.title)) {
                return Err(Violation::EdgeThroughTitle(e, k));
            }
        }
    }
    Ok(())
}

/// The first edge segment through a node the edge does not join, if any. Soft like
/// [`title_crossing`]: the ports keep an edge's ends clear of its own rank, but dagre's straight
/// segments between ranks can still cut a node, so the caller prefers a rung without one.
pub(crate) fn node_crossing(input: &LayoutInput, placed: &Placed) -> Result<(), Violation> {
    for (e, (le, pe)) in input.edges.iter().zip(&placed.edges).enumerate() {
        for (n, r) in placed.nodes.iter().enumerate() {
            if n != le.from && n != le.to && pe.points.windows(2).any(|s| crosses(s[0], s[1], *r)) {
                return Err(Violation::EdgeThroughNode(e, n));
            }
        }
    }
    Ok(())
}

/// An endpoint closer than this to a corner of its node on both axes is on the corner: 12 px.
#[cfg(test)]
pub(crate) const CORNER_CLEAR: Half = 24;

/// The first edge end within [`CORNER_CLEAR`] of a corner of its own node, self-loops and
/// diamonds aside (a diamond end is drawn on its outline, far from the box corners). A metric for
/// tests, not a rung criterion: the ports put every end at least that far in, and a wider spacing
/// would not move an end the ports leave alone.
#[cfg(test)]
pub(crate) fn port_at_corner(input: &LayoutInput, placed: &Placed) -> Result<(), Violation> {
    let near = |r: Rect, p: (Half, Half)| {
        [r.x, r.x + r.w]
            .iter()
            .any(|&x| (p.0 - x).abs() < CORNER_CLEAR)
            && [r.y, r.y + r.h]
                .iter()
                .any(|&y| (p.1 - y).abs() < CORNER_CLEAR)
    };
    for (e, (le, pe)) in input.edges.iter().zip(&placed.edges).enumerate() {
        if le.from == le.to {
            continue;
        }
        for (n, p) in [(le.from, pe.points.first()), (le.to, pe.points.last())] {
            let diamond = input
                .nodes
                .get(n)
                .is_some_and(|x| x.shape == Shape::Diamond);
            if let (Some(&r), Some(&p)) = (placed.nodes.get(n), p)
                && !diamond
                && near(r, p)
            {
                return Err(Violation::PortAtCorner(e, n));
            }
        }
    }
    Ok(())
}

/// Check `placed` against `input`; the first violation found, nodes before edges.
pub(crate) fn verify(input: &LayoutInput, placed: &Placed) -> Result<(), Violation> {
    let (width, height) = (placed.width, placed.height);
    for (i, (n, r)) in input.nodes.iter().zip(&placed.nodes).enumerate() {
        if !fits(n, r.w, r.h) {
            return Err(Violation::LabelOverflow(i));
        }
        for (j, other) in placed.nodes.iter().enumerate().skip(i + 1) {
            if overlap(*r, *other) {
                return Err(Violation::NodeOverlap(i, j));
            }
        }
    }
    containment(input, placed)?;
    labels(placed)?;
    let boxes_ok = placed
        .nodes
        .iter()
        .chain(placed.containers.iter().map(|c| &c.rect))
        .all(|r| inside(*r, width, height));
    let edges_ok = placed.edges.iter().all(|e| {
        let points_ok = e
            .points
            .iter()
            .all(|&(x, y)| (0..=width).contains(&x) && (0..=height).contains(&y));
        points_ok && e.label.is_none_or(|r| inside(r, width, height))
    });
    if boxes_ok && edges_ok {
        Ok(())
    } else {
        Err(Violation::OutsideView)
    }
}

#[cfg(test)]
mod tests;
