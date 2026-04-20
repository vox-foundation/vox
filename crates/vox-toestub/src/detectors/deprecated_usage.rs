use crate::rules::{DetectionRule, Finding, Language, Severity, SourceFile};
use regex::Regex;

/// Detects the presence of `@deprecated` annotations in Vox files,
/// and warns against raw JSX leakage (e.g., `className=`, `onClick=`) per Item 16.
pub struct DeprecatedUsageDetector {
    deprecated_re: Regex,
    jsx_leak_re: Regex,
}

impl Default for DeprecatedUsageDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl DeprecatedUsageDetector {
    /// Builds a detector with a precompiled `@deprecated` line regex (see [`Default`] for the same).
    pub fn new() -> Self {
        Self {
            deprecated_re: Regex::new(r"^\s*@deprecated").expect("valid"),
            jsx_leak_re: Regex::new(r"(className=|onClick=|onChange=|onSubmit=)").expect("valid"),
        }
    }
}

impl DetectionRule for DeprecatedUsageDetector {
    fn id(&self) -> &'static str {
        "deprecated-usage"
    }

    fn name(&self) -> &'static str {
        "Deprecated Usage Detector"
    }

    fn description(&self) -> &'static str {
        "Flags the presence of @deprecated annotations to encourage removing obsolete code"
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn languages(&self) -> &[Language] {
        &[Language::Vox]
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        for (i, line) in file.lines.iter().enumerate() {
            if self.deprecated_re.is_match(line) {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: i + 1,
                    column: 0,
                    message: "Found @deprecated annotation. Consider removing this obsolete code."
                        .to_string(),
                    suggestion: Some(
                        "Refactor dependents and remove this deprecated item.".to_string(),
                    ),
                    context: file.context_around(i + 1, 1),
                    confidence: None,
                    evidence: None,
                });
            }

            if let Some(mat) = self.jsx_leak_re.find(line) {
                let attr = mat.as_str().trim_end_matches('=');
                let mut vox_attr = attr.to_lowercase();
                if vox_attr.starts_with("on") {
                    vox_attr = format!("on:{}", &vox_attr[2..]);
                }
                if vox_attr == "classname" {
                    vox_attr = "class".to_string();
                }

                findings.push(Finding {
                    rule_id: "raw-jsx-leakage".to_string(),
                    rule_name: "Raw JSX Leakage Detector".to_string(),
                    severity: Severity::Warning,
                    file: file.path.clone(),
                    line: i + 1,
                    column: 0,
                    message: format!("Raw JSX '{}' leaks into Vox source (Item 16).", attr),
                    suggestion: Some(format!(
                        "Use Vox-native syntax: '{}=' instead of '{}='.",
                        vox_attr, attr
                    )),
                    context: file.context_around(i + 1, 1),
                    confidence: None,
                    evidence: None,
                });
            }
        }

        findings
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn source(code: &str) -> SourceFile {
        SourceFile::new(PathBuf::from("test.vox"), code.to_string())
    }

    #[test]
    fn detects_deprecated_annotation() {
        let d = DeprecatedUsageDetector::new();
        let f = source("@deprecated\nfn old_stuff():\n    pass");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].rule_id, "deprecated-usage");
    }

    #[test]
    fn detects_jsx_leakage() {
        let d = DeprecatedUsageDetector::new();
        let f = source("<div className=\"hello\" onClick={handler}>");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 2);
        assert_eq!(findings[0].rule_id, "raw-jsx-leakage");
        assert_eq!(
            findings[0].suggestion.as_deref(),
            Some("Use Vox-native syntax: 'class=' instead of 'className='.".into())
        );
    }

    #[test]
    fn ignores_pure_annotation() {
        let d = DeprecatedUsageDetector::new();
        let f = source("@pure\nfn clean():\n    pass");
        let findings = d.detect(&f, None);
        assert!(findings.is_empty());
    }
}
