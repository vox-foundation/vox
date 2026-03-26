use crate::typeck::diagnostics::Diagnostic;
use serde::{Deserialize, Serialize};

/// A suggested fix for a diagnostic.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixSuggestion {
    pub message: String,
    pub diff: String,
    pub explanation: String,
}

pub trait AutoFixer {
    fn suggest_fixes(&self, source: &str, diagnostics: &[Diagnostic]) -> Vec<FixSuggestion>;
}

/// A rule that can propose an autofix for a diagnostic.
pub trait FixRule {
    fn name(&self) -> &'static str;
    fn suggest(&self, source: &str, diagnostic: &Diagnostic) -> Option<FixSuggestion>;
}

/// Baseline rule: use diagnostic suggestion/context to craft a patch-like diff.
struct SuggestedTextRule;

impl FixRule for SuggestedTextRule {
    fn name(&self) -> &'static str {
        "suggested_text"
    }

    fn suggest(&self, _source: &str, diagnostic: &Diagnostic) -> Option<FixSuggestion> {
        let suggestion_text = diagnostic.suggestions.first()?.as_str();
        let old_code = diagnostic.context.as_ref().map_or_else(String::new, |ctx| {
            ctx.split('\n')
                .map(|l| format!("-{l}"))
                .collect::<Vec<_>>()
                .join("\n")
        });
        Some(FixSuggestion {
            message: format!("Fix for: {}", diagnostic.message),
            diff: format!("{}\n+{}", old_code, suggestion_text),
            explanation: format!(
                "Rule '{}' applying suggested replacement: {}",
                self.name(),
                suggestion_text
            ),
        })
    }
}

/// Rule-driven autofixer foundation for future targeted fix classes.
pub struct RuleBasedAutoFixer {
    rules: Vec<Box<dyn FixRule + Send + Sync>>,
}

impl Default for RuleBasedAutoFixer {
    fn default() -> Self {
        Self {
            rules: vec![Box::new(SuggestedTextRule)],
        }
    }
}

impl AutoFixer for RuleBasedAutoFixer {
    fn suggest_fixes(&self, source: &str, diagnostics: &[Diagnostic]) -> Vec<FixSuggestion> {
        diagnostics
            .iter()
            .map(|diag| {
                for rule in &self.rules {
                    if let Some(suggestion) = rule.suggest(source, diag) {
                        return suggestion;
                    }
                }
                FixSuggestion {
                    message: format!("Fix for: {}", diag.message),
                    diff: "// No automated fix available for this diagnostic.".to_string(),
                    explanation: "Manual review required for this issue.".to_string(),
                }
            })
            .collect()
    }
}

/// Backward-compatible entrypoint while Wave 1 migrates callsites to rule-based naming.
pub struct StubAutoFixer {
    inner: RuleBasedAutoFixer,
}

impl Default for StubAutoFixer {
    fn default() -> Self {
        Self {
            inner: RuleBasedAutoFixer::default(),
        }
    }
}

impl AutoFixer for StubAutoFixer {
    fn suggest_fixes(&self, source: &str, diagnostics: &[Diagnostic]) -> Vec<FixSuggestion> {
        self.inner.suggest_fixes(source, diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::typeck::diagnostics::{DiagnosticCategory, Severity};

    #[test]
    fn stub_autofixer_one_fix_per_diagnostic() {
        let fixer = StubAutoFixer::default();
        let diags = vec![Diagnostic {
            severity: Severity::Error,
            message: "Undefined x".to_string(),
            span: Span { start: 0, end: 1 },
            expected_type: None,
            found_type: None,
            context: Some("let x = 1".to_string()),
            suggestions: vec!["let x: int = 1".to_string()],
            category: DiagnosticCategory::Typecheck,
        }];
        let fixes = fixer.suggest_fixes("let x = 1", &diags);
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].diff.contains("-let x = 1"));
        assert!(fixes[0].diff.contains("+let x: int = 1"));
    }

    #[test]
    fn rule_based_autofixer_falls_back_without_suggestion() {
        let fixer = RuleBasedAutoFixer::default();
        let diags = vec![Diagnostic {
            severity: Severity::Error,
            message: "Type mismatch".to_string(),
            span: Span { start: 0, end: 1 },
            expected_type: None,
            found_type: None,
            context: Some("ret x".to_string()),
            suggestions: vec![],
            category: DiagnosticCategory::Typecheck,
        }];
        let fixes = fixer.suggest_fixes("ret x", &diags);
        assert_eq!(fixes.len(), 1);
        assert!(fixes[0].diff.contains("No automated fix available"));
    }
}
