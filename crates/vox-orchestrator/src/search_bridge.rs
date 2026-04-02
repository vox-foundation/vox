//! Bridges orchestrator memory files into [`vox_search::LexicalMemoryFallback`].

use vox_search::LexicalMemoryFallback;

use crate::memory::{MemoryConfig, MemoryManager};

/// Substring scan fallback using [`MemoryManager::search`] (daily logs + MEMORY.md).
pub struct MemorySubstringFallback(pub MemoryConfig);

impl LexicalMemoryFallback for MemorySubstringFallback {
    fn substring_search_lines(&self, query: &str) -> Result<Vec<String>, String> {
        let mgr = MemoryManager::new(self.0.clone()).map_err(|e| e.to_string())?;
        let hits = mgr.search(query).map_err(|e| e.to_string())?;
        Ok(hits
            .into_iter()
            .map(|h| format!("[{}:{}] {}", h.source, h.line, h.content))
            .collect())
    }
}
