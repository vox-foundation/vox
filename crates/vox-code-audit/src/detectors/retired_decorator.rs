use crate::diagnostics::catalog;
use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};
use regex::Regex;

/// Detects retired Vox decorator and import forms per
/// [`AGENTS.md` §Retired Surfaces (LLM Guard)](../../../../../AGENTS.md).
///
/// Each pattern has a canonical replacement that the agent should use instead.
/// This detector is the first slice of the CR-L6 retirement-guard parity gate
/// (council ratified 2026-05-15, D6/D25 in
/// [`v1-llm-target-implementation-plan-2026.md`](../../../../../docs/src/architecture/v1-llm-target-implementation-plan-2026.md)
/// §8.1). The remaining retirement-guard rules (`recall()`, `@capacitor/*`,
/// `axum::serve` in generated apps, `rust-embed` in generated apps,
/// `vox-sherpa-transcribe`) land in implementation-plan P1.4.
///
/// Patterns covered by this detector:
///
/// | Retired form               | Canonical replacement                  |
/// |----------------------------|----------------------------------------|
/// | `@component fn Name()`     | `component Name() { ... }`             |
/// | `@server fn ...`           | `@endpoint(kind: server) fn ...`       |
/// | `@query fn ...`            | `@endpoint(kind: query) fn ...`        |
/// | `@mutation fn ...`         | `@endpoint(kind: mutation) fn ...`     |
/// | `@py.import ...`           | (removed; Python interop retired)      |
///
/// Severity is `Warning` at land; the [vox-language-rules Phase 2 plan](../../../../../docs/src/architecture/vox-language-rules-phase2-lint-extension-2026.md)
/// describes the escalation path to `Error` after one minor version.
pub struct RetiredDecoratorDetector {
    component_fn: Regex,
    server_query_mutation_fn: Regex,
    py_import: Regex,
    supported_langs: Vec<Language>,
}

impl Default for RetiredDecoratorDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl RetiredDecoratorDetector {
    pub fn new() -> Self {
        Self {
            component_fn: Regex::new(r"@component\s+fn\b").expect("valid regex"),
            server_query_mutation_fn: Regex::new(r"@(server|query|mutation)\s+fn\b")
                .expect("valid regex"),
            py_import: Regex::new(r"@py\.import\b").expect("valid regex"),
            supported_langs: vec![Language::Vox],
        }
    }

    fn build_finding(
        &self,
        file: &SourceFile,
        line: usize,
        column: usize,
        message: String,
        suggestion: String,
        rationale: &'static str,
    ) -> Finding {
        Finding {
            rule_id: self.id().to_string(),
            diagnostic_id: self.diagnostic_id().map(str::to_string),
            rule_name: self.name().to_string(),
            severity: Severity::Warning,
            file: file.path.clone(),
            line,
            column,
            message,
            suggestion: Some(suggestion),
            alternatives: vec![],
            rationale: Some(rationale.to_string()),
            context: file.context_around(line, 2),
            confidence: Some(FindingConfidence::High),
            evidence: None,
        }
    }
}

