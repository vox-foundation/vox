//! AST mapping for navigating to specific nodes without dictating exact characters.

use serde::{Deserialize, Serialize};

/// An AST-aware target that Vox Oratio can map to in the IDE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AstTarget {
    /// The type of node (e.g. "function", "struct", "impl").
    pub node_kind: String,
    /// Extracted symbol name, if identified.
    pub symbol_name: Option<String>,
}

/// Simple heuristic to extract AST targets from transcript (Phase 1C MVP).
#[must_use]
pub fn map_to_ast_target(transcript: &str) -> Option<AstTarget> {
    let lower = transcript.to_ascii_lowercase();
    let parts: Vec<&str> = lower.split_whitespace().collect();
    
    for (i, p) in parts.iter().enumerate() {
        if *p == "function" || *p == "fn" || *p == "struct" || *p == "type" {
            let kind = if *p == "fn" { "function".to_string() } else { p.to_string() };
            let mut name = None;
            if i + 1 < parts.len() {
                // Next word is probably the symbol name
                let clean = parts[i+1].trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
                if !clean.is_empty() {
                    name = Some(clean.to_string());
                }
            }
            return Some(AstTarget {
                node_kind: kind,
                symbol_name: name,
            });
        }
    }
    None
}
