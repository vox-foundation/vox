//! Serializable inventory schema types.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub kind: String,
    pub lines_total: u64,
    pub lines_triple_slash: u64,
    pub lines_inner_doc: u64,
    pub lines_plain_comment: u64,
    pub lines_other_doc_signal: u64,
    pub hotspot_tier: i32,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHint {
    pub doc_line: usize,
    pub item_line: usize,
    pub item_preview: String,
    pub containing_symbol: Option<serde_json::Value>,
    pub doc_preview: String,
    pub comment_type: String,
    pub quality_tag: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolHintGroup {
    pub path: String,
    pub hints: Vec<SymbolHint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocInventory {
    pub schema_version: i32,
    pub generated_at: String,
    pub description: String,
    pub first_read_for_agents: Vec<String>,
    pub files: Vec<FileEntry>,
    pub symbol_hints: Vec<SymbolHintGroup>,
}
