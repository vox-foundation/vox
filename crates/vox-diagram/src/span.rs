//! Byte spans, 1-based line/col locations and the line index that maps one to the other.

use serde::{Deserialize, Serialize};

/// Half-open byte range `start..end` into the original source.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    /// First byte of the range.
    pub start: u32,
    /// One past the last byte of the range.
    pub end: u32,
}

/// A [`Span`] plus 1-based line/col (col counted in Unicode scalar values).
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Location {
    /// First byte of the range.
    pub start: u32,
    /// One past the last byte of the range.
    pub end: u32,
    /// 1-based line of `start`.
    pub line: u32,
    /// 1-based column of `start`.
    pub col: u32,
    /// 1-based line of `end`.
    pub end_line: u32,
    /// 1-based column of `end`.
    pub end_col: u32,
}

/// Saturating `usize` -> `u32` (sources are capped far below `u32::MAX` before lexing).
pub(crate) fn to_u32(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// Sorted line-start offsets with binary-search lookup. CRLF is one line break; offsets stay
/// in original bytes.
pub(crate) struct LineIndex {
    starts: Vec<u32>,
}

impl LineIndex {
    pub(crate) fn new(src: &str) -> Self {
        let mut starts = vec![0u32];
        starts.extend(src.match_indices('\n').map(|(i, _)| to_u32(i + 1)));
        Self { starts }
    }

    /// 1-based (line, col) for a byte offset; col counts scalar values from the line start.
    fn locate(&self, src: &str, off: u32) -> (u32, u32) {
        let line = self.starts.partition_point(|&s| s <= off).max(1) - 1;
        let from = self.starts.get(line).copied().unwrap_or(0) as usize;
        let col = src.get(from..off as usize).map_or(0, |s| s.chars().count());
        (to_u32(line + 1), to_u32(col + 1))
    }

    pub(crate) fn location(&self, src: &str, span: Span) -> Location {
        let (line, col) = self.locate(src, span.start);
        let (end_line, end_col) = self.locate(src, span.end);
        Location {
            start: span.start,
            end: span.end,
            line,
            col,
            end_line,
            end_col,
        }
    }
}

/// Panic-free slice of `src` for `span` (`None` when out of range or off a char boundary).
// Consumed by the formatter, fixes and fence work in plans 02-06.
#[allow(dead_code)]
pub(crate) fn slice(src: &str, span: Span) -> Option<&str> {
    src.get(span.start as usize..span.end as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(src: &str, start: u32, end: u32) -> Location {
        LineIndex::new(src).location(src, Span { start, end })
    }

    #[test]
    fn counts_columns_in_scalar_values() {
        // "é" is 2 bytes; the `x` after it starts at byte 3 but is column 3.
        let loc = at("aéx", 3, 4);
        assert_eq!((loc.line, loc.col, loc.end_col), (1, 3, 4));
    }

    #[test]
    fn crlf_is_one_line_break() {
        let src = "a\r\nb\r\nc";
        assert_eq!((at(src, 3, 4).line, at(src, 3, 4).col), (2, 1));
        assert_eq!((at(src, 6, 7).line, at(src, 6, 7).col), (3, 1));
    }

    #[test]
    fn offset_at_eof_is_valid() {
        let src = "ab\ncd";
        let loc = at(src, 5, 5);
        assert_eq!((loc.line, loc.col, loc.end_line, loc.end_col), (2, 3, 2, 3));
    }

    #[test]
    fn slice_refuses_non_boundaries_and_overruns() {
        assert_eq!(slice("aéx", Span { start: 0, end: 1 }), Some("a"));
        assert_eq!(slice("aéx", Span { start: 2, end: 3 }), None);
        assert_eq!(slice("ab", Span { start: 1, end: 9 }), None);
    }
}
