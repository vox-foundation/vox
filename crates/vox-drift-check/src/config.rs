use serde::{Deserialize, Serialize};
use vox_code_audit::rules::Severity;

fn default_3() -> usize {
    3
}
fn default_8() -> usize {
    8
}
fn default_2() -> usize {
    2
}
fn default_5() -> usize {
    5
}
fn default_warn() -> Severity {
    Severity::Warning
}
fn default_info() -> Severity {
    Severity::Info
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct ConfigMeta {
    pub version: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LiteralDedupConfig {
    #[serde(default = "default_3")]
    pub threshold: usize,
    #[serde(default = "default_8")]
    pub min_length: usize,
    #[serde(default)]
    pub ignore_in_paths: Vec<String>,
    #[serde(default = "default_info")]
    pub severity: Severity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NumericDedupConfig {
    #[serde(default = "default_3")]
    pub threshold: usize,
    #[serde(default = "default_warn")]
    pub severity: Severity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct BodyDedupConfig {
    #[serde(default = "default_2")]
    pub threshold: usize,
    #[serde(default = "default_5")]
    pub min_lines: usize,
    #[serde(default = "default_warn")]
    pub severity: Severity,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForbiddenCall {
    pub id: String,
    #[serde(rename = "match")]
    pub patterns: Vec<String>,
    #[serde(default)]
    pub allow_in_crate: Vec<String>,
    #[serde(default)]
    pub allow_in_test: bool,
    pub severity: Severity,
    pub suggestion: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ForbiddenLiteral {
    pub id: String,
    pub pattern: String,
    #[serde(default)]
    pub allow_in_crate: Vec<String>,
    pub severity: Severity,
    pub suggestion: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub struct DriftConfig {
    pub meta: Option<ConfigMeta>,
    #[serde(default)]
    pub duplicated_literal: LiteralDedupConfig,
    #[serde(default)]
    pub duplicated_numeric: NumericDedupConfig,
    #[serde(default)]
    pub duplicated_body: BodyDedupConfig,
    #[serde(default)]
    pub forbidden_call: Vec<ForbiddenCall>,
    #[serde(default)]
    pub forbidden_literal: Vec<ForbiddenLiteral>,
}

impl Default for LiteralDedupConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            min_length: 8,
            ignore_in_paths: vec![],
            severity: Severity::Info,
        }
    }
}

impl Default for NumericDedupConfig {
    fn default() -> Self {
        Self {
            threshold: 3,
            severity: Severity::Warning,
        }
    }
}

impl Default for BodyDedupConfig {
    fn default() -> Self {
        Self {
            threshold: 2,
            min_lines: 5,
            severity: Severity::Warning,
        }
    }
}

impl DriftConfig {
    pub fn load(root: &std::path::Path) -> Self {
        let path = root.join("drift-patterns.toml");
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_minimal_config() {
        let toml_str = r#"
[meta]
version = 1

[duplicated_literal]
threshold = 5
min_length = 10
severity = "info"
"#;
        let cfg: DriftConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.duplicated_literal.threshold, 5);
    }

    #[test]
    fn default_config_has_sensible_thresholds() {
        let cfg = DriftConfig::default();
        assert_eq!(cfg.duplicated_literal.threshold, 3);
        assert_eq!(cfg.duplicated_numeric.threshold, 3);
    }
}
