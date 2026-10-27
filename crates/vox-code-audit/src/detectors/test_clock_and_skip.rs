//! Detector `vox/test/flaky-wait-and-unreasoned-skip` (`T3-TOESTUB-RULES-01`):
//! Flags real-clock sleeps in tests, `.only` test focuses, and unreasoned `#[ignore]` / `.skip`.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct TestClockAndSkipDetector {
    supported_langs: Vec<Language>,
}

impl Default for TestClockAndSkipDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TestClockAndSkipDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::Rust, Language::TypeScript],
        }
    }

    fn is_test_path(path: &std::path::Path) -> bool {
        let s = path.to_string_lossy();
        s.contains("/tests/")
            || s.ends_with("_test.rs")
            || s.ends_with(".test.ts")
            || s.ends_with(".test.tsx")
            || s.ends_with(".spec.ts")
    }
}

impl DetectionRule for TestClockAndSkipDetector {
    fn id(&self) -> &'static str {
        "vox/test/flaky-wait-and-unreasoned-skip"
    }

    fn name(&self) -> &'static str {
        "Flaky Test Sleep and Unreasoned Skip Detector"
    }

    fn description(&self) -> &'static str {
        "Flags std::thread::sleep / waitForTimeout in tests, committed .only focuses, and bare #[ignore] without a reason."
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
        let in_test_file = Self::is_test_path(&file.path);
        let mut in_rust_cfg_test = false;

        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if file.language == Language::Rust && trimmed.starts_with("#[cfg(test)]") {
                in_rust_cfg_test = true;
            }
            let in_test_scope = in_test_file || in_rust_cfg_test;
            let prev_has_comment = idx > 0 && file.lines[idx - 1].trim().starts_with("//");

            let bare_ignore =
                file.language == Language::Rust && trimmed == "#[ignore]" && !prev_has_comment;
            let focus_leak = file.language == Language::TypeScript
                && (trimmed.starts_with("it.only(")
                    || trimmed.starts_with("describe.only(")
                    || trimmed.starts_with("test.only("));
            let unreasoned_ts_skip = file.language == Language::TypeScript
                && (trimmed.starts_with("it.skip(")
                    || trimmed.starts_with("test.skip(")
                    || trimmed.starts_with("describe.skip(")
                    || trimmed.starts_with("xit(")
                    || trimmed.starts_with("it.todo(")
                    || trimmed.starts_with("test.todo("))
                && !trimmed.contains("//")
                && !prev_has_comment;
            // Sleep / waitForTimeout inside test scope — including polling loops —
            // is flaky; also catch TS `await new Promise(...requestAnimationFrame)`.
            let flaky_sleep = in_test_scope
                && !trimmed.starts_with("//")
                && !trimmed.contains("toestub-ignore")
                && (trimmed.contains("std::thread::sleep(")
                    || trimmed.contains("thread::sleep(")
                    || trimmed.contains("page.waitForTimeout(")
                    || trimmed.contains("waitForTimeout(")
                    || (file.language == Language::TypeScript
                        && trimmed.contains("requestAnimationFrame")
                        && (trimmed.contains("new Promise") || trimmed.contains("await "))));

            if bare_ignore || focus_leak || unreasoned_ts_skip || flaky_sleep {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::TEST_FLAKY_WAIT_AND_UNREASONED_SKIP
                            .to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message:
                        "Flaky test wait (`sleep`/`waitForTimeout`), `.only` focus leak, or unreasoned `#[ignore]` detected."
                            .to_string(),
                    suggestion: Some(
                        "Use deterministic virtual clocks/signals, remove `.only`, or add `#[ignore = \"reason\"]`."
                            .to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: None,
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
    fn catches_bare_ignore_and_only_focus() {
        let det = TestClockAndSkipDetector::new();
        let f = SourceFile::new(
            PathBuf::from("tests/sample.test.ts"),
            "it.only('focused', async () => { await page.waitForTimeout(500); });\nit.skip('unreasoned', () => {});\n".to_string(),
        );
        // Same-line `it.only` + `waitForTimeout` coalesce to one finding.
        assert_eq!(det.detect(&f, None).len(), 2);
    }

    #[test]
    fn catches_sleep_inside_cfg_test_polling_loop() {
        let det = TestClockAndSkipDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-cli/src/foo.rs"),
            "#[cfg(test)]\nmod tests {\n  #[test]\n  fn waits() {\n    loop {\n      std::thread::sleep(std::time::Duration::from_millis(10));\n    }\n  }\n}\n"
                .to_string(),
        );
        assert_eq!(det.detect(&f, None).len(), 1);
    }

    #[test]
    fn catches_todo_and_xit_without_reason() {
        let det = TestClockAndSkipDetector::new();
        let f = SourceFile::new(
            PathBuf::from("src/widget.test.ts"),
            "xit('legacy', () => {});\ntest.todo('later');\n".to_string(),
        );
        assert_eq!(det.detect(&f, None).len(), 2);
    }
}
