//! Container finishing. The engine lays each title out as a box on top of the container's content
//! (ranked before it top-down, first in the content's first column sideways), which leaves a rank
//! or a node gap between the two; the title is then snapped down to its content. A frame is only
//! the box around what the container holds (its title box, its own nodes, its child frames and the
//! labels of edges inside it) padded 8 px on every side. Nothing here reads the engine's own
//! container rect, so engine slack never reaches a frame.

use std::cmp::Reverse;

use super::input::TITLE_PAD;
use super::text::{WRAP_CELLS, cells, measure, pad_x, wrap_at};
use super::verify::{crosses, encloses, overlap};
use super::{Half, LayoutInput, PlacedContainer, PlacedEdge, Rect};

/// Padding between a frame and the box around its children, every side: 8 px.
pub(crate) const CONTAINER_PAD: Half = 16;

/// Gap between a title box and the content under it: 8 px.
pub(crate) const TITLE_GAP: Half = 16;

/// Step by which a title that cannot hug its content backs off towards the engine's spot: 2 px.
pub(crate) const SNAP_STEP: Half = 4;

/// Line pitch of a container title, px.
pub(crate) const TITLE_STEP: Half = 16;

/// Height of a title box for `lines` title lines: 16 px a line plus 1 px above and below.
pub(crate) fn title_h(lines: usize) -> Half {
    4 + 32 * Half::try_from(lines).unwrap_or(0)
}

fn union(a: Rect, b: Rect) -> Rect {
    let (x, y) = (a.x.min(b.x), a.y.min(b.y));
    Rect {
        x,
        y,
        w: (a.x + a.w).max(b.x + b.w) - x,
        h: (a.y + a.h).max(b.y + b.h) - y,
    }
}

fn pad(r: Rect) -> Rect {
    Rect {
        x: r.x - CONTAINER_PAD,
        y: r.y - CONTAINER_PAD,
        w: r.w + 2 * CONTAINER_PAD,
        h: r.h + 2 * CONTAINER_PAD,
    }
}

/// Whether `r` is clear of every edge segment and edge label.
fn clear_of_edges(r: Rect, edges: &[PlacedEdge]) -> bool {
    edges.iter().all(|e| {
        e.label.is_none_or(|l| !overlap(l, r))
            && e.points.windows(2).all(|s| !crosses(s[0], s[1], r))
    })
}

/// The frame of every container and the title box it ends up with: framed once with the engine's
/// titles, then again with each title snapped to its content ([`snap`]). The first frames stand in
/// for frames not yet final: a snap only ever shrinks a frame, so they are a safe superset.
pub(crate) fn frames(
    input: &LayoutInput,
    nodes: &[Rect],
    titles: &[Rect],
    edges: &[PlacedEdge],
) -> Vec<PlacedContainer> {
    let first: Vec<Rect> = pass(input, nodes, titles, edges, None)
        .iter()
        .map(|c| c.rect)
        .collect();
    pass(input, nodes, titles, edges, Some(&first))
}

/// Title `t` moved as close above its content `c` (the boxes `items`) as `free` allows:
/// [`TITLE_GAP`] above the content under it, or level with the content's top when that is higher
/// still (sideways, where the title tops one column), never above both the engine's spot and that
/// top. A blocked spot backs off towards the engine's spot by [`SNAP_STEP`]; the engine's spot is
/// the last resort.
fn snap(t: Rect, c: Rect, items: &[Rect], free: impl Fn(Rect) -> bool) -> Rect {
    let below = items
        .iter()
        .filter(|r| r.x < t.x + t.w && t.x < r.x + r.w)
        .map(|r| r.y)
        .min()
        .unwrap_or(c.y);
    let target = (below - TITLE_GAP - t.h).min(c.y).max(t.y.min(c.y));
    let step = if target < t.y { SNAP_STEP } else { -SNAP_STEP };
    let mut y = target;
    while (t.y - y) * step > 0 {
        let r = Rect { y, ..t };
        if free(r) {
            return r;
        }
        y += step;
    }
    t
}

