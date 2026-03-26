//! Per-file Rust analysis: token map + optional `syn` AST.

use super::token_map::TokenMap;

/// Shared context for Rust detectors: lexical non-code spans + parsed AST when possible.
#[derive(Debug)]
pub struct RustFileContext {
    pub token_map: TokenMap,
    pub ast: Result<syn::File, syn::Error>,
}

impl RustFileContext {
    /// Parse `content` as Rust source (UTF-8).
    pub fn parse(content: &str) -> Self {
        let token_map = TokenMap::from_rust_source(content);
        let ast = syn::parse_file(content);
        Self { token_map, ast }
    }

    /// True if every byte on line `line_1_indexed` (1-based) that overlaps `content` is in a code span.
    /// Lines are split the same way as [`crate::rules::SourceFile::lines`].
    pub fn line_is_prose_safe(&self, content: &str, line_1_indexed: usize) -> bool {
        if line_1_indexed == 0 {
            return false;
        }
        let line_idx = line_1_indexed.saturating_sub(1);
        let mut start = 0usize;
        for (i, line) in content.lines().enumerate() {
            if i == line_idx {
                let end = start.saturating_add(line.len());
                if (start..end).any(|b| self.token_map.is_code_byte(b)) {
                    return false;
                }
                return true;
            }
            start = start.saturating_add(line.len()).saturating_add(1);
        }
        false
    }
}
