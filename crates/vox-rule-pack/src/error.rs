//! Error type returned by RulePack loaders.

use thiserror::Error;

pub type RulePackResult<T> = Result<T, RulePackError>;

#[derive(Debug, Error)]
pub enum RulePackError {
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("I/O error reading rule pack at {path}: {source}")]
    Io {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("Invalid regex for rule '{rule_id}': {source}")]
    InvalidRegex {
        rule_id: String,
        #[source]
        source: regex::Error,
    },
    #[error("Duplicate rule id: '{0}'")]
    DuplicateId(String),
    #[error("Unsupported rule pack version: {0} (this build supports v1)")]
    UnsupportedVersion(u32),
}
