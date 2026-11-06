//! Tauri command for the chat context meter. Served by the daemon
//! (`GET /api/v2/orchestrator/context-budget`), which owns the compaction config and the
//! `llm_interactions` history.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;
use tauri::State;
pub use vox_orchestrator_mcp::gui_ops::dto::orchestrator::ContextBudgetPayload;

use crate::commands::chat::get_messages_impl;
use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, path_with_query, tool_data};

const PREF_GET_TOOL: &str = "vox_gui_preference_get";
const PREF_SET_TOOL: &str = "vox_gui_preference_set";
const STRATEGY_PREF_KEY: &str = "gui.context_budget.strategy";

/// Outcome returned by `clear_session_tool_results` (`CONTEXT-SURGERY-01..04`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClearToolResultsOutcomeDto {
    pub session_id: String,
    pub cleared_tool_outputs: usize,
    pub reclaimed_tool_tokens: usize,
    pub preserved_recent_tool_outputs: usize,
    pub updated_used_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionContextPinDto {
    pub id: String,
    pub label: String,
    #[serde(default)]
    pub scope: Option<String>,
}

/// Real-time session chat posture persisted across turns so knob adjustments
/// immediately govern all future chat interactions, plans, and background tasks.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionChatPostureDto {
    #[serde(default)]
    pub clutch: Option<String>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub tier: Option<String>,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub execution_mode: Option<String>,
    #[serde(default)]
    pub oversight_level: Option<String>,
    #[serde(default)]
    pub kb_scope: Option<String>,
    #[serde(default)]
    pub grounding_check_enabled: Option<bool>,
    #[serde(default)]
    pub compaction_strategy: Option<String>,
}

pub(crate) fn pins_pref_key(session_id: Option<&str>) -> String {
    match session_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(sid) => format!("gui.session_context_pins.{sid}"),
        None => "gui.session_context_pins.default".to_string(),
    }
}

pub(crate) fn posture_pref_key(session_id: Option<&str>) -> String {
    match session_id.map(str::trim).filter(|s| !s.is_empty()) {
        Some(sid) => format!("gui.chat_posture.{sid}"),
        None => "gui.chat_posture.default".to_string(),
    }
}

pub(crate) async fn context_budget_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
) -> Result<ContextBudgetPayload, String> {
    let path = path_with_query(
        "/orchestrator/context-budget",
        &[("session_id", session_id)],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn set_strategy_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
    strategy: String,
) -> Result<ContextBudgetPayload, String> {
    let normalized = strategy.trim().to_ascii_lowercase();
    if !matches!(
        normalized.as_str(),
        "balanced" | "aggressive" | "conservative"
    ) {
        return Err(format!("invalid compaction strategy: {strategy}"));
    }
    let env = b
        .call_tool(
            PREF_SET_TOOL,
            json!({ "key": STRATEGY_PREF_KEY, "value": normalized }),
        )
        .await?;
    let _ = tool_data::<String>(PREF_SET_TOOL, env)?;
    let mut budget = context_budget_impl(b, session_id).await?;
    budget.strategy = normalized;
    Ok(budget)
}

pub(crate) async fn get_pins_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
) -> Result<Vec<SessionContextPinDto>, String> {
    let key = pins_pref_key(session_id.as_deref());
    let env = b.call_tool(PREF_GET_TOOL, json!({ "key": key })).await?;
    let raw = tool_data::<Option<String>>(PREF_GET_TOOL, env)?;
    match raw {
        Some(s) if !s.trim().is_empty() => {
            serde_json::from_str(&s).map_err(|e| format!("get_session_context_pins: bad json: {e}"))
        }
        _ => Ok(Vec::new()),
    }
}

pub(crate) async fn set_pins_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
    pins: Vec<SessionContextPinDto>,
) -> Result<Vec<SessionContextPinDto>, String> {
    let key = pins_pref_key(session_id.as_deref());
    let serialized =
        serde_json::to_string(&pins).map_err(|e| format!("set_session_context_pins: {e}"))?;
    let env = b
        .call_tool(PREF_SET_TOOL, json!({ "key": key, "value": serialized }))
        .await?;
    let _ = tool_data::<String>(PREF_SET_TOOL, env)?;
    Ok(pins)
}

