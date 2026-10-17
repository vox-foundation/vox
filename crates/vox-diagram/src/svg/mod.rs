//! Deterministic, injection-safe, themeable SVG for graph diagrams. All text goes through the
//! single writer in [`writer`]; every paint is a `var(--vd-NAME,FALLBACK)` built from
//! [`THEME_VARS`]; no attribute value is derived from a label.

mod paths;
mod shapes;
pub(crate) mod writer;

use std::fmt;

use crate::ir::{GraphIr, Ir};
use crate::layout::dagre_impl::DagreEngine;
use crate::layout::{self, Engine, LEdge, LayoutInput, Placed};
use writer::{Writer, num};

/// Options for [`render`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SvgOpts {
    /// Prefix of every generated id (`<prefix>-title`, `-desc`, `-arrow`); `[a-z][a-z0-9-]*`,
    /// at most 32 characters.
    pub id_prefix: String,
    /// Accessible title; `Diagram` when `None`.
    pub title: Option<String>,
}

impl Default for SvgOpts {
    fn default() -> Self {
        Self {
            id_prefix: "vd".to_owned(),
            title: None,
        }
    }
}

/// Why a diagram could not be rendered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderError {
    /// `id_prefix` is not `[a-z][a-z0-9-]*` or is longer than 32 characters.
    InvalidIdPrefix,
    /// More than `MAX_RENDER_NODES` nodes plus containers or `MAX_RENDER_EDGES` edges.
    TooLarge {
        /// Nodes plus containers in the IR.
        nodes: usize,
        /// Edges in the IR.
        edges: usize,
    },
    /// The IR is inconsistent (names the offending item by kind and index).
    InvalidIr(String),
    /// The layout engine panicked or returned a non-finite or missing coordinate.
    Layout(&'static str),
    /// The laid-out geometry violates a hard invariant on every separation rung.
    Geometry(String),
}

impl fmt::Display for RenderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidIdPrefix => {
                f.write_str("id prefix must match [a-z][a-z0-9-]* and be at most 32 characters")
            }
            Self::TooLarge { nodes, edges } => write!(
                f,
                "diagram too large to render: {nodes} nodes and containers, {edges} edges"
            ),
            Self::InvalidIr(m) => write!(f, "invalid diagram: {m}"),
            Self::Layout(m) => write!(f, "layout failed: {m}"),
            Self::Geometry(m) => write!(f, "layout geometry rejected: {m}"),
        }
    }
}

impl std::error::Error for RenderError {}

/// Theme variables as `(name, literal fallback)`; every paint is `var(name,fallback)`.
pub const THEME_VARS: [(&str, &str); 9] = [
    ("--vd-text", "#1f2328"),
    ("--vd-muted", "#57606a"),
    ("--vd-node-fill", "#fff"),
    ("--vd-node-stroke", "#57606a"),
    ("--vd-edge", "#57606a"),
    ("--vd-container-fill", "#f6f8fa"),
    ("--vd-container-stroke", "#d0d7de"),
    ("--vd-bg", "#fff"),
    ("--vd-font", "system-ui,sans-serif"),
];

/// `var(NAME,FALLBACK)` for a name in [`THEME_VARS`].
pub(super) fn paint(name: &str) -> String {
    THEME_VARS
        .iter()
        .find(|(n, _)| *n == name)
        .map_or_else(String::new, |(n, f)| format!("var({n},{f})"))
}

/// Render `ir` to an SVG document.
pub fn render(ir: &Ir, opts: &SvgOpts) -> Result<String, RenderError> {
    render_with(&DagreEngine, ir, opts)
}

pub(crate) fn render_with(
    engine: &dyn Engine,
    ir: &Ir,
    opts: &SvgOpts,
) -> Result<String, RenderError> {
    valid_prefix(&opts.id_prefix)?;
    match ir {
        Ir::Graph(g) => render_graph(engine, g, opts),
    }
}

fn valid_prefix(p: &str) -> Result<(), RenderError> {
    let mut chars = p.chars();
    let ok = p.len() <= 32
        && chars.next().is_some_and(|c| c.is_ascii_lowercase())
        && chars.all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
    if ok {
        Ok(())
    } else {
        Err(RenderError::InvalidIdPrefix)
    }
}

