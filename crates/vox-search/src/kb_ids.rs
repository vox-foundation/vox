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

/// Body lines of `text` after a leading `---` frontmatter block (all lines if none).
fn body_lines(text: &str) -> (Option<&str>, Vec<&str>) {
    let mut lines = text.lines();
    let mut all: Vec<&str> = Vec::new();
    let mut title: Option<&str> = None;
    match lines.next() {
        Some(first) if first.trim_end() == "---" => {
            let mut closed = false;
            let mut front: Vec<&str> = Vec::new();
            for l in lines.by_ref() {
                if l.trim_end() == "---" {
                    closed = true;
                    break;
                }
                front.push(l);
            }
            if closed {
                title = front.iter().find_map(|l| l.strip_prefix("title:"));
            } else {
                // Unterminated block: not frontmatter; treat everything as body.
                all.push(first);
                all.extend(front);
            }
        }
        Some(first) => all.push(first),
        None => {}
    }
    all.extend(lines);
    (title, all)
}

/// Document title: frontmatter `title:`, else the first H1 (outside fences), else the file stem.
#[must_use]
pub fn doc_title(doc_path: &str, text: &str) -> String {
    let (fm_title, lines) = body_lines(text);
    if let Some(t) = fm_title {
        let t = t.trim().trim_matches(|c| c == '"' || c == '\'');
        let t = normalize_heading_text(t);
        if !t.is_empty() {
            return t;
        }
    }
    let mut in_fence = false;
    for line in lines {
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

/// Repo-relative, `/`-separated `.md` path under a KB root with no excluded component.
#[must_use]
pub fn is_kb_corpus_path(rel: &str) -> bool {
    if rel.is_empty() || rel.starts_with('/') || rel.contains(['\\', ':']) || !rel.ends_with(".md")
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

/// Every heading_path in a doc (root = title included). Frontmatter and fenced code are skipped.
#[must_use]
pub fn doc_heading_paths(doc_path: &str, text: &str) -> BTreeSet<String> {
    let title = doc_title(doc_path, text);
    let (_, lines) = body_lines(text);
    let mut out = BTreeSet::new();
    out.insert(title.clone());
    // Stack of (level, text) for levels >= 2.
    let mut stack: Vec<(usize, String)> = Vec::new();
    let mut in_fence = false;
    for line in lines {
        if is_fence_line(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        let Some((level, text)) = parse_heading(line) else {
            continue;
        };
        if level == 1 {
            stack.clear();
            continue;
        }
        while stack.last().is_some_and(|(l, _)| *l >= level) {
            stack.pop();
        }
        stack.push((level, text));
        let mut path = title.clone();
        for (_, t) in &stack {
            path.push_str(HEADING_PATH_SEP);
            path.push_str(t);
        }
        out.insert(path);
    }
    out
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
}
