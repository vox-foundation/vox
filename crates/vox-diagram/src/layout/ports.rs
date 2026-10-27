//! Edge ports (Phase 90 review CR-01). dagre ends an edge where the ray from a node's centre to the
//! next route point leaves the node's box, and puts a long edge's first bend only half a rank away.
//! A shallow first segment therefore leaves through a side, next to a corner, and runs on through
//! the neighbour beside it in the same rank. Each end is moved to the side facing the next point,
//! kept off the corners, and the edge turns halfway into the gap beyond the rank, which holds no
//! node and, short of the next half rank, no edge label.

use super::verify::crosses;
use super::{Half, LayoutInput, PlacedEdge, Rect};
use crate::ir::{Direction, Shape};

type Pt = (Half, Half);

/// Nearest a port sits to a corner: 16 px, or the middle of a side shorter than 32 px. No wider:
/// a port moved off dagre's straight run into a node draws a jog just above it.
const MIN_INSET: Half = 32;

/// `p` in the flow frame, where the flow runs down (+y).
fn to_flow(d: Direction, (x, y): Pt) -> Pt {
    match d {
        Direction::Down => (x, y),
        Direction::Up => (x, -y),
        Direction::Right => (y, x),
        Direction::Left => (y, -x),
    }
}

/// Inverse of [`to_flow`].
fn from_flow(d: Direction, (x, y): Pt) -> Pt {
    match d {
        Direction::Down => (x, y),
        Direction::Up => (x, -y),
        Direction::Right => (y, x),
        Direction::Left => (-y, x),
    }
}

fn rect_to_flow(d: Direction, r: Rect) -> Rect {
    let (a, b) = (to_flow(d, (r.x, r.y)), to_flow(d, (r.x + r.w, r.y + r.h)));
    Rect {
        x: a.0.min(b.0),
        y: a.1.min(b.1),
        w: (a.0 - b.0).abs(),
        h: (a.1 - b.1).abs(),
    }
}

/// Nearest a straightened segment passes a box it does not enter: 4 px, so it never grazes one.
const CLEARANCE: Half = 8;

/// Closest two ports on one side sit, when the side has room: 12 px, an arrowhead's width.
const SPREAD: Half = 24;

/// Where an edge leaving `r` (flow frame) towards `next` crosses its rank, `band` being the rank's
/// top and bottom: the side of `r` facing `next` and the turn `reach` beyond the rank, at most
/// halfway to `next`. `None` when `next` is level with the rank, or `r` is degenerate.
fn exit(r: Rect, band: (Half, Half), next: Pt, reach: Half) -> Option<(Half, Half)> {
    if r.w <= 0 || r.h <= 0 {
        None
    } else if next.1 > band.1 {
        Some((r.y + r.h, band.1 + ((next.1 - band.1) / 2).min(reach)))
    } else if next.1 < band.0 {
        Some((r.y, band.0 - ((band.0 - next.1) / 2).min(reach)))
    } else {
        None
    }
}

/// Port positions in `lo..=hi` for the ends wanting `wants`, sorted: each clamped to the range,
/// then pushed [`SPREAD`] apart, or evenly apart when they cannot all fit, so arrowheads never merge.
fn spread(lo: Half, hi: Half, wants: &[Half]) -> Vec<Half> {
    let mut xs: Vec<Half> = wants.iter().map(|w| (*w).clamp(lo, hi)).collect();
    let last = Half::try_from(xs.len()).unwrap_or(1).max(1) - 1;
    let gap = if last == 0 {
        0
    } else {
        SPREAD.min((hi - lo) / last)
    };
    for i in 1..xs.len() {
        xs[i] = xs[i].max(xs[i - 1] + gap);
    }
    for (i, x) in xs.iter_mut().enumerate().rev() {
        let after = last - Half::try_from(i).unwrap_or(0);
        *x = (*x).min(hi - gap * after);
    }
    xs
}

/// One edge end to re-port: its edge and node, whether it is the edge's start, the side and turn
/// lines of [`exit`] and the x of the route point beyond it.
struct End {
    edge: usize,
    start: bool,
    node: usize,
    side: Half,
    turn: Half,
    want: Half,
}

