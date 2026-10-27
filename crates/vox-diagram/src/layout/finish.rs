//! Container finishing. The engine lays each title out as a box on top of the container's content
//! (ranked before it top-down, first in the content's first column sideways), so a frame is only
//! the box around what the container holds (its title box, its own nodes, its child frames and the
//! labels of edges inside it) padded 8 px on every side. Nothing here reads the engine's own
//! container rect, so engine slack never reaches a frame.

use std::cmp::Reverse;

use super::verify::{crosses, encloses, overlap};
use super::{Half, LayoutInput, PlacedContainer, PlacedEdge, Rect};
use crate::ir::Direction;

/// Padding between a frame and the box around its children, every side: 8 px.
pub(crate) const CONTAINER_PAD: Half = 16;

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

/// The frame of every container, deepest first so a child frame exists before its parent's,
/// and the title box it ends up with. Sideways, the title tops the content's first column; when
/// content in another column sits higher, the title moves up to the content's top if nothing
/// crosses it there. An edge label counts when both endpoints of its edge sit inside the
/// container, or when one does and the label would otherwise straddle the frame.
pub(crate) fn frames(
    input: &LayoutInput,
    nodes: &[Rect],
    titles: &[Rect],
    edges: &[PlacedEdge],
) -> Vec<PlacedContainer> {
    let n = input.containers.len();
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&k| (Reverse(input.containers[k].depth), k));
    let mut rects: Vec<Option<Rect>> = vec![None; n];
    let mut placed_titles: Vec<Rect> = titles.to_vec();
    let sideways = matches!(input.direction, Direction::Right | Direction::Left);
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
        let content = own.chain(subs).chain(inner).reduce(union);
        let mut title = dagre_title;
        if let Some(c) = content.filter(|c| sideways && c.y < title.y) {
            let top = Rect { y: c.y, ..title };
            if clear_of_edges(top, edges) {
                title = top;
            }
        }
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
    }
    input
        .containers
        .iter()
        .zip(rects)
        .zip(placed_titles)
        .map(|((c, r), title)| PlacedContainer {
            rect: r.unwrap_or(title),
            title,
            title_lines: c.title_lines.clone(),
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
        assert_eq!(f[1].rect, rect(84, 104, 360, 232));
        assert_eq!(f[0].rect, rect(68, 24, 392, 328));
        assert_eq!(f[0].title, titles[0]);
        assert_eq!(f[1].title_lines, vec!["T".to_owned()]);
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
