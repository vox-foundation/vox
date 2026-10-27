//! SC2 read back from the drawn SVG: for generated graphs every `Ok` render has disjoint node
//! boxes, nodes inside their container, containers inside their parent, everything inside the
//! viewBox and labels centred in their boxes. The numbers come from `support::svgcheck`, never
//! from the renderer's own verifier. `Err` is allowed only as `Geometry` (the renderer fails
//! closed) and is counted, not asserted away.

mod support;

use std::cell::Cell;

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use support::clearance::{clearance, through};
use support::svgcheck::{Shape, Svg, check as check_svg};
use support::{model_graph, noise, noisy_source};
use unicode_width::UnicodeWidthStr;
use vox_diagram::check;
use vox_diagram::ir::{GraphIr, Ir};
use vox_diagram::svg::{RenderError, SvgOpts, render};

fn right(s: &Shape) -> i64 {
    s.x + s.w
}

fn bottom(s: &Shape) -> i64 {
    s.y + s.h
}

fn inside(inner: &Shape, outer: &Shape) -> bool {
    inner.x >= outer.x
        && inner.y >= outer.y
        && right(inner) <= right(outer)
        && bottom(inner) <= bottom(outer)
}

fn overlaps(a: &Shape, b: &Shape) -> bool {
    a.x < right(b) && b.x < right(a) && a.y < bottom(b) && b.y < bottom(a)
}

/// Derived nesting depth of each IR container (1 for a top-level one), from parent links only.
fn depths(g: &GraphIr) -> Vec<usize> {
    g.containers
        .iter()
        .map(|c| {
            let (mut d, mut cur) = (1, c.parent.as_deref());
            while let Some(id) = cur {
                d += 1;
                cur = g
                    .containers
                    .iter()
                    .find(|p| p.id == id)
                    .and_then(|p| p.parent.as_deref());
                if d > 64 {
                    break;
                }
            }
            d
        })
        .collect()
}

/// For each IR container index, its position in the SVG's container list (depth, then IR order).
fn svg_positions(g: &GraphIr) -> Vec<usize> {
    let d = depths(g);
    let mut order: Vec<usize> = (0..d.len()).collect();
    order.sort_by_key(|&i| (d[i], i));
    let mut pos = vec![0; d.len()];
    for (at, &i) in order.iter().enumerate() {
        pos[i] = at;
    }
    pos
}

fn container_index(g: &GraphIr, id: &str) -> Option<usize> {
    g.containers.iter().position(|c| c.id == id)
}

/// Floor on successful renders out of 128 cases (a reject-everything verifier fails it).
const MIN_DRAWN: usize = 96;

/// Ceiling on drawn cases with an edge through a container title, out of 128 (measured rate about
/// 0.5%, so more than 6 is a regression, not noise).
const MAX_CROSSED: usize = 6;

/// Ceilings on drawn cases with an edge through a node it does not join, and on edge ends within
/// 12 px of a node corner, out of 128. With the ports both measured 0 of 1024 cases; without them
/// 110 cases and 875 ends.
const MAX_THROUGH: usize = 2;
const MAX_CORNERS: usize = 4;

fn fail<T>(m: String) -> Result<T, TestCaseError> {
    Err(TestCaseError::fail(m))
}

fn boxes_hold(g: &GraphIr, s: &Svg) -> Result<(), TestCaseError> {
    if s.nodes.len() != g.nodes.len() || s.containers.len() != g.containers.len() {
        return fail(format!(
            "drew {} nodes and {} containers for {} and {}",
            s.nodes.len(),
            s.containers.len(),
            g.nodes.len(),
            g.containers.len()
        ));
    }
    for (i, a) in s.nodes.iter().enumerate() {
        for (j, b) in s.nodes.iter().enumerate().skip(i + 1) {
            if overlaps(a, b) {
                return fail(format!("nodes {i} and {j} overlap: {a:?} {b:?}"));
            }
        }
    }
    let pos = svg_positions(g);
    for (k, n) in g.nodes.iter().enumerate() {
        let Some(id) = n.container.as_deref() else {
            continue;
        };
        let Some(c) = container_index(g, id) else {
            return fail(format!("node {k} names unknown container {id}"));
        };
        if !inside(&s.nodes[k], &s.containers[pos[c]]) {
            return fail(format!(
                "node {k} {:?} leaves container {id} {:?}",
                s.nodes[k], s.containers[pos[c]]
            ));
        }
    }
    for (i, c) in g.containers.iter().enumerate() {
        let Some(p) = c.parent.as_deref().and_then(|p| container_index(g, p)) else {
            continue;
        };
        if !inside(&s.containers[pos[i]], &s.containers[pos[p]]) {
            return fail(format!("container {} leaves its parent", c.id));
        }
    }
    Ok(())
}

