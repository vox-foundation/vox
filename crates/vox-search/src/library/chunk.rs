//! Library chunker. Identity (title, heading text, heading_path, H1 rule) comes only from
//! [`crate::kb_ids`]; this module adds hashing, token estimates and the frontmatter blob.

use crate::kb_ids;

/// Recorded in `kb_index_state.chunker_version`; bump on any change that alters chunk bodies,
/// boundaries or hashes. `tok-ceil4` = the `ceil(bytes / 4)` sizer (same estimator as Phase 1's
/// answer-in-budget).
pub const CHUNKER_VERSION: &str = "sections-v1+tok-ceil4";

/// What a chunk is: one `Card` per doc (index 0), then the heading `Section`s.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkKind {
    Card,
    Section,
}

impl ChunkKind {
    /// Stored form in `search_document_chunks.chunk_kind`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Card => "card",
            Self::Section => "section",
        }
    }
}

/// One chunk, in document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Chunk {
    pub hash: String,
    pub kind: ChunkKind,
    pub heading_path: String,
    pub anchor: Option<String>,
    pub token_estimate: usize,
    pub body: String,
}

/// A chunked document.
#[derive(Debug, Clone)]
pub struct ParsedDoc {
    pub title: String,
    /// Frontmatter as a JSON object string (`"{}"` when absent or unparsable).
    pub frontmatter_json: String,
    pub chunks: Vec<Chunk>,
}

/// Deterministic token estimate: `ceil(bytes / 4)`.
#[must_use]
pub fn estimate_tokens(s: &str) -> usize {
    s.len().div_ceil(4)
}

/// The frontmatter block between the `---` fences as a JSON object; any parse failure gives `{}`.
fn frontmatter_value(text: &str) -> serde_json::Value {
    let empty = || serde_json::Value::Object(Default::default());
    let Some(range) = kb_ids::frontmatter_range(text) else {
        return empty();
    };
    let lines: Vec<&str> = text[range].lines().collect();
    let inner = lines[1..lines.len() - 1].join("\n");
    match serde_yaml::from_str::<serde_json::Value>(&inner) {
        Ok(v @ serde_json::Value::Object(_)) => v,
        _ => empty(),
    }
}

/// Slug of a heading as the docs site (Astro/Starlight, github-slugger) renders its `id`.
///
/// Lowercase; every character except letters, digits, `-`, `_` and space is dropped; each space
/// becomes `-` (no collapsing). `seen` is shared across the headings of one doc in document
/// order: a repeat gets `-1`, `-2`, ... appended.
// ponytail: combining marks (Unicode M*) are dropped and inline link/emphasis syntax is not
// stripped first; docs/src has neither in headings. Upgrade: run the heading through a Markdown
// inline-text pass.
pub fn github_slug(
    heading_text: &str,
    seen: &mut std::collections::BTreeMap<String, usize>,
) -> String {
    let base: String = heading_text
        .to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | ' '))
        .map(|c| if c == ' ' { '-' } else { c })
        .collect();
    let mut slug = base.clone();
    while seen.contains_key(&slug) {
        let n = seen.entry(base.clone()).or_insert(0);
        *n += 1;
        slug = format!("{base}-{n}");
    }
    seen.insert(slug.clone(), 0);
    slug
}

/// Longest first-paragraph excerpt kept in a card.
const CARD_PARAGRAPH_MAX_BYTES: usize = 1024;

/// First prose paragraph of the intro (preamble and H1 section, i.e. before any H2+): the
/// first run of non-blank lines that is not a heading line or fenced code.
fn first_paragraph(intro: &str) -> String {
    let mut in_fence = false;
    let mut para: Vec<&str> = Vec::new();
    for line in intro.lines() {
        if kb_ids::is_fence_line(line) {
            in_fence = !in_fence;
        } else if in_fence || kb_ids::parse_heading(line).is_some() || line.trim().is_empty() {
            // A blank line (or a heading/fence) ends a paragraph that has started.
        } else {
            para.push(line.trim_end());
            continue;
        }
        if !para.is_empty() {
            break;
        }
    }
    let mut p = para.join("\n");
    if p.len() > CARD_PARAGRAPH_MAX_BYTES {
        let mut cut = CARD_PARAGRAPH_MAX_BYTES;
        while !p.is_char_boundary(cut) {
            cut -= 1;
        }
        p.truncate(cut);
    }
    p
}

