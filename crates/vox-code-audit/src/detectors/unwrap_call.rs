//! Heuristic detection of `.unwrap()` in Rust sources (informational).
//!
//! Skips integration test trees, `tests.rs` files, `#[cfg(test)] mod tests { ... }` bodies, and
//! `#[cfg(test)]` / `#[test]` lines. Intended to nudge toward `?`, `expect(\"…\")`, or explicit
//! error handling — not a substitute for Clippy in CI.

use crate::rules::{DetectionRule, Finding, Language, Severity, SourceFile};
use regex::Regex;

/// Flags `.unwrap()` calls outside obvious test contexts (informational).
pub struct UnwrapCallDetector {
    re: Regex,
}

impl UnwrapCallDetector {
    /// Builds the detector with a compiled pattern for `.unwrap()`.
    pub fn new() -> Self {
        Self {
            re: Regex::new(r"\.unwrap\s*\(\s*\)").expect("unwrap detector regex"),
        }
    }

    fn should_skip_file(path: &std::path::Path) -> bool {
        let s = path.to_string_lossy();
        if s.contains("/tests/")
            || s.contains("\\tests\\")
            || s.ends_with("_test.rs")
            || s.ends_with("tests.rs")
        {
            return true;
        }
        path.file_name()
            .and_then(|n| n.to_str())
            .is_some_and(|n| n.ends_with("_tests_body.rs"))
    }

    fn make_finding(file: &SourceFile, line: usize, message: &str) -> Finding {
        Finding {
            rule_id: "rust/unwrap-call".to_string(),
            diagnostic_id: None,
            rule_name: "Unwrap call (heuristic)".to_string(),
            severity: Severity::Info,
            file: file.path.clone(),
            line,
            column: 0,
            message: message.to_string(),
            suggestion: Some(
                "Prefer `?`, `context`, or `expect(\"static reason\")` over `.unwrap()` in production paths."
                    .into(),
            ),
            alternatives: vec![],
            rationale: None,
            context: file.context_around(line, 2),
            confidence: None,
            evidence: None,
        }
    }
}

impl Default for UnwrapCallDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// `#[cfg(...)]` that enables `test` but is not `not(test)`.
fn line_enables_test_cfg(line: &str) -> bool {
    let s = line.trim_start();
    if !s.starts_with("#[") || !s.contains("cfg(") {
        return false;
    }
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.contains("not(test)") {
        return false;
    }
    compact.contains("test")
}

fn line_is_mod_tests_with_brace(line: &str) -> bool {
    let compact: String = line.chars().filter(|c| !c.is_whitespace()).collect();
    compact.contains("modtests{") || (compact.contains("modtests") && compact.contains('{'))
}

fn brace_delta(line: &str) -> i32 {
    let opens = line.chars().filter(|&c| c == '{').count() as i32;
    let closes = line.chars().filter(|&c| c == '}').count() as i32;
    opens - closes
}

impl DetectionRule for UnwrapCallDetector {
    fn id(&self) -> &'static str {
        "rust/unwrap-call"
    }

    fn name(&self) -> &'static str {
        "UnwrapCallDetector"
    }

    fn description(&self) -> &'static str {
        "Heuristic: `.unwrap()` in Rust (info); skips common test paths and cfg(test) lines."
    }

    fn severity(&self) -> Severity {
        Severity::Info
    }

    fn languages(&self) -> &[Language] {
        &[Language::Rust]
    }

    fn detect(
        &self,
        file: &SourceFile,
        _rust: Option<&crate::analysis::RustFileContext>,
    ) -> Vec<Finding> {
        if Self::should_skip_file(&file.path) {
            return Vec::new();
        }
        let mut out = Vec::new();
        let mut skip_test_mod_depth = 0i32;
        let mut expect_mod_tests_after_cfg_test = false;

        for (i, line) in file.lines.iter().enumerate() {
            let line_no = i + 1;

            if skip_test_mod_depth > 0 {
                skip_test_mod_depth += brace_delta(line);
                if skip_test_mod_depth <= 0 {
                    skip_test_mod_depth = 0;
                }
                continue;
            }

            if expect_mod_tests_after_cfg_test {
                let t = line.trim_start();
                if t.is_empty() || t.starts_with("//") {
                    continue;
                }
                expect_mod_tests_after_cfg_test = false;
                if line_is_mod_tests_with_brace(line) {
                    let d = brace_delta(line);
                    if d > 0 {
                        skip_test_mod_depth = d;
                    }
                    continue;
                }
            }

            if line_enables_test_cfg(line) {
                if line_is_mod_tests_with_brace(line) {
                    let d = brace_delta(line);
                    if d > 0 {
                        skip_test_mod_depth = d;
                    }
                } else {
                    expect_mod_tests_after_cfg_test = true;
                }
                continue;
            }

            if line.contains("#[cfg(test)]")
                || line.contains("#[test]")
                || line.trim_start().starts_with("//")
            {
                continue;
            }
            if self.re.is_match(line) {
                out.push(Self::make_finding(
                    file,
                    line_no,
                    "`.unwrap()` — consider explicit error handling",
                ));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;
    use crate::rules::Language;

    fn sf(lines: &[&str]) -> SourceFile {
        SourceFile {
            path: PathBuf::from("crates/demo/src/lib.rs"),
            language: Language::Rust,
            content: lines.join("\n"),
            lines: lines.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    #[test]
    fn skips_cfg_test_mod_block_body() {
        let lines = vec![
            "pub fn f() { x.unwrap(); }",
            "#[cfg(test)]",
            "mod tests {",
            "    #[test]",
            "    fn t() { let _ = y.unwrap(); }",
            "}",
            "pub fn g() { z.unwrap(); }",
        ];
        let file = sf(&lines);
        let d = UnwrapCallDetector::new();
        let hits: Vec<_> = d.detect(&file, None).into_iter().map(|f| f.line).collect();
        assert_eq!(hits, vec![1, 7]);
    }

    #[test]
    fn skips_single_line_cfg_test_mod() {
        let lines = vec![
            "#[cfg(test)] mod tests { fn x() { a.unwrap(); } }",
            "pub fn prod() { b.unwrap(); }",
        ];
        let file = sf(&lines);
        let d = UnwrapCallDetector::new();
        let hits: Vec<_> = d.detect(&file, None).into_iter().map(|f| f.line).collect();
        assert_eq!(hits, vec![2]);
    }

    #[test]
    fn skips_all_feature_gated_test_mod() {
        let lines = vec![
            "#[cfg(all(test, feature = \"foo\"))]",
            "mod tests {",
            "    fn x() { a.unwrap(); }",
            "}",
        ];
        let file = sf(&lines);
        let d = UnwrapCallDetector::new();
        assert!(d.detect(&file, None).is_empty());
    }

    #[test]
    fn does_not_skip_not_test_cfg() {
        let lines = vec!["#[cfg(not(test))]", "fn x() { a.unwrap(); }"];
        let file = sf(&lines);
        let d = UnwrapCallDetector::new();
        assert_eq!(d.detect(&file, None).len(), 1);
    }

    #[test]
    fn skips_tests_body_include_files_by_name() {
        let file = SourceFile {
            path: PathBuf::from("crates/vox-cli/src/commands/mens/populi/gpu_tests_body.rs"),
            language: Language::Rust,
            content: String::new(),
            lines: vec!["fn x() { a.unwrap(); }".into()],
        };
        assert!(UnwrapCallDetector::should_skip_file(&file.path));
        let d = UnwrapCallDetector::new();
        assert!(d.detect(&file, None).is_empty());
    }
}