/// IR indices of container `i` and all its ancestors, from parent links only.
fn chain(g: &GraphIr, i: usize) -> Vec<usize> {
    let mut out = vec![i];
    let mut cur = g.containers[i].parent.as_deref();
    while let Some(k) = cur.and_then(|id| container_index(g, id)) {
        if out.contains(&k) {
            break;
        }
        out.push(k);
        cur = g.containers[k].parent.as_deref();
    }
    out
}

/// The title text of each IR container, read back from the drawn label: lines 16 px apart with
/// the first baseline `(n-1)*8 - 5` px above the centre, so the block is the centre plus or minus
/// 8 px a line. Its width is a lower bound on the renderer's width model, 2 px per terminal cell
/// (the narrowest advance is about 2.9 px), so the box never claims space the text may not use.
fn bands(g: &GraphIr, s: &Svg) -> Vec<Shape> {
    let pos = svg_positions(g);
    (0..g.containers.len())
        .map(|i| {
            let Some(l) = s
                .labels
                .iter()
                .find(|l| l.class == "vd-container-label" && l.owner == pos[i])
            else {
                return Shape {
                    w: 0,
                    h: 0,
                    ..s.containers[pos[i]]
                };
            };
            let n = i64::try_from(l.lines.len()).unwrap_or(0);
            let cy = l.first_baseline + (n - 1) * 16 - 10;
            let w = l
                .lines
                .iter()
                .map(|t| 4 * i64::try_from(UnicodeWidthStr::width(t.as_str())).unwrap_or(0))
                .max()
                .unwrap_or(0);
            Shape {
                x: l.anchor_x - w / 2,
                y: cy - 16 * n,
                w,
                h: 32 * n,
            }
        })
        .collect()
}

/// Sibling containers disjoint, foreign nodes clear of a frame, no child on a title.
fn frames_hold(g: &GraphIr, s: &Svg) -> Result<(), TestCaseError> {
    let pos = svg_positions(g);
    let band = bands(g, s);
    for (i, a) in g.containers.iter().enumerate() {
        for (j, b) in g.containers.iter().enumerate().skip(i + 1) {
            if a.parent == b.parent && overlaps(&s.containers[pos[i]], &s.containers[pos[j]]) {
                return fail(format!("sibling containers {} and {} overlap", a.id, b.id));
            }
        }
        let parent = a.parent.as_deref().and_then(|p| container_index(g, p));
        if parent.is_some_and(|p| overlaps(&s.containers[pos[i]], &band[p])) {
            return fail(format!("container {} sits on its parent's title", a.id));
        }
    }
    for (k, n) in g.nodes.iter().enumerate() {
        let own = n.container.as_deref().and_then(|c| container_index(g, c));
        let held = own.map(|c| chain(g, c)).unwrap_or_default();
        for (j, c) in g.containers.iter().enumerate() {
            if !held.contains(&j) && overlaps(&s.nodes[k], &s.containers[pos[j]]) {
                return fail(format!("node {k} overlaps foreign container {}", c.id));
            }
        }
        if own.is_some_and(|c| overlaps(&s.nodes[k], &band[c])) {
            return fail(format!("node {k} sits on its container's title"));
        }
    }
    Ok(())
}

/// Edge-label boxes clear of every node box, every other label box and every title.
fn edge_labels_clear(g: &GraphIr, s: &Svg) -> Result<(), TestCaseError> {
    let band = bands(g, s);
    for (e, l) in s.edge_labels.iter().enumerate() {
        if let Some(k) = s.nodes.iter().position(|n| overlaps(l, n)) {
            return fail(format!("edge label {e} {l:?} covers node {k}"));
        }
        if let Some(k) = band.iter().position(|b| overlaps(l, b)) {
            return fail(format!(
                "edge label {e} {l:?} covers the title of {}",
                g.containers[k].id
            ));
        }
        if let Some(o) = s
            .edge_labels
            .iter()
            .skip(e + 1)
            .position(|o| overlaps(l, o))
        {
            return fail(format!("edge labels {e} and {} overlap", e + 1 + o));
        }
    }
    Ok(())
}

/// No edge path strikes through a container title.
fn titles_clear(g: &GraphIr, s: &Svg) -> Result<(), TestCaseError> {
    for (k, t) in bands(g, s).iter().enumerate() {
        for (e, path) in s.edge_paths.iter().enumerate() {
            if path.windows(2).any(|p| through(p[0], p[1], t)) {
                return fail(format!(
                    "edge {e} strikes through the title of {}",
                    g.containers[k].id
                ));
            }
        }
    }
    Ok(())
}

fn view_box_holds(s: &Svg) -> Result<(), TestCaseError> {
    let frame = Shape {
        x: 0,
        y: 0,
        w: s.width,
        h: s.height,
    };
    let shapes = s.nodes.iter().chain(&s.containers).chain(&s.edge_labels);
    for sh in shapes {
        if !inside(sh, &frame) {
            return fail(format!("{sh:?} leaves the viewBox {frame:?}"));
        }
    }
    for &(x, y) in &s.edge_points {
        if x < 0 || y < 0 || x > s.width || y > s.height {
            return fail(format!(
                "edge point ({x}, {y}) leaves the viewBox {frame:?}"
            ));
        }
    }
    Ok(())
}

