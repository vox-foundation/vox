//! The only file that names the `dagre` crate. It builds a dagre graph from the layout input,
//! runs it and reads every coordinate back as an unjudged `Option`; judging is `lay_out`'s job.

use dagre::graph::{Graph, GraphOptions};
use dagre::layout::types::LabelPos;
use dagre::{EdgeLabel, LayoutOptions, NodeLabel, RankDir};

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

impl Engine for DagreEngine {
    fn solve(&self, input: &LayoutInput, sep: Separations) -> Result<RawLayout, LayoutError> {
        let mut g: Graph<NodeLabel, EdgeLabel> = Graph::with_options(GraphOptions {
            directed: true,
            multigraph: true,
            compound: true,
        });
        for (i, n) in input.nodes.iter().enumerate() {
            let label = NodeLabel {
                width: px(n.w),
                height: px(n.h),
                ..NodeLabel::default()
            };
            g.set_node(format!("n{i}"), Some(label));
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
        let nodes = (0..input.nodes.len())
            .map(|i| {
                g.node(&format!("n{i}"))
                    .map_or_else(RawBox::default, |l| RawBox {
                        cx: l.x,
                        cy: l.y,
                        w: Some(l.width),
                        h: Some(l.height),
                    })
            })
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
        Ok(RawLayout { nodes, edges })
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
        }
    }

    #[test]
    fn two_nodes_are_ranked_apart() {
        let input = LayoutInput {
            direction: Direction::Down,
            nodes: vec![node(), node()],
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
