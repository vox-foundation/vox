//! Edge routing and drawing: rounded polylines, diamond endpoint clipping and edge labels. All
//! geometry is integer half-pixels (`isqrt` and integer division); no float exists here.

use super::paint;
use super::shapes;
use super::writer::{Writer, num};
use crate::ir::Shape;
use crate::layout::{Half, LayoutInput, Placed, Rect};

/// Corner radius in half-pixels (8 px).
const RADIUS: i64 = 16;

fn half(v: i64) -> Half {
    Half::try_from(v).unwrap_or(Half::MAX)
}

fn xy((x, y): (i64, i64)) -> String {
    format!("{} {}", num(half(x)), num(half(y)))
}

/// Distance between two points, rounded down.
fn dist(a: (i64, i64), b: (i64, i64)) -> i64 {
    let (dx, dy) = (a.0 - b.0, a.1 - b.1);
    (dx * dx + dy * dy).isqrt()
}

/// Append the rounded corner at `cur`: a line to `RADIUS` (or half the shorter leg) before it, a
/// quadratic through it, ending the same distance after it. A straight run just gets a line.
fn corner(d: &mut String, prev: (i64, i64), cur: (i64, i64), next: (i64, i64)) {
    let cross = (cur.0 - prev.0) * (next.1 - cur.1) - (cur.1 - prev.1) * (next.0 - cur.0);
    let (l1, l2) = (dist(prev, cur), dist(cur, next));
    if cross == 0 || l1 == 0 || l2 == 0 {
        d.push('L');
        d.push_str(&xy(cur));
        return;
    }
    let r = RADIUS.min(l1 / 2).min(l2 / 2);
    let toward = |to: (i64, i64), l: i64| {
        (
            cur.0 + (to.0 - cur.0) * r / l,
            cur.1 + (to.1 - cur.1) * r / l,
        )
    };
    d.push('L');
    d.push_str(&xy(toward(prev, l1)));
    d.push('Q');
    d.push_str(&xy(cur));
    d.push(' ');
    d.push_str(&xy(toward(next, l2)));
}

/// `M`/`L`/`Q` path data through `points` with rounded corners; repeated points are dropped.
pub(super) fn rounded(points: &[(Half, Half)]) -> String {
    let mut pts: Vec<(i64, i64)> = Vec::with_capacity(points.len());
    for &(x, y) in points {
        let p = (i64::from(x), i64::from(y));
        if pts.last() != Some(&p) {
            pts.push(p);
        }
    }
    let Some(&first) = pts.first() else {
        return String::new();
    };
    let mut d = format!("M{}", xy(first));
    for (i, &cur) in pts.iter().enumerate().skip(1) {
        match (pts.get(i - 1), pts.get(i + 1)) {
            (Some(&prev), Some(&next)) => corner(&mut d, prev, cur, next),
            _ => {
                d.push('L');
                d.push_str(&xy(cur));
            }
        }
    }
    d
}

/// Move `p`, an engine endpoint on the bounding box of a diamond occupying `r`, onto the diamond
/// outline along the ray from its centre: `s = |dx|/(w/2) + |dy|/(h/2); p = c + d/s`. A point on
/// or inside the outline is returned unchanged.
pub(super) fn clip_diamond(r: Rect, p: (Half, Half)) -> (Half, Half) {
    let (w, h) = (i64::from(r.w), i64::from(r.h));
    let (cx, cy) = (i64::from(r.x) + w / 2, i64::from(r.y) + h / 2);
    let (dx, dy) = (i64::from(p.0) - cx, i64::from(p.1) - cy);
    let den = 2 * dx.abs() * h + 2 * dy.abs() * w;
    if den <= w * h {
        return p;
    }
    (half(cx + dx * w * h / den), half(cy + dy * w * h / den))
}

/// Draw every edge as one rounded path ending in the arrow marker.
pub(super) fn write_edges(w: &mut Writer, input: &LayoutInput, placed: &Placed, arrow_id: &str) {
    let style = format!("fill:none;stroke:{};stroke-width:1.5", paint("--vd-edge"));
    let marker = format!("url(#{arrow_id})");
    let clip = |node: usize, p: (Half, Half)| match (input.nodes.get(node), placed.nodes.get(node))
    {
        (Some(n), Some(&r)) if n.shape == Shape::Diamond => clip_diamond(r, p),
        _ => p,
    };
    w.open("g", &[("class", "vd-edges"), ("style", &style)]);
    for (le, pe) in input.edges.iter().zip(&placed.edges) {
        let mut pts = pe.points.clone();
        if let Some(p) = pts.first_mut() {
            *p = clip(le.from, *p);
        }
        if let Some(p) = pts.last_mut() {
            *p = clip(le.to, *p);
        }
        let d = rounded(&pts);
        w.empty(
            "path",
            &[("class", "vd-edge"), ("d", &d), ("marker-end", &marker)],
        );
    }
    w.close("g");
}

