use super::*;
use crate::layout::verify::crosses;
use crate::layout::{LEdge, LNode};

const DIRS: [Direction; 4] = [
    Direction::Down,
    Direction::Up,
    Direction::Right,
    Direction::Left,
];

fn rect(x: Half, y: Half, w: Half, h: Half) -> Rect {
    Rect { x, y, w, h }
}

/// `r`, given in the flow frame, on screen for direction `d`.
fn rect_from_flow(d: Direction, r: Rect) -> Rect {
    let (a, b) = (
        from_flow(d, (r.x, r.y)),
        from_flow(d, (r.x + r.w, r.y + r.h)),
    );
    rect(
        a.0.min(b.0),
        a.1.min(b.1),
        (a.0 - b.0).abs(),
        (a.1 - b.1).abs(),
    )
}

fn input(d: Direction, shape: Shape) -> LayoutInput {
    let node = |shape| LNode {
        shape,
        lines: vec!["A".into()],
        w: 0,
        h: 0,
        container: None,
    };
    LayoutInput {
        direction: d,
        nodes: vec![node(Shape::Rect), node(shape), node(Shape::Rect)],
        containers: vec![],
        edges: vec![LEdge {
            from: 1,
            to: 2,
            label: None,
        }],
    }
}

/// Flow frame: node 1 beside node 0 in one rank, feeding node 2 two ranks down through a dummy at
/// the half rank. dagre's start leaves node 1 through its left side, on through node 0's corner.
const NODES: [Rect; 3] = [
    Rect {
        x: 0,
        y: 0,
        w: 400,
        h: 100,
    },
    Rect {
        x: 480,
        y: 0,
        w: 300,
        h: 100,
    },
    Rect {
        x: 0,
        y: 400,
        w: 200,
        h: 100,
    },
];
const DAGRE: [Pt; 3] = [(480, 81), (100, 160), (100, 400)];

/// The route of [`DAGRE`] re-ported in direction `d` with `shape` for the source, read back in
/// the flow frame.
fn routed(d: Direction, shape: Shape) -> Vec<Pt> {
    let nodes: Vec<Rect> = NODES.iter().map(|r| rect_from_flow(d, *r)).collect();
    let mut edges = [PlacedEdge {
        points: DAGRE.iter().map(|&p| from_flow(d, p)).collect(),
        label: None,
    }];
    route(&input(d, shape), &nodes, &[], 60, &mut edges);
    edges[0].points.iter().map(|&p| to_flow(d, p)).collect()
}

#[test]
fn the_flow_frame_round_trips() {
    for d in DIRS {
        for p in [(3, 7), (-5, 11), (0, 0)] {
            assert_eq!(from_flow(d, to_flow(d, p)), p, "{d:?}");
        }
        assert_eq!(rect_to_flow(d, rect_from_flow(d, NODES[1])), NODES[1]);
    }
}

#[test]
fn an_edge_leaves_the_side_facing_the_next_rank_off_its_corner_and_turns_beyond_the_rank() {
    assert!(
        crosses(DAGRE[0], DAGRE[1], NODES[0]),
        "the fixture must cross"
    );
    for d in DIRS {
        let pts = routed(d, Shape::Rect);
        // Bottom side 16 px in from the corner, a 15 px stub out of the rank, then straight to
        // the stub above the target, past dagre's dummy at the half rank.
        assert_eq!(
            pts,
            vec![(512, 100), (512, 130), (100, 370), (100, 400)],
            "{d:?}"
        );
        assert!(
            pts.windows(2).all(|s| !crosses(s[0], s[1], NODES[0])),
            "{d:?}"
        );
    }
}

#[test]
fn a_diamond_end_moves_only_off_a_foreign_node_and_stays_near_its_vertex() {
    for d in DIRS {
        // dagre's end cuts node 0: re-ported within the middle half of the side.
        assert_eq!(
            routed(d, Shape::Diamond)[..2],
            [(555, 100), (555, 130)],
            "{d:?}"
        );
    }
    let mut edges = [PlacedEdge {
        points: vec![(600, 100), (300, 160), (100, 400)],
        label: None,
    }];
    route(
        &input(Direction::Down, Shape::Diamond),
        &NODES,
        &[],
        60,
        &mut edges,
    );
    assert_eq!(
        edges[0].points,
        vec![(600, 100), (300, 160), (168, 370), (168, 400)]
    );
}

