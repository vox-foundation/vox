//! Chat, inline edit, planning, ghost text, and ambient editor tools for the Vox MCP server.
//!
//! These back the VS Code extension thin-client layer. All context gathering,
//! @mention resolution, LLM routing, and history persistence happen here in Rust.

pub mod params;

mod ambient;
mod chat;
mod ghost_text;
mod inline_edit;
mod plan;

pub use ambient::ambient_state;
pub use chat::{chat_history, chat_message};
pub use ghost_text::ghost_text;
pub use inline_edit::inline_edit;
pub use params::*;
pub use plan::{plan_goal, plan_replan, plan_status};

use std::time::{SystemTime, UNIX_EPOCH};

use super::chat_socrates_meta::socrates_system_rider;
use crate::server::ServerState;

pub(crate) fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

/// Simple ISO date formatter (YYYY-MM-DD) without external chrono/time deps.
pub(crate) fn ts_to_date_str(secs: u64) -> String {
    let days = secs / 86400;
    // Base 1970-01-01 was a Thursday
    // Simple proleptic Gregorian algorithm (good until 2100)
    let z = (days as i64) + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}-{:02}-{:02}", y + if m <= 2 { 1 } else { 0 }, m, d)
}

/// Build the full system prompt for the Vox chat assistant.
pub(crate) async fn build_system_prompt(state: &ServerState) -> String {
    let ws_root = state
        .workspace_root
        .as_deref()
        .unwrap_or(std::path::Path::new("."));

    let mut prompt = String::from(
        "You are assisting with the **Vox** programming language and its ecosystem. \
         Vox is AI-native, full-stack, and compiles to Rust/TypeScript/WASM. \
         Prefer `Option[T]` and explicit errors over null.\n\n",
    );

    for rel in ["VOX.md", ".vox/MEMORY.md"] {
        let p = ws_root.join(rel);
        if let Ok(content) = crate::bounded_fs::read_utf8_path_capped(&p) {
            prompt.push_str("## ");
            prompt.push_str(rel);
            prompt.push_str("\n\n");
            prompt.push_str(&content);
            prompt.push_str("\n\n");
        }
    }

    prompt.push_str(&format!(
        "## Environment\nWorkspace Root: {}\n\nYou are Vox, an elite AI coding assistant. You have access to the Vox MCP toolbelt. You can read and modify files, run tests, inspect VCS history, manage agents, and query the knowledge graph.\n\nRules:\n- Be concise and precise. Prefer code over prose.\n- Always cite which files you modified or plan to modify.\n- When generating code, produce valid, complete implementations — no stubs or placeholders.\n- Use Markdown code blocks with language tags.\n- For multi-file changes, use a structured diff or list each file separately.\n- When asked to plan, produce a numbered task list in Markdown.\n",
        ws_root.display()
    ));

    prompt.push_str(params::ANTI_LAZINESS_RIDER);

    let ts = now_ts();
    let date_str = ts_to_date_str(ts);
    let last_call = state.orchestrator.last_activity_ms() / 1000;
    let server_idle_secs = ts.saturating_sub(last_call);

    prompt.push_str(&format!(
        "\n\n## Temporal Context\nCurrent date: {date_str}.\nUnix timestamp: {ts}s.\n\
         Server last active: {server_idle_secs}s ago.\n\
         **Enforcement**: Before triggering any compilation, re-reindexing, or full file walk, \
         check if things are fresh (< 30s since last run).\n"
    ));

    let pol = state.orchestrator_config.effective_socrates_policy();
    prompt.push_str(&socrates_system_rider(&pol));
    prompt
}

#[cfg(test)]
mod routing_tests {
    use super::super::chat_socrates_meta::{SocratesJsonMeta, socrates_tool_meta};
    use super::chat::{chat_grounding_score, safe_truncate_for_prompt};
    use super::ghost_text::ghost_grounding_score;
    use super::params::{ChatMessageParams, GhostTextParams, PlanTask};
    use crate::llm_bridge::clamp_http_max_output_tokens;
    use vox_socrates_policy::ConfidencePolicy;

    #[test]
    fn clamp_http_max_output_respects_bounds() {
        assert_eq!(clamp_http_max_output_tokens(0), 1);
        assert_eq!(clamp_http_max_output_tokens(100), 100);
        assert_eq!(clamp_http_max_output_tokens(9000), 8192);
    }