fn render_graph(engine: &dyn Engine, g: &GraphIr, opts: &SvgOpts) -> Result<String, RenderError> {
    let input = layout::input::build(g)?;
    let placed = layout::lay_out(engine, &input)?;
    let p = opts.id_prefix.as_str();
    let (title_id, desc_id, arrow_id) = (
        format!("{p}-title"),
        format!("{p}-desc"),
        format!("{p}-arrow"),
    );
    let view_box = format!("0 0 {} {}", num(placed.width), num(placed.height));
    let labelled_by = format!("{title_id} {desc_id}");
    let root_style = format!("font-family:{};font-size:14px", paint("--vd-font"));
    let mut w = Writer::new();
    w.open(
        "svg",
        &[
            ("xmlns", "http://www.w3.org/2000/svg"),
            ("viewBox", &view_box),
            ("role", "img"),
            ("aria-labelledby", &labelled_by),
            ("class", "vd-diagram"),
            ("style", &root_style),
        ],
    );
    w.text(
        "title",
        &[("id", &title_id)],
        opts.title.as_deref().unwrap_or("Diagram"),
    );
    w.text("desc", &[("id", &desc_id)], &describe(g, &input.edges));
    w.open("defs", &[]);
    w.open(
        "marker",
        &[
            ("id", &arrow_id),
            ("viewBox", "0 0 10 10"),
            ("refX", "9"),
            ("refY", "5"),
            ("markerWidth", "8"),
            ("markerHeight", "8"),
            ("orient", "auto"),
        ],
    );
    let arrow_style = format!("fill:{}", paint("--vd-edge"));
    w.empty("path", &[("d", "M0 0L10 5L0 10z"), ("style", &arrow_style)]);
    w.close("marker");
    w.close("defs");
    write_containers(&mut w, &input, &placed);
    paths::write_edges(&mut w, &input, &placed, &arrow_id);
    write_nodes(&mut w, &input, &placed);
    paths::write_edge_labels(&mut w, &input, &placed);
    w.close("svg");
    Ok(w.finish())
}

/// The container frames, a parent before its children (derived depth, then IR order); nothing
/// when the diagram has none.
fn write_containers(w: &mut Writer, input: &LayoutInput, placed: &Placed) {
    let mut order: Vec<_> = input
        .containers
        .iter()
        .zip(&placed.containers)
        .enumerate()
        .collect();
    if order.is_empty() {
        return;
    }
    order.sort_by_key(|(i, (c, _))| (c.depth, *i));
    w.open("g", &[("class", "vd-containers")]);
    for (_, (_, c)) in order {
        shapes::write_container(w, c);
    }
    w.close("g");
}

fn write_nodes(w: &mut Writer, input: &LayoutInput, placed: &Placed) {
    w.open("g", &[("class", "vd-nodes")]);
    for (n, r) in input.nodes.iter().zip(&placed.nodes) {
        shapes::write_node(w, n, *r);
    }
    w.close("g");
}

/// Collapse whitespace of the first label line and cut it to at most `max` chars with `…`.
fn short(label: &str, max: usize) -> String {
    let first = label.split('\n').next().unwrap_or_default();
    cut(&first.split_whitespace().collect::<Vec<_>>().join(" "), max)
}

/// `s` itself, or its first `max - 1` chars plus `…` when longer than `max` chars.
fn cut(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_owned();
    }
    let mut t: String = s.chars().take(max.saturating_sub(1)).collect();
    t.push('\u{2026}');
    t
}

/// The accessible description: counts, then `from → to` pairs by label.
fn describe(g: &GraphIr, edges: &[LEdge]) -> String {
    let plural = |n: usize, one: &str| format!("{n} {one}{}", if n == 1 { "" } else { "s" });
    let mut d = format!(
        "{}, {}",
        plural(g.nodes.len(), "node"),
        plural(g.edges.len(), "edge")
    );
    if !edges.is_empty() {
        let label_of = |i: usize| {
            g.nodes
                .get(i)
                .map_or_else(String::new, |n| short(&n.label, 24))
        };
        let pairs: Vec<String> = edges
            .iter()
            .map(|e| format!("{} \u{2192} {}", label_of(e.from), label_of(e.to)))
            .collect();
        d.push_str(": ");
        d.push_str(&pairs.join("; "));
    }
    cut(&d, 300)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefix_rules() {
        assert!(valid_prefix("vd").is_ok() && valid_prefix("a-1").is_ok());
        for bad in ["", "1a", "-a", "A", "a_b"] {
            assert_eq!(
                valid_prefix(bad),
                Err(RenderError::InvalidIdPrefix),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn cut_keeps_short_and_marks_long() {
        assert_eq!(cut("abc", 3), "abc");
        assert_eq!(cut("abcd", 3), "ab\u{2026}");
        assert_eq!(short("  a   b \nzzz", 24), "a b");
    }

    #[test]
    fn paint_reads_the_table() {
        assert_eq!(paint("--vd-edge"), "var(--vd-edge,#57606a)");
        assert_eq!(paint("--nope"), "");
    }
}
