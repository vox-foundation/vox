//! `.voxignore` reader — simple line-by-line gitignore-compatible pattern list.
//!
//! Loaded once per query; patterns are applied as additional skip predicates
//! in WalkDir. No globset dep in this MVP — use the existing `glob` crate.

use std::path::Path;

pub struct VoxIgnore {
    patterns: Vec<String>,
}

impl VoxIgnore {
    pub fn load(repo_root: &Path) -> Self {
        let path = repo_root.join(".voxignore");
        let patterns = std::fs::read_to_string(&path)
            .unwrap_or_default()
            .lines()
            .filter(|l| !l.trim().is_empty() && !l.trim_start().starts_with('#'))
            .map(str::to_string)
            .collect();
        Self { patterns }
    }

    /// Returns true if `rel_path` (forward-slash, repo-relative) matches any ignore pattern.
    pub fn is_ignored(&self, rel_path: &str) -> bool {
        self.patterns.iter().any(|pat| {
            glob::Pattern::new(pat)
                .ok()
                .map(|p| p.matches(rel_path))
                .unwrap_or(false)
        })
    }
}