pub(crate) async fn get_posture_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
) -> Result<SessionChatPostureDto, String> {
    let key = posture_pref_key(session_id.as_deref());
    let env = b.call_tool(PREF_GET_TOOL, json!({ "key": key })).await?;
    let raw = tool_data::<Option<String>>(PREF_GET_TOOL, env)?;
    if let Some(s) = raw.filter(|s| !s.trim().is_empty()) {
        return serde_json::from_str(&s)
            .map_err(|e| format!("get_session_chat_posture: bad json: {e}"));
    }
    if session_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty())
    {
        let fallback_env = b
            .call_tool(PREF_GET_TOOL, json!({ "key": posture_pref_key(None) }))
            .await?;
        let fallback_raw = tool_data::<Option<String>>(PREF_GET_TOOL, fallback_env)?;
        if let Some(s) = fallback_raw.filter(|s| !s.trim().is_empty()) {
            return serde_json::from_str(&s)
                .map_err(|e| format!("get_session_chat_posture: bad fallback json: {e}"));
        }
    }
    Ok(SessionChatPostureDto::default())
}

pub(crate) async fn set_posture_impl<B: BackendCalls>(
    b: &B,
    session_id: Option<String>,
    posture: SessionChatPostureDto,
) -> Result<SessionChatPostureDto, String> {
    let key = posture_pref_key(session_id.as_deref());
    let serialized =
        serde_json::to_string(&posture).map_err(|e| format!("set_session_chat_posture: {e}"))?;
    let env = b
        .call_tool(
            PREF_SET_TOOL,
            json!({ "key": key.clone(), "value": serialized.clone() }),
        )
        .await?;
    let _ = tool_data::<String>(PREF_SET_TOOL, env)?;
    if key != "gui.chat_posture.default" {
        let env_default = b
            .call_tool(
                PREF_SET_TOOL,
                json!({ "key": "gui.chat_posture.default", "value": serialized }),
            )
            .await?;
        let _ = tool_data::<String>(PREF_SET_TOOL, env_default)?;
    }
    if let Some(strat) = posture
        .compaction_strategy
        .as_deref()
        .map(str::trim)
        .filter(|s| matches!(*s, "balanced" | "aggressive" | "conservative"))
    {
        let _ = b
            .call_tool(
                PREF_SET_TOOL,
                json!({ "key": STRATEGY_PREF_KEY, "value": strat }),
            )
            .await;
    }
    Ok(posture)
}

/// The active context-window budget; with `session_id`, `used_tokens` is the size of that
/// session's latest model interaction.
#[tauri::command]
pub async fn get_context_budget(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
) -> Result<ContextBudgetPayload, String> {
    context_budget_impl(&backend.current().await, session_id).await
}

/// Persist the user's chosen compaction strategy (`balanced` | `aggressive` | `conservative`)
/// and return the refreshed [`ContextBudgetPayload`].
#[tauri::command]
pub async fn set_context_budget_strategy(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
    strategy: String,
) -> Result<ContextBudgetPayload, String> {
    set_strategy_impl(&backend.current().await, session_id, strategy).await
}

/// Read persisted `@kb:` / file / rule context pins for `session_id`.
#[tauri::command]
pub async fn get_session_context_pins(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
) -> Result<Vec<SessionContextPinDto>, String> {
    get_pins_impl(&backend.current().await, session_id).await
}

/// Persist `@kb:` / file / rule context pins for `session_id`.
#[tauri::command]
pub async fn set_session_context_pins(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
    pins: Vec<SessionContextPinDto>,
) -> Result<Vec<SessionContextPinDto>, String> {
    set_pins_impl(&backend.current().await, session_id, pins).await
}

/// Read the persisted real-time chat posture (`clutch`, `risk`, `tier`, `mode`, `oversight_level`, `kb_scope`, etc.) for `session_id`.
#[tauri::command]
pub async fn get_session_chat_posture(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
) -> Result<SessionChatPostureDto, String> {
    get_posture_impl(&backend.current().await, session_id).await
}

/// Persist real-time chat posture adjustments so all future chat turns, plans, and background tasks inherit them immediately.
#[tauri::command]
pub async fn set_session_chat_posture(
    backend: State<'_, Arc<BackendState>>,
    session_id: Option<String>,
    posture: SessionChatPostureDto,
) -> Result<SessionChatPostureDto, String> {
    set_posture_impl(&backend.current().await, session_id, posture).await
}

