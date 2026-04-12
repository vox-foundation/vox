use semver::Version;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Supported grammar export formats.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum GrammarFormat {
    /// ISO EBNF (Extended Backus-Naur Form) — authoritative.
    Ebnf,
    /// GBNF for GGML constrained sampling (left-recursion free).
    Gbnf,
    /// JSON Schema draft-2020-12 for VoxAstNode.
    JsonSchema,
    /// Lark grammar for Python-native bridge integrations.
    Lark,
    /// Tree-sitter grammar (reserved for future use).
    TreeSitterGrammar,
}

impl GrammarFormat {
    /// Human-readable lowercase format name.
    pub fn as_str(&self) -> &'static str {
        match self {
            GrammarFormat::Ebnf => "ebnf",
            GrammarFormat::Gbnf => "gbnf",
            GrammarFormat::JsonSchema => "json-schema",
            GrammarFormat::Lark => "lark",
            GrammarFormat::TreeSitterGrammar => "tree-sitter",
        }
    }
}

/// Configuration for a grammar export run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExportConfig {
    /// The target format to emit.
    pub format: GrammarFormat,
    /// Semantic version to embed in output.
    pub version: Version,
    /// Include deprecated/legacy constructs in the output.
    pub include_deprecated: bool,
    /// Optional path for MENS training to load grammar from (for gating constrained inference).
    pub grammar_export_path: Option<PathBuf>,
}

impl Default for GrammarExportConfig {
    fn default() -> Self {
        Self {
            format: GrammarFormat::Ebnf,
            version: Version::new(0, 4, 0),
            include_deprecated: false,
            grammar_export_path: None,
        }
    }
}

/// Result of a grammar export operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExportResult {
    /// The emitted grammar text.
    pub grammar_text: String,
    /// Number of top-level named constructs.
    pub construct_count: usize,
    /// Number of grammar rules/productions.
    pub rule_count: usize,
    /// Version string embedded in output.
    pub version: String,
}

pub mod automaton;
pub mod compact_prompt;
pub mod ebnf;
pub mod gbnf;
pub mod json_schema;
pub mod lark;
pub mod versioning;

/// Dispatch grammar export to the appropriate emitter based on `config.format`.
pub fn export(config: &GrammarExportConfig) -> GrammarExportResult {
    let grammar_text = match config.format {
        GrammarFormat::Ebnf => ebnf::emit_ebnf(),
        GrammarFormat::Gbnf => gbnf::emit_gbnf(),
        GrammarFormat::JsonSchema => json_schema::emit_json_schema(),
        GrammarFormat::Lark => lark::emit_lark(),
        GrammarFormat::TreeSitterGrammar => {
            // Reserved — emit stub text so callers get a non-empty string.
            "// tree-sitter grammar generation not yet implemented\n".to_string()
        }
    };
    let rule_count = grammar_text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('/') && !l.starts_with("(*"))
        .count();
    GrammarExportResult {
        construct_count: rule_count,
        rule_count,
        version: config.version.to_string(),
        grammar_text,
    }
}

/// Check that the grammar crate version matches the compiler crate version.
pub fn grammar_version_matches_compiler(version: &Version) -> bool {
    if let Ok(compiler_version) = Version::parse(env!("CARGO_PKG_VERSION")) {
        version == &compiler_version
    } else {
        false
    }
}