    #[test]
    fn socrates_meta_contains_required_fields() {
        let p = ConfidencePolicy::workspace_default();
        let v = socrates_tool_meta(&p, 0.61, false, 0, 0, 0);
        assert!(v.get("risk_decision").is_some());
        assert!(v.get("confidence_estimate").is_some());
        assert!(v.get("contradiction_ratio").is_some());
    }

    #[test]
    fn socrates_tool_meta_matches_telemetry_deserializer() {
        let p = ConfidencePolicy::workspace_default();
        let v = socrates_tool_meta(&p, 0.71, true, 0, 0, 0);
        let m: SocratesJsonMeta = serde_json::from_value(v).expect("telemetry JSON must parse");
        assert!((m.confidence_estimate - 0.71).abs() < 1e-9);
        assert!((m.contradiction_ratio - 0.35).abs() < 1e-9);
    }

    #[test]
    fn ghost_grounding_score_respects_file_and_fim_boundaries() {
        let thin = GhostTextParams {
            prefix: "a".into(),
            suffix: "".into(),
            language: None,
            file_path: None,
            max_tokens: None,
            session_id: None,
        };
        let rich = GhostTextParams {
            prefix: "fn main() {\n    let x = 1;\n".into(),
            suffix: "\n}\n".into(),
            language: Some("rust".into()),
            file_path: Some("src/main.rs".into()),
            max_tokens: None,
            session_id: None,
        };
        assert!(ghost_grounding_score(&rich) > ghost_grounding_score(&thin));
    }

    #[test]
    fn grounding_score_increases_with_context() {
        let empty = ChatMessageParams {
            prompt: "Hi".into(),
            context_files: vec![],
            open_files: vec![],
            active_file: None,
            active_line: None,
            selected_text: None,
            diagnostics: vec![],
            session_id: None,
            cognitive_profile: None,
            json_mode: false,
        };
        let rich = ChatMessageParams {
            prompt: "Hi".into(),
            context_files: vec!["foo.rs".into()],
            open_files: vec!["bar.rs".into()],
            active_file: Some("src/main.rs".into()),
            active_line: Some(42),
            selected_text: Some("let x = 1;".into()),
            diagnostics: vec![],
            session_id: None,
            cognitive_profile: None,
            json_mode: false,
        };
        let a = chat_grounding_score(&empty, 0);
        let b = chat_grounding_score(&rich, 3);
        assert!(b > a);
    }

    #[test]
    fn test_plan_response_schema_extraction() {
        let json = r#"{
            "summary": "Fixing the bug",
            "tasks": [
                { "id": 1, "description": "Identify root cause", "files": ["src/main.rs"], "estimated_complexity": 2, "depends_on": [] },
                { "id": 2, "description": "Write fix", "files": ["src/main.rs"], "estimated_complexity": 3, "depends_on": [1] }
            ]
        }"#;
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(parsed["summary"], "Fixing the bug");
        let tasks = parsed["tasks"].as_array().expect("tasks array");
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0]["id"], 1);
        let deps: Vec<usize> = serde_json::from_value(tasks[1]["depends_on"].clone()).unwrap();
        assert_eq!(deps, vec![1]);
    }

    #[test]
    fn test_plan_schema_empty_tasks_is_valid() {
        let json = r#"{"summary": "Empty plan", "tasks": []}"#;
        let parsed: serde_json::Value = serde_json::from_str(json).expect("valid JSON");
        assert_eq!(parsed["summary"], "Empty plan");
        assert_eq!(parsed["tasks"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn test_plan_schema_raw_json_no_fence() {
        let json = r#"{
            "summary": "Raw JSON",
            "tasks": [
                { "id": 1, "description": "Do thing", "files": [], "estimated_complexity": 1, "depends_on": [] }
            ]
        }"#;
        let tasks: Vec<PlanTask> = serde_json::from_value(
            serde_json::from_str::<serde_json::Value>(json).unwrap()["tasks"].clone(),
        )
        .expect("PlanTask deserialization");
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].description, "Do thing");
        assert_eq!(tasks[0].estimated_complexity, 1);
        assert!(tasks[0].depends_on.is_empty());
    }

    #[test]
    fn truncate_for_prompt_keeps_utf8_boundaries() {
        let s = "abc🙂def🙂ghi";
        let t = safe_truncate_for_prompt(s, 7);
        assert!(t.contains("...[truncated]..."));
        let prefix = t.split("\n...[truncated]...").next().unwrap_or("");
        assert!(s.starts_with(prefix));
        assert!(!prefix.contains('\u{FFFD}'));
    }
}
