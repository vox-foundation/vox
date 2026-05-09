//! Conversions between vox-rule-pack and vox-code-audit enums.
//! Kept in vox-code-audit (not vox-rule-pack) so the lower-layer crate stays domain-free.

use crate::rules::{FindingConfidence, Language, Severity};
use vox_rule_pack::{RuleConfidence, RuleLanguage, RuleSeverity};

impl From<RuleSeverity> for Severity {
    fn from(value: RuleSeverity) -> Self {
        match value {
            RuleSeverity::Info => Severity::Info,
            RuleSeverity::Warning => Severity::Warning,
            RuleSeverity::Error => Severity::Error,
            RuleSeverity::Critical => Severity::Critical,
        }
    }
}

impl From<RuleConfidence> for FindingConfidence {
    fn from(value: RuleConfidence) -> Self {
        match value {
            RuleConfidence::High => FindingConfidence::High,
            RuleConfidence::Medium => FindingConfidence::Medium,
            RuleConfidence::Low => FindingConfidence::Low,
        }
    }
}

impl From<RuleLanguage> for Language {
    fn from(value: RuleLanguage) -> Self {
        match value {
            RuleLanguage::Rust => Language::Rust,
            RuleLanguage::TypeScript => Language::TypeScript,
            RuleLanguage::Python => Language::Python,
            RuleLanguage::GDScript => Language::GDScript,
            RuleLanguage::Vox => Language::Vox,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severity_round_trip_warning() {
        let s: Severity = RuleSeverity::Warning.into();
        assert_eq!(s, Severity::Warning);
    }

    #[test]
    fn severity_round_trip_critical() {
        let s: Severity = RuleSeverity::Critical.into();
        assert_eq!(s, Severity::Critical);
    }

    #[test]
    fn confidence_round_trip_medium() {
        let c: FindingConfidence = RuleConfidence::Medium.into();
        assert_eq!(c, FindingConfidence::Medium);
    }

    #[test]
    fn language_round_trip_rust() {
        let l: Language = RuleLanguage::Rust.into();
        assert_eq!(l, Language::Rust);
    }

    #[test]
    fn language_round_trip_gdscript() {
        let l: Language = RuleLanguage::GDScript.into();
        assert_eq!(l, Language::GDScript);
    }
}
