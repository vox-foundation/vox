//! Node drawing: one outline element per node (a `rect`, or a `polygon` for a diamond) plus an
//! optional cylinder rim, then the label. Nothing here reads a label for anything but text.

use super::paint;
use super::writer::{Writer, num};
use crate::ir::Shape;
use crate::layout::{Half, LNode, Rect};

/// Corner radius of a rounded rectangle, px.
const ROUND_RX: &str = "10";
/// Corner radius of a plain rectangle, px.
const RECT_RX: &str = "3";
/// Vertical radius of a cylinder cap, px.
const CAP_RY: &str = "8";
/// Cap height in half-pixels (8 px).
const CAP: Half = 16;

/// Draw one node: `<g class="vd-node">` holding its outline and label.
pub(crate) fn write_node(w: &mut Writer, n: &LNode, r: Rect) {
    let stroke_w = if n.shape == Shape::End { "3" } else { "1.5" };
    let style = format!(
        "fill:{};stroke:{};stroke-width:{stroke_w}",
        paint("--vd-node-fill"),
        paint("--vd-node-stroke")
    );
    let (x, y, bw, bh) = (num(r.x), num(r.y), num(r.w), num(r.h));
    w.open("g", &[("class", "vd-node")]);
    match n.shape {
        Shape::Rect => rect(w, [&x, &y, &bw, &bh], &[("rx", RECT_RX)], &style),
        Shape::Rounded => rect(w, [&x, &y, &bw, &bh], &[("rx", ROUND_RX)], &style),
        Shape::Start | Shape::End => {
            let rx = num(r.h / 2);
            rect(w, [&x, &y, &bw, &bh], &[("rx", &rx)], &style);
        }
        Shape::Cylinder => {
            let rx = num(r.w / 2);
            rect(
                w,
                [&x, &y, &bw, &bh],
                &[("rx", &rx), ("ry", CAP_RY)],
                &style,
            );
            let rim = format!(
                "M{x} {}A{rx} {CAP_RY} 0 0 0 {} {}",
                num(r.y + CAP),
                num(r.x + r.w),
                num(r.y + CAP)
            );
            let rim_style = format!(
                "fill:none;stroke:{};stroke-width:1.5",
                paint("--vd-node-stroke")
            );
            w.empty(
                "path",
                &[("class", "vd-node-rim"), ("d", &rim), ("style", &rim_style)],
            );
        }
        Shape::Diamond => {
            let (mx, my) = (r.x + r.w / 2, r.y + r.h / 2);
            let pts = [(mx, r.y), (r.x + r.w, my), (mx, r.y + r.h), (r.x, my)]
                .map(|(px, py)| format!("{},{}", num(px), num(py)))
                .join(" ");
            w.empty(
                "polygon",
                &[
                    ("class", "vd-node-shape"),
                    ("points", &pts),
                    ("style", &style),
                ],
            );
        }
    }
    label(w, &n.lines, r, "vd-label", "--vd-text");
    w.close("g");
}

/// The outline `rect`; `extra` carries the shape-specific radii.
fn rect(w: &mut Writer, [x, y, bw, bh]: [&str; 4], extra: &[(&'static str, &str)], style: &str) {
    let mut attrs = vec![
        ("class", "vd-node-shape"),
        ("x", x),
        ("y", y),
        ("width", bw),
        ("height", bh),
    ];
    attrs.extend_from_slice(extra);
    attrs.push(("style", style));
    w.empty("rect", &attrs);
}

/// Centred label of class `class` painted with theme variable `fill`, one `tspan` per line,
/// first baseline `cy - (n-1)*9 + 5` px; nothing when empty.
pub(super) fn label(w: &mut Writer, lines: &[String], r: Rect, class: &'static str, fill: &str) {
    if lines.is_empty() {
        return;
    }
    let style = format!("fill:{};stroke:none", paint(fill));
    let cx = num(r.x + r.w / 2);
    let count = Half::try_from(lines.len()).unwrap_or(1);
    let first = num(r.y + r.h / 2 - (count - 1) * 18 + 10);
    w.start(
        "text",
        &[
            ("class", class),
            ("x", &cx),
            ("y", &first),
            ("text-anchor", "middle"),
            ("style", &style),
        ],
    );
    for (i, line) in lines.iter().enumerate() {
        w.leaf(
            "tspan",
            &[("x", &cx), ("dy", if i == 0 { "0" } else { "18" })],
            line,
        );
    }
    w.end("text");
    w.nl();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(shape: Shape, lines: &[&str]) -> LNode {
        LNode {
            shape,
            lines: lines.iter().map(|s| (*s).to_owned()).collect(),
            w: 128,
            h: 76,
        }
    }

    fn drawn(shape: Shape, lines: &[&str]) -> String {
        let mut w = Writer::new();
        write_node(
            &mut w,
            &node(shape, lines),
            Rect {
                x: 32,
                y: 32,
                w: 128,
                h: 76,
            },
        );
        w.finish()
    }

    #[test]
    fn each_shape_has_one_outline_element() {
        for shape in [
            Shape::Rect,
            Shape::Rounded,
            Shape::Cylinder,
            Shape::Start,
            Shape::End,
        ] {
            let s = drawn(shape, &["A"]);
            assert_eq!(s.matches("class=\"vd-node-shape\"").count(), 1, "{shape:?}");
            assert!(s.contains("<rect "), "{shape:?}");
        }
        let d = drawn(Shape::Diamond, &["A"]);
        assert!(
            d.contains("<polygon ") && d.contains("points=\"48,16 80,35 48,54 16,35\""),
            "{d}"
        );
    }

    #[test]
    fn radii_and_strokes_per_shape() {
        assert!(drawn(Shape::Rect, &["A"]).contains("rx=\"3\""));
        assert!(drawn(Shape::Rounded, &["A"]).contains("rx=\"10\""));
        assert!(drawn(Shape::Start, &["A"]).contains("rx=\"19\""));
        assert!(drawn(Shape::End, &["A"]).contains("stroke-width:3"));
        let c = drawn(Shape::Cylinder, &["A"]);
        assert!(
            c.contains("rx=\"32\" ry=\"8\"") && c.contains("class=\"vd-node-rim\""),
            "{c}"
        );
    }

    #[test]
    fn label_lines_and_empty_label() {
        let two = drawn(Shape::Rect, &["A", "B"]);
        assert!(
            two.contains("y=\"31\"") && two.contains("dy=\"0\">A<") && two.contains("dy=\"18\">B<"),
            "{two}"
        );
        assert!(!drawn(Shape::Rect, &[]).contains("<text"));
    }
}