impl DetectionRule for RetiredDecoratorDetector {
    fn id(&self) -> &'static str {
        "retired/decorator-usage"
    }

    fn name(&self) -> &'static str {
        "Retired Decorator Usage Detector"
    }

    fn description(&self) -> &'static str {
        "Detects decorator and import forms retired per AGENTS.md §Retired Surfaces (LLM Guard)."
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn languages(&self) -> &[Language] {
        &self.supported_langs
    }

    fn diagnostic_id(&self) -> Option<&'static str> {
        Some(catalog::RETIRED_DECORATOR_USAGE)
    }

    fn explain(&self) -> &'static str {
        "AGENTS.md §Retired Surfaces lists decorator and import forms retired in favor of \
canonical alternatives. LLMs trained on pre-2026 corpora may emit these; this lint \
catches them at audit time so the agent can rewrite to the canonical form.\n\n\
Retired → Canonical:\n\
  @component fn Name() {...}  →  component Name() {...}\n\
  @server fn ...              →  @endpoint(kind: server) fn ...\n\
  @query fn ...               →  @endpoint(kind: query) fn ...\n\
  @mutation fn ...            →  @endpoint(kind: mutation) fn ...\n\
  @py.import ...               →  Python interop retired; use Vox-native or external HTTP.\n\n\
This detector is part of the CR-L6 retirement-guard parity gate; council ratified \
2026-05-15. Severity escalates to Error one minor version after land."
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        if file.language != Language::Vox {
            return vec![];
        }

        let mut findings = Vec::new();

        for (i, line) in file.lines.iter().enumerate() {
            let line_num = i + 1;
            let trimmed = line.trim();

            // Skip comment-style lines (vox uses `//` and `/*`; some glue scripts use `#`).
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('#') {
                continue;
            }

            if let Some(m) = self.component_fn.find(line) {
                findings.push(self.build_finding(
                    file,
                    line_num,
                    m.start() + 1,
                    "Retired form `@component fn` — use the bare `component` keyword instead."
                        .to_string(),
                    "Replace `@component fn Name()` with `component Name() { ... }`. The bare \
                     `component` keyword is canonical per AGENTS.md §Grammar Unification."
                        .to_string(),
                    "AGENTS.md §Retired Surfaces: `@component fn` was retired during the 2026-Q1 \
                     grammar unification. The bare `component` keyword opens its own scope with \
                     component-specific rules and replaces the decorator+fn pair.",
                ));
            }

            if let Some(caps) = self.server_query_mutation_fn.captures(line) {
                let kind = caps
                    .get(1)
                    .map(|m| m.as_str())
                    .expect("regex group 1 always present");
                let full = caps
                    .get(0)
                    .expect("regex group 0 always present");
                findings.push(self.build_finding(
                    file,
                    line_num,
                    full.start() + 1,
                    format!(
                        "Retired form `@{kind} fn` — use `@endpoint(kind: {kind}) fn ...` instead."
                    ),
                    format!(
                        "Replace `@{kind} fn` with `@endpoint(kind: {kind}) fn`. The unified \
                         `@endpoint(kind: server | query | mutation)` decorator subsumed the \
                         three separate forms."
                    ),
                    "AGENTS.md §Retired Surfaces: `@server`, `@query`, and `@mutation` were \
                     collapsed into the unified `@endpoint(kind: ...)` decorator for grammar \
                     parsimony and to centralize endpoint policy (auth, CORS, rate limits).",
                ));
            }

            if let Some(m) = self.py_import.find(line) {
                findings.push(self.build_finding(
                    file,
                    line_num,
                    m.start() + 1,
                    "Retired form `@py.import` — Python interop has been removed.".to_string(),
                    "Replace with a Vox-native equivalent or call the upstream service via HTTP. \
                     If Python automation glue is needed, port the script to `.vox` per AGENTS.md \
                     §VoxScript-First Glue Code."
                        .to_string(),
                    "AGENTS.md §Retired Surfaces + §VoxScript-First Glue Code: Python is no \
                     longer a Vox glue surface. `@py.import` directives leak Python-side state \
                     into the Vox compiler and cannot be analyzed by the effect system.",
                ));
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
    fn flags_at_component_fn() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@component fn Dashboard() {}");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1, "should flag `@component fn`");
        assert!(findings[0].message.contains("@component fn"));
        assert_eq!(findings[0].severity, Severity::Warning);
        assert_eq!(
            findings[0].diagnostic_id.as_deref(),
            Some(catalog::RETIRED_DECORATOR_USAGE)
        );
    }

    #[test]
    fn does_not_flag_bare_component_keyword() {
        let d = RetiredDecoratorDetector::new();
        let f = source("component Dashboard() {}");
        let findings = d.detect(&f, None);
        assert!(
            findings.is_empty(),
            "bare `component` keyword is canonical, not retired"
        );
    }

    #[test]
    fn flags_at_server_fn() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@server fn list_items() {}");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1, "should flag `@server fn`");
        assert!(findings[0].message.contains("@server fn"));
        assert!(
            findings[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("kind: server")
        );
    }

    #[test]
    fn flags_at_query_fn() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@query fn list_items() {}");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1, "should flag `@query fn`");
        assert!(findings[0].message.contains("@query fn"));
        assert!(
            findings[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("kind: query")
        );
    }

    #[test]
    fn flags_at_mutation_fn() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@mutation fn add_item() {}");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1, "should flag `@mutation fn`");
        assert!(findings[0].message.contains("@mutation fn"));
        assert!(
            findings[0]
                .suggestion
                .as_ref()
                .unwrap()
                .contains("kind: mutation")
        );
    }

    #[test]
    fn does_not_flag_canonical_endpoint_with_kind() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@endpoint(kind: server) fn list_items() {}");
        let findings = d.detect(&f, None);
        assert!(
            findings.is_empty(),
            "canonical `@endpoint(kind: ...)` form should not fire"
        );
    }

    #[test]
    fn flags_at_py_import() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@py.import pandas as pd");
        let findings = d.detect(&f, None);
        assert_eq!(findings.len(), 1, "should flag `@py.import`");
        assert!(findings[0].message.contains("@py.import"));
    }

    #[test]
    fn ignores_comment_lines() {
        let d = RetiredDecoratorDetector::new();
        let f = source(
            "// @component fn Dashboard() {}\n// @server fn x() {}\n// @py.import pandas",
        );
        let findings = d.detect(&f, None);
        assert!(findings.is_empty(), "comment lines should be skipped");
    }

    #[test]
    fn ignores_block_comment_lines() {
        let d = RetiredDecoratorDetector::new();
        let f = source("/* @component fn Dashboard() {} */");
        let findings = d.detect(&f, None);
        assert!(findings.is_empty(), "block-comment lines should be skipped");
    }

    #[test]
    fn does_not_fire_on_rust_files() {
        let d = RetiredDecoratorDetector::new();
        let f = SourceFile::new(
            PathBuf::from("test.rs"),
            "@component fn Dashboard() {}".to_string(),
        );
        let findings = d.detect(&f, None);
        assert!(findings.is_empty(), "rust files should be ignored");
    }

    #[test]
    fn flags_all_three_endpoint_kinds_independently() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@server fn a() {}\n@query fn b() {}\n@mutation fn c() {}");
        let findings = d.detect(&f, None);
        assert_eq!(
            findings.len(),
            3,
            "should flag all three retired endpoint forms independently"
        );
    }

    #[test]
    fn flags_mixed_retirement_patterns_in_one_file() {
        let d = RetiredDecoratorDetector::new();
        let f = source(
            "@component fn Dashboard() {}\n\
             @server fn list() {}\n\
             @py.import os",
        );
        let findings = d.detect(&f, None);
        assert_eq!(
            findings.len(),
            3,
            "should flag component + server + py.import on three separate lines"
        );
    }

    #[test]
    fn finding_has_high_confidence() {
        let d = RetiredDecoratorDetector::new();
        let f = source("@component fn Foo() {}");
        let findings = d.detect(&f, None);
        assert_eq!(findings[0].confidence, Some(FindingConfidence::High));
    }
}
