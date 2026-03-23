//! Structured test diagnostics for Vox compiler and tooling pipelines
//! (`VOX_DIAGNOSIS` JSON lines).
//!
//! This module was the original content of `vox-test-harness`. It is kept
//! here for CI and doctor tooling that consumes `VOX_DIAGNOSIS:` lines.

use serde::{Deserialize, Serialize};

/// Structured failure record emitted to stdout for CI and doctor tooling.
#[derive(Debug, Serialize, Deserialize)]
pub struct TestDiagnosis {
    /// Name of the failing test (Rust test name or scenario id).
    pub test: String,
    /// Crate under test.
    pub crate_name: String,
    /// Source file path.
    pub file: String,
    /// 1-based line number in `file`.
    pub line: usize,
    /// Which compiler phase failed.
    pub category: TestCategory,
    /// Expected snippet or message, if applicable.
    pub expected: Option<String>,
    /// Actual snippet or message, if applicable.
    pub actual: Option<String>,
    /// Related declaration names for cross-navigation.
    #[serde(default)]
    pub related_decls: Vec<String>,
    /// Suggested follow-up files for humans or agents.
    #[serde(default)]
    pub suggested_files: Vec<String>,
}

/// Compiler / tooling stage associated with a failure.
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestCategory {
    /// Lexer tokenization.
    Lexer,
    /// Parser / AST build.
    Parser,
    /// Type checker.
    Typeck,
    /// TypeScript backend.
    CodegenTs,
    /// Rust backend.
    CodegenRust,
    /// Multi-agent orchestrator.
    Orchestrator,
    /// Runtime / process tests.
    Runtime,
    /// MCP integration.
    Mcp,
    /// Uncategorized failure.
    Unknown,
}

impl TestDiagnosis {
    /// Build a diagnosis with required fields; diff and related metadata start empty.
    pub fn new(
        test: impl Into<String>,
        crate_name: impl Into<String>,
        file: impl Into<String>,
        line: usize,
        category: TestCategory,
    ) -> Self {
        Self {
            test: test.into(),
            crate_name: crate_name.into(),
            file: file.into(),
            line,
            category,
            expected: None,
            actual: None,
            related_decls: Vec::new(),
            suggested_files: Vec::new(),
        }
    }

    /// Attach expected vs actual strings for diff-oriented reporting.
    pub fn with_diff(mut self, expected: impl Into<String>, actual: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self.actual = Some(actual.into());
        self
    }

    /// Print one line `VOX_DIAGNOSIS: {json}` for machine consumption.
    pub fn emit_json(&self) {
        if let Ok(json) = serde_json::to_string(self) {
            println!("VOX_DIAGNOSIS: {}", json);
        }
    }
}