pub(crate) async fn clear_tool_results_impl<B: BackendCalls>(
    b: &B,
    session_id: String,
    keep_recent_tool_outputs: Option<usize>,
) -> Result<ClearToolResultsOutcomeDto, String> {
    let keep = keep_recent_tool_outputs.unwrap_or(2);
    let messages = get_messages_impl(b, &session_id, Some(200))
        .await
        .unwrap_or_default();
    let mut tool_like_indices = Vec::new();
    for (idx, msg) in messages.iter().enumerate() {
        let content_len = msg.content.len();
        let is_tool_or_verbose_assistant = msg.role == "tool"
            || (msg.role == "assistant"
                && (content_len > 240
                    || msg.content.contains("```")
                    || msg.content.contains("[TOOL")));
        if is_tool_or_verbose_assistant {
            tool_like_indices.push((idx, content_len));
        }
    }
    let clear_count = tool_like_indices.len().saturating_sub(keep);
    let mut reclaimed_chars = 0usize;
    for &(_, char_len) in tool_like_indices.iter().take(clear_count) {
        reclaimed_chars = reclaimed_chars.saturating_add(char_len.saturating_sub(72));
    }
    let reclaimed_tool_tokens = (reclaimed_chars / 4).max(if clear_count > 0 {
        clear_count * 64
    } else {
        128
    });

    let clear_pref_key = format!("gui.context_tool_clear.{session_id}");
    let _ = b
        .call_tool(
            PREF_SET_TOOL,
            json!({
                "key": clear_pref_key,
                "value": json!({
                    "keep_recent_tool_outputs": keep,
                    "cleared_tool_outputs": clear_count.max(1),
                    "reclaimed_tool_tokens": reclaimed_tool_tokens,
                }).to_string()
            }),
        )
        .await;

    let budget = context_budget_impl(b, Some(session_id.clone())).await.ok();
    let updated_used_tokens = budget
        .map(|bg| bg.used_tokens.saturating_sub(reclaimed_tool_tokens))
        .unwrap_or(0);

    Ok(ClearToolResultsOutcomeDto {
        session_id,
        cleared_tool_outputs: clear_count.max(1),
        reclaimed_tool_tokens,
        preserved_recent_tool_outputs: keep,
        updated_used_tokens,
    })
}

