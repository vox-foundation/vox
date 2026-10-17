//! Graph layout behind an engine seam. The engine speaks `f64`; everything after
//! [`quantize`] is integer half-pixels, so no float is ever formatted and no `-0` exists.

pub(crate) mod dagre_impl;
pub(crate) mod input;
pub(crate) mod text;
pub(crate) mod verify;

use std::panic::{AssertUnwindSafe, catch_unwind};

use crate::ir::{Direction, Shape};
use crate::svg::RenderError;

/// Integer coordinate where 1 unit is 0.5 px.
pub(crate) type Half = i32;

/// Outer margin around everything drawn, in half-pixels (16 px).
const MARGIN: Half = 32;

/// Axis-aligned box, top-left origin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Rect {
    pub x: Half,
    pub y: Half,
    pub w: Half,
    pub h: Half,
}

/// Engine spacing in px.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Separations {
    pub nodesep: i32,
    pub ranksep: i32,
    pub edgesep: i32,
}

/// Escalating separation rungs; the tracer uses the first.
pub(crate) const RUNGS: [Separations; 3] = [
    Separations {
        nodesep: 40,
        ranksep: 60,
        edgesep: 70,
    },
    Separations {
        nodesep: 50,
        ranksep: 90,
        edgesep: 120,
    },
    Separations {
        nodesep: 60,
        ranksep: 140,
        edgesep: 200,
    },
];

/// A sized node (index = IR node index).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LNode {
    pub shape: Shape,
    pub lines: Vec<String>,
    pub w: Half,
    pub h: Half,
}

/// A wrapped edge label and its box (text block plus padding), half-pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LLabel {
    pub lines: Vec<String>,
    pub w: Half,
    pub h: Half,
}

/// An edge between two node indices.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LEdge {
    pub from: usize,
    pub to: usize,
    pub label: Option<LLabel>,
}

/// Everything an engine needs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LayoutInput {
    pub direction: Direction,
    pub nodes: Vec<LNode>,
    pub edges: Vec<LEdge>,
}

/// One node box as the engine reports it (centre and size, px), unjudged.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct RawBox {
    pub cx: Option<f64>,
    pub cy: Option<f64>,
    pub w: Option<f64>,
    pub h: Option<f64>,
}

/// One edge route as the engine reports it (px), unjudged.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RawEdge {
    pub points: Vec<(f64, f64)>,
    /// Label centre `(x, y)`, present only for a labelled edge.
    pub label: Option<(Option<f64>, Option<f64>)>,
}

/// Raw engine output, one entry per input node and edge.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct RawLayout {
    pub nodes: Vec<RawBox>,
    pub edges: Vec<RawEdge>,
}

/// Why a layout was rejected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum LayoutError {
    EnginePanic,
    NonFinite,
    Missing,
}

impl From<LayoutError> for RenderError {
    fn from(e: LayoutError) -> Self {
        RenderError::Layout(match e {
            LayoutError::EnginePanic => "engine panic",
            LayoutError::NonFinite => "non-finite coordinate",
            LayoutError::Missing => "missing coordinate",
        })
    }
}

/// A layout engine; tests substitute doubles that panic or return garbage.
pub(crate) trait Engine {
    /// Lay `input` out with the given spacing.
    fn solve(&self, input: &LayoutInput, sep: Separations) -> Result<RawLayout, LayoutError>;
}

/// Run `f`, turning a panic into [`LayoutError::EnginePanic`]. The workspace keeps unwinding on
/// purpose (see the root `Cargo.toml` profile comment), so this catches.
pub(crate) fn guarded<T>(f: impl FnOnce() -> T) -> Result<T, LayoutError> {
    catch_unwind(AssertUnwindSafe(f)).map_err(|_| LayoutError::EnginePanic)
}

/// Convert an engine coordinate to half-pixels: present, finite and below 1e7.
pub(crate) fn quantize(v: Option<f64>) -> Result<Half, LayoutError> {
    let v = v.ok_or(LayoutError::Missing)?;
    if !v.is_finite() || v.abs() >= 1.0e7 {
        return Err(LayoutError::NonFinite);
    }
    Ok((v * 2.0).round() as Half)
}

/// A routed edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PlacedEdge {
    pub points: Vec<(Half, Half)>,
    /// The label box, present only for a labelled edge.
    pub label: Option<Rect>,
}

