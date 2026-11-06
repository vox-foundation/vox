//! Per-file Rust analysis: token map + optional `syn` AST.

use super::token_map::TokenMap;

/// Shared context for Rust detectors: lexical non-code spans + parsed AST when possible.
#[derive(Debug)]
pub struct RustFileContext {
    pub token_map: TokenMap,
    pub ast: Result<syn::File, syn::Error>,
    pub cfg_test_ranges: Vec<std::ops::Range<usize>>,
}

impl RustFileContext {
    /// Parse `content` as Rust source (UTF-8).
    pub fn parse(content: &str) -> Self {
        let token_map = TokenMap::from_rust_source(content);
        let ast = syn::parse_file(content);
        let cfg_test_ranges = Self::compute_ignored_test_ranges(content, &token_map);
        Self {
            token_map,
            ast,
            cfg_test_ranges,
        }
    }

    /// True if `byte` falls inside a `#[cfg(test)]` item or `fn minimal_repro` body.
    #[inline]
    pub fn is_cfg_test_byte(&self, byte: usize) -> bool {
        self.cfg_test_ranges.iter().any(|r| r.contains(&byte))
    }

    fn compute_ignored_test_ranges(
        content: &str,
        token_map: &TokenMap,
    ) -> Vec<std::ops::Range<usize>> {
        let mut ranges = Vec::new();
        for pat in ["#[cfg(test)]", "#[cfg(all(test,", "fn minimal_repro"] {
            let mut search_from = 0usize;
            while let Some(rel) = content[search_from..].find(pat) {
                let start = search_from + rel;
                search_from = start + pat.len();
                if !token_map.is_code_byte(start) {
                    continue;
                }
                if let Some(end) = Self::find_item_end(content, token_map, start + pat.len()) {
                    ranges.push(start..end);
                    search_from = end;
                }
            }
        }
        ranges
    }

    fn find_item_end(content: &str, token_map: &TokenMap, from: usize) -> Option<usize> {
        let bytes = content.as_bytes();
        let mut i = from;
        let mut bracket_depth = 0usize;
        let mut brace_depth = 0usize;
        let mut started_brace = false;
        while i < bytes.len() {
            if token_map.is_code_byte(i) {
                match bytes[i] {
                    b'[' | b'(' => bracket_depth += 1,
                    b']' | b')' => bracket_depth = bracket_depth.saturating_sub(1),
                    b'{' if bracket_depth == 0 => {
                        brace_depth += 1;
                        started_brace = true;
                    }
                    b'}' if bracket_depth == 0 && started_brace => {
                        brace_depth = brace_depth.saturating_sub(1);
                        if brace_depth == 0 {
                            return Some(i + 1);
                        }
                    }
                    b';' if bracket_depth == 0 && !started_brace => {
                        return Some(i + 1);
                    }
                    _ => {}
                }
            }
            i += 1;
        }
        if started_brace {
            Some(bytes.len())
        } else {
            None
        }
    }

    /// True if every byte on line `line_1_indexed` (1-based) that overlaps `content` is in a non-code span.
    /// Lines are split the same way as [`crate::rules::SourceFile::lines`].
    pub fn line_is_prose_safe(&self, content: &str, line_1_indexed: usize) -> bool {
        if line_1_indexed == 0 {
            return false;
        }
        let line_idx = line_1_indexed.saturating_sub(1);
        for (i, line) in content.lines().enumerate() {
            if i == line_idx {
                let start = (line.as_ptr() as usize).saturating_sub(content.as_ptr() as usize);
                let end = start.saturating_add(line.len());
                if (start..end).any(|b| self.token_map.is_code_byte(b)) {
                    return false;
                }
                return true;
            }
        }
        false
    }
}

#[cfg(test)]
mod semcov_wave1e_tests {
    #![allow(unused_imports)]
    use super::*;

    #[test]
    fn comment_only_line_is_prose_safe() {
        let content = "// this is a comment\nlet x = 1;";
        let ctx = RustFileContext::parse(content);
        // Line 1 is entirely a comment — no code bytes
        assert!(ctx.line_is_prose_safe(content, 1));
    }

    #[test]
    fn code_line_is_not_prose_safe() {
        let content = "let x = 1;\n// comment";
        let ctx = RustFileContext::parse(content);
        // Line 1 has code bytes
        assert!(!ctx.line_is_prose_safe(content, 1));
    }

    #[test]
    fn second_line_code_detected() {
        let content = "// comment\nlet y = 2;";
        let ctx = RustFileContext::parse(content);
        assert!(ctx.line_is_prose_safe(content, 1));
        assert!(!ctx.line_is_prose_safe(content, 2));
    }

    #[test]
    fn zero_line_returns_false() {
        let content = "let x = 1;";
        let ctx = RustFileContext::parse(content);
        assert!(!ctx.line_is_prose_safe(content, 0));
    }

    #[test]
    fn out_of_range_line_returns_false() {
        let content = "let x = 1;";
        let ctx = RustFileContext::parse(content);
        assert!(!ctx.line_is_prose_safe(content, 99));
    }

    #[test]
    fn string_literal_line_is_not_prose_safe() {
        // A string literal is non-code but NOT a comment; the function checks is_code_byte.
        // String bytes are non-code, so a line with only a string is prose-safe.
        let content = r#"let _ = "hello world";"#;
        let ctx = RustFileContext::parse(content);
        // The line has code bytes (let, =, ;) so it's NOT prose safe
        assert!(!ctx.line_is_prose_safe(content, 1));
    }
}