/// Draw each edge label on a `--vd-bg` background rect, after the nodes; no group when no edge
/// has a label.
pub(super) fn write_edge_labels(w: &mut Writer, input: &LayoutInput, placed: &Placed) {
    let labels: Vec<_> = input
        .edges
        .iter()
        .zip(&placed.edges)
        .filter_map(|(le, pe)| Some((le.label.as_ref()?, pe.label?)))
        .collect();
    if labels.is_empty() {
        return;
    }
    let bg = format!("fill:{};stroke:none", paint("--vd-bg"));
    w.open("g", &[("class", "vd-edge-labels")]);
    for (l, r) in labels {
        let (x, y, bw, bh) = (num(r.x), num(r.y), num(r.w), num(r.h));
        w.empty(
            "rect",
            &[
                ("class", "vd-edge-label-bg"),
                ("x", &x),
                ("y", &y),
                ("width", &bw),
                ("height", &bh),
                ("style", &bg),
            ],
        );
        shapes::label(
            w,
            &l.lines,
            r,
            ("vd-edge-label", "--vd-muted"),
            shapes::LABEL_STEP,
        );
    }
    w.close("g");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_points_are_a_line() {
        assert_eq!(rounded(&[(0, 0), (100, 0)]), "M0 0L50 0");
    }

    #[test]
    fn collinear_points_have_no_curve() {
        let d = rounded(&[(0, 0), (40, 0), (80, 0)]);
        assert_eq!(d, "M0 0L20 0L40 0");
        assert!(!d.contains('Q'));
    }

    #[test]
    fn an_l_shape_rounds_its_corner_with_radius_8px() {
        assert_eq!(
            rounded(&[(0, 0), (100, 0), (100, 100)]),
            "M0 0L42 0Q50 0 50 8L50 50"
        );
    }

    #[test]
    fn a_short_leg_halves_the_radius() {
        // Shorter leg is 6 px (12 half-pixels): radius 3 px.
        assert_eq!(
            rounded(&[(0, 0), (100, 0), (100, 12)]),
            "M0 0L47 0Q50 0 50 3L50 6"
        );
    }

    #[test]
    fn repeated_points_are_dropped() {
        assert_eq!(rounded(&[(0, 0), (0, 0), (100, 0)]), "M0 0L50 0");
        assert_eq!(rounded(&[(4, 4)]), "M2 2");
        assert_eq!(rounded(&[]), "");
    }

    /// 96x64 px diamond centred at (100, 100) px.
    const DIAMOND: Rect = Rect {
        x: 104,
        y: 136,
        w: 192,
        h: 128,
    };

    #[test]
    fn a_point_straight_above_lands_on_the_top_vertex() {
        assert_eq!(clip_diamond(DIAMOND, (200, 20)), (200, 136));
        assert_eq!(clip_diamond(DIAMOND, (200, 400)), (200, 264));
        assert_eq!(clip_diamond(DIAMOND, (0, 200)), (104, 200));
    }

    #[test]
    fn a_diagonal_approach_lands_on_the_outline() {
        let (w, h) = (i64::from(DIAMOND.w), i64::from(DIAMOND.h));
        for p in [(296, 150), (104, 136), (250, 136), (140, 264), (296, 264)] {
            let (x, y) = clip_diamond(DIAMOND, p);
            let (dx, dy) = (i64::from(x - 200).abs(), i64::from(y - 200).abs());
            let on = 2 * dx * h + 2 * dy * w;
            // One half-pixel of rounding per coordinate.
            assert!((on - w * h).abs() <= 2 * (w + h), "{p:?} -> {x},{y}: {on}");
        }
    }

    #[test]
    fn a_point_inside_is_unchanged() {
        assert_eq!(clip_diamond(DIAMOND, (210, 205)), (210, 205));
        assert_eq!(clip_diamond(DIAMOND, (200, 200)), (200, 200));
    }
}
