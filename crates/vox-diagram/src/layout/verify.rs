//! Hard-invariant geometry check on quantized integer geometry: exact comparisons, no epsilon.
//! A layout that fails is never drawn. Soft defects (an edge crossing a non-endpoint node, a
//! label touching a node) are deliberately not checked; dagre does not route around nodes.

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
    /// A node, an edge point or an edge label lies outside the view box.
    OutsideView,
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
    let boxes_ok = placed.nodes.iter().all(|r| inside(*r, width, height));
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
    use crate::layout::{LNode, PlacedEdge};

    fn node(shape: Shape, text: &str, w: Half, h: Half) -> LNode {
        LNode {
            shape,
            lines: vec![text.to_owned()],
            w,
            h,
        }
    }

    fn input(nodes: Vec<LNode>) -> LayoutInput {
        LayoutInput {
            direction: crate::ir::Direction::Down,
            nodes,
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
}
