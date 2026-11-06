//! Detector `vox/test/wall-clock` — Date/Instant/performance wall-clock in tests
//! (t3code `no-wall-clock-in-tests` + `no-real-time-wait-in-tests` subset).

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct TestWallClockDetector {
    langs: Vec<Language>,
}

impl Default for TestWallClockDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TestWallClockDetector {
    pub fn new() -> Self {
        Self {
            langs: vec![Language::Rust, Language::TypeScript],
        }
    }
}

fn in_test_path(path: &str) -> bool {
    let p = path.replace('\\', "/");
    p.contains("/tests/")
        || p.contains("/__tests__/")
        || p.contains(".test.")
        || p.contains(".spec.")
        || p.contains("_test.rs")
        || p.ends_with("/tests.rs")
}

impl DetectionRule for TestWallClockDetector {
    fn id(&self) -> &'static str {
        "vox/test/wall-clock"
    }
    fn name(&self) -> &'static str {
        "Wall-Clock / Real-Time Wait In Tests Detector"
    }
    fn description(&self) -> &'static str {
        "Flags Instant::now / SystemTime::now / Date.now / performance.now / tokio::time::sleep in tests."
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
        let path = file.path.to_string_lossy().replace('\\', "/");
        if !in_test_path(&path) && file.language != Language::Rust {
            // Rust: also scan #[cfg(test)] modules via path heuristic only for TS;
            // for Rust non-test paths, still scan for cfg(test) blocks lightly via line context.
        }
        let scan = in_test_path(&path) || path.ends_with(".rs");
        if !scan {
            return out;
        }
        // Scope wall-clock checks to test paths, or to `#[cfg(test)]` modules only
        // (brace-depth bounded — do not leave the flag sticky for the rest of the file).
        let mut in_cfg_test = in_test_path(&path);
        let mut cfg_test_depth: i32 = 0;
        let mut awaiting_cfg_test_block = false;
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.contains("#[cfg(test)]") {
                awaiting_cfg_test_block = true;
            }
            if awaiting_cfg_test_block
                && (trimmed.starts_with("mod ")
                    || trimmed.starts_with("fn ")
                    || trimmed.contains("{"))
            {
                in_cfg_test = true;
                awaiting_cfg_test_block = false;
                cfg_test_depth = 0;
            }
            if in_cfg_test && !in_test_path(&path) {
                cfg_test_depth += trimmed.chars().filter(|c| *c == '{').count() as i32;
                cfg_test_depth -= trimmed.chars().filter(|c| *c == '}').count() as i32;
                if cfg_test_depth <= 0 && trimmed.contains('}') {
                    in_cfg_test = false;
                    cfg_test_depth = 0;
                }
            }
            if !in_cfg_test && !in_test_path(&path) {
                continue;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            let hit = trimmed.contains("Instant::now(")
                || trimmed.contains("SystemTime::now(")
                || trimmed.contains("Date.now(")
                || trimmed.contains("performance.now(")
                || trimmed.contains("tokio::time::sleep(")
                || trimmed.contains("page.waitForTimeout(")
                || trimmed.contains("waitForTimeout(");
            if hit {
                out.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::TEST_WALL_CLOCK.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message: "Wall-clock or real-time wait in a test; use a test clock / fixed epoch / await a milestone.".to_string(),
                    suggestion: Some(
                        "Prefer deterministic clocks (TestClock / mock time) over Instant::now / sleep / waitForTimeout.".to_string(),
                    ),
                    alternatives: Vec::new(),
                    rationale: Some(
                        "t3code no-wall-clock-in-tests / no-real-time-wait-in-tests / no-frame-waits.".to_string(),
                    ),
                    context: file.context_around(line_num, 1),
                    confidence: Some(FindingConfidence::Medium),
                    evidence: None,
                });
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
    fn flags_instant_in_test_file() {
        let d = TestWallClockDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/foo/tests/bar.rs"),
            "let t = Instant::now();\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }

    #[test]
    fn flags_wait_for_timeout_in_e2e() {
        let d = TestWallClockDetector::new();
        let f = SourceFile::new(
            PathBuf::from("crates/vox-gui/ui/e2e/foo.spec.ts"),
            "await page.waitForTimeout(500);\n".to_string(),
        );
        assert!(!d.detect(&f, None).is_empty());
    }

    #[test]
    fn does_not_flag_production_after_cfg_test_module() {
        let d = TestWallClockDetector::new();
        let src = concat!(
            "#[cfg(test)]\n",
            "mod tests {\n",
            "  #[test] fn t() {}\n",
            "}\n",
            "pub fn prod() { let _ = Instant::now(); }\n",
        );
        let f = SourceFile::new(PathBuf::from("crates/foo/src/lib.rs"), src.to_string());
        assert!(
            d.detect(&f, None).is_empty(),
            "Instant::now after closed #[cfg(test)] mod must not be flagged"
        );
    }
}
