//! The only file that names the `dagre` crate. It builds a dagre graph from the layout input,
//! runs it and reads every coordinate back as an unjudged `Option`; judging is `lay_out`'s job.

use dagre::graph::{Graph, GraphOptions};
use dagre::layout::types::LabelPos;
use dagre::{EdgeLabel, LayoutOptions, NodeLabel, RankDir};

use super::verify::encloses;
use super::{Engine, LayoutError, LayoutInput, RawBox, RawEdge, RawLayout, Separations};
use crate::ir::Direction;

/// dagre 0.1.1 behind the [`Engine`] seam.
pub(crate) struct DagreEngine;

fn rank_dir(d: Direction) -> RankDir {
    match d {
        Direction::Down => RankDir::TB,
        Direction::Right => RankDir::LR,
        Direction::Up => RankDir::BT,
        Direction::Left => RankDir::RL,
    }
}

/// Half-pixels to px.
fn px(h: i32) -> f64 {
    f64::from(h) / 2.0
}

/// A dagre node: a diagram node or a container's title box.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Item {
    Node(usize),
    Title(usize),
}

impl Item {
    fn name(self) -> String {
        match self {
            Self::Node(i) => format!("n{i}"),
            Self::Title(i) => format!("t{i}"),
        }
    }
}

/// The invisible edges that rank each container's title box before its content, so dagre keeps the
/// title clear of nodes and routes edges beside it: from the title to every child container's
/// title and to each own node that no edge reaches from inside the container (every own node when
/// a cycle leaves none). Up and left layouts point them the other way, so a title still lands on
/// the top (down, up) or the left (right, left) of its content.
fn title_links(input: &LayoutInput) -> Vec<(Item, Item)> {
    let reverse = matches!(input.direction, Direction::Up | Direction::Left);
    let mut out = Vec::new();
    for k in 0..input.containers.len() {
        let inside = |n: usize| encloses(input, k, input.nodes.get(n).and_then(|n| n.container));
        let own: Vec<usize> = (0..input.nodes.len())
            .filter(|&n| input.nodes[n].container == Some(k))
            .collect();
        let first = |n: &usize| {
            !input.edges.iter().any(|e| {
                let (near, far) = if reverse {
                    (e.from, e.to)
                } else {
                    (e.to, e.from)
                };
                near == *n && far != *n && inside(far)
            })
        };
        let mut firsts: Vec<usize> = own.iter().copied().filter(first).collect();
        if firsts.is_empty() {
            firsts = own;
        }
        let subs = (0..input.containers.len()).filter(|&c| input.containers[c].parent == Some(k));
        for to in subs
            .map(Item::Title)
            .chain(firsts.into_iter().map(Item::Node))
        {
            out.push(if reverse {
                (to, Item::Title(k))
            } else {
                (Item::Title(k), to)
            });
        }
    }
    out
}

