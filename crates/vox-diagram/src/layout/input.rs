//! IR to [`LayoutInput`]: render caps, reference checks and node sizing. The IR is public and
//! hand-buildable, so nothing the DSL guarantees is trusted here.

use std::collections::BTreeMap;

use super::text::{LINE_H, measure, node_size, wrap};
use super::{Half, LEdge, LLabel, LNode, LayoutInput};
use crate::ir::GraphIr;
use crate::limits::{MAX_RENDER_EDGES, MAX_RENDER_NODES};
use crate::svg::RenderError;

/// Padding around an edge label's text block on each side: 4 px.
const EDGE_LABEL_PAD: Half = 8;

/// The wrapped label box of an edge; a missing, empty or blank label draws nothing.
fn edge_label(label: Option<&str>) -> Option<LLabel> {
    let lines = wrap(label?);
    if lines.is_empty() {
        return None;
    }
    let tw = lines.iter().map(|l| measure(l)).max().unwrap_or(0);
    let th = Half::try_from(lines.len()).unwrap_or(0) * LINE_H * 2;
    Some(LLabel {
        lines,
        w: tw + 2 * EDGE_LABEL_PAD,
        h: th + 2 * EDGE_LABEL_PAD,
    })
}

/// Validate `g` against the render caps and build the layout input.
pub(crate) fn build(g: &GraphIr) -> Result<LayoutInput, RenderError> {
    let boxes = g.nodes.len() + g.containers.len();
    if boxes > MAX_RENDER_NODES || g.edges.len() > MAX_RENDER_EDGES {
        return Err(RenderError::TooLarge {
            nodes: boxes,
            edges: g.edges.len(),
        });
    }
    // Plan 85-03 owns containers; until then refuse rather than silently drop what would be drawn.
    if !g.containers.is_empty() {
        return Err(RenderError::InvalidIr(
            "container 0: containers are not drawn yet".into(),
        ));
    }
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in g.nodes.iter().enumerate() {
        if index.insert(n.id.as_str(), i).is_some() {
            return Err(RenderError::InvalidIr(format!("node {i}: duplicate id")));
        }
    }
    let mut edges = Vec::with_capacity(g.edges.len());
    for (i, e) in g.edges.iter().enumerate() {
        let (Some(&from), Some(&to)) = (index.get(e.from.as_str()), index.get(e.to.as_str()))
        else {
            return Err(RenderError::InvalidIr(format!(
                "edge {i}: unknown endpoint"
            )));
        };
        edges.push(LEdge {
            from,
            to,
            label: edge_label(e.label.as_deref()),
        });
    }
    let nodes = g
        .nodes
        .iter()
        .map(|n| {
            let lines = wrap(&n.label);
            let (w, h) = node_size(n.shape, &lines);
            LNode {
                shape: n.shape,
                lines,
                w,
                h,
            }
        })
        .collect();
    Ok(LayoutInput {
        direction: g.direction,
        nodes,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, IrEdge, IrNode, Shape};

    fn node(id: &str) -> IrNode {
        IrNode {
            id: id.into(),
            local_id: id.into(),
            label: id.into(),
            shape: Shape::Rect,
            container: None,
            span: Default::default(),
        }
    }

    fn graph(nodes: Vec<IrNode>, edges: Vec<IrEdge>) -> GraphIr {
        GraphIr {
            direction: Direction::Down,
            nodes,
            containers: vec![],
            edges,
        }
    }

    #[test]
    fn caps_come_first() {
        let many = (0..101).map(|i| node(&format!("n{i}"))).collect();
        assert_eq!(
            build(&graph(many, vec![])),
            Err(RenderError::TooLarge {
                nodes: 101,
                edges: 0
            })
        );
    }

    #[test]
    fn resolves_endpoints_by_id() {
        let e = IrEdge {
            from: "b".into(),
            to: "a".into(),
            label: None,
            span: Default::default(),
        };
        let Ok(input) = build(&graph(vec![node("a"), node("b")], vec![e])) else {
            panic!("build failed");
        };
        assert_eq!(
            input.edges,
            vec![LEdge {
                from: 1,
                to: 0,
                label: None
            }]
        );
    }

    #[test]
    fn blank_edge_labels_draw_nothing() {
        for blank in [None, Some(""), Some("  \t "), Some("\n\n")] {
            assert_eq!(edge_label(blank), None, "{blank:?}");
        }
    }

    #[test]
    fn an_edge_label_box_is_the_text_block_plus_4px_each_side() {
        let Some(one) = edge_label(Some("go")) else {
            panic!("no label");
        };
        assert_eq!(one.lines, vec!["go".to_owned()]);
        assert_eq!((one.w, one.h), (measure("go") + 16, 36 + 16));
        let Some(two) = edge_label(Some("a\nbb")) else {
            panic!("no label");
        };
        assert_eq!((two.w, two.h), (measure("bb") + 16, 72 + 16));
    }
}
