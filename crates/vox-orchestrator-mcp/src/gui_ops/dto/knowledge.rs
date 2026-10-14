//! Knowledge / memory / search wire DTOs (plan 07 Task 6). Field names are part of the
//! Axis UI contract — do NOT rename.
use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeNodeRow {
    pub id: String,
    pub label: String,
    pub snippet: String,
    pub node_type: Option<String>,
    pub created_at: String,
}

impl KnowledgeNodeRow {
    /// Truncates `content` to a 200-character snippet (`...` suffix when cut).
    pub fn from_record(r: vox_db::KnowledgeNodeRecord) -> Self {
        let snippet = if r.content.chars().count() > 200 {
            let first_200: String = r.content.chars().take(200).collect();
            format!("{first_200}...")
        } else {
            r.content
        };
        Self {
            id: r.id,
            label: r.label,
            snippet,
            node_type: r.node_type,
            created_at: r.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KbHealthDto {
    pub node_count: usize,
    pub edge_count: usize,
    pub fts_available: bool,
    pub corpus_counts: HashMap<String, usize>,
}

/// `GET /api/v2/knowledge/corpus-counts`: the Memory surface's corpus tiles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CorpusCountsDto {
    pub proj: u32,
    pub docs: u32,
    pub chats: u32,
    pub rules: u32,
    pub web: u32,
    pub recent_memories: u32,
}

impl From<vox_db::SearchCorpusCounts> for CorpusCountsDto {
    fn from(c: vox_db::SearchCorpusCounts) -> Self {
        Self {
            proj: c.proj,
            docs: c.docs,
            chats: c.chats,
            rules: c.rules,
            web: c.web,
            recent_memories: c.recent_memories,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiHitResult {
    pub src: String,
    pub line: usize,
    pub score: f64,
    pub kind: String,
    pub text: String,
}

/// Locator used to open a hit from the UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenLocatorDto {
    pub kind: String, // "file" | "web" | "memory" | "chat" | "command" | "none"
    pub value: String,
}

/// One search hit forwarded to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedHitDto {
    pub source: String,
    pub kind: String,
    pub path: Option<String>,
    pub title: Option<String>,
    pub snippet: String,
    pub score: f64,
    pub provenance: Vec<String>,
    pub locator: OpenLocatorDto,
}

/// A facet bucket (source or kind) with its count.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FacetCount {
    pub value: String,
    pub count: u32,
}

/// Top-level search response forwarded to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponseDto {
    pub hits: Vec<UnifiedHitDto>,
    pub facets_by_source: Vec<FacetCount>,
    pub facets_by_kind: Vec<FacetCount>,
    pub total: usize,
    pub next_cursor: Option<usize>,
    /// Names of the `SearchPlan` corpora that were consulted.
    pub corpora: Vec<String>,
    /// True when the WalkDir file scan capped at `repo_inventory_max_files`.
    pub repo_truncated: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(content: String) -> vox_db::KnowledgeNodeRecord {
        vox_db::KnowledgeNodeRecord {
            id: "doc:1".into(),
            label: "Doc".into(),
            content,
            node_type: Some("document".into()),
            metadata: None,
            created_at: "2026-10-09".into(),
        }
    }

    #[test]
    fn node_row_snippet_truncates_at_200_chars() {
        let long = KnowledgeNodeRow::from_record(record("é".repeat(250)));
        assert_eq!(long.snippet.chars().count(), 203);
        assert!(long.snippet.ends_with("..."));
        let short = KnowledgeNodeRow::from_record(record("short".into()));
        assert_eq!(short.snippet, "short");
    }

    #[test]
    fn corpus_counts_carry_every_field() {
        let dto = CorpusCountsDto::from(vox_db::SearchCorpusCounts {
            proj: 1,
            docs: 2,
            chats: 3,
            rules: 4,
            web: 5,
            recent_memories: 6,
        });
        let v = serde_json::to_value(dto).unwrap();
        assert_eq!(v["docs"], 2);
        assert_eq!(v["recent_memories"], 6);
    }
}
