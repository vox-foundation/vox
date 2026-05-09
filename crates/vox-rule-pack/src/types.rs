//! Public enum types used in the rule SSOT. Mirror vox-code-audit's domain
//! types so consumers can `From`-convert without circular crate dependencies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleConfidence {
    High,
    Medium,
    Low,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleLanguage {
    Rust,
    TypeScript,
    Python,
    #[serde(rename = "gdscript")]
    GDScript,
    Vox,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_serializes_lowercase() {
        let s = serde_yaml::to_string(&RuleSeverity::Warning).unwrap();
        assert_eq!(s.trim(), "warning");
    }

    #[test]
    fn confidence_round_trips() {
        let original = RuleConfidence::Medium;
        let yaml = serde_yaml::to_string(&original).unwrap();
        let back: RuleConfidence = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(original, back);
    }

    #[test]
    fn language_parses_from_string() {
        let langs: Vec<RuleLanguage> =
            serde_yaml::from_str("[rust, typescript, python, vox, gdscript]").unwrap();
        assert_eq!(langs.len(), 5);
        assert_eq!(langs[0], RuleLanguage::Rust);
        assert_eq!(langs[4], RuleLanguage::GDScript);
    }
}
