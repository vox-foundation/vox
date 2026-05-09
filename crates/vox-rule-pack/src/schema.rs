//! Deserialization schema for `contracts/code-audit/rules.v1.yaml`.
//! Validated separately by JSON Schema; this is the structural binding.

use crate::types::{RuleConfidence, RuleLanguage, RuleSeverity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleFile {
    pub version: u32,
    pub rules: Vec<RuleSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleSpec {
    pub id: String,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub name: String,
    pub description: String,
    pub severity: RuleSeverity,
    #[serde(default)]
    pub confidence: Option<RuleConfidence>,
    pub languages: Vec<RuleLanguage>,
    #[serde(rename = "match")]
    pub match_spec: MatchSpec,
    pub message: String,
    #[serde(default)]
    pub suggestion: Option<String>,
    #[serde(default)]
    pub fixtures: FixtureSpec,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MatchSpec {
    pub kind: MatchKind,
    pub pattern: String,
    #[serde(default)]
    pub skip_in: Vec<SkipScope>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum MatchKind {
    LineRegex,
    MultilineRegex,
    Substring,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum SkipScope {
    /// Skip lines that begin with `///` or `//!`.
    RustDocComment,
    /// Skip bytes the caller's TokenMap reports as comment+string.
    RustNonCode,
    /// Skip bytes the caller's TokenMap reports as comment only.
    RustComment,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct FixtureSpec {
    #[serde(default)]
    pub positive: Vec<String>,
    #[serde(default)]
    pub negative: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
version: 1
rules:
  - id: victory-claim/premature
    parent_id: victory-claim
    name: "Premature victory claim"
    description: "Detects 'done' / 'complete' claims in comments."
    severity: warning
    confidence: medium
    languages: [rust, typescript, python, vox, gdscript]
    match:
      kind: line-regex
      pattern: "(?i)//.*done"
      skip_in: [rust-doc-comment]
    message: "Premature victory claim"
    suggestion: "Remove the comment or describe what is actually done."
    fixtures:
      positive: []
      negative: []
"#;

    #[test]
    fn parses_minimal_valid_file() {
        let parsed: RuleFile = serde_yaml::from_str(SAMPLE).unwrap();
        assert_eq!(parsed.version, 1);
        assert_eq!(parsed.rules.len(), 1);
        let r = &parsed.rules[0];
        assert_eq!(r.id, "victory-claim/premature");
        assert_eq!(r.severity, RuleSeverity::Warning);
        assert_eq!(r.confidence, Some(RuleConfidence::Medium));
        assert_eq!(r.languages.len(), 5);
        assert_eq!(r.match_spec.kind, MatchKind::LineRegex);
        assert_eq!(r.match_spec.skip_in, vec![SkipScope::RustDocComment]);
    }

    #[test]
    fn rejects_unknown_severity() {
        let bad = SAMPLE.replace("severity: warning", "severity: catastrophic");
        let err = serde_yaml::from_str::<RuleFile>(&bad).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("catastrophic") || msg.contains("variant") || msg.contains("unknown"),
            "unexpected error: {msg}"
        );
    }

    #[test]
    fn missing_id_errors() {
        // Supply a rule entry with no `id` field at all.
        let bad = r#"
version: 1
rules:
  - name: "No ID rule"
    description: "Missing required id field."
    severity: warning
    languages: [rust]
    match: { kind: line-regex, pattern: "x" }
    message: "m"
"#;
        let err = serde_yaml::from_str::<RuleFile>(bad).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("id") || msg.contains("missing") || msg.contains("field"),
            "unexpected error: {msg}"
        );
    }
}
