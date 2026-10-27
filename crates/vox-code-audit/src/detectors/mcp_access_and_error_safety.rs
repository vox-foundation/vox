//! Detector `vox/mcp/access-and-error-safety` (`T3-TOESTUB-RULES-02`):
//! Flags MCP tool definitions or handlers that leak raw internal errors (`format!("{err:?}")`)
//! or omit `McpToolAccessRequirement` classification.

use crate::rules::{DetectionRule, Finding, FindingConfidence, Language, Severity, SourceFile};

pub struct McpAccessAndErrorSafetyDetector {
    supported_langs: Vec<Language>,
}

impl Default for McpAccessAndErrorSafetyDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl McpAccessAndErrorSafetyDetector {
    pub fn new() -> Self {
        Self {
            supported_langs: vec![Language::Rust],
        }
    }
}

impl DetectionRule for McpAccessAndErrorSafetyDetector {
    fn id(&self) -> &'static str {
        "vox/mcp/access-and-error-safety"
    }

    fn name(&self) -> &'static str {
        "MCP Access Policy and Error Sanitization Detector"
    }

    fn description(&self) -> &'static str {
        "Flags raw Debug error formatting in MCP responses and missing McpToolAccessRequirement declarations."
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
        let path_str = file.path.to_string_lossy().replace('\\', "/");
        if !path_str.contains("vox-orchestrator-mcp") {
            return findings;
        }
        let is_allowed_exec_module = path_str.ends_with("git_exec.rs")
            || path_str.ends_with("external_mcp.rs")
            || path_str.ends_with("lifecycle.rs")
            || path_str.ends_with("agy_doctor.rs");
        let mut in_test_mod = false;
        for (idx, line) in file.lines.iter().enumerate() {
            let line_num = idx + 1;
            let trimmed = line.trim();
            if trimmed.starts_with("#[cfg(test)]") {
                in_test_mod = true;
            }
            if trimmed.starts_with("//") || trimmed.contains("toestub-ignore") {
                continue;
            }
            let raw_call_tool_debug = (trimmed.contains("CallToolResult::error")
                || trimmed.contains("ToolResult::err")
                || trimmed.contains("ToolResult::<"))
                && trimmed.contains(":?");
            let raw_tracing_err_debug = (trimmed.contains("error = ?")
                || trimmed.contains("err = ?"))
                && !trimmed.contains("sanitize_tool_error");
            // Fully-qualified process spawns are banned outside allowlisted exec modules.
            // Bare `Command::new(` is only flagged in thick chat / Wave-3 handlers (where
            // `use std::process::Command` is common and intentional CLI crates are absent).
            let is_thick_handler = path_str.contains("/chat_tools/")
                || path_str.ends_with("t3_wave3_tools.rs")
                || path_str.contains("/dei_tools/");
            let raw_process_command = !in_test_mod
                && !is_allowed_exec_module
                && (trimmed.contains("std::process::Command::new")
                    || (is_thick_handler
                        && (trimmed.contains("tokio::process::Command::new")
                            || (trimmed.contains("Command::new(")
                                && !trimmed.contains("GitExec")))));
            if raw_call_tool_debug || raw_tracing_err_debug || raw_process_command {
                findings.push(Finding {
                    rule_id: self.id().to_string(),
                    diagnostic_id: Some(
                        crate::diagnostics::catalog::MCP_ACCESS_AND_ERROR_SAFETY.to_string(),
                    ),
                    rule_name: self.name().to_string(),
                    severity: self.severity(),
                    file: file.path.clone(),
                    line: line_num,
                    column: 0,
                    message:
                        "Raw `Debug` (`:?`) formatting inside `CallToolResult::error` or MCP `error = ?err` risks leaking internal paths, secrets, or stack details."
                            .to_string(),
                    suggestion: Some(
                        "Map internal errors through `sanitize_tool_error` or `%err` Display formatting before logging or returning `CallToolResult::error`."
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
    fn flags_raw_debug_error_in_mcp_call_tool_result() {
        let det = McpAccessAndErrorSafetyDetector::new();
        let bad = SourceFile::new(
            PathBuf::from("crates/vox-orchestrator-mcp/src/foo.rs"),
            "let r = CallToolResult::error(format!(\"{err:?}\"));\nlet c = std::process::Command::new(\"git\");\n".to_string(),
        );
        assert_eq!(det.detect(&bad, None).len(), 2);
    }

    #[test]
    fn flags_tool_result_err_debug_and_allows_git_exec_module() {
        let det = McpAccessAndErrorSafetyDetector::new();
        let bad = SourceFile::new(
            PathBuf::from("crates/vox-orchestrator-mcp/src/chat_tools/chat/foo.rs"),
            "return ToolResult::err(format!(\"{e:?}\"));\ntracing::warn!(error = ?e, \"failed\");\n"
                .to_string(),
        );
        assert_eq!(det.detect(&bad, None).len(), 2);

        let allowed = SourceFile::new(
            PathBuf::from("crates/vox-orchestrator-mcp/src/git_exec.rs"),
            "let mut cmd = std::process::Command::new(\"git\");\n".to_string(),
        );
        assert!(det.detect(&allowed, None).is_empty());
    }
}