/// Title `t` (wrapped as `lines`) re-wrapped at the widest line limit whose box still fits the
/// content `c`'s width, when that takes fewer lines: the box keeps its bottom edge and its centre,
/// clamped so it stays over the content. A title wraps at the node width before layout, so without
/// this a wide frame carries a narrow, stacked title.
fn widen(label: &str, t: Rect, c: Rect, lines: &[String]) -> Option<(Rect, Vec<String>)> {
    let mut best = None;
    for limit in WRAP_CELLS + 1..=cells(label) {
        let wrapped = wrap_at(label, limit);
        let widest = wrapped.iter().map(|l| measure(l)).max().unwrap_or(0);
        let w = widest + 2 * pad_x(widest, TITLE_PAD);
        if w > c.w {
            break;
        }
        if wrapped.len() < lines.len() {
            best = Some((w, wrapped));
        }
    }
    let (w, wrapped) = best?;
    let h = title_h(wrapped.len());
    let x = (t.x + t.w / 2 - w / 2).clamp(c.x, c.x + c.w - w);
    Some((
        Rect {
            x,
            y: t.y + t.h - h,
            w,
            h,
        },
        wrapped,
    ))
}

/// One framing pass, deepest container first so a child frame exists before its parent's. With
/// `first` (the frames of an earlier pass), each title is snapped to its content where it is clear
/// of every edge, label, node and frame not holding it. An edge label counts when both endpoints of
/// its edge sit inside the container, or when one does and the label would otherwise straddle the
/// frame.
fn pass(
    input: &LayoutInput,
    nodes: &[Rect],
    titles: &[Rect],
    edges: &[PlacedEdge],
    first: Option<&[Rect]>,
) -> Vec<PlacedContainer> {
    let n = input.containers.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&k| (Reverse(input.containers[k].depth), k));
    let mut rects: Vec<Option<Rect>> = vec![None; n];
    let mut placed_titles: Vec<Rect> = titles.to_vec();
    let mut placed_lines: Vec<Vec<String>> = input
        .containers
        .iter()
        .map(|c| c.title_lines.clone())
        .collect();
    for k in order {
        let Some(&dagre_title) = titles.get(k) else {
            continue;
        };
        let held = |n: usize| encloses(input, k, input.nodes.get(n).and_then(|n| n.container));
        let own = input
            .nodes
            .iter()
            .zip(nodes)
            .filter(|(n, _)| n.container == Some(k))
            .map(|(_, r)| *r);
        let subs = input
            .containers
            .iter()
            .zip(&rects)
            .filter_map(|(c, r)| r.filter(|_| c.parent == Some(k)));
        let inner = input
            .edges
            .iter()
            .zip(edges)
            .filter(|(e, _)| held(e.from) && held(e.to))
            .filter_map(|(_, p)| p.label);
        let items: Vec<Rect> = own.chain(subs).chain(inner).collect();
        let content = items.iter().copied().reduce(union);
        let mut lines = input.containers[k].title_lines.clone();
        let title = match (first, content) {
            (Some(first), Some(c)) => {
                let foreign = |r: Rect| {
                    (0..n).any(|j| {
                        !encloses(input, j, Some(k))
                            && rects[j]
                                .or(first.get(j).copied())
                                .is_some_and(|f| overlap(f, r))
                    })
                };
                let free = |r: Rect| {
                    clear_of_edges(r, edges) && !nodes.iter().any(|x| overlap(*x, r)) && !foreign(r)
                };
                let wide = widen(&input.containers[k].label, dagre_title, c, &lines)
                    .map(|(t, l)| (snap(t, c, &items, free), l))
                    .filter(|(t, _)| free(*t));
                match wide {
                    Some((t, l)) => {
                        lines = l;
                        t
                    }
                    None => snap(dagre_title, c, &items, free),
                }
            }
            _ => dagre_title,
        };
        let mut b = content.map_or(title, |c| union(c, title));
        let touching: Vec<Rect> = input
            .edges
            .iter()
            .zip(edges)
            .filter(|(e, _)| held(e.from) || held(e.to))
            .filter_map(|(_, p)| p.label)
            .collect();
        // Each pass adds at least one label or stops, so the loop is bounded by the label count.
        while let Some(l) = touching
            .iter()
            .find(|l| overlap(**l, pad(b)) && union(b, **l) != b)
        {
            b = union(b, *l);
        }
        rects[k] = Some(pad(b));
        placed_titles[k] = title;
        placed_lines[k] = lines;
    }
    rects
        .into_iter()
        .zip(placed_titles)
        .zip(placed_lines)
        .map(|((r, title), title_lines)| PlacedContainer {
            rect: r.unwrap_or(title),
            title,
            title_lines,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, Shape};
    use crate::layout::{LContainer, LEdge, LLabel, LNode};

    fn rect(x: Half, y: Half, w: Half, h: Half) -> Rect {
        Rect { x, y, w, h }
    }

    fn container(parent: Option<usize>, depth: u32) -> LContainer {
        LContainer {
            parent,
            depth,
            label: "T".into(),
            title_lines: vec!["T".into()],
            title_w: 40,
            title_h: title_h(1),
        }
    }

    fn node(container: Option<usize>) -> LNode {
        LNode {
            shape: Shape::Rect,
            lines: vec!["A".into()],
            w: 128,
            h: 76,
            container,
        }
    }

    #[test]
    fn a_title_box_is_16px_a_line_plus_2px() {
        assert_eq!(title_h(1), 36);
        assert_eq!(title_h(3), 100);
    }

    #[test]
    fn a_frame_is_its_children_padded_8px_and_a_parent_holds_its_child_frame() {
        let label = LLabel {
            lines: vec!["go".into()],
            w: 40,
            h: 20,
        };
        let input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![node(Some(1)), node(Some(1)), node(None)],
            containers: vec![container(None, 1), container(Some(0), 2)],
            edges: vec![
                LEdge {
                    from: 0,
                    to: 1,
                    label: Some(label.clone()),
                },
                LEdge {
                    from: 2,
                    to: 0,
                    label: Some(label),
                },
            ],
        };
        let nodes = [
            rect(100, 200, 128, 76),
            rect(300, 200, 128, 76),
            rect(900, 900, 128, 76),
        ];
        let titles = [rect(150, 40, 40, 36), rect(200, 120, 40, 36)];
        let edge = |l| PlacedEdge {
            points: vec![(0, 0)],
            label: Some(l),
        };
        // The inner edge's label widens the inner frame; the label of the edge from outside does not.
        let edges = [edge(rect(240, 300, 40, 20)), edge(rect(600, 600, 40, 20))];
        let f = frames(&input, &nodes, &titles, &edges);
        // Each title snaps to 8 px above its content: the inner one above its nodes (y 148), the
        // outer one above the inner frame (y 80).
        assert_eq!(f[1].title, rect(200, 148, 40, 36));
        assert_eq!(f[1].rect, rect(84, 132, 360, 204));
        assert_eq!(f[0].title, rect(150, 80, 40, 36));
        assert_eq!(f[0].rect, rect(68, 64, 392, 288));
        assert_eq!(f[1].title_lines, vec!["T".to_owned()]);
    }

    /// Phase 90 review WR-01: the title leaves the engine's rank gap and sits 8 px above its
    /// content, unless something is in the way; then it stops at the lowest clear spot.
    #[test]
    fn a_title_snaps_down_to_its_content_and_stops_short_of_an_edge() {
        let t = rect(100, 0, 40, 36);
        let items = [rect(80, 200, 128, 76)];
        let snapped = snap(t, items[0], &items, |_| true);
        assert_eq!(snapped, rect(100, 148, 40, 36));
        // A horizontal edge at y 150 blocks every spot from 115 (its box reaches 151) down.
        let edges = [PlacedEdge {
            points: vec![(0, 150), (400, 150)],
            label: None,
        }];
        let stopped = snap(t, items[0], &items, |r| clear_of_edges(r, &edges));
        assert_eq!(stopped, rect(100, 112, 40, 36));
        // Fully blocked: the engine's spot.
        assert_eq!(snap(t, items[0], &items, |_| false), t);
        // Sideways, content higher beside the title: the title tops the content, no higher.
        let beside = [rect(80, 200, 128, 76), rect(400, 10, 128, 76)];
        let c = union(beside[0], beside[1]);
        assert_eq!(snap(rect(100, 120, 40, 36), c, &beside, |_| true).y, 10);
    }

    #[test]
    fn an_empty_container_is_its_title_box_padded() {
        let input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![],
            containers: vec![container(None, 1)],
            edges: vec![],
        };
        let f = frames(&input, &[], &[rect(100, 100, 40, 36)], &[]);
        assert_eq!(f[0].rect, rect(84, 84, 72, 68));
    }
}
