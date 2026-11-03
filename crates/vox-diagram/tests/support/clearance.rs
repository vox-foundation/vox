//! Edge clearance read back from the drawn SVG, never from the renderer's verifier: a path through
//! a node its edge does not join, an endpoint on a corner of its own node, and a path grazing a
//! foreign node. A path is read as the polyline through its points, a rounded corner at its vertex.
#![allow(dead_code)]

use vox_diagram::ir::GraphIr;

use super::svgcheck::{Shape, Svg};

/// An endpoint closer than this to a corner on both axes is on the corner: 12 px.
pub const CORNER: i64 = 24;
/// A path closer than this to a foreign node grazes it: 4 px.
pub const GRAZE: i64 = 8;

/// `(edge, node)` pairs per defect, edges in IR order and nodes by IR index.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Clearance {
    /// A segment through the interior of a node the edge does not join.
    pub through: Vec<(usize, usize)>,
    /// An endpoint within [`CORNER`] of a corner of its own node.
    pub corners: Vec<(usize, usize)>,
    /// A path within [`GRAZE`] of a node the edge does not join, without entering it.
    pub grazing: Vec<(usize, usize)>,
}

/// Whether segment `a`-`b` passes through the interior of `r`: the extents overlap and the line
/// through the segment splits the corners (touching a side is clear).
pub fn through(a: (i64, i64), b: (i64, i64), r: &Shape) -> bool {
    if a.0.max(b.0) <= r.x
        || a.0.min(b.0) >= r.x + r.w
        || a.1.max(b.1) <= r.y
        || a.1.min(b.1) >= r.y + r.h
    {
        return false;
    }
    let side = |x: i64, y: i64| (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
    let s = [
        side(r.x, r.y),
        side(r.x + r.w, r.y),
        side(r.x, r.y + r.h),
        side(r.x + r.w, r.y + r.h),
    ];
    !(s.iter().all(|&v| v >= 0) || s.iter().all(|&v| v <= 0))
}

fn grown(r: &Shape, d: i64) -> Shape {
    Shape {
        x: r.x - d,
        y: r.y - d,
        w: r.w + 2 * d,
        h: r.h + 2 * d,
    }
}

/// Whether `p` lies within [`CORNER`] of a corner of `r` on both axes.
pub fn near_corner(r: &Shape, p: (i64, i64)) -> bool {
    [r.x, r.x + r.w].iter().any(|&x| (p.0 - x).abs() < CORNER)
        && [r.y, r.y + r.h].iter().any(|&y| (p.1 - y).abs() < CORNER)
}

/// Every clearance defect of the drawn graph `s` of `g`.
pub fn clearance(g: &GraphIr, s: &Svg) -> Clearance {
    let index = |id: &str| g.nodes.iter().position(|n| n.id == id);
    let mut out = Clearance::default();
    for (e, (edge, path)) in g.edges.iter().zip(&s.edge_paths).enumerate() {
        let ends = [index(&edge.from), index(&edge.to)];
        for (k, r) in s.nodes.iter().enumerate() {
            if ends.contains(&Some(k)) {
                continue;
            }
            let hits = |r: &Shape| path.windows(2).any(|w| through(w[0], w[1], r));
            if hits(r) {
                out.through.push((e, k));
            } else if hits(&grown(r, GRAZE)) {
                out.grazing.push((e, k));
            }
        }
        if ends[0] == ends[1] {
            continue;
        }
        for (end, p) in [(ends[0], path.first()), (ends[1], path.last())] {
            if let (Some(k), Some(&p)) = (end, p)
                && near_corner(&s.nodes[k], p)
            {
                out.corners.push((e, k));
            }
        }
    }
    out
}
