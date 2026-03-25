use serde::{Deserialize, Serialize};

/// A matching line found by `MemoryManager::search`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchHit {
    /// Source file identifier (e.g. `daily:2026-02-27` or `memory.md`).
    pub source: String,
    /// 1-based line number in the source file.
    pub line: usize,
    /// Matching line text.
    pub content: String,
}
