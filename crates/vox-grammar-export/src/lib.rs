//! Grammar export schema + manifest types (TextMate, tree-sitter, EBNF targets).

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
    /// Canonical GRAMMAR_SSOT.md markdown.
    SsotMarkdown,
    /// XGrammar-2 for high-integrity constrained inference.
    XGrammar2,
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
            GrammarFormat::SsotMarkdown => "ssot-markdown",
            GrammarFormat::XGrammar2 => "x-grammar-2",
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
    /// SHA256 hash of the emitted grammar.
    pub grammar_hash: String,
}

pub mod compact_prompt;
pub mod ebnf;
pub mod gbnf;
pub mod grammar_ir;
pub mod json_schema;
pub mod lark;
pub mod ssot_markdown;
pub mod versioning;
pub mod x_grammar_2;

/// Dispatch grammar export to the appropriate emitter based on `config.format`.
pub fn export(config: &GrammarExportConfig) -> anyhow::Result<GrammarExportResult> {
    let grammar_text = match config.format {
        GrammarFormat::Ebnf => ebnf::emit_ebnf(),
        GrammarFormat::Gbnf => {
            return Err(anyhow::anyhow!(
                "GBNF format is DEPRECATED due to CVE-2026-2069 (ReDoS vulnerability in recursive rule expansion). \
                 Please migrate to XGrammar-2 or Lark for constrained sampling."
            ));
        }
        GrammarFormat::JsonSchema => json_schema::emit_json_schema(),
        GrammarFormat::Lark => lark::emit_lark(),
        GrammarFormat::TreeSitterGrammar => {
            return Err(anyhow::anyhow!(
                "Tree-sitter grammar generation is not yet implemented (Wave 3 backlog)."
            ));
        }
        GrammarFormat::SsotMarkdown => ssot_markdown::emit_ssot_markdown(),
        GrammarFormat::XGrammar2 => x_grammar_2::emit_x_grammar_2(),
    };
    let rule_count = grammar_text
        .lines()
        .filter(|l| !l.trim().is_empty() && !l.starts_with('/') && !l.starts_with("(*"))
        .count();
    Ok(GrammarExportResult {
        construct_count: rule_count,
        rule_count,
        version: config.version.to_string(),
        grammar_hash: versioning::compute_ebnf_hash(),
        grammar_text,
    })
}

/// Check that the grammar crate version matches the compiler crate version.
pub fn grammar_version_matches_compiler(version: &Version) -> bool {
    if let Ok(compiler_version) = Version::parse(env!("CARGO_PKG_VERSION")) {
        version == &compiler_version
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_names_every_format() {
        let cases = [
            (GrammarFormat::Ebnf, "ebnf"),
            (GrammarFormat::Gbnf, "gbnf"),
            (GrammarFormat::JsonSchema, "json-schema"),
            (GrammarFormat::Lark, "lark"),
            (GrammarFormat::TreeSitterGrammar, "tree-sitter"),
            (GrammarFormat::SsotMarkdown, "ssot-markdown"),
            (GrammarFormat::XGrammar2, "x-grammar-2"),
        ];
        for (format, name) in cases {
            assert_eq!(format.as_str(), name);
        }
    }

    #[test]
    fn export_ebnf_returns_non_empty_grammar() {
        let result = export(&GrammarExportConfig::default()).expect("ebnf export");
        assert!(!result.grammar_text.trim().is_empty());
        assert!(result.rule_count > 0);
        assert_eq!(result.version, "0.4.0");
    }

    #[test]
    fn grammar_version_matches_own_crate_version_only() {
        let own = Version::parse(env!("CARGO_PKG_VERSION")).expect("crate version");
        assert!(grammar_version_matches_compiler(&own));
        let other = Version::new(own.major + 1, 0, 0);
        assert!(!grammar_version_matches_compiler(&other));
    }
}
