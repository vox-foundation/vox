//! The one line-exact `---` frontmatter splitter shared by lint, indexer and MENS.

use std::ops::Range;

/// `(start, end, line)` for each line of `text[from..]`: `start`/`end` are byte offsets into
/// `text` (`end` includes the terminator); `line` has its `\n` / `\r\n` terminator removed
/// (the same lines `str::lines` yields).
pub(crate) fn line_spans(text: &str, from: usize) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut off = from;
    text[from..].split_inclusive('\n').map(move |raw| {
        let start = off;
        off += raw.len();
        let l = raw.strip_suffix('\n').unwrap_or(raw);
        (start, off, l.strip_suffix('\r').unwrap_or(l))
    })
}

/// Byte range of the leading `---` frontmatter block, from offset 0 through the end of the
/// closing `---` line (terminator included). `None` when the text does not start with `---` or
/// the block is never closed (an unterminated block is body, not frontmatter).
// vox:defactored-from vox-search 2026-10-10
#[must_use]
pub fn frontmatter_range(text: &str) -> Option<Range<usize>> {
    let mut lines = line_spans(text, 0);
    let (_, _, first) = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    lines
        .find(|(_, _, l)| l.trim_end() == "---")
        .map(|(_, end, _)| 0..end)
}

/// The YAML between the opening and closing fence lines (fences excluded), if the block is closed.
#[must_use]
pub fn frontmatter_yaml(text: &str) -> Option<&str> {
    let mut lines = line_spans(text, 0);
    let (_, open_end, first) = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    lines
        .find(|(_, _, l)| l.trim_end() == "---")
        .map(|(close_start, _, _)| &text[open_end..close_start])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_range_covers_the_closed_block_only() {
        let t = "---\ntitle: A\n---\n# A\n";
        assert_eq!(frontmatter_range(t), Some(0..17));
        assert_eq!(&t[17..], "# A\n");
        assert_eq!(frontmatter_range("# A\n---\n"), None);
        assert_eq!(frontmatter_range("---\ntitle: A\n# A\n"), None);
        assert_eq!(frontmatter_range(""), None);
    }

    #[test]
    fn frontmatter_range_handles_crlf() {
        let t = "---\r\ntitle: A\r\n---\r\nbody\r\n";
        let r = frontmatter_range(t).unwrap();
        assert_eq!(&t[r.end..], "body\r\n");
        assert_eq!(frontmatter_yaml(t), Some("title: A\r\n"));
    }

    #[test]
    fn frontmatter_ignores_dashes_inside_a_quoted_value() {
        let t = "---\ntitle: \"a --- b\"\n---\nbody\n";
        assert_eq!(frontmatter_yaml(t), Some("title: \"a --- b\"\n"));
        assert_eq!(&t[frontmatter_range(t).unwrap().end..], "body\n");
    }

    #[test]
    fn frontmatter_yaml_none_when_unterminated_or_absent() {
        assert_eq!(frontmatter_yaml("---\ntitle: A\n"), None);
        assert_eq!(frontmatter_yaml("# A\n"), None);
        assert_eq!(frontmatter_yaml("---\n---\n"), Some(""));
    }
}
