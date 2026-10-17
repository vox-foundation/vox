//! Hard-invariant geometry check on quantized integer geometry: exact comparisons, no epsilon.
//! A layout that fails is never drawn. Soft defects (an edge crossing a non-endpoint node, a
//! label touching a node) are deliberately not checked; dagre does not route around nodes.

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
mod tests {
    use super::*;
    use crate::ir::Shape;
    use crate::layout::text::{LINE_H, PAD_X, PAD_Y, measure};
    use crate::layout::{LContainer, LNode, PlacedContainer, PlacedEdge};

    fn node(shape: Shape, text: &str, w: Half, h: Half) -> LNode {
        LNode {
            shape,
            lines: vec![text.to_owned()],
            w,
            h,
            container: None,
        }
    }

    fn input(nodes: Vec<LNode>) -> LayoutInput {
        LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes,
            containers: vec![],
            edges: vec![],
        }
    }

    fn rect(x: Half, y: Half, w: Half, h: Half) -> Rect {
        Rect { x, y, w, h }
    }

    fn placed(nodes: Vec<Rect>, edges: Vec<PlacedEdge>) -> Placed {
        Placed {
            width: 400,
            height: 300,
            nodes,
            containers: vec![],
            edges,
        }
    }

    fn two() -> LayoutInput {
        input(vec![
            node(Shape::Rect, "A", 128, 76),
            node(Shape::Rect, "B", 128, 76),
        ])
    }

    #[test]
    fn a_clean_layout_is_ok() {
        let p = placed(vec![rect(32, 32, 128, 76), rect(200, 32, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &p), Ok(()));
    }

    #[test]
    fn overlapping_nodes_are_rejected_but_touching_ones_are_not() {
        let overlap = placed(vec![rect(32, 32, 128, 76), rect(100, 60, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &overlap), Err(Violation::NodeOverlap(0, 1)));
        let touch = placed(vec![rect(32, 32, 128, 76), rect(160, 32, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &touch), Ok(()));
        let stacked = placed(vec![rect(32, 32, 128, 76), rect(32, 108, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &stacked), Ok(()));
    }

    #[test]
    fn a_label_wider_than_its_node_overflows() {
        let tw = measure("Hello");
        let fit_w = tw + 4 * PAD_X;
        let fit_h = LINE_H * 2 + 4 * PAD_Y;
        let i = input(vec![node(Shape::Rect, "Hello", fit_w, fit_h)]);
        assert_eq!(
            verify(&i, &placed(vec![rect(32, 32, fit_w, fit_h)], vec![])),
            Ok(())
        );
        let narrow = placed(vec![rect(32, 32, fit_w - 1, fit_h)], vec![]);
        assert_eq!(verify(&i, &narrow), Err(Violation::LabelOverflow(0)));
        let short = placed(vec![rect(32, 32, fit_w, fit_h - 1)], vec![]);
        assert_eq!(verify(&i, &short), Err(Violation::LabelOverflow(0)));
    }

    #[test]
    fn a_diamond_uses_the_half_extent_rule() {
        let tw = measure("Hello");
        let th = LINE_H * 2;
        // tw/w + th/h <= 1 with w = 2*tw and h = 2*th is exactly satisfied.
        let (w, h) = (2 * tw, 2 * th);
        let i = input(vec![node(Shape::Diamond, "Hello", w, h)]);
        assert_eq!(
            verify(&i, &placed(vec![rect(32, 32, w, h)], vec![])),
            Ok(())
        );
        let thin = placed(vec![rect(32, 32, w - 2, h)], vec![]);
        assert_eq!(verify(&i, &thin), Err(Violation::LabelOverflow(0)));
    }

    #[test]
    fn anything_outside_the_view_box_is_rejected() {
        let ok = vec![rect(32, 32, 128, 76), rect(200, 32, 128, 76)];
        let right = placed(vec![rect(32, 32, 128, 76), rect(300, 32, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &right), Err(Violation::OutsideView));
        let above = placed(vec![rect(32, -2, 128, 76), rect(200, 32, 128, 76)], vec![]);
        assert_eq!(verify(&two(), &above), Err(Violation::OutsideView));
        let edge = |points| PlacedEdge {
            points,
            label: None,
        };
        let far = placed(ok.clone(), vec![edge(vec![(40, 40), (401, 40)])]);
        assert_eq!(verify(&two(), &far), Err(Violation::OutsideView));
        let neg = placed(ok.clone(), vec![edge(vec![(-1, 40), (40, 40)])]);
        assert_eq!(verify(&two(), &neg), Err(Violation::OutsideView));
        let label = PlacedEdge {
            points: vec![(40, 40), (50, 50)],
            label: Some(rect(380, 100, 40, 20)),
        };
        assert_eq!(
            verify(&two(), &placed(ok.clone(), vec![label])),
            Err(Violation::OutsideView)
        );
        let inside = PlacedEdge {
            points: vec![(0, 0), (400, 300)],
            label: Some(rect(360, 280, 40, 20)),
        };
        assert_eq!(verify(&two(), &placed(ok, vec![inside])), Ok(()));
    }

    #[test]
    fn a_violation_names_kind_and_indices_only() {
        assert_eq!(
            Violation::NodeOverlap(0, 1).to_string(),
            "NodeOverlap(0, 1)"
        );
        assert_eq!(Violation::LabelOverflow(3).to_string(), "LabelOverflow(3)");
        assert_eq!(Violation::OutsideView.to_string(), "OutsideView");
    }

    fn cnode(container: usize) -> LNode {
        LNode {
            container: Some(container),
            ..node(Shape::Rect, "A", 128, 76)
        }
    }

    fn cont(parent: Option<usize>) -> LContainer {
        LContainer {
            parent,
            depth: 1,
            title_lines: vec!["T".to_owned()],
            strut_w: 40,
        }
    }

    fn framed(rect: Rect) -> PlacedContainer {
        PlacedContainer {
            rect,
            title_lines: vec!["T".to_owned()],
        }
    }

    /// Outer container with one nested container holding one node. A one-line title band is
    /// 52 half-pixels tall.
    fn nested() -> (LayoutInput, Placed) {
        let input = LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes: vec![cnode(1)],
            containers: vec![cont(None), cont(Some(0))],
            edges: vec![],
        };
        let p = Placed {
            width: 400,
            height: 300,
            nodes: vec![rect(60, 140, 128, 76)],
            containers: vec![
                framed(rect(20, 20, 360, 260)),
                framed(rect(40, 80, 200, 160)),
            ],
            edges: vec![],
        };
        (input, p)
    }

    #[test]
    fn a_clean_nested_layout_is_ok_and_touching_is_allowed() {
        let (input, mut p) = nested();
        assert_eq!(verify(&input, &p), Ok(()));
        // The child container starts exactly where the outer title band ends.
        p.containers[1].rect = rect(40, 72, 200, 168);
        p.nodes[0] = rect(60, 124, 128, 76);
        assert_eq!(verify(&input, &p), Ok(()));
    }

    #[test]
    fn a_leaf_outside_its_container_is_rejected() {
        let (input, mut p) = nested();
        p.nodes[0] = rect(250, 140, 128, 76);
        assert_eq!(verify(&input, &p), Err(Violation::OutsideContainer(0)));
    }

    #[test]
    fn a_container_outside_its_parent_is_rejected() {
        let (input, mut p) = nested();
        p.containers[0].rect = rect(20, 20, 300, 260);
        p.containers[1].rect = rect(40, 80, 300, 160);
        assert_eq!(verify(&input, &p), Err(Violation::OutsideParent(1)));
    }

    #[test]
    fn sibling_containers_sharing_area_are_rejected() {
        let input = LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes: vec![],
            containers: vec![cont(None), cont(None)],
            edges: vec![],
        };
        let mut p = placed(vec![], vec![]);
        p.containers = vec![
            framed(rect(20, 20, 200, 200)),
            framed(rect(100, 100, 200, 200)),
        ];
        assert_eq!(verify(&input, &p), Err(Violation::SiblingOverlap(0, 1)));
        p.containers[1].rect = rect(220, 20, 160, 200);
        assert_eq!(verify(&input, &p), Ok(()));
    }

    #[test]
    fn a_leaf_overlapping_a_container_it_does_not_belong_to_is_rejected() {
        let input = LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes: vec![node(Shape::Rect, "A", 128, 76)],
            containers: vec![cont(None)],
            edges: vec![],
        };
        let mut p = placed(vec![rect(60, 100, 128, 76)], vec![]);
        p.containers = vec![framed(rect(20, 20, 200, 250))];
        assert_eq!(verify(&input, &p), Err(Violation::ForeignOverlap(0, 0)));
        p.nodes[0] = rect(220, 100, 128, 76);
        assert_eq!(verify(&input, &p), Ok(()));
    }

    #[test]
    fn a_child_inside_the_title_band_is_rejected() {
        let input = LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes: vec![cnode(0)],
            containers: vec![cont(None)],
            edges: vec![],
        };
        let mut p = placed(vec![rect(60, 40, 128, 76)], vec![]);
        p.containers = vec![framed(rect(20, 20, 360, 260))];
        assert_eq!(verify(&input, &p), Err(Violation::TitleBand(0)));
        p.nodes[0] = rect(60, 72, 128, 76);
        assert_eq!(verify(&input, &p), Ok(()));
    }

    #[test]
    fn a_container_outside_the_view_box_is_rejected() {
        let (input, mut p) = nested();
        p.containers[0].rect = rect(20, 20, 381, 260);
        assert_eq!(verify(&input, &p), Err(Violation::OutsideView));
    }
}