impl Engine for DagreEngine {
    fn solve(&self, input: &LayoutInput, sep: Separations) -> Result<RawLayout, LayoutError> {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::with_options(GraphOptions {
            directed: true,
            multigraph: true,
            compound: true,
        });
        for i in 0..input.containers.len() {
            g.set_node(format!("c{i}"), Some(NodeLabel::default()));
        }
        for (i, c) in input.containers.iter().enumerate() {
            let title = NodeLabel {
                width: px(c.title_w),
                height: px(c.title_h),
                ..NodeLabel::default()
            };
            g.set_node(format!("t{i}"), Some(title));
            g.set_parent(&format!("t{i}"), Some(&format!("c{i}")));
            if let Some(p) = c.parent {
                g.set_parent(&format!("c{i}"), Some(&format!("c{p}")));
            }
        }
        for (i, n) in input.nodes.iter().enumerate() {
            let label = NodeLabel {
                width: px(n.w),
                height: px(n.h),
                ..NodeLabel::default()
            };
            g.set_node(format!("n{i}"), Some(label));
            if let Some(c) = n.container {
                g.set_parent(&format!("n{i}"), Some(&format!("c{c}")));
            }
        }
        for (i, e) in input.edges.iter().enumerate() {
            let name = format!("e{i}");
            let label = e
                .label
                .as_ref()
                .map_or_else(EdgeLabel::default, |l| EdgeLabel {
                    width: px(l.w),
                    height: px(l.h),
                    labelpos: LabelPos::Center,
                    ..EdgeLabel::default()
                });
            g.set_edge(
                format!("n{}", e.from),
                format!("n{}", e.to),
                Some(label),
                Some(&name),
            );
        }
        for (j, (a, b)) in title_links(input).into_iter().enumerate() {
            g.set_edge(
                a.name(),
                b.name(),
                Some(EdgeLabel::default()),
                Some(&format!("l{j}")),
            );
        }
        let opts = LayoutOptions {
            rankdir: rank_dir(input.direction),
            nodesep: f64::from(sep.nodesep),
            ranksep: f64::from(sep.ranksep),
            edgesep: f64::from(sep.edgesep),
            marginx: 16.0,
            marginy: 16.0,
            ..LayoutOptions::default()
        };
        dagre::layout(&mut g, Some(opts));
        let read = |name: String| {
            g.node(&name).map_or_else(RawBox::default, |l| RawBox {
                cx: l.x,
                cy: l.y,
                w: Some(l.width),
                h: Some(l.height),
            })
        };
        let nodes = (0..input.nodes.len())
            .map(|i| read(format!("n{i}")))
            .collect();
        let titles = (0..input.containers.len())
            .map(|i| read(format!("t{i}")))
            .collect();
        let edges = input
            .edges
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let found = g.edge(
                    &format!("n{}", e.from),
                    &format!("n{}", e.to),
                    Some(&format!("e{i}")),
                );
                RawEdge {
                    points: found
                        .map_or_else(Vec::new, |l| l.points.iter().map(|p| (p.x, p.y)).collect()),
                    label: e.label.as_ref().and(found).map(|l| (l.x, l.y)),
                }
            })
            .collect();
        Ok(RawLayout {
            nodes,
            titles,
            edges,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Shape;
    use crate::layout::{LEdge, LLabel, LNode, RUNGS};

    fn node() -> LNode {
        LNode {
            shape: Shape::Rect,
            lines: vec!["A".into()],
            w: 128,
            h: 76,
            container: None,
        }
    }

    #[test]
    fn two_nodes_are_ranked_apart() {
        let input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![node(), node()],
            containers: vec![],
            edges: vec![LEdge {
                from: 0,
                to: 1,
                label: None,
            }],
        };
        let Ok(raw) = DagreEngine.solve(&input, RUNGS[0]) else {
            panic!("solve failed");
        };
        let (Some(a), Some(b)) = (raw.nodes[0].cy, raw.nodes[1].cy) else {
            panic!("missing y");
        };
        assert!(b > a, "{a} {b}");
        assert!(!raw.edges[0].points.is_empty());
    }

    #[test]
    fn titles_link_to_child_titles_and_to_nodes_entered_only_from_outside() {
        use crate::layout::LContainer;
        let boxed = |container| LNode {
            container: Some(container),
            ..node()
        };
        let edge = |from, to| LEdge {
            from,
            to,
            label: None,
        };
        let title = |parent| LContainer {
            parent,
            depth: 1,
            title_lines: vec!["T".into()],
            title_w: 40,
            title_h: 36,
        };
        // n0 -> n1 inside container 0; n2 in nested container 1; n3 outside feeds n1 and n2.
        let mut input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![boxed(0), boxed(0), boxed(1), node()],
            containers: vec![title(None), title(Some(0))],
            edges: vec![edge(0, 1), edge(3, 1), edge(3, 2)],
        };
        let (t, n) = (Item::Title, Item::Node);
        assert_eq!(
            title_links(&input),
            vec![(t(0), t(1)), (t(0), n(0)), (t(1), n(2))]
        );
        input.direction = Direction::Up;
        assert_eq!(
            title_links(&input),
            vec![(t(1), t(0)), (n(1), t(0)), (n(2), t(1))]
        );
        // A cycle leaves no entry, so the title links to every own node.
        input.direction = Direction::Down;
        input.edges = vec![edge(0, 1), edge(1, 0)];
        assert_eq!(
            title_links(&input),
            vec![(t(0), t(1)), (t(0), n(0)), (t(0), n(1)), (t(1), n(2))]
        );
    }

    #[test]
    fn a_labelled_edge_reports_a_label_centre() {
        let label = LLabel {
            lines: vec!["go".into()],
            w: 60,
            h: 52,
        };
        let edge = |label| LEdge {
            from: 0,
            to: 1,
            label,
        };
        let input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![node(), node()],
            containers: vec![],
            edges: vec![edge(Some(label)), edge(None)],
        };
        let Ok(raw) = DagreEngine.solve(&input, RUNGS[0]) else {
            panic!("solve failed");
        };
        let Some((Some(x), Some(y))) = raw.edges[0].label else {
            panic!("no label centre: {:?}", raw.edges[0]);
        };
        assert!(x.is_finite() && y.is_finite());
        assert_eq!(raw.edges[1].label, None);
    }
}