/// Final geometry, shifted so the minimum of everything drawn sits at the margin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Placed {
    pub width: Half,
    pub height: Half,
    pub nodes: Vec<Rect>,
    pub edges: Vec<PlacedEdge>,
}

/// Top-left corner from a centre and a size, both unjudged.
fn top_left(centre: Option<f64>, size: Option<f64>) -> Option<f64> {
    Some(centre? - size? / 2.0)
}

/// Quantize one raw engine layout into [`Placed`], shifted so everything drawn starts at the
/// margin. Fails closed on a missing, non-finite or huge coordinate or a wrong item count.
fn place(input: &LayoutInput, raw: &RawLayout) -> Result<Placed, LayoutError> {
    if raw.nodes.len() != input.nodes.len() || raw.edges.len() != input.edges.len() {
        return Err(LayoutError::Missing);
    }
    let mut nodes = Vec::with_capacity(raw.nodes.len());
    for b in &raw.nodes {
        nodes.push(Rect {
            x: quantize(top_left(b.cx, b.w))?,
            y: quantize(top_left(b.cy, b.h))?,
            w: quantize(b.w)?,
            h: quantize(b.h)?,
        });
    }
    let mut edges = Vec::with_capacity(raw.edges.len());
    for (e, le) in raw.edges.iter().zip(&input.edges) {
        if e.points.is_empty() {
            return Err(LayoutError::Missing);
        }
        let mut points = Vec::with_capacity(e.points.len());
        for &(x, y) in &e.points {
            points.push((quantize(Some(x))?, quantize(Some(y))?));
        }
        let label = match &le.label {
            None => None,
            Some(l) => {
                let (cx, cy) = e.label.ok_or(LayoutError::Missing)?;
                Some(Rect {
                    x: quantize(top_left(cx, Some(f64::from(l.w) / 2.0)))?,
                    y: quantize(top_left(cy, Some(f64::from(l.h) / 2.0)))?,
                    w: l.w,
                    h: l.h,
                })
            }
        };
        edges.push(PlacedEdge { points, label });
    }
    let labels = edges.iter().filter_map(|e| e.label);
    let xs = nodes
        .iter()
        .copied()
        .chain(labels.clone())
        .flat_map(|r| [r.x, r.x + r.w])
        .chain(edges.iter().flat_map(|e| e.points.iter().map(|p| p.0)));
    let ys = nodes
        .iter()
        .copied()
        .chain(labels)
        .flat_map(|r| [r.y, r.y + r.h])
        .chain(edges.iter().flat_map(|e| e.points.iter().map(|p| p.1)));
    let (min_x, max_x) = bounds(xs);
    let (min_y, max_y) = bounds(ys);
    let (dx, dy) = (MARGIN - min_x, MARGIN - min_y);
    for r in &mut nodes {
        r.x += dx;
        r.y += dy;
    }
    for e in &mut edges {
        for p in &mut e.points {
            *p = (p.0 + dx, p.1 + dy);
        }
        if let Some(r) = &mut e.label {
            r.x += dx;
            r.y += dy;
        }
    }
    Ok(Placed {
        width: max_x + dx + MARGIN,
        height: max_y + dy + MARGIN,
        nodes,
        edges,
    })
}

/// Lay `input` out through `engine`: try each [`RUNGS`] separation in order and return the first
/// quantized layout that passes [`verify::verify`]. An engine failure is final (no further rung
/// is tried); a layout failing on every rung is [`RenderError::Geometry`], never drawn.
pub(crate) fn lay_out(engine: &dyn Engine, input: &LayoutInput) -> Result<Placed, RenderError> {
    let mut last = None;
    for sep in RUNGS {
        let raw = guarded(|| engine.solve(input, sep))??;
        let placed = place(input, &raw)?;
        match verify::verify(input, &placed) {
            Ok(()) => return Ok(placed),
            Err(v) => last = Some(v),
        }
    }
    Err(RenderError::Geometry(
        last.map_or_else(String::new, |v| v.to_string()),
    ))
}

/// Minimum and maximum of `it`, `(0, 0)` when empty.
fn bounds(it: impl Iterator<Item = Half>) -> (Half, Half) {
    it.fold(None, |acc: Option<(Half, Half)>, v| match acc {
        None => Some((v, v)),
        Some((lo, hi)) => Some((lo.min(v), hi.max(v))),
    })
    .unwrap_or((0, 0))
}

#[cfg(test)]
mod tests;
