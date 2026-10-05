//! KB identity SSOT: corpus roots, `heading_path` encoding, chunk ids, ancestor matching.
//!
//! Shared between the Phase 1 retrieval-eval engine ([`crate::kb_eval`]) and the Phase 2
//! library-index chunker, which MUST reuse these definitions rather than re-derive them.
//!
//! - **Doc id** = repo-relative path with `/` separators (e.g. `docs/src/a.md`).
//! - **heading_path** = `title › H2 › H3`: normalized heading display texts joined by
//!   [`HEADING_PATH_SEP`]. H1 is never a segment; the root is always the doc title
//!   (frontmatter `title`, else the first H1, else the file stem).
//! - **Chunk id** = `doc_path#heading_path` (split at the FIRST `#`).
//!
//! Only ordered std collections are used here (determinism: no hash-based iteration).

use std::collections::BTreeSet;
use std::ops::Range;

/// Repo-relative directories whose Markdown forms the KB corpus.
pub const KB_CORPUS_ROOTS: &[&str] = &["docs/src"];

/// Directory names that are never part of the KB corpus, at any depth.
pub const KB_EXCLUDED_DIR_NAMES: &[&str] = &["archive"];

/// Separator between `heading_path` segments: U+203A with one space on each side.
pub const HEADING_PATH_SEP: &str = " › ";