/// The doc's summary card body: title, `description`, first intro paragraph, then
/// `category:` / `status:` facets when present. Never any raw YAML.
fn card_body(title: &str, fm: &serde_json::Value, intro: &str) -> String {
    let field = |k: &str| {
        fm.get(k)
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    let mut parts = vec![title.to_string()];
    parts.extend(field("description").map(str::to_string));
    let p = first_paragraph(intro);
    if !p.is_empty() {
        parts.push(p);
    }
    parts.extend(field("category").map(|c| format!("category: {c}")));
    parts.extend(field("status").map(|c| format!("status: {c}")));
    parts.join("\n")
}

/// Split `text` (the doc at `rel_path`) into hash-keyed chunks: one card, then the sections.
///
/// hash = `hasher(rel_path 0x00 heading_path 0x00 trimmed_body)`; a later chunk with the same
/// (heading_path, body) gets `0x00 "#n"` appended to the hash input so hashes stay unique.
pub fn chunk_document(
    rel_path: &str,
    text: &str,
    hasher: &(dyn Fn(&[u8]) -> String + Sync),
) -> ParsedDoc {
    let text = vox_bounded_fs::normalize_text(text.to_string());
    let title = kb_ids::doc_title(rel_path, &text);
    let fm = frontmatter_value(&text);
    let sections = kb_ids::doc_sections(rel_path, &text);
    let mut seen: std::collections::BTreeMap<(String, String), usize> = Default::default();
    let mut slugs: std::collections::BTreeMap<String, usize> = Default::default();
    let mut chunks = Vec::new();
    let mut push = |kind: ChunkKind, heading_path: &str, anchor: Option<String>, body: &str| {
        let n = seen
            .entry((heading_path.to_string(), body.to_string()))
            .or_insert(0);
        let mut input = format!("{rel_path}\0{heading_path}\0{body}");
        if *n > 0 {
            input.push_str(&format!("\0#{n}"));
        }
        *n += 1;
        chunks.push(Chunk {
            hash: hasher(input.as_bytes()),
            kind,
            heading_path: heading_path.to_string(),
            anchor,
            token_estimate: estimate_tokens(body),
            body: body.to_string(),
        });
    };

    // AP-23: the card carries the root path, so `rel_path#Title` always resolves.
    let intro_end = sections
        .iter()
        .find(|s| s.level >= 2)
        .map_or(text.len(), |s| s.range.start);
    let intro_start = sections.first().map_or(0, |s| s.range.start);
    push(
        ChunkKind::Card,
        &title,
        None,
        &card_body(&title, &fm, &text[intro_start..intro_end]),
    );

    for sec in &sections {
        let body = text[sec.range.clone()].trim();
        // Every heading consumes a slug in document order, empty body or not.
        let anchor = (sec.level > 0)
            .then(|| kb_ids::parse_heading(body.lines().next().unwrap_or("")))
            .flatten()
            .map(|(_, h)| github_slug(&h, &mut slugs));
        if body.is_empty() {
            continue;
        }
        push(ChunkKind::Section, &sec.heading_path, anchor, body);
    }
    ParsedDoc {
        frontmatter_json: fm.to_string(),
        title,
        chunks,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn h(b: &[u8]) -> String {
        // Test hasher: length-tagged copy of the input; unique per input, no crypto needed here.
        format!(
            "{}:{}",
            b.len(),
            String::from_utf8_lossy(b).replace('\0', "|")
        )
    }

    #[test]
    fn estimate_tokens_is_ceil_div_4() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
    }

    #[test]
    fn chunk_kind_wire_strings() {
        assert_eq!(ChunkKind::Card.as_str(), "card");
        assert_eq!(ChunkKind::Section.as_str(), "section");
    }

    #[test]
    fn chunk_document_uses_kb_ids_paths_and_skips_blank_preamble() {
        let text = "---\ntitle: Guide\ntags: [a, b]\n---\n# Other\nintro\n## Install\nbody\n";
        let doc = chunk_document("docs/src/g.md", text, &h);
        assert_eq!(doc.title, "Guide");
        assert_eq!(
            doc.frontmatter_json,
            r#"{"title":"Guide","tags":["a","b"]}"#
        );
        let paths: Vec<&str> = doc.chunks.iter().map(|c| c.heading_path.as_str()).collect();
        // Card first (root path), then the H1 section (root path), then Install; the blank
        // preamble is skipped.
        assert_eq!(paths, ["Guide", "Guide", "Guide › Install"]);
        assert_eq!(doc.chunks[0].kind, ChunkKind::Card);
        assert_eq!(doc.chunks[1].body, "# Other\nintro");
    }

    #[test]
    fn duplicate_sections_get_distinct_hashes() {
        let text = "# T\n## Same\nx\n## Same\nx\n";
        let doc = chunk_document("docs/src/d.md", text, &h);
        let hashes: Vec<&str> = doc.chunks.iter().map(|c| c.hash.as_str()).collect();
        assert_eq!(hashes.len(), 4); // card, "# T", two "## Same"
        assert_ne!(hashes[2], hashes[3]);
    }

    #[test]
    fn github_slug_matches_the_site_ids() {
        let mut seen = Default::default();
        // Real headings from docs/src/architecture/search-retrieval-ssot-2026.md (as kb_ids
        // normalizes them: backticks removed).
        assert_eq!(
            github_slug(
                "3. Two distinct “VoxDB” surfaces (do not conflate)",
                &mut seen
            ),
            "3-two-distinct-voxdb-surfaces-do-not-conflate"
        );
        assert_eq!(
            github_slug("3.1 Rust vox-db (Turso / libSQL)", &mut seen),
            "31-rust-vox-db-turso--libsql"
        );
        assert_eq!(
            github_slug("Language surface (decision)", &mut seen),
            "language-surface-decision"
        );
        assert_eq!(
            github_slug("snake_case & more", &mut seen),
            "snake_case--more"
        );
        assert_eq!(github_slug("Usage", &mut seen), "usage");
        assert_eq!(github_slug("Usage", &mut seen), "usage-1");
        assert_eq!(github_slug("Usage", &mut seen), "usage-2");
    }

    #[test]
    fn card_body_uses_facets_and_first_intro_paragraph_only() {
        let fm =
            serde_json::json!({"description": " Does X. ", "category": "Guides", "status": ""});
        let intro = "# T\n\n```\nfenced\n```\nReal first\nparagraph.\n\nSecond.\n";
        assert_eq!(
            card_body("T", &fm, intro),
            "T\nDoes X.\nReal first\nparagraph.\ncategory: Guides"
        );
        assert_eq!(card_body("T", &serde_json::json!({}), ""), "T");
        let long = "é".repeat(CARD_PARAGRAPH_MAX_BYTES);
        assert!(first_paragraph(&long).len() <= CARD_PARAGRAPH_MAX_BYTES);
    }

    #[test]
    fn bad_frontmatter_yields_empty_object() {
        let doc = chunk_document("docs/src/b.md", "---\n: : [\n---\n# T\n", &h);
        assert_eq!(doc.frontmatter_json, "{}");
    }
}
