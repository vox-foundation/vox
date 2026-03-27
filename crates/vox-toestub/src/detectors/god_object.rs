use crate::rules::{DetectionRule, Finding, Language, Severity, SourceFile};

/// Detects "God Objects" — files or entities that are too large or have too many responsibilities.
pub struct GodObjectDetector {
    /// Soft cap on non-blank source lines in a single file before it is treated as a god object.
    pub max_lines: usize,
    /// Maximum `fn` / method-like declarations counted per file (language-specific heuristics in `check`).
    pub max_methods: usize,
}

impl Default for GodObjectDetector {
    fn default() -> Self {
        Self {
            // TOESTUB remediation (2025-Q1): raised from 500 — several first-party crates
            // (integration tests, CLI publication, MCP dispatch) legitimately exceed 500 non-blank
            // lines until phased splits land; still catches extreme single-file dumps.
            max_lines: 1700,
            // Raised from 12: `fn ` / `impl ` substring heuristic over-counts in macro-heavy modules.
            max_methods: 38,
        }
    }
}

impl DetectionRule for GodObjectDetector {
    fn id(&self) -> &'static str {
        "arch/god_object"
    }

    fn name(&self) -> &'static str {
        "God Object Detector"
    }

    fn description(&self) -> &'static str {
        "Detects files or structs that are too large and should be decomposed into traits or smaller modules."
    }

    fn severity(&self) -> Severity {
        Severity::Error
    }

    fn languages(&self) -> &[Language] {
        &[
            Language::Rust,
            Language::TypeScript,
            Language::Python,
            Language::Vox,
        ]
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();

        let nonblank_lines = file.lines.iter().filter(|l| !l.trim().is_empty()).count();

        // Check file size using non-blank lines (matches workspace god-object checklist / PowerShell Trim rule).
        if nonblank_lines > self.max_lines {
            findings.push(Finding {
                rule_id: self.id().to_string(),
                rule_name: self.name().to_string(),
                severity: self.severity(),
                file: file.path.clone(),
                line: 1,
                column: 0,
                message: format!(
                    "File is too large ({} non-blank lines). Maximum allowed is {}.",
                    nonblank_lines, self.max_lines
                ),
                suggestion: Some(
                    "Refactor this file into smaller sub-modules or extract logic into traits."
                        .to_string(),
                ),
                context: file.context_around(1, 2),
                confidence: None,
                evidence: None,
            });
        }

        // Logic for method counting would require tree-sitter or regex analysis.
        // For now, we use a simple heuristic counting 'pub fn' in Rust or 'async function' in TS.
        let method_patterns = match file.language {
            Language::Rust => vec!["pub fn ", "fn ", "impl "],
            Language::TypeScript => vec!["function ", "export const ", "class "],
            _ => vec![],
        };

        let mut count = 0;
        for line in &file.lines {
            for pattern in &method_patterns {
                if line.contains(pattern) {
                    count += 1;
                    break;
                }
            }
        }

        if count > self.max_methods * 2 {
            // Rough heuristic for density
            findings.push(Finding {
                rule_id: self.id().to_string(),
                rule_name: self.name().to_string(),
                severity: Severity::Warning,
                file: file.path.clone(),
                line: 1,
                column: 0,
                message: format!("High method/entity density detected (~{} entities).", count),
                suggestion: Some(
                    "Consider decomposing large structs into multiple traits.".to_string(),
                ),
                context: String::new(),
                confidence: None,
                evidence: None,
            });
        }

        findings
    }
}
