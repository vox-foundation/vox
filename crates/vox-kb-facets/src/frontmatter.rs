//! The one line-exact `---` frontmatter splitter shared by lint, indexer and MENS.

use std::ops::Range;

/// `(start, end, line)` for each line of `text[from..]`: `start`/`end` are byte offsets into
/// `text` (`end` includes the terminator); `line` has its `\n`, `\r\n` or lone `\r` terminator
/// removed. A lone `\r` ends a line so a CR-only (classic Mac) doc still splits.
pub(crate) fn line_spans(text: &str, from: usize) -> impl Iterator<Item = (usize, usize, &str)> {
    let mut off = from;
    std::iter::from_fn(move || {
        let rest = &text[off..];
        if rest.is_empty() {
            return None;
        }
        let (len, term) = match rest.find(['\n', '\r']) {
            None => (rest.len(), 0),
            Some(i) if rest[i..].starts_with("\r\n") => (i, 2),
            Some(i) => (i, 1),
        };
        let start = off;
        off += len + term;
        Some((start, off, &rest[..len]))
    })
}

/// Byte length of a leading UTF-8 BOM (`0` when absent).
fn bom_len(text: &str) -> usize {
    if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    }
}

/// `(open fence end, close fence start, close fence end)` of the leading closed `---` block.
/// The opening fence must be the first line, after an optional UTF-8 BOM.
fn fence_offsets(text: &str) -> Option<(usize, usize, usize)> {
    let mut lines = line_spans(text, bom_len(text));
    let (_, open_end, first) = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    lines
        .find(|(_, _, l)| l.trim_end() == "---")
        .map(|(close_start, close_end, _)| (open_end, close_start, close_end))
}

/// Byte range of the leading `---` frontmatter block, from offset 0 through the end of the
/// closing `---` line (terminator included). `None` when the text does not start with `---`
/// (after an optional BOM) or the block is never closed.
// vox:defactored-from vox-search 2026-10-10
#[must_use]
pub fn frontmatter_range(text: &str) -> Option<Range<usize>> {
    fence_offsets(text).map(|(_, _, end)| 0..end)
}

/// The YAML between the opening and closing fence lines (fences excluded), if the block is closed.
#[must_use]
pub fn frontmatter_yaml(text: &str) -> Option<&str> {
    fence_offsets(text).map(|(open_end, close_start, _)| &text[open_end..close_start])
}

/// True when the first non-blank line (BOM, blank lines and indentation tolerated) is a bare
/// `---` fence: the author meant to write frontmatter, whether or not the splitter recognises it.
pub(crate) fn fence_intended(text: &str) -> bool {
    line_spans(text, bom_len(text))
        .map(|(_, _, l)| l.trim())
        .find(|l| !l.is_empty())
        .is_some_and(|l| l == "---")
}

/// Fenced-code tracker (CommonMark): a fence opens with 3+ backticks or tildes with up to 3
/// leading spaces and closes only on a line of the same character, at least as long, with up to
/// 3 leading spaces and nothing after it.
#[derive(Default)]
pub(crate) struct FenceTracker {
    open: Option<(char, usize)>,
}

impl FenceTracker {
    /// Feed the next line; true when it is a fence line or lies inside a fence.
    pub(crate) fn step(&mut self, line: &str) -> bool {
        let leading_spaces = line.bytes().take_while(|&b| b == b' ').count();
        let trimmed = if leading_spaces <= 3 {
            &line[leading_spaces..]
        } else {
            line
        };
        let run = |c: char| trimmed.chars().take_while(|&x| x == c).count();
        if let Some((c, n)) = self.open {
            if leading_spaces <= 3 {
                let m = run(c);
                if m >= n && trimmed[m * c.len_utf8()..].trim().is_empty() {
                    self.open = None;
                }
            }
            return true;
        }
        if leading_spaces <= 3 {
            for c in ['`', '~'] {
                let n = run(c);
                // a backtick fence's info string cannot hold a backtick (that is inline code)
                if n >= 3 && !(c == '`' && trimmed[n..].contains('`')) {
                    self.open = Some((c, n));
                    return true;
                }
            }
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_tracker_follows_commonmark_open_and_close_rules() {
        let run = |lines: &[&str]| {
            let mut f = FenceTracker::default();
            lines.iter().map(|l| f.step(l)).collect::<Vec<_>>()
        };
        // a shorter or different-char fence inside does not close
        assert_eq!(
            run(&["````", "```", "~~~", "x", "````", "y"]),
            [true, true, true, true, true, false]
        );
        assert_eq!(
            run(&["```", "~~~", "x", "```", "y"]),
            [true, true, true, true, false]
        );
        // an info string on the would-be closer keeps the fence open
        assert_eq!(
            run(&["```", "```rust", "x", "```", "y"]),
            [true, true, true, true, false]
        );
        // a longer closer closes; inline code is not a fence
        assert_eq!(run(&["```", "x", "`````", "y"]), [true, true, true, false]);
        assert_eq!(run(&["```inline``` text", "y"]), [false, false]);
        assert_eq!(run(&["~~~ js", "x", "~~~", "y"]), [true, true, true, false]);
    }

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
    fn frontmatter_tolerates_a_bom_and_cr_only_line_endings() {
        let bom = "\u{feff}---\ntitle: A\n---\nbody\n";
        assert_eq!(frontmatter_yaml(bom), Some("title: A\n"));
        assert_eq!(&bom[frontmatter_range(bom).unwrap().end..], "body\n");
        let cr = "---\rtitle: A\r---\rbody\r";
        assert_eq!(frontmatter_yaml(cr), Some("title: A\r"));
        assert_eq!(&cr[frontmatter_range(cr).unwrap().end..], "body\r");
    }

    #[test]
    fn fence_intended_sees_through_bom_blanks_and_indent() {
        for t in [
            "---\nx",
            "\u{feff}---\nx",
            "\n\n---\nx",
            "  ---  \nx",
            "---\rx",
        ] {
            assert!(fence_intended(t), "{t:?}");
        }
        for t in ["", "# T\n---\n", "----\nx", "--- x\n"] {
            assert!(!fence_intended(t), "{t:?}");
        }
    }

    #[test]
    fn frontmatter_yaml_none_when_unterminated_or_absent() {
        assert_eq!(frontmatter_yaml("---\ntitle: A\n"), None);
        assert_eq!(frontmatter_yaml("# A\n"), None);
        assert_eq!(frontmatter_yaml("---\n---\n"), Some(""));
    }
}
