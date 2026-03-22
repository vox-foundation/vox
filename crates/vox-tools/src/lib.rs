//! Direct (in-process) tool execution for Populi chat and similar clients.
//!
//! Tool **names** match MCP (`vox-mcp`) so registry-derived allowlists stay aligned.
//!
//! OpenAI-compatible tool lists and execution for models live in [`populi_chat`].

/// OpenAI-style tool definitions and execution for Populi chat (registry ∩ [`DirectToolExecutor`]).
pub mod populi_chat;

use std::path::{Path, PathBuf};

/// Runs MCP-named tools without an MCP transport (workspace-relative paths use `workspace_root`).
#[derive(Debug, Clone)]
pub struct DirectToolExecutor {
    workspace_root: PathBuf,
}

impl Default for DirectToolExecutor {
    fn default() -> Self {
        Self {
            workspace_root: std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        }
    }
}

impl DirectToolExecutor {
    /// Executor with a fixed repo root (tests, daemons).
    #[must_use]
    pub fn with_workspace_root(root: PathBuf) -> Self {
        Self {
            workspace_root: root,
        }
    }

    /// Tool names this executor implements (keep in sync with `vox-mcp` + capability registry).
    #[must_use]
    pub fn supported_tools() -> &'static [&'static str] {
        &["vox_oratio_transcribe", "vox_oratio_status"]
    }

    /// Whether `name` is implemented here.
    #[must_use]
    pub fn supports(name: &str) -> bool {
        Self::supported_tools().contains(&name)
    }

    fn resolve_path(&self, path: &str) -> PathBuf {
        let p = Path::new(path);
        if p.is_absolute() {
            p.to_path_buf()
        } else {
            self.workspace_root.join(p)
        }
    }

    /// Execute tool; returns JSON string for the model.
    pub fn execute(&self, name: &str, args: serde_json::Value) -> anyhow::Result<String> {
        match name {
            "vox_oratio_transcribe" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing string field `path`"))?;
                let full = self.resolve_path(path);
                let t = vox_oratio::transcribe_path(&full)?;
                Ok(serde_json::to_string(&serde_json::json!({
                    "path": full,
                    "raw_text": t.raw_text,
                    "refined_text": t.refined_text,
                    "text": t.display_text(),
                }))?)
            }
            "vox_oratio_status" => Ok(serde_json::to_string(&serde_json::json!({
                "summary": vox_oratio::transcript_status(),
                "candle": vox_oratio::candle_backend_status_json(),
            }))?),
            other => anyhow::bail!("unsupported tool: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::populi_chat::{ToolCall, chat_tool_definitions, execute_tool_calls};
    use super::*;
    use serde_json::json;

    #[test]
    fn status_returns_json_with_summary() {
        let ex = DirectToolExecutor::default();
        let s = ex
            .execute("vox_oratio_status", serde_json::json!({}))
            .expect("status");
        assert!(s.contains("summary"));
    }

    #[test]
    fn chat_tools_match_executor_one_to_one() {
        let defs = chat_tool_definitions();
        let supported: std::collections::HashSet<_> = DirectToolExecutor::supported_tools()
            .iter()
            .copied()
            .collect();
        for d in &defs {
            let name = d
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .expect("function name");
            assert!(
                supported.contains(name),
                "advertised tool {name} must be in DirectToolExecutor"
            );
        }
        assert_eq!(
            defs.len(),
            supported.len(),
            "expect 1:1 with executor tools"
        );
    }

    #[test]
    fn parameters_are_object_typed() {
        for d in chat_tool_definitions() {
            let name = d
                .pointer("/function/name")
                .and_then(|v| v.as_str())
                .expect("function name");
            let Some(params) = d.pointer("/function/parameters") else {
                panic!("tool {name}: missing function.parameters");
            };
            let Some(ty) = params.get("type").and_then(|v| v.as_str()) else {
                panic!("tool {name}: parameters.type must be a string (got {params:?})");
            };
            assert_eq!(
                ty, "object",
                "tool {name}: parameters must be type object, got {ty:?}"
            );
        }
    }

    #[test]
    fn unknown_tool_denied_with_allowlist_error() {
        let calls = vec![ToolCall {
            id: "golden-tc-1".to_string(),
            name: "__populi_golden_unknown_tool__".to_string(),
            arguments: Some("{}".to_string()),
        }];
        let out = execute_tool_calls(&calls);
        assert_eq!(out.len(), 1);
        let body: serde_json::Value = serde_json::from_str(&out[0].1).expect("deny envelope");
        assert_eq!(body.get("success"), Some(&json!(false)));
        let err = body
            .get("error")
            .and_then(|v| v.as_str())
            .expect("error string");
        assert!(
            err.contains("allowlist"),
            "expected allowlist denial, got: {err}"
        );
    }
}
