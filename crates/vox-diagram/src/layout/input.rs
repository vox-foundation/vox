//! IR to [`LayoutInput`]: render caps, reference checks and node sizing. The IR is public and
//! hand-buildable, so nothing the DSL guarantees is trusted here.

use std::collections::BTreeMap;

use super::finish::title_h;
use super::text::{LINE_H, measure, node_size, pad_x, wrap};
use super::{Half, LContainer, LEdge, LLabel, LNode, LayoutInput};
use crate::ir::GraphIr;
use crate::limits::{MAX_DEPTH, MAX_LABEL_CHARS, MAX_RENDER_EDGES, MAX_RENDER_NODES};
use crate::svg::RenderError;

/// Padding around an edge label's text block on each side: 4 px.
const EDGE_LABEL_PAD: Half = 8;

/// Room around a container title inside its title box: 10 px each side, in half-pixels.
pub(crate) const TITLE_PAD: Half = 20;

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
        w: tw + 2 * pad_x(tw, EDGE_LABEL_PAD),
        h: th + 2 * EDGE_LABEL_PAD,
    })
}

/// Reject a label over [`MAX_LABEL_CHARS`] characters; `what` names the item.
pub(crate) fn check_label(what: &str, label: &str) -> Result<(), RenderError> {
    if label.chars().count() > MAX_LABEL_CHARS {
        return Err(RenderError::InvalidIr(format!("{what}: label too long")));
    }
    Ok(())
}

