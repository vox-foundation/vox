//! Heuristic detection of `.unwrap()` in Rust sources (informational).
//!
//! Skips integration test trees and `#[cfg(test)]` lines. Intended to nudge toward
//! `?`, `expect(\"…\")`, or explicit error handling — not a substitute for Clippy in CI.

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
        s.contains("/tests/")
            || s.contains("\\tests\\")
            || s.ends_with("_test.rs")
            || s.ends_with("tests.rs")
    }

    fn make_finding(file: &SourceFile, line: usize, message: &str) -> Finding {
        Finding {
            rule_id: "rust/unwrap-call".to_string(),
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
            context: file.context_around(line, 2),
        }
    }
}

impl Default for UnwrapCallDetector {
    fn default() -> Self {
        Self::new()
    }
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

    fn detect(&self, file: &SourceFile) -> Vec<Finding> {
        if Self::should_skip_file(&file.path) {
            return Vec::new();
        }
        let mut out = Vec::new();
        for (i, line) in file.lines.iter().enumerate() {
            if line.contains("#[cfg(test)]")
                || line.contains("#[test]")
                || line.trim_start().starts_with("//")
            {
                continue;
            }
            if self.re.is_match(line) {
                out.push(Self::make_finding(
                    file,
                    i + 1,
                    "`.unwrap()` — consider explicit error handling",
                ));
            }
        }
        out
    }
}
