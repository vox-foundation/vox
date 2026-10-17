//! Hard-invariant geometry check on quantized integer geometry: exact comparisons, no epsilon.
//! A layout that fails is never drawn. Soft defects (an edge crossing a non-endpoint node, a
//! label merely touching a node) are deliberately not checked; dagre does not route around nodes.
//! An edge label sharing interior area with a node, a title band or another label is hard.

use std::fmt;

use super::finish::title_band;
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
    /// A child (node or container) intrudes into a container's title band.
    TitleBand(usize),
    /// An edge label box overlaps a container's title band (edge, container): its opaque
    /// background would erase title text.
    LabelInTitleBand(usize, usize),
    /// An edge label box overlaps a node box (edge, node).
    LabelOnNode(usize, usize),
    /// Two edge label boxes share interior area (earlier edge, later edge).
    LabelOnLabel(usize, usize),
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

fn overlap(a: Rect, b: Rect) -> bool {
    a.x < b.x + b.w && b.x < a.x + a.w && a.y < b.y + b.h && b.y < a.y + a.h
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
fn encloses(input: &LayoutInput, c: usize, start: Option<usize>) -> bool {
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
/// inside a container, and no child in a title band. Containers are checked before the view box.
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
        let band = Rect {
            h: title_band(c.title_lines.len()),
            ..c.rect
        };
        let node_in = input
            .nodes
            .iter()
            .zip(&placed.nodes)
            .any(|(n, r)| n.container == Some(k) && overlap(*r, band));
        let box_in = input
            .containers
            .iter()
            .zip(&rects)
            .any(|(sub, r)| sub.parent == Some(k) && overlap(*r, band));
        if node_in || box_in {
            return Err(Violation::TitleBand(k));
        }
    }
    Ok(())
}

/// Edge-label boxes clear of every title band, node box and later label box. Labels are drawn on an
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
            let band = Rect {
                h: title_band(c.title_lines.len()),
                ..c.rect
            };
            if overlap(l, band) {
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