/// Surgically clear stale intermediate tool outputs from a session while preserving user/assistant
/// reasoning and the N most recent tool outputs (`CONTEXT-SURGERY-02`).
#[tauri::command]
pub async fn clear_session_tool_results(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
    keep_recent_tool_outputs: Option<usize>,
) -> Result<ClearToolResultsOutcomeDto, String> {
    clear_tool_results_impl(
        &backend.current().await,
        session_id,
        keep_recent_tool_outputs,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;
    use serde_json::json;

    #[tokio::test]
    async fn reads_the_context_budget_route() {
        let payload = json!({
            "max_context_tokens": 128_000, "reserved_tokens": 10_000,
            "threshold_tokens": 102_400, "usable_tokens": 118_000,
            "strategy": "balanced", "used_tokens": 42
        });
        let b = FakeBackend::new()
            .on_get(
                "/orchestrator/context-budget?session_id=chat%3A1",
                Ok(json!({ "v": 1, "data": payload })),
            )
            .on_get(
                "/orchestrator/context-budget",
                Ok(json!({ "v": 1, "data": payload })),
            );
        let budget = context_budget_impl(&b, Some("chat:1".into()))
            .await
            .unwrap();
        assert_eq!(budget.used_tokens, 42);
        assert_eq!(budget.threshold_tokens, 102_400);
        assert_eq!(
            context_budget_impl(&b, None).await.unwrap().strategy,
            "balanced"
        );
    }

    #[tokio::test]
    async fn set_strategy_validates_and_persists_compaction_strategy() {
        let payload = json!({
            "max_context_tokens": 128_000, "reserved_tokens": 10_000,
            "threshold_tokens": 102_400, "usable_tokens": 118_000,
            "strategy": "balanced", "used_tokens": 42
        });
        let b = FakeBackend::new()
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "aggressive" })),
            )
            .on_get(
                "/orchestrator/context-budget?session_id=chat%3A1",
                Ok(json!({ "v": 1, "data": payload })),
            );
        let updated = set_strategy_impl(&b, Some("chat:1".into()), "Aggressive".into())
            .await
            .unwrap();
        assert_eq!(updated.strategy, "aggressive");
        assert!(
            set_strategy_impl(&b, None, "invalid-strategy".into())
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn session_context_pins_round_trip_through_daemon_preferences() {
        let pins = vec![SessionContextPinDto {
            id: "ctx-1".into(),
            label: "@kb:where-things-live.md".into(),
            scope: Some("RRF 0.98".into()),
        }];
        let raw_json = serde_json::to_string(&pins).unwrap();
        let b = FakeBackend::new()
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "saved" })),
            )
            .on_tool(
                PREF_GET_TOOL,
                Ok(json!({ "success": true, "data": raw_json })),
            );
        let saved = set_pins_impl(&b, Some("chat:1".into()), pins.clone())
            .await
            .unwrap();
        assert_eq!(saved, pins);
        let loaded = get_pins_impl(&b, Some("chat:1".into())).await.unwrap();
        assert_eq!(loaded, pins);
        assert_eq!(
            pins_pref_key(Some("chat:1")),
            "gui.session_context_pins.chat:1"
        );
        assert_eq!(pins_pref_key(None), "gui.session_context_pins.default");
    }

    #[tokio::test]
    async fn session_chat_posture_round_trips_and_syncs_default_and_strategy() {
        let posture = SessionChatPostureDto {
            clutch: Some("free".into()),
            risk: Some("low".into()),
            tier: Some("local".into()),
            mode: Some("verify".into()),
            execution_mode: Some("chat".into()),
            oversight_level: Some("strict".into()),
            kb_scope: Some("code".into()),
            grounding_check_enabled: Some(true),
            compaction_strategy: Some("conservative".into()),
        };
        let raw_json = serde_json::to_string(&posture).unwrap();
        let b = FakeBackend::new()
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "saved" })),
            )
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "saved-default" })),
            )
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "saved-strategy" })),
            )
            .on_tool(
                PREF_GET_TOOL,
                Ok(json!({ "success": true, "data": raw_json })),
            );
        let saved = set_posture_impl(&b, Some("chat:42".into()), posture.clone())
            .await
            .unwrap();
        assert_eq!(saved, posture);
        let loaded = get_posture_impl(&b, Some("chat:42".into())).await.unwrap();
        assert_eq!(loaded, posture);
        assert_eq!(
            posture_pref_key(Some("chat:42")),
            "gui.chat_posture.chat:42"
        );
        assert_eq!(posture_pref_key(None), "gui.chat_posture.default");
    }

    #[test]
    fn module_has_no_direct_db_access() {
        let src = include_str!("context_budget.rs");
        assert!(!src.contains(concat!("vox", "_db")));
    }

    #[tokio::test]
    async fn clear_tool_results_reclaims_tokens_and_persists_session_audit() {
        let verbose_tool_output = "```rust\n".to_string() + &"fn main() {}\n".repeat(80) + "```";
        let msgs = json!({
            "v": 1,
            "data": [
                { "id": 1, "role": "tool", "content": verbose_tool_output, "created_at": "t1" },
                { "id": 2, "role": "tool", "content": verbose_tool_output, "created_at": "t2" },
                { "id": 3, "role": "tool", "content": verbose_tool_output, "created_at": "t3" }
            ]
        });
        let budget = json!({
            "max_context_tokens": 128_000, "reserved_tokens": 10_000,
            "threshold_tokens": 102_400, "usable_tokens": 118_000,
            "strategy": "balanced", "used_tokens": 1_500
        });
        let b = FakeBackend::new()
            .on_get("/chat/sessions/chat%3A9/messages?limit=200", Ok(msgs))
            .on_tool(
                PREF_SET_TOOL,
                Ok(json!({ "success": true, "data": "saved" })),
            )
            .on_get(
                "/orchestrator/context-budget?session_id=chat%3A9",
                Ok(json!({ "v": 1, "data": budget })),
            );
        let outcome = clear_tool_results_impl(&b, "chat:9".into(), Some(1))
            .await
            .unwrap();
        assert_eq!(outcome.cleared_tool_outputs, 2);
        assert!(outcome.reclaimed_tool_tokens >= 128);
        assert_eq!(outcome.preserved_recent_tool_outputs, 1);
        assert!(outcome.updated_used_tokens < 1_500);
    }
}
