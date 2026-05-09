//! Generic line-by-line and multiline dispatcher backed by [`CompiledRule`]s from the embedded pack.
//!
//! Detectors whose entire logic is "match a regex, emit a finding" can be reduced to
//! `RulePackDetector::for_ids(&[...]).detect_file(file)`.
//!
//! Detectors with skip logic, captures, or dynamic messages keep their Rust code but call
//! `rule.regex()` instead of hand-compiling the same pattern.

use crate::rules::{Finding, Language, SourceFile};
use vox_rule_pack::{CompiledRule, MatchKind, RuleLanguage};

/// Dispatches a fixed set of rules from the embedded pack against a [`SourceFile`].
///
/// Line-regex and substring rules run per-line; multiline-regex rules scan the whole content.
/// Languages are enforced: a rule only fires on files whose language appears in the rule's list.
pub struct RulePackDetector {
    line_rules: Vec<&'static CompiledRule>,
    multiline_rules: Vec<&'static CompiledRule>,
}

impl RulePackDetector {
    /// Build a dispatcher for the given rule IDs.
    ///
    /// # Panics
    ///
    /// Panics if any ID is not present in the embedded pack — this indicates a YAML/code sync bug.
    pub fn for_ids(ids: &[&str]) -> Self {
        use crate::embedded_rules::embedded_pack;
        let pack = embedded_pack();
        let mut line_rules = Vec::new();
        let mut multiline_rules = Vec::new();
        for id in ids {
            let rule = pack.rule(id).unwrap_or_else(|| {
                panic!("RulePackDetector: rule '{id}' not found in embedded pack")
            });
            match rule.kind {
                MatchKind::MultilineRegex => multiline_rules.push(rule),
                _ => line_rules.push(rule),
            }
        }
        Self {
            line_rules,
            multiline_rules,
        }
    }

    /// Run all rules against `file`, filtering by language.
    pub fn detect_file(&self, file: &SourceFile) -> Vec<Finding> {
        let mut findings = Vec::new();

        // Multiline pass — scan whole content once per rule.
        for rule in &self.multiline_rules {
            if !rule_applies_to_file(rule, file) {
                continue;
            }
            for line_num in rule.matches_in_content(&file.content) {
                findings.push(make_finding(rule, file, line_num));
            }
        }

        // Per-line pass.
        for (i, line) in file.lines.iter().enumerate() {
            let line_num = i + 1;
            for rule in &self.line_rules {
                if !rule_applies_to_file(rule, file) {
                    continue;
                }
                if rule.matches_line(line) {
                    findings.push(make_finding(rule, file, line_num));
                }
            }
        }

        findings
    }
}

fn rule_applies_to_file(rule: &CompiledRule, file: &SourceFile) -> bool {
    let file_lang = match file.language {
        Language::Rust => RuleLanguage::Rust,
        Language::TypeScript => RuleLanguage::TypeScript,
        Language::Python => RuleLanguage::Python,
        Language::GDScript => RuleLanguage::GDScript,
        Language::Vox => RuleLanguage::Vox,
        _ => return false,
    };
    rule.languages.contains(&file_lang)
}

pub fn make_finding(rule: &'static CompiledRule, file: &SourceFile, line_num: usize) -> Finding {
    Finding {
        rule_id: rule.id.clone(),
        rule_name: rule.name.clone(),
        severity: rule.severity.into(),
        file: file.path.clone(),
        line: line_num,
        column: 0,
        message: rule.message.clone(),
        suggestion: rule.suggestion.clone(),
        diagnostic_id: None,
        alternatives: vec![],
        rationale: None,
        context: file.context_around(line_num, 2),
        confidence: rule.confidence.map(|c| c.into()),
        evidence: None,
    }
}

/// Look up a single rule from the embedded pack. Panics if the ID is absent.
pub fn pack_rule(id: &str) -> &'static CompiledRule {
    use crate::embedded_rules::embedded_pack;
    embedded_pack()
        .rule(id)
        .unwrap_or_else(|| panic!("pack_rule: '{id}' not found in embedded pack"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn vox_file(code: &str) -> SourceFile {
        SourceFile::new(PathBuf::from("test.vox"), code.to_string())
    }

    #[test]
    fn deprecated_usage_fires() {
        let d = RulePackDetector::for_ids(&["deprecated-usage"]);
        let f = vox_file("@deprecated\nfn old() {}");
        let findings = d.detect_file(&f);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "deprecated-usage");
    }

    #[test]
    fn raw_jsx_fires() {
        let d = RulePackDetector::for_ids(&["raw-jsx-leakage"]);
        let f = vox_file("<div className=\"x\">");
        let findings = d.detect_file(&f);
        assert_eq!(findings.len(), 1);
    }

    #[test]
    fn language_filter_skips_wrong_lang() {
        // deprecated-usage only applies to Vox — should not fire on a .rs file.
        let d = RulePackDetector::for_ids(&["deprecated-usage"]);
        let f = SourceFile::new(
            PathBuf::from("test.rs"),
            "@deprecated\nfn old() {}".to_string(),
        );
        assert!(d.detect_file(&f).is_empty());
    }
}