#[test]
fn the_turn_stays_halfway_to_a_close_next_point_and_a_back_edge_leaves_the_top() {
    let r = NODES[1];
    let band = (0, 100);
    assert_eq!(exit(r, band, (900, 110), 30), Some((100, 105)));
    assert_eq!(exit(r, band, (900, 400), 30), Some((100, 130)));
    assert_eq!(exit(r, band, (500, -40), 30), Some((0, -20)));
    // Level with the rank, or a degenerate box: left as dagre drew it.
    assert_eq!(exit(r, band, (100, 50), 30), None);
    assert_eq!(exit(rect(0, 0, 0, 10), band, (0, 300), 30), None);
}

#[test]
fn ports_on_one_side_are_clamped_and_never_merge() {
    assert_eq!(spread(0, 100, &[50]), vec![50]);
    assert_eq!(spread(0, 100, &[-9, 300]), vec![0, 100]);
    // Both want past the right end: 12 px apart, ending at it.
    assert_eq!(spread(0, 100, &[150, 200]), vec![76, 100]);
    assert_eq!(spread(0, 100, &[10, 12, 13]), vec![10, 34, 58]);
    // No room for 12 px: evenly apart.
    assert_eq!(spread(0, 20, &[5, 5, 5]), vec![0, 10, 20]);
}

/// Two edges into node 2 from dummies right of it: both ports clamp to its right inset, so they
/// are pushed apart and each turns above the node.
#[test]
fn two_ends_on_one_side_get_their_own_ports() {
    let mut i = input(Direction::Down, Shape::Rect);
    i.edges.push(LEdge {
        from: 0,
        to: 2,
        label: None,
    });
    let mut edges = [
        PlacedEdge {
            points: vec![(600, 100), (600, 300), (190, 400)],
            label: None,
        },
        PlacedEdge {
            points: vec![(300, 100), (500, 300), (180, 400)],
            label: None,
        },
    ];
    route(&i, &NODES, &[], 60, &mut edges);
    let ends: Vec<Pt> = edges
        .iter()
        .filter_map(|e| e.points.last().copied())
        .collect();
    assert_eq!(ends, vec![(168, 400), (144, 400)]);
    assert_eq!(edges[0].points[edges[0].points.len() - 2], (168, 370));
}

#[test]
fn a_self_loop_and_a_one_point_route_are_untouched() {
    let mut i = input(Direction::Down, Shape::Rect);
    i.edges[0].to = 1;
    let pts = vec![(480, 81), (100, 160), (480, 90)];
    let mut edges = [PlacedEdge {
        points: pts.clone(),
        label: None,
    }];
    route(&i, &NODES, &[], 60, &mut edges);
    assert_eq!(edges[0].points, pts);
    let mut one = [PlacedEdge {
        points: vec![(1, 1)],
        label: None,
    }];
    route(
        &input(Direction::Down, Shape::Rect),
        &NODES,
        &[],
        60,
        &mut one,
    );
    assert_eq!(one[0].points, vec![(1, 1)]);
}

#[test]
fn a_route_runs_straight_between_anchors_unless_a_box_is_in_the_way() {
    let pts = [(0, 0), (0, 100), (100, 100), (100, 200)];
    assert_eq!(straighten(&pts, &[0, 3], &[]), vec![(0, 0), (100, 200)]);
    let block = [rect(30, 40, 40, 40)];
    assert_eq!(
        straighten(&pts, &[0, 3], &block),
        vec![(0, 0), (0, 100), (100, 200)]
    );
    // An anchor is never skipped.
    assert_eq!(
        straighten(&pts, &[0, 2, 3], &[]),
        vec![(0, 0), (100, 100), (100, 200)]
    );
}

#[test]
fn collinear_and_repeated_points_are_dropped_but_a_reversal_is_kept() {
    let pts = vec![(0, 0), (0, 0), (0, 5), (0, 10), (5, 10)];
    assert_eq!(drop_collinear(pts), vec![(0, 0), (0, 10), (5, 10)]);
    assert_eq!(
        drop_collinear(vec![(0, 0), (0, 10), (0, 5)]),
        vec![(0, 0), (0, 10), (0, 5)]
    );
}

/// The point an edge's label sits on stays on the route.
#[test]
fn a_label_point_is_kept() {
    let mut edges = [PlacedEdge {
        points: vec![(512, 100), (512, 130), (300, 250), (100, 370), (100, 400)],
        label: Some(rect(280, 240, 40, 20)),
    }];
    route(
        &input(Direction::Down, Shape::Rect),
        &NODES,
        &[],
        60,
        &mut edges,
    );
    assert!(
        edges[0].points.contains(&(300, 250)),
        "{:?}",
        edges[0].points
    );
}
