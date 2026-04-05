use semver::Version;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GrammarFormat {
    Ebnf,
    Gbnf,
    JsonSchema,
    TreeSitterGrammar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExportConfig {
    pub format: GrammarFormat,
    pub version: Version,
    pub include_deprecated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrammarExportResult {
    pub grammar_text: String,
    pub construct_count: usize,
    pub rule_count: usize,
    pub version: String,
}

pub mod ebnf;
pub mod gbnf;
pub mod json_schema;
pub mod versioning;

pub fn grammar_version_matches_compiler(version: &Version) -> bool {
    if let Ok(compiler_version) = Version::parse(env!("CARGO_PKG_VERSION")) {
        version == &compiler_version
    } else {
        false
    }
}
