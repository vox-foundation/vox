//! Direct (in-process) tool execution for Mens chat and similar clients.
//!
//! Tool **names** match MCP (`vox-mcp`) so registry-derived allowlists stay aligned.
//!
//! OpenAI-compatible tool lists and execution for models live in [`mens_chat`].

/// OpenAI-style tool definitions and execution for Mens chat (registry ∩ [`DirectToolExecutor`]).
pub mod mens_chat;

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
        &[
            "vox_oratio_transcribe",
            "vox_oratio_status",
            "vox_oratio_listen",
        ]
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
                let language_hint = args
                    .get("language_hint")
                    .and_then(|v| v.as_str())
                    .map(ToOwned::to_owned);
                let debug_parser_payload = args
                    .get("debug_parser_payload")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let profile = match args
                    .get("profile")
                    .and_then(|v| v.as_str())
                    .unwrap_or("balanced")
                {
                    "conservative" => vox_oratio::refine::OratioCorrectionProfile::Conservative,
                    "aggressive" => vox_oratio::refine::OratioCorrectionProfile::Aggressive,
                    _ => vox_oratio::refine::OratioCorrectionProfile::Balanced,
                };
                let rtc = vox_oratio::OratioRuntimeConfig::resolve();
                let ctx = vox_oratio::refine::CorrectionContext::from_runtime(
                    &rtc,
                    profile,
                    debug_parser_payload,
                );
                let detail =
                    vox_oratio::transcribe_path_detailed(&full, &ctx, language_hint.as_deref())?;
                let mut out = serde_json::json!({
                    "path": full,
                    "raw_text": detail.raw_text,
                    "refined_text": detail.refined_text,
                    "text": detail.refined_text,
                    "confidence": detail.confidence,
                });
                if debug_parser_payload {
                    out["correction_trace"] = serde_json::json!(detail.correction_trace);
                    out["runtime_config"] = vox_oratio::runtime_config_diagnostic_json(&rtc);
                }
                Ok(serde_json::to_string(&out)?)
            }
            "vox_oratio_status" => Ok(serde_json::to_string(&serde_json::json!({
                "summary": vox_oratio::transcript_status(),
                "candle": vox_oratio::candle_backend_status_json(),
                "runtime": vox_oratio::runtime_config_diagnostic_json(&vox_oratio::OratioRuntimeConfig::resolve()),
            }))?),
            "vox_oratio_listen" => {
                let path = args
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| anyhow::anyhow!("missing string field `path`"))?;
                let full = self.resolve_path(path);
                let rtc = vox_oratio::OratioRuntimeConfig::resolve();
                let timeout_ms = args
                    .get("timeout_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(rtc.session_timing.capture_timeout_ms);
                let max_duration_ms = args
                    .get("max_duration_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(rtc.session_timing.max_duration_ms);
                let inference_deadline_ms =
                    args.get("inference_deadline_ms").and_then(|v| v.as_u64());
                let heartbeat_ms = args
                    .get("heartbeat_ms")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(rtc.session_timing.heartbeat_ms);
                let language_hint = args
                    .get("language_hint")
                    .and_then(|v| v.as_str())
                    .map(ToOwned::to_owned);
                let debug_parser_payload = args
                    .get("debug_parser_payload")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let profile = match args
                    .get("profile")
                    .and_then(|v| v.as_str())
                    .unwrap_or("balanced")
                {
                    "conservative" => vox_oratio::refine::OratioCorrectionProfile::Conservative,
                    "aggressive" => vox_oratio::refine::OratioCorrectionProfile::Aggressive,
                    _ => vox_oratio::refine::OratioCorrectionProfile::Balanced,
                };
                let route_mode = match args
                    .get("route_mode")
                    .and_then(|v| v.as_str())
                    .unwrap_or("none")
                {
                    "tool" => vox_oratio::RouteMode::Tool,
                    "chat" => vox_oratio::RouteMode::Chat,
                    "orchestrator" => vox_oratio::RouteMode::Orchestrator,
                    _ => vox_oratio::RouteMode::None,
                };
                let session = vox_oratio::transcribe_path_session_with_runtime(
                    &full,
                    &vox_oratio::OratioSessionConfig {
                        timeout_ms,
                        max_duration_ms,
                        inference_deadline_ms,
                        language_hint,
                        correction_profile: profile,
                        debug_parser_payload,
                        heartbeat_ms,
                        session_id: None,
                    },
                    &rtc,
                )?;
                let route = vox_oratio::route_transcript_with_options(
                    route_mode,
                    &session.session_id,
                    &session.text,
                    session.confidence,
                    &rtc,
                );
                if let Some(out_path) = args.get("emit_asr_refine_path").and_then(|v| v.as_str()) {
                    let out = self.resolve_path(out_path);
                    if let Some(parent) = out.parent() {
                        std::fs::create_dir_all(parent)?;
                    }
                    use std::io::Write;
                    let mut f = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open(&out)?;
                    writeln!(
                        f,
                        "{}",
                        serde_json::to_string(&serde_json::json!({
                            "noisy_text": session.raw_text,
                            "corrected_text": session.refined_text,
                        }))?
                    )?;
                }
                Ok(serde_json::to_string(&serde_json::json!({
                    "session": session,
                    "route": route,
                }))?)
            }
            other => anyhow::bail!("unsupported tool: {other}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::mens_chat::{ToolCall, chat_tool_definitions, execute_tool_calls};
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
    fn oratio_transcribe_matches_thin_contract() {
        let ex = DirectToolExecutor::default();
        let path = std::env::temp_dir().join(format!(
            "vox_oratio_transcribe_test_{}.txt",
            std::process::id()
        ));
        std::fs::write(&path, "vox check\n").expect("write");
        let j: serde_json::Value = serde_json::from_str(
            &ex.execute(
                "vox_oratio_transcribe",
                serde_json::json!({ "path": path.to_string_lossy() }),
            )
            .expect("transcribe"),
        )
        .expect("json");
        let _ = std::fs::remove_file(&path);
        for key in ["path", "raw_text", "refined_text", "text", "confidence"] {
            assert!(j.get(key).is_some(), "missing `{key}` in {j:?}");
        }
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
