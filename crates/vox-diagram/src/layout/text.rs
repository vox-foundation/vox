//! Text sizing: a static advance-width model (no font access, integer arithmetic only), label
//! wrapping and per-shape node sizes. Everything is in half-pixels ([`Half`]).

use unicode_width::UnicodeWidthChar;

use super::Half;
use crate::ir::Shape;

/// Font size in px of every label.
pub(crate) const FONT_PX: u64 = 14;
/// Line height in px.
pub(crate) const LINE_H: Half = 18;
/// Safety slack added to every measured width, in percent.
pub(crate) const SLACK_PCT: u64 = 8;
/// Horizontal node padding in px (each side).
pub(crate) const PAD_X: Half = 14;
/// Vertical node padding in px (above and below).
pub(crate) const PAD_Y: Half = 10;

/// Advance widths per 1000 em for chars 32..=126 of Arimo Regular 1.341 (SIL OFL 1.1,
/// metric-compatible with Arial/Helvetica). Advance widths only, no glyph data.
const ADV: [u16; 95] = [
    278, 278, 355, 556, 556, 889, 667, 191, 333, 333, 389, 584, 278, 333, 278,
    278, // ` !"#$%&'()*+,-./`
    556, 556, 556, 556, 556, 556, 556, 556, 556, 556, // digits
    278, 278, 584, 584, 584, 556, 1015, // `:;<=>?@`
    667, 667, 722, 722, 667, 611, 778, 722, 278, 500, 667, 556, 833, // A-M
    722, 778, 667, 778, 722, 667, 611, 722, 667, 944, 667, 667, 611, // N-Z
    278, 278, 278, 469, 556, 333, // `[\]^_` and backtick
    556, 556, 500, 556, 556, 278, 556, 556, 222, 222, 500, 222, 833, // a-m
    556, 556, 556, 556, 333, 500, 278, 556, 500, 722, 500, 500, 500, // n-z
    334, 260, 334, 584, // `{|}~`
];

/// Advance of `c` per 1000 em; `next_is_vs16` marks a char followed by U+FE0F (emoji style).
fn adv1000(c: char, next_is_vs16: bool) -> u64 {
    let cp = u32::from(c);
    if next_is_vs16 || (0x1F000..=0x1FFFF).contains(&cp) {
        return 1300;
    }
    match c {
        '\t' | '\r' => u64::from(ADV[0]),
        ' '..='~' => ADV.get(cp as usize - 32).copied().map_or(1000, u64::from),
        '\u{a0}'..='\u{17f}' => 600,
        _ => match c.width() {
            Some(0) => 0,
            // Two cells of 500 per 1000 em.
            _ => 1000,
        },
    }
}

/// Width of one line in half-pixels: ceiling of the summed advances at 14 px with 8% slack.
pub(crate) fn measure(line: &str) -> Half {
    let mut sum = 0u64;
    let mut it = line.chars().peekable();
    while let Some(c) = it.next() {
        sum += adv1000(c, it.peek() == Some(&'\u{fe0f}'));
    }
    let scaled = sum * FONT_PX * 2 * (100 + SLACK_PCT);
    Half::try_from(scaled.div_ceil(100_000)).unwrap_or(Half::MAX)
}

/// Split a label into lines. The tracer splits on explicit newlines only; wrapping lands next.
pub(crate) fn wrap(label: &str) -> Vec<String> {
    label.split('\n').map(str::to_owned).collect()
}

/// Size `(w, h)` of a node holding `lines`, in half-pixels.
pub(crate) fn node_size(shape: Shape, lines: &[String]) -> (Half, Half) {
    let tw = lines.iter().map(|l| measure(l)).max().unwrap_or(0);
    let th = Half::try_from(lines.len()).unwrap_or(0) * LINE_H * 2;
    let (w, h) = (tw + 4 * PAD_X, th + 4 * PAD_Y);
    // Task 2 gives every shape its own formula; the tracer draws rectangles only.
    let _ = shape;
    (w.max(2 * 64), h.max(2 * 38))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_has_the_arimo_anchors() {
        assert_eq!(ADV.len(), 95);
        assert_eq!((ADV[0], ADV[b'A' as usize - 32], ADV[94]), (278, 667, 584));
        assert_eq!(ADV[b'm' as usize - 32], 833);
    }

    #[test]
    fn measure_ascii_and_empty() {
        assert_eq!(measure("A"), 21);
        assert_eq!(measure(""), 0);
    }

    #[test]
    fn wrap_splits_on_newlines() {
        assert_eq!(wrap("a\nb"), ["a", "b"]);
    }

    #[test]
    fn rect_has_minimum_and_grows() {
        let one = ["A".to_owned()];
        assert_eq!(node_size(Shape::Rect, &one), (128, 76));
        let wide = ["W".repeat(30)];
        let (w, h) = node_size(Shape::Rect, &wide);
        assert!(w > 128 && h == 76, "{w}x{h}");
    }
}