/// Re-port every end of an edge between two distinct nodes. A diamond end is re-ported only when
/// dagre's segment cuts a foreign node, and then within the middle half of its side: a diamond has
/// no box corner to avoid, and `svg::paths` already moves dagre's end onto its outline along the
/// ray from its centre, which fans its edges out where vertex ports would bundle them. Each end leaves
/// the side facing its next route point, at least the inset from each corner and apart from the
/// other ends on that side, then turns beyond the rank and runs on to that point; the turn is
/// dropped when the port lies straight before it. The rank bands come from the node and title
/// boxes dagre placed (titles share ranks with nodes); `ranksep` is the rung's rank separation in
/// px: the gap beyond a rank is at least half of it, so a turn `ranksep / 4` px out is halfway.
pub(crate) fn route(
    input: &LayoutInput,
    nodes: &[Rect],
    titles: &[Rect],
    ranksep: i32,
    edges: &mut [PlacedEdge],
) {
    let d = input.direction;
    let flow: Vec<Rect> = nodes.iter().map(|r| rect_to_flow(d, *r)).collect();
    let boxes: Vec<Rect> = flow
        .iter()
        .copied()
        .chain(titles.iter().map(|r| rect_to_flow(d, *r)))
        .collect();
    let band = |r: Rect| {
        boxes
            .iter()
            .filter(|o| o.y < r.y + r.h && r.y < o.y + o.h)
            .fold((r.y, r.y + r.h), |(t, b), o| (t.min(o.y), b.max(o.y + o.h)))
    };
    let routes: Vec<Vec<Pt>> = edges
        .iter()
        .map(|e| e.points.iter().map(|&p| to_flow(d, p)).collect())
        .collect();
    let mut ends = Vec::new();
    for (edge, (le, pts)) in input.edges.iter().zip(&routes).enumerate() {
        if le.from == le.to || pts.len() < 2 {
            continue;
        }
        for (start, node, next) in [(true, le.from, pts[1]), (false, le.to, pts[pts.len() - 2])] {
            let (Some(n), Some(&r)) = (input.nodes.get(node), flow.get(node)) else {
                continue;
            };
            let end = if start { pts[0] } else { pts[pts.len() - 1] };
            let foreign =
                |(k, b): (usize, &Rect)| k != le.from && k != le.to && crosses(end, next, *b);
            if n.shape == Shape::Diamond && !flow.iter().enumerate().any(foreign) {
                continue;
            }
            if let Some((side, turn)) = exit(r, band(r), next, ranksep / 2) {
                ends.push(End {
                    edge,
                    start,
                    node,
                    side,
                    turn,
                    want: next.0,
                });
            }
        }
    }
    ends.sort_by_key(|e| (e.node, e.side, e.want, e.edge, !e.start));
    let mut heads: Vec<Option<Vec<Pt>>> = vec![None; routes.len()];
    let mut tails = heads.clone();
    for group in ends.chunk_by(|a, b| (a.node, a.side) == (b.node, b.side)) {
        let r = flow[group[0].node];
        let inset = if input.nodes[group[0].node].shape == Shape::Diamond {
            r.w / 4
        } else {
            MIN_INSET.min(r.w / 2)
        };
        let wants: Vec<Half> = group.iter().map(|e| e.want).collect();
        for (e, x) in group
            .iter()
            .zip(spread(r.x + inset, r.x + r.w - inset, &wants))
        {
            let mut piece = vec![(x, e.side), (x, e.turn)];
            if e.start {
                heads[e.edge] = Some(piece);
            } else {
                piece.reverse();
                tails[e.edge] = Some(piece);
            }
        }
    }
    let labels: Vec<Option<Rect>> = edges
        .iter()
        .map(|e| e.label.map(|l| rect_to_flow(d, l)))
        .collect();
    for (i, ((le, pe), pts)) in input.edges.iter().zip(edges).zip(routes).enumerate() {
        if le.from == le.to || pts.len() < 2 {
            continue;
        }
        let n = pts.len();
        let head = heads[i].take().unwrap_or_else(|| vec![pts[0]]);
        let tail = tails[i].take().unwrap_or_else(|| vec![pts[n - 1]]);
        let mut all = head.clone();
        all.extend_from_slice(&pts[1..n - 1]);
        let mid = all.len();
        all.extend_from_slice(&tail);
        // The ports, the turns and the point the edge's label sits on stay; the rest is straightened.
        let own = labels[i];
        let mut anchors = vec![0, head.len() - 1];
        anchors.extend((head.len()..mid).filter(|&k| own.is_some_and(|l| holds(l, all[k]))));
        anchors.extend([mid, all.len() - 1]);
        anchors.dedup();
        let obstacles: Vec<Rect> = boxes
            .iter()
            .copied()
            .chain(
                labels
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| *j != i)
                    .filter_map(|(_, l)| *l),
            )
            .map(|r| Rect {
                x: r.x - CLEARANCE,
                y: r.y - CLEARANCE,
                w: r.w + 2 * CLEARANCE,
                h: r.h + 2 * CLEARANCE,
            })
            .collect();
        let straight = straighten(&all, &anchors, &obstacles);
        pe.points = drop_collinear(straight)
            .into_iter()
            .map(|p| from_flow(d, p))
            .collect();
    }
}

fn holds(r: Rect, p: Pt) -> bool {
    (r.x..=r.x + r.w).contains(&p.0) && (r.y..=r.y + r.h).contains(&p.1)
}

/// `pts` straightened by line of sight: between consecutive `anchors` (ascending indices, the first
/// and last included), each kept point runs straight to the farthest later point it sees past no
/// obstacle, so the bend dagre puts half a rank out is not spent in that narrow gap alone.
fn straighten(pts: &[Pt], anchors: &[usize], obstacles: &[Rect]) -> Vec<Pt> {
    let clear = |a: Pt, b: Pt| obstacles.iter().all(|r| !crosses(a, b, *r));
    let mut out = vec![pts[0]];
    for w in anchors.windows(2) {
        let (mut i, end) = (w[0], w[1]);
        while i < end {
            let mut j = i + 1;
            while j < end && clear(pts[i], pts[j + 1]) {
                j += 1;
            }
            out.push(pts[j]);
            i = j;
        }
    }
    out
}

/// `pts` without repeats and without a point that lies straight on between its neighbours.
fn drop_collinear(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last() == Some(&p) {
            continue;
        }
        if let [.., a, b] = out[..] {
            let (u, v) = ((b.0 - a.0, b.1 - a.1), (p.0 - b.0, p.1 - b.1));
            let cross = i64::from(u.0) * i64::from(v.1) - i64::from(u.1) * i64::from(v.0);
            let dot = i64::from(u.0) * i64::from(v.0) + i64::from(u.1) * i64::from(v.1);
            if cross == 0 && dot > 0 {
                out.pop();
            }
        }
        out.push(p);
    }
    out
}

#[cfg(test)]
mod tests;