/// Node and edge label lines are 18 px apart, so the first baseline is the box centre minus
/// `(n-1)*9` px plus 5 px. A box whose size is an odd number of half-pixels has no representable
/// centre: the anchor may sit one half-pixel (0.25 px) off it, never more.
fn labels_hold(s: &Svg) -> Result<(), TestCaseError> {
    for l in &s.labels {
        let n = i64::try_from(l.lines.len()).unwrap_or(0);
        let boxed = match l.class.as_str() {
            "vd-label" => s.nodes.get(l.owner),
            "vd-edge-label" => s.edge_labels.get(l.owner),
            _ => None,
        };
        let Some(b) = boxed else {
            continue;
        };
        if (2 * l.anchor_x - (2 * b.x + b.w)).abs() > 1 {
            return fail(format!(
                "{} not centred horizontally in {b:?}: x={}",
                l.class, l.anchor_x
            ));
        }
        let centre = l.first_baseline + (n - 1) * 18 - 10;
        if (2 * centre - (2 * b.y + b.h)).abs() > 1 {
            return fail(format!(
                "{} not centred vertically in {b:?}: centre {centre}",
                l.class
            ));
        }
        // Nodes pad 10 px above and below; an edge label's background only has to hold its lines.
        let pad = if l.class == "vd-label" { 40 } else { 0 };
        if n * 36 + pad > b.h {
            return fail(format!("{} of {n} lines does not fit {b:?}", l.class));
        }
    }
    Ok(())
}

/// Counts of rejected (`Err(Geometry)`) and drawn (`Ok`) renders, drawn ones with an edge through
/// a title or through a node it does not join, and edge ends on a corner of their node.
#[derive(Default)]
struct Tally {
    rejected: Cell<usize>,
    drawn: Cell<usize>,
    crossed: Cell<usize>,
    through: Cell<usize>,
    corners: Cell<usize>,
}

fn hold(ir: &Ir, tally: &Tally) -> Result<(), TestCaseError> {
    let Ir::Graph(g) = ir else {
        panic!("expected a graph ir");
    };
    let svg = match render(ir, &SvgOpts::default()) {
        Ok(s) => s,
        Err(RenderError::Geometry(_)) => {
            tally.rejected.set(tally.rejected.get() + 1);
            return Ok(());
        }
        Err(e) => return fail(format!("render failed with {e:?}")),
    };
    let parsed = match check_svg(&svg, "vd") {
        Ok(p) => p,
        Err(e) => return fail(format!("{e}\n{svg}")),
    };
    tally.drawn.set(tally.drawn.get() + 1);
    boxes_hold(g, &parsed)?;
    frames_hold(g, &parsed)?;
    edge_labels_clear(g, &parsed)?;
    if titles_clear(g, &parsed).is_err() {
        tally.crossed.set(tally.crossed.get() + 1);
    }
    let c = clearance(g, &parsed);
    tally
        .through
        .set(tally.through.get() + usize::from(!c.through.is_empty()));
    tally.corners.set(tally.corners.get() + c.corners.len());
    view_box_holds(&parsed)?;
    labels_hold(&parsed)
}

#[test]
fn geometry_holds() {
    let tally = Tally::default();
    let mut runner = TestRunner::new(Config::with_cases(128));
    let result = runner.run(&(model_graph(), noise()), |(m, n)| {
        let src = noisy_source(&m, &n);
        match check(&src).ir {
            Some(ir) => hold(&ir, &tally).map_err(|e| TestCaseError::fail(format!("{e}\n{src}"))),
            None => fail(format!("does not check:\n{src}")),
        }
    });
    let (rejected, drawn) = (tally.rejected.get(), tally.drawn.get());
    let (crossed, through, corners) = (
        tally.crossed.get(),
        tally.through.get(),
        tally.corners.get(),
    );
    println!(
        "geometry_holds: {rejected} Err(Geometry), {drawn} drawn of 128 cases, {crossed} with an edge through a title, {through} through a node, {corners} edge ends on a corner"
    );
    if let Err(e) = result {
        panic!("{e}");
    }
    // A verifier that rejects everything would otherwise leave the oracle above with nothing to read.
    assert!(
        drawn >= MIN_DRAWN,
        "only {drawn} of 128 cases rendered ({rejected} rejected)"
    );
    // Dagre routes straight segments between ranks, so an edge can still clip a title box when every
    // rung does; the renderer then draws the first valid rung. Measured 5 of 1024 cases before
    // the ports, 0 after.
    assert!(
        crossed <= MAX_CROSSED,
        "{crossed} of {drawn} drawn cases have an edge through a title"
    );
    assert!(
        through <= MAX_THROUGH && corners <= MAX_CORNERS,
        "{through} drawn cases have an edge through a node, {corners} edge ends sit on a corner"
    );
}