/// Resolve containers: parent indices through an id map, depth from the parent links (the IR
/// `depth` and `local_id` are never read). A cycle never ends before [`MAX_DEPTH`], so the walk
/// is bounded by it.
fn containers(g: &GraphIr) -> Result<(Vec<LContainer>, BTreeMap<&str, usize>), RenderError> {
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, c) in g.containers.iter().enumerate() {
        check_label(&format!("container {i}"), &c.label)?;
        if index.insert(c.id.as_str(), i).is_some() {
            return Err(RenderError::InvalidIr(format!(
                "container {i}: duplicate id"
            )));
        }
    }
    let mut parents = Vec::with_capacity(g.containers.len());
    for (i, c) in g.containers.iter().enumerate() {
        parents.push(match &c.parent {
            None => None,
            Some(p) => {
                Some(*index.get(p.as_str()).ok_or_else(|| {
                    RenderError::InvalidIr(format!("container {i}: unknown parent"))
                })?)
            }
        });
    }
    let mut out = Vec::with_capacity(parents.len());
    for (i, c) in g.containers.iter().enumerate() {
        let (mut depth, mut cur) = (1usize, i);
        while let Some(p) = parents.get(cur).copied().flatten() {
            depth += 1;
            if depth > MAX_DEPTH {
                return Err(RenderError::InvalidIr(format!(
                    "container {i}: nesting deeper than {MAX_DEPTH} or a parent cycle"
                )));
            }
            cur = p;
        }
        let title_lines = wrap(&c.label);
        let widest = title_lines.iter().map(|l| measure(l)).max().unwrap_or(0);
        out.push(LContainer {
            parent: parents.get(i).copied().flatten(),
            depth: u32::try_from(depth).unwrap_or(u32::MAX),
            title_w: widest + 2 * pad_x(widest, TITLE_PAD),
            title_h: title_h(title_lines.len()),
            label: c.label.clone(),
            title_lines,
        });
    }
    Ok((out, index))
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
    let (containers, container_index) = containers(g)?;
    let mut index: BTreeMap<&str, usize> = BTreeMap::new();
    for (i, n) in g.nodes.iter().enumerate() {
        check_label(&format!("node {i}"), &n.label)?;
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
        check_label(&format!("edge {i}"), e.label.as_deref().unwrap_or_default())?;
        edges.push(LEdge {
            from,
            to,
            label: edge_label(e.label.as_deref()),
        });
    }
    let mut nodes = Vec::with_capacity(g.nodes.len());
    for (i, n) in g.nodes.iter().enumerate() {
        let container =
            match &n.container {
                None => None,
                Some(c) => Some(*container_index.get(c.as_str()).ok_or_else(|| {
                    RenderError::InvalidIr(format!("node {i}: unknown container"))
                })?),
            };
        let lines = wrap(&n.label);
        let (w, h) = node_size(n.shape, &lines);
        nodes.push(LNode {
            shape: n.shape,
            lines,
            w,
            h,
            container,
        });
    }
    Ok(LayoutInput {
        direction: g.direction,
        nodes,
        containers,
        edges,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Direction, IrContainer, IrEdge, IrNode, Shape};

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

    fn boxed(id: &str, parent: Option<&str>, label: &str) -> IrContainer {
        IrContainer {
            id: id.into(),
            local_id: id.into(),
            label: label.into(),
            parent: parent.map(Into::into),
            // Hand-built IR may carry any depth; the renderer must never read it.
            depth: 99,
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

    #[test]
    fn a_long_edge_label_box_fits_a_wide_font() {
        let Some(l) = edge_label(Some(&"m".repeat(28))) else {
            panic!("no label");
        };
        let tw = measure(&"m".repeat(28));
        assert!(l.w * 100 >= tw * 112, "{} for {tw}", l.w);
    }

    #[test]
    fn a_long_title_box_uses_the_padding_floor() {
        let title = "m".repeat(16);
        let tw = measure(&title);
        let mut g = graph(vec![], vec![]);
        g.containers = vec![boxed("a", None, &title)];
        let Ok(input) = build(&g) else {
            panic!("build failed");
        };
        let w = input.containers[0].title_w;
        assert_eq!(w, tw + 2 * pad_x(tw, TITLE_PAD));
        assert!(w * 100 >= tw * 112, "{w} for {tw}");
    }

    #[test]
    fn parent_links_resolve_and_depth_is_derived_from_them() {
        let mut g = graph(vec![node("a")], vec![]);
        g.nodes[0].container = Some("c".into());
        g.containers = vec![
            boxed("a", None, "A"),
            boxed("b", Some("a"), "B"),
            boxed("c", Some("b"), "C"),
        ];
        let Ok(input) = build(&g) else {
            panic!("build failed");
        };
        let parents: Vec<_> = input.containers.iter().map(|c| c.parent).collect();
        let depths: Vec<_> = input.containers.iter().map(|c| c.depth).collect();
        assert_eq!(parents, vec![None, Some(0), Some(1)]);
        assert_eq!(depths, vec![1, 2, 3]);
        assert_eq!(input.nodes[0].container, Some(2));
    }

    /// WR-04: the old 260 px strut cap and the frame re-wrap both dropped the floor for any title
    /// line wider than about 232 px; a full 28-cell line of the widest glyphs must keep it.
    #[test]
    fn a_full_width_title_line_keeps_the_padding_floor() {
        for glyph in ["W", "m"] {
            let mut g = graph(vec![], vec![]);
            g.containers = vec![boxed("a", None, &glyph.repeat(60))];
            let Ok(input) = build(&g) else {
                panic!("build failed");
            };
            let c = &input.containers[0];
            let tw = c.title_lines.iter().map(|l| measure(l)).max().unwrap_or(0);
            assert!(
                tw > 464,
                "{glyph}: a line of {tw} half-px does not exercise the old cap"
            );
            assert!(
                c.title_w * 100 >= tw * 112,
                "{glyph}: box {} for {tw}",
                c.title_w
            );
        }
    }

    #[test]
    fn a_title_is_wrapped_and_its_box_is_the_text_plus_10px_each_side() {
        let mut g = graph(vec![], vec![]);
        g.containers = vec![boxed("a", None, "Backend services\nand jobs")];
        let Ok(input) = build(&g) else {
            panic!("build failed");
        };
        let c = &input.containers[0];
        assert_eq!(c.title_lines, wrap("Backend services\nand jobs"));
        assert_eq!(c.title_w, measure("Backend services") + 40);
        assert_eq!(c.title_h, title_h(2));
    }

    /// A title wraps at the node width in every direction, so no word that fits a line is broken.
    /// The old 16-cell left-to-right column drew "HarnessRiverPane" / "l (Dockable +".
    #[test]
    fn a_title_never_breaks_a_word_that_fits_a_line() {
        let title = "HarnessRiverPanel (Dockable + Full Surface) EnforcementLayers \
                     Inbound_discovery_planned";
        let words: Vec<&str> = title.split_whitespace().collect();
        for direction in [Direction::Right, Direction::Left, Direction::Down] {
            let mut g = graph(vec![], vec![]);
            (g.direction, g.containers) = (direction, vec![boxed("a", None, title)]);
            let Ok(input) = build(&g) else {
                panic!("build failed");
            };
            let lines = &input.containers[0].title_lines;
            let drawn: Vec<&str> = lines.iter().flat_map(|l| l.split_whitespace()).collect();
            assert_eq!(drawn, words, "{direction:?}: {lines:?}");
        }
    }

    #[test]
    fn unknown_duplicate_cyclic_and_too_deep_containers_are_invalid_ir() {
        let invalid = |g: &GraphIr| matches!(build(g), Err(RenderError::InvalidIr(_)));
        let mut unknown_parent = graph(vec![], vec![]);
        unknown_parent.containers = vec![boxed("a", Some("zz"), "A")];
        assert!(invalid(&unknown_parent));
        let mut dup = graph(vec![], vec![]);
        dup.containers = vec![boxed("a", None, "A"), boxed("a", None, "B")];
        assert!(invalid(&dup));
        let mut selfie = graph(vec![], vec![]);
        selfie.containers = vec![boxed("a", Some("a"), "A")];
        assert!(invalid(&selfie));
        let mut deep = graph(vec![], vec![]);
        deep.containers = vec![
            boxed("a", None, "A"),
            boxed("b", Some("a"), "B"),
            boxed("c", Some("b"), "C"),
            boxed("d", Some("c"), "D"),
            boxed("e", Some("d"), "E"),
        ];
        assert!(invalid(&deep));
        deep.containers.pop();
        assert!(build(&deep).is_ok());
        let mut unknown_node = graph(vec![node("a")], vec![]);
        unknown_node.nodes[0].container = Some("zz".into());
        assert!(invalid(&unknown_node));
    }

    #[test]
    fn labels_over_the_cap_are_invalid_ir() {
        let long = "x".repeat(MAX_LABEL_CHARS + 1);
        let mut n = graph(vec![node("a")], vec![]);
        n.nodes[0].label = long.clone();
        let mut c = graph(vec![], vec![]);
        c.containers = vec![boxed("a", None, &long)];
        let mut e = graph(vec![node("a"), node("b")], vec![]);
        e.edges = vec![IrEdge {
            from: "a".into(),
            to: "b".into(),
            label: Some(long),
            span: Default::default(),
        }];
        for g in [n, c, e] {
            assert!(matches!(build(&g), Err(RenderError::InvalidIr(_))));
        }
        let mut ok = graph(vec![node("a")], vec![]);
        ok.nodes[0].label = "x".repeat(MAX_LABEL_CHARS);
        assert!(build(&ok).is_ok());
    }
}
