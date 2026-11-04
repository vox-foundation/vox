//! Detector `vox/test/in-loop` — `#[test]` / `it(`/`test(` nested under for-loops.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct TestInLoopDetector {
    langs: Vec<Language>,
}

impl Default for TestInLoopDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TestInLoopDetector {
    pub fn new() -> Self {
        Self {
            langs: vec![Language::Rust, Language::TypeScript],
        }
    }
}

impl DetectionRule for TestInLoopDetector {
    fn id(&self) -> &'static str {
        "vox/test/in-loop"
    }
    fn name(&self) -> &'static str {
        "Test Declared Inside Loop Detector"
    }
    fn description(&self) -> &'static str {
        "Flags test cases nested inside for/for-of loops; prefer parameterized tables."
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn languages(&self) -> &[Language] {
        &self.langs
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust_ctx: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        let mut out = Vec::new();
        let mut loop_depth = 0i32;
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("//") {
                continue;
            }
            // crude brace/loop heuristic sufficient for CI ratchet
            if trimmed.starts_with("for ")
                || trimmed.starts_with("for(")
                || trimmed.contains(" for (")
                || trimmed.starts_with("for await")
            {
                loop_depth += 1;
            }
            if trimmed == "}" || trimmed.starts_with("}") {
                loop_depth = (loop_depth - 1).max(0);
            }
            if loop_depth > 0 {
                let testish = trimmed.starts_with("#[test]")
                    || trimmed.starts_with("it(")
                    || trimmed.starts_with("it.only(")
                    || trimmed.starts_with("test(")
                    || trimmed.starts_with("test.only(")
                    || trimmed.starts_with("describe(");
                if testish {
                    out.push(Finding {
                        rule_id: self.id().to_string(),
                        diagnostic_id: Some(crate::diagnostics::catalog::TEST_IN_LOOP.to_string()),
                        rule_name: self.name().to_string(),
                        severity: self.severity(),
                        file: file.path.clone(),
                        line: line_num,
                        column: 0,
                        message:
                            "Test declaration inside a loop; use a parameterized table instead."
                                .to_string(),
                        suggestion: Some(
                            "Move cases to a table/`#[test_case]` / `it.each` outside the loop."
                                .to_string(),
                        ),
                        alternatives: Vec::new(),
                        rationale: Some("t3code no-test-in-loop.".to_string()),
                        context: file.context_around(line_num, 1),
                        confidence: Some(FindingConfidence::Low),
                        evidence: None,
                    });
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn flags_it_inside_for() {
        let d = TestInLoopDetector::new();
        let f = SourceFile::new(
            PathBuf::from("src/foo.spec.ts"),
            "for (const x of xs) {\n  it('case', () => {});\n}\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }
}
