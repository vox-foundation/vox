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
        title_w: 40,
        title_h: 36,
    }
}

/// A frame whose title box is its top 52 half-pixels.
fn framed(rect: Rect) -> PlacedContainer {
    PlacedContainer {
        rect,
        title: Rect { h: 52, ..rect },
        title_lines: vec!["T".to_owned()],
    }
}

/// Outer container with one nested container holding one node; each title box is the top
/// 52 half-pixels of its frame.
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
    // The child container starts exactly where the outer title box ends.
    p.containers[1] = framed(rect(40, 72, 200, 168));
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
    p.containers[1] = framed(rect(220, 20, 160, 200));
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

fn labelled(label: Rect) -> PlacedEdge {
    PlacedEdge {
        points: vec![(40, 40), (50, 50)],
        label: Some(label),
    }
}

#[test]
fn an_edge_label_in_a_title_band_is_rejected() {
    let (input, mut p) = nested();
    // The outer title box spans y 20..72; the nested container's spans y 80..132.
    p.edges = vec![labelled(rect(250, 30, 40, 20))];
    assert_eq!(verify(&input, &p), Err(Violation::LabelInTitleBand(0, 0)));
    p.edges = vec![labelled(rect(100, 90, 40, 20))];
    assert_eq!(verify(&input, &p), Err(Violation::LabelInTitleBand(0, 1)));
    // Just below the outer band, beside the nested container: clear.
    p.edges = vec![labelled(rect(250, 72, 40, 20))];
    assert_eq!(verify(&input, &p), Ok(()));
}

#[test]
fn an_edge_label_over_a_node_or_another_label_is_rejected() {
    let (input, mut p) = nested();
    p.edges = vec![labelled(rect(100, 150, 40, 20))];
    assert_eq!(verify(&input, &p), Err(Violation::LabelOnNode(0, 0)));
    p.edges = vec![
        labelled(rect(250, 100, 40, 20)),
        labelled(rect(270, 110, 40, 20)),
    ];
    assert_eq!(verify(&input, &p), Err(Violation::LabelOnLabel(0, 1)));
    p.edges[1].label = Some(rect(290, 100, 40, 20));
    assert_eq!(verify(&input, &p), Ok(()));
}

#[test]
fn a_segment_crosses_a_box_only_through_its_interior() {
    let r = rect(100, 100, 50, 20);
    assert!(crosses((90, 110), (160, 110), r), "straight through");
    assert!(crosses((90, 90), (160, 130), r), "diagonal through");
    assert!(crosses((120, 110), (120, 300), r), "starts inside");
    assert!(!crosses((90, 100), (160, 100), r), "along the top side");
    assert!(!crosses((90, 90), (100, 100), r), "ends on a corner");
    assert!(!crosses((80, 90), (110, 99), r), "passes above");
    assert!(!crosses((90, 140), (160, 125), r), "passes below");
    assert!(!crosses((60, 60), (99, 160), r), "passes left");
}

#[test]
fn an_edge_through_a_title_box_is_reported_apart_from_the_hard_rules() {
    let (input, mut p) = nested();
    p.edges = vec![PlacedEdge {
        points: vec![(250, 10), (250, 100)],
        label: None,
    }];
    assert_eq!(verify(&input, &p), Ok(()));
    assert_eq!(title_crossing(&p), Err(Violation::EdgeThroughTitle(0, 0)));
    p.edges[0].points = vec![(10, 10), (10, 100)];
    assert_eq!(title_crossing(&p), Ok(()));
}
