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
        // Zero-width marks take no room; wide chars are two cells of 500 and every other
        // char, controls included (they print as U+FFFD), is 1000.
        _ if c.width() == Some(0) => 0,
        _ => 1000,
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

/// Minimum total horizontal padding of a text box, in percent of the measured width. `measure`
/// already carries the 8% slack, so 1.12 x measured = 1.21 x the Arimo table: a font up to ~21%
/// wider (Verdana measured 1.186) still fits.
pub(crate) const WIDE_FONT_PCT: Half = 12;

/// Padding per side of a text box `tw` wide: `base`, or half of [`WIDE_FONT_PCT`] of `tw` rounded
/// up when that is more. The one place the formula lives; every padded box goes through it.
pub(crate) fn pad_x(tw: Half, base: Half) -> Half {
    let tw = tw.max(0);
    base.max(tw.saturating_mul(WIDE_FONT_PCT).saturating_add(199) / 200)
}

/// Cells per wrapped line.
pub(crate) const WRAP_CELLS: usize = 28;

/// Longest word kept whole, in cells. A word of narrow characters between [`WRAP_CELLS`] and this
/// takes a line of its own and widens its box (an identifier is never split mid-word); only a
/// longer one is broken. A word holding a wide character (CJK and other scripts written without
/// spaces) is a run of text, not an identifier, and breaks at the line width.
pub(crate) const WORD_CELLS: usize = 48;

/// Terminal cells of `s`: wide chars 2, zero-width 0, everything else (controls included, which
/// print as U+FFFD) 1.
pub(crate) fn cells(s: &str) -> usize {
    s.chars().map(cell).sum()
}

fn cell(c: char) -> usize {
    c.width().unwrap_or(1)
}

fn is_separator(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\r')
}

/// Break an over-long word into chunks of at most `limit` cells; a zero-width char never starts
/// a chunk, so it stays with the char before it.
fn hard_break(word: &str, limit: usize) -> Vec<String> {
    let mut chunks = vec![String::new()];
    let mut used = 0;
    for c in word.chars() {
        let w = cell(c);
        if w > 0 && used + w > limit {
            chunks.push(String::new());
            used = 0;
        }
        if let Some(last) = chunks.last_mut() {
            last.push(c);
        }
        used += w;
    }
    chunks
}

/// Wrap a label: explicit newlines are kept, words (split on space, tab and CR only) are filled
/// greedily up to [`WRAP_CELLS`]; a narrow word longer than a line keeps a line of its own and is
/// broken mid-word only past [`WORD_CELLS`], any other word past a line. Runs of separators
/// collapse, lines are trimmed and blank lines are dropped.
pub(crate) fn wrap(label: &str) -> Vec<String> {
    wrap_at(label, WRAP_CELLS)
}

/// [`wrap`] with a line limit of `limit` cells (at least 1) instead of [`WRAP_CELLS`].
pub(crate) fn wrap_at(label: &str, limit: usize) -> Vec<String> {
    let limit = limit.max(1);
    let mut out = Vec::new();
    for line in label.split('\n') {
        let mut cur = String::new();
        for word in line.split(is_separator).filter(|w| !w.is_empty()) {
            let wc = cells(word);
            let word_cap = if word.chars().all(|c| cell(c) <= 1) {
                limit.max(WORD_CELLS)
            } else {
                limit
            };
            if wc > word_cap {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
                let mut chunks = hard_break(word, word_cap);
                cur = chunks.pop().unwrap_or_default();
                out.extend(chunks);
            } else if cur.is_empty() {
                cur = word.to_owned();
            } else if cells(&cur) + 1 + wc <= limit {
                cur.push(' ');
                cur.push_str(word);
            } else {
                out.push(std::mem::replace(&mut cur, word.to_owned()));
            }
        }
        if !cur.is_empty() {
            out.push(cur);
        }
    }
    out
}

/// Size `(w, h)` of a node holding `lines`, in half-pixels. The text block is the widest line by
/// `LINE_H` per line; a diamond contains every corner of it.
pub(crate) fn node_size(shape: Shape, lines: &[String]) -> (Half, Half) {
    let tw = lines.iter().map(|l| measure(l)).max().unwrap_or(0);
    let th = Half::try_from(lines.len()).unwrap_or(0) * LINE_H * 2;
    let (px, py) = (pad_x(tw, PAD_X * 2), PAD_Y * 2);
    // (w, h, min w px, min h px)
    let (w, h, min_w, min_h) = match shape {
        Shape::Rect | Shape::Rounded => (tw + 2 * px, th + 2 * py, 64, 38),
        Shape::Diamond => (2 * (tw + px), 2 * (th + py), 96, 64),
        Shape::Cylinder => (tw + 2 * px, th + 36 * 2, 64, 54),
        Shape::Start | Shape::End => (tw + 40 * 2, th + 2 * py, 72, 38),
    };
    (w.max(min_w * 2), h.max(min_h * 2))
}

#[cfg(test)]
mod tests;