/// Strip ATX markers and inline backticks from heading text and collapse whitespace.
///
/// `"##  The `vox` CLI ##"` becomes `"The vox CLI"`. A closing `#` run is stripped only
/// when separated from the text by whitespace, so `"C#"` survives.
#[must_use]
pub fn normalize_heading_text(raw: &str) -> String {
    let s = raw.trim().trim_start_matches('#');
    let s = s.trim_end();
    let without_hashes = s.trim_end_matches('#');
    let s = if without_hashes.is_empty() || without_hashes.ends_with(char::is_whitespace) {
        without_hashes
    } else {
        s
    };
    s.replace('`', "")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// True for a fenced-code delimiter line (starts with three backticks or three tildes).
#[must_use]
pub fn is_fence_line(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

/// ATX heading at line start: `(level 1..=6, normalized text)`.
#[must_use]
pub fn parse_heading(line: &str) -> Option<(usize, String)> {
    let level = line.bytes().take_while(|&b| b == b'#').count();
    if !(1..=6).contains(&level) || !line[level..].starts_with(' ') {
        return None;
    }
    Some((level, normalize_heading_text(line)))
}

/// `(start, end, line)` for each line of `text[from..]`: `start`/`end` are byte offsets into
/// `text` (`end` includes the terminator); `line` has its `\n` / `\r\n` terminator removed
/// (the same lines `str::lines` yields).
fn line_spans(text: &str, from: usize) -> impl Iterator<Item = (usize, usize, &str)> {
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

/// Start of the body: just past the frontmatter block, else 0.
fn body_start(text: &str) -> usize {
    frontmatter_range(text).map_or(0, |r| r.end)
}

/// Document title: frontmatter `title:`, else the first H1 (outside fences), else the file stem.
#[must_use]
pub fn doc_title(doc_path: &str, text: &str) -> String {
    let fm_title = frontmatter_range(text).and_then(|fm| {
        line_spans(text, 0)
            .skip(1)
            .take_while(|(_, end, _)| *end < fm.end)
            .find_map(|(_, _, l)| l.strip_prefix("title:"))
    });
    if let Some(t) = fm_title {
        let t = t.trim().trim_matches(|c| c == '"' || c == '\'');
        let t = normalize_heading_text(t);
        if !t.is_empty() {
            return t;
        }
    }
    let mut in_fence = false;
    for (_, _, line) in line_spans(text, body_start(text)) {
        if is_fence_line(line) {
            in_fence = !in_fence;
        } else if !in_fence
            && let Some((1, t)) = parse_heading(line)
            && !t.is_empty()
        {
            return t;
        }
    }
    let name = doc_path.rsplit('/').next().unwrap_or(doc_path);
    name.strip_suffix(".md").unwrap_or(name).to_string()
}

/// `doc_path#heading_path`.
#[must_use]
pub fn chunk_id(doc_path: &str, heading_path: &str) -> String {
    format!("{doc_path}#{heading_path}")
}

/// Split a chunk id at the FIRST `#` into `(doc_path, heading_path)`.
#[must_use]
pub fn split_chunk_id(id: &str) -> Option<(&str, &str)> {
    id.split_once('#')
}

/// True when `ancestor` equals `path` or is a segment-wise prefix of it.
#[must_use]
pub fn heading_path_is_ancestor_or_equal(ancestor: &str, path: &str) -> bool {
    let mut a = ancestor.split(HEADING_PATH_SEP);
    let mut p = path.split(HEADING_PATH_SEP);
    loop {
        match (a.next(), p.next()) {
            (None, _) => return true,
            (Some(x), Some(y)) if x == y => {}
            _ => return false,
        }
    }
}

/// Repo-relative, `/`-separated `.md` path under a KB root with no excluded component and no
/// `#` (chunk ids split at the first `#`, so such a doc could never match its qrels).
#[must_use]
pub fn is_kb_corpus_path(rel: &str) -> bool {
    if rel.is_empty()
        || rel.starts_with('/')
        || rel.contains(['\\', ':', '#'])
        || !rel.ends_with(".md")
    {
        return false;
    }
    let comps: Vec<&str> = rel.split('/').collect();
    if comps
        .iter()
        .any(|c| c.is_empty() || *c == "." || *c == "..")
    {
        return false;
    }
    if comps.iter().any(|c| KB_EXCLUDED_DIR_NAMES.contains(c)) {
        return false;
    }
    KB_CORPUS_ROOTS
        .iter()
        .any(|root| rel.starts_with(&format!("{root}/")))
}

/// One heading section of a doc: the unit the Library chunker stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KbSection {
    /// `title › H2 › H3`; the root (title) for the preamble and for every H1 section.
    pub heading_path: String,
    /// `0` = preamble (text before the first heading), `1..=6` = heading level.
    pub level: u8,
    /// Byte range into the text: from the heading line start (the body start for the preamble)
    /// to the next heading line start, or the end of the text.
    pub range: Range<usize>,
}

/// Every section of a doc, in document order, including the (possibly empty) preamble.
///
/// This is the single heading walk: [`doc_heading_paths`] projects from it, so the chunker and
/// the Phase 1 qrels can never disagree about a `heading_path`. Frontmatter and fenced code are
/// skipped; an H1 resets the stack to the root (it is never a path segment).
#[must_use]
pub fn doc_sections(doc_path: &str, text: &str) -> Vec<KbSection> {
    let title = doc_title(doc_path, text);
    let start = body_start(text);
    let mut out = vec![KbSection {
        heading_path: title.clone(),
        level: 0,
        range: start..text.len(),
    }];
    // Stack of (level, text) for levels >= 2.
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut in_fence = false;
    for (line_start, _, line) in line_spans(text, start) {
        if is_fence_line(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some((level, heading)) = parse_heading(line) else {
            continue;
        };
        if level == 1 {
            stack.clear();
        } else {
            while stack.last().is_some_and(|(l, _)| *l >= level) {
                stack.pop();
            }
            stack.push((level, heading));
        }
        let mut path = title.clone();
        for (_, t) in &stack {
            path.push_str(HEADING_PATH_SEP);
            path.push_str(t);
        }
        if let Some(prev) = out.last_mut() {
            prev.range.end = line_start;
        }
        out.push(KbSection {
            heading_path: path,
            level: level as u8,
            range: line_start..text.len(),
        });
    }
    out
}

/// Every heading_path in a doc (root = title included). Frontmatter and fenced code are skipped.
#[must_use]
pub fn doc_heading_paths(doc_path: &str, text: &str) -> BTreeSet<String> {
    doc_sections(doc_path, text)
        .into_iter()
        .map(|s| s.heading_path)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_markers_backticks_and_whitespace() {
        assert_eq!(
            normalize_heading_text("##  The `vox` CLI ##"),
            "The vox CLI"
        );
        assert_eq!(normalize_heading_text("## C#"), "C#");
    }

    #[test]
    fn doc_title_precedence() {
        let fm = "---\ntitle: \"Alpha Guide\"\n---\n# Other\n";
        assert_eq!(doc_title("docs/src/a.md", fm), "Alpha Guide");
        assert_eq!(
            doc_title("docs/src/a.md", "intro\n# Beta Notes\n"),
            "Beta Notes"
        );
        assert_eq!(doc_title("docs/src/gamma.md", "no heading\n"), "gamma");
        // An H1 inside a fence is not a title.
        assert_eq!(doc_title("docs/src/x.md", "```\n# fake\n```\n"), "x");
    }

    #[test]
    fn chunk_id_round_trip_splits_at_first_hash() {
        let id = chunk_id("docs/src/a.md", "A › B");
        assert_eq!(id, "docs/src/a.md#A › B");
        assert_eq!(split_chunk_id(&id), Some(("docs/src/a.md", "A › B")));
        assert_eq!(
            split_chunk_id("docs/src/a.md#C#"),
            Some(("docs/src/a.md", "C#"))
        );
        assert_eq!(split_chunk_id("nohash"), None);
    }

    #[test]
    fn ancestor_or_equal_is_segment_wise() {
        assert!(heading_path_is_ancestor_or_equal("A › B", "A › B › C"));
        assert!(heading_path_is_ancestor_or_equal("A › B", "A › B"));
        assert!(!heading_path_is_ancestor_or_equal("A › B", "A › Bx"));
        assert!(!heading_path_is_ancestor_or_equal("A › B › C", "A › B"));
    }

    #[test]
    fn corpus_path_rules() {
        assert!(is_kb_corpus_path("docs/src/a/b.md"));
        assert!(!is_kb_corpus_path("docs/src/archive/b.md"));
        assert!(!is_kb_corpus_path("docs/src/../x.md"));
        assert!(!is_kb_corpus_path("/docs/src/a.md"));
        assert!(!is_kb_corpus_path("docs/src/a.txt"));
        assert!(!is_kb_corpus_path("docs/other/a.md"));
        assert!(!is_kb_corpus_path("docs\\src\\a.md"));
        // Chunk ids split at the first '#', so a doc path may not contain one.
        assert!(!is_kb_corpus_path("docs/src/c#-notes.md"));
    }

    #[test]
    fn heading_paths_nest_skip_fences_and_frontmatter() {
        let text = "---\ntitle: Alpha Guide\n---\n## Install\n```\n## Fake\n```\n## Usage\n### Flags\n#### Deep\n## Last\n";
        let got: Vec<String> = doc_heading_paths("docs/src/alpha.md", text)
            .into_iter()
            .collect();
        let want = [
            "Alpha Guide",
            "Alpha Guide › Install",
            "Alpha Guide › Last",
            "Alpha Guide › Usage",
            "Alpha Guide › Usage › Flags",
            "Alpha Guide › Usage › Flags › Deep",
        ];
        assert_eq!(got, want);
    }

    #[test]
    fn h1_resets_stack_and_is_not_a_segment() {
        let got = doc_heading_paths("docs/src/b.md", "# Beta\n## One\n# Again\n## Two\n");
        assert!(got.contains("Beta › One") && got.contains("Beta › Two"));
        assert!(!got.iter().any(|p| p.contains("Again")));
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
    fn kb_sections_project_heading_paths() {
        let texts = [
            "",
            "plain text only\n",
            "---\ntitle: Guide\n---\n# Guide\nintro\n## Install\nx\n```\n## Fake\n```\n## Usage\n### Flags\n# Again\n## Two\n",
            "---\ntitle: Alpha Guide\n---\n## Install\n```\n## Fake\n```\n## Usage\n### Flags\n#### Deep\n## Last\n",
            "no newline at end\n## Tail",
            "crlf\r\n# T\r\n## S\r\nbody\r\n",
        ];
        for t in texts {
            let secs = doc_sections("docs/src/x.md", t);
            let mut want: BTreeSet<String> = secs.iter().map(|s| s.heading_path.clone()).collect();
            want.insert(doc_title("docs/src/x.md", t));
            assert_eq!(doc_heading_paths("docs/src/x.md", t), want, "{t:?}");
            // Ranges are contiguous, in order, and cover the whole body.
            let body = body_start(t);
            assert_eq!(secs[0].range.start, body, "{t:?}");
            for w in secs.windows(2) {
                assert_eq!(w[0].range.end, w[1].range.start, "{t:?}");
            }
            assert_eq!(secs.last().unwrap().range.end, t.len(), "{t:?}");
        }
    }

    #[test]
    fn sections_split_at_headings_and_h1_is_root() {
        let t = "---\ntitle: Guide\n---\n# Other Name\nintro\n## Install\nbody\n";
        let secs = doc_sections("docs/src/g.md", t);
        let got: Vec<(&str, u8, &str)> = secs
            .iter()
            .map(|s| (s.heading_path.as_str(), s.level, &t[s.range.clone()]))
            .collect();
        assert_eq!(
            got,
            [
                ("Guide", 0, ""),
                ("Guide", 1, "# Other Name\nintro\n"),
                ("Guide › Install", 2, "## Install\nbody\n"),
            ]
        );
    }
}
