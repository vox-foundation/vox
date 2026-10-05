//! Library chunker. Identity (title, heading text, heading_path, H1 rule) comes only from
//! [`crate::kb_ids`]; this module adds hashing, token estimates and the frontmatter blob.

use crate::kb_ids;

/// Recorded in `kb_index_state.chunker_version`; bump on any change that alters chunk bodies,
/// boundaries or hashes. `tok-ceil4` = the `ceil(bytes / 4)` sizer (same estimator as Phase 1's
/// answer-in-budget).
pub const CHUNKER_VERSION: &str = "sections-v1+tok-ceil4";

/// What a chunk is. The tracer emits only `Section`; `Card` arrives with 02-03.
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

/// The frontmatter block between the `---` fences, lenient: any parse failure gives `"{}"`.
fn frontmatter_json(text: &str) -> String {
    let Some(range) = kb_ids::frontmatter_range(text) else {
        return "{}".into();
    };
    let lines: Vec<&str> = text[range].lines().collect();
    let inner = lines[1..lines.len() - 1].join("\n");
    match serde_yaml::from_str::<serde_json::Value>(&inner) {
        Ok(v @ serde_json::Value::Object(_)) => v.to_string(),
        _ => "{}".into(),
    }
}

/// Split `text` (the doc at `rel_path`) into hash-keyed section chunks.
///
/// hash = `hasher(rel_path 0x00 heading_path 0x00 trimmed_body)`; a second chunk with the same
/// (heading_path, body) gets `0x00 "#n"` appended to the hash input so hashes stay unique.
pub fn chunk_document(
    rel_path: &str,
    text: &str,
    hasher: &(dyn Fn(&[u8]) -> String + Sync),
) -> ParsedDoc {
    let text = vox_bounded_fs::normalize_text(text.to_string());
    let title = kb_ids::doc_title(rel_path, &text);
    let mut seen: std::collections::BTreeMap<(String, String), usize> = Default::default();
    let mut chunks = Vec::new();
    for sec in kb_ids::doc_sections(rel_path, &text) {
        let body = text[sec.range.clone()].trim();
        if body.is_empty() {
            continue;
        }
        let n = seen
            .entry((sec.heading_path.clone(), body.to_string()))
            .or_insert(0);
        let mut input = format!("{rel_path}\0{}\0{body}", sec.heading_path);
        if *n > 0 {
            input.push_str(&format!("\0#{n}"));
        }
        *n += 1;
        chunks.push(Chunk {
            hash: hasher(input.as_bytes()),
            kind: ChunkKind::Section,
            heading_path: sec.heading_path,
            anchor: None,
            token_estimate: estimate_tokens(body),
            body: body.to_string(),
        });
    }
    ParsedDoc {
        frontmatter_json: frontmatter_json(&text),
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
        assert_eq!(paths, ["Guide", "Guide › Install"]);
        assert_eq!(doc.chunks[0].body, "# Other\nintro");
    }

    #[test]
    fn duplicate_sections_get_distinct_hashes() {
        let text = "# T\n## Same\nx\n## Same\nx\n";
        let doc = chunk_document("docs/src/d.md", text, &h);
        let hashes: Vec<&str> = doc.chunks.iter().map(|c| c.hash.as_str()).collect();
        assert_eq!(hashes.len(), 3);
        assert_ne!(hashes[1], hashes[2]);
    }

    #[test]
    fn bad_frontmatter_yields_empty_object() {
        let doc = chunk_document("docs/src/b.md", "---\n: : [\n---\n# T\n", &h);
        assert_eq!(doc.frontmatter_json, "{}");
    }
}
