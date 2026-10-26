//! Detector `vox/lint/unreasoned-suppression` (`T3-TOESTUB-RULES-01`):
//! Flags `#[allow(...)]`, `eslint-disable`, `oxlint-disable`, `@ts-ignore`, and `@ts-expect-error`
//! that lack an explicit rationale (`reason = "..."` or `-- <reason>`).

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct UnreasonedSuppressionDetector {
    supported_langs: Vec<Language>,
}

impl Default for UnreasonedSuppressionDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl UnreasonedSuppressionDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::Rust, Language::TypeScript, Language::Vox],
        }
    }
}

impl DetectionRule for UnreasonedSuppressionDetector {
    fn id(&self) -> &'static str {
        "vox/lint/unreasoned-suppression"
    }

    fn name(&self) -> &'static str {
        "Unreasoned Lint or Type Suppression Detector"
    }

    fn description(&self) -> &'static str {
        "Flags #[allow(...)], @ts-ignore, @ts-expect-error, and eslint/oxlint-disable without an explicit reason."
    }

    fn severity(&self) -> Severity {
        Severity::Warning
    }

    fn languages(&self) -> &[Language] {
        &self.supported_langs
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut findings = Vec::new();
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            let prev_has_comment = idx > 0 && {
                let prev = file.lines[idx - 1].trim();
                prev.starts_with("//") && !prev.contains("toestub-ignore")
            };

            let is_rust_allow_or_expect = file.language == Language::Rust
                && (trimmed.starts_with("#[allow(")
                    || trimmed.starts_with("#![allow(")
                    || trimmed.starts_with("#[expect(")
                    || trimmed.starts_with("#![expect("));
            let is_unreasoned_rust_allow = if is_rust_allow_or_expect {
                let mut window = trimmed.to_string();
                let mut offset = 1;
                while !window.contains(")]") && idx + offset < file.lines.len() && offset <= 5 {
                    window.push(' ');
                    window.push_str(file.lines[idx + offset].trim());
                    offset += 1;
                }
                !window.contains("reason =") && !window.contains("//") && !prev_has_comment
            } else {
                false
            };

            let is_unreasoned_toestub_ignore = trimmed.contains("toestub-ignore(")
                && !trimmed.contains("-- ")
                && !trimmed.contains("— ")
                && !trimmed.contains("reason");

            let is_ts_comment = trimmed.starts_with("//") || trimmed.starts_with("/*");
            let has_inline_ts_explanation = if let Some(rest) = trimmed
                .split("@ts-expect-error")
                .nth(1)
                .or_else(|| trimmed.split("@ts-ignore").nth(1))
            {
                rest.trim().len() > 3
            } else {
                false
            };
            let is_unreasoned_ts_suppress = file.language == Language::TypeScript
                && is_ts_comment
                && (trimmed.contains("@ts-ignore")
                    || trimmed.contains("@ts-expect-error")
                    || trimmed.contains("eslint-disable")
                    || trimmed.contains("oxlint-disable"))
                && !trimmed.contains("-- ")
                && !trimmed.contains("reason:")
                && !has_inline_ts_explanation
                && !prev_has_comment;

            if is_unreasoned_rust_allow || is_unreasoned_toestub_ignore || is_unreasoned_ts_suppress
            {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::LINT_UNREASONED_SUPPRESSION.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message:
                        "Lint or type suppression lacks an explicit reason (`reason = \"...\"` or `-- <reason>`)."
                            .to_string(),
                    suggestion: Some(
                        "Add `reason = \"...\"` in Rust or `-- <reason>` in TypeScript comments."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: Some(
                        "Unreasoned suppressions hide technical debt and prevent automated retirement audits."
                            .to_string(),
                    ),
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::High),
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

    #[test]
    fn flags_unreasoned_rust_and_ts_suppressions() {
        let det = UnreasonedSuppressionDetector::new();
        let rs_bad = SourceFile::new(
            PathBuf::from("src/foo.rs"),
            "#[allow(dead_code)]\npub fn x() {}\n".to_string(),
        );
        assert_eq!(det.detect(&rs_bad, None).len(), 1);

        let rs_good = SourceFile::new(
            PathBuf::from("src/foo.rs"),
            "#[allow(dead_code, reason = \"used by macro\")]\npub fn x() {}\n".to_string(),
        );
        assert!(det.detect(&rs_good, None).is_empty());

        let ts_bad = SourceFile::new(
            PathBuf::from("src/app.ts"),
            "// @ts-expect-error\nconst x: number = 'bad';\n".to_string(),
        );
        assert_eq!(det.detect(&ts_bad, None).len(), 1);
    }

    #[test]
    fn accepts_expect_reason_inline_ts_explanation_and_vox_toestub() {
        let det = UnreasonedSuppressionDetector::new();
        let rs_expect = SourceFile::new(
            PathBuf::from("src/foo.rs"),
            "#[expect(dead_code, reason = \"retained for ABI\")]\npub fn x() {}\n".to_string(),
        );
        assert!(det.detect(&rs_expect, None).is_empty());

        let rs_bare_expect = SourceFile::new(
            PathBuf::from("src/foo.rs"),
            "#[expect(dead_code)]\npub fn x() {}\n".to_string(),
        );
        assert_eq!(det.detect(&rs_bare_expect, None).len(), 1);

        let ts_explained = SourceFile::new(
            PathBuf::from("src/app.ts"),
            "// @ts-expect-error simulate missing matchMedia\nconst x = window.matchMedia;\n"
                .to_string(),
        );
        assert!(det.detect(&ts_explained, None).is_empty());

        let vox_bare = SourceFile::new(
            PathBuf::from("scripts/demo.vox"),
            "// toestub-ignore(stub)\nfn demo() {}\n".to_string(),
        );
        assert_eq!(det.detect(&vox_bare, None).len(), 1);

        let vox_reasoned = SourceFile::new(
            PathBuf::from("scripts/demo.vox"),
            "// toestub-ignore(stub) -- intentional corpus fixture\nfn demo() {}\n".to_string(),
        );
        assert!(det.detect(&vox_reasoned, None).is_empty());
    }
    #[test]
    fn pr_diff_style_unreasoned_allow_fails() {
        // Simulates a PR hunk that introduces #[allow(dead_code)] without reason —
        // ratchet: detector must flag it (CI can scan changed files only).
        let det = UnreasonedSuppressionDetector::new();
        let file = SourceFile::new(
            std::path::PathBuf::from("crates/example/src/lib.rs"),
            "#[allow(dead_code)]
fn unused() {}
".to_string(),
        );
        let hits = det.detect(&file, None);
        assert!(
            !hits.is_empty(),
            "unreasoned #[allow] in a PR-shaped snippet must fail closed"
        );
    }

}
