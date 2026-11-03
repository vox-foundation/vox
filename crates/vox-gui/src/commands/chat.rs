//! GUI chat sessions, served by the daemon (`GET /api/v2/chat/*` and the `vox_chat_session_*`
//! ops) so they work against a local or a remote Vox server.

use std::sync::Arc;

use serde::Deserialize;
use serde_json::json;
use tauri::State;
use vox_orchestrator_mcp::gui_ops::dto::chat::reply_metadata_json;
pub use vox_orchestrator_mcp::gui_ops::dto::chat::{ChatMessageDto, ChatSessionDto};

use crate::remote::backend::BackendState;
use crate::remote::calls::{BackendCalls, api_data, encode_component, path_with_query, tool_data};

const CREATE_TOOL: &str = "vox_chat_session_create";
const APPEND_TOOL: &str = "vox_chat_session_append";
const RENAME_TOOL: &str = "vox_chat_session_rename";
const ARCHIVE_TOOL: &str = "vox_chat_session_archive";
const CHAT_TOOL: &str = "vox_chat_message";

pub(crate) async fn create_session_impl<B: BackendCalls>(
    b: &B,
    title: Option<String>,
) -> Result<ChatSessionDto, String> {
    tool_data(
        CREATE_TOOL,
        b.call_tool(CREATE_TOOL, json!({ "title": title })).await?,
    )
}

pub(crate) async fn list_sessions_impl<B: BackendCalls>(
    b: &B,
    limit: Option<usize>,
    include_archived: Option<bool>,
) -> Result<Vec<ChatSessionDto>, String> {
    let path = path_with_query(
        "/chat/sessions",
        &[
            ("limit", limit.map(|l| l.to_string())),
            (
                "include_archived",
                Some(include_archived.unwrap_or(false).to_string()),
            ),
        ],
    );
    api_data(b.api_v2_get(&path).await?)
}

pub(crate) async fn get_messages_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    limit: Option<usize>,
) -> Result<Vec<ChatMessageDto>, String> {
    let path = path_with_query(
        &format!("/chat/sessions/{}/messages", encode_component(session_id)),
        &[("limit", limit.map(|l| l.to_string()))],
    );
    api_data(b.api_v2_get(&path).await?)
}

async fn append_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    role: &str,
    content: &str,
    metadata_json: Option<String>,
) -> Result<ChatMessageDto, String> {
    let args = json!({
        "session_id": session_id,
        "role": role,
        "content": content,
        "metadata_json": metadata_json,
    });
    tool_data(APPEND_TOOL, b.call_tool(APPEND_TOOL, args).await?)
}

#[tauri::command]
pub async fn chat_create_session(
    backend: State<'_, Arc<BackendState>>,
    title: Option<String>,
) -> Result<ChatSessionDto, String> {
    create_session_impl(&backend.current().await, title).await
}

#[tauri::command]
pub async fn chat_list_sessions(
    backend: State<'_, Arc<BackendState>>,
    limit: Option<usize>,
    include_archived: Option<bool>,
) -> Result<Vec<ChatSessionDto>, String> {
    list_sessions_impl(&backend.current().await, limit, include_archived).await
}

#[tauri::command]
pub async fn chat_get_messages(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
    limit: Option<usize>,
) -> Result<Vec<ChatMessageDto>, String> {
    get_messages_impl(&backend.current().await, &session_id, limit).await
}

#[derive(Debug, Deserialize)]
pub struct ChatAppendInput {
    pub session_id: String,
    pub role: String,
    pub content: String,
    pub task_id: Option<String>,
    /// Optional model id to record in the message payload (e.g. for assistant messages).
    #[serde(default)]
    pub model_id: Option<String>,
    /// True when the composer already dispatched this message as a task
    /// (`submit_orchestrator_task`). The secretary must not submit it again
    /// (C2: every actionable composer message used to be SUBMIT_TASK'd twice).
    #[serde(default)]
    pub already_submitted: bool,
}

/// `payload_json` for an appended message: `task_id` / `model_id` when present.
fn append_metadata_json(input: &ChatAppendInput) -> Option<String> {
    let mut obj = serde_json::Map::new();
    if let Some(t) = &input.task_id {
        obj.insert("task_id".into(), t.clone().into());
    }
    if let Some(m) = &input.model_id {
        obj.insert("model_id".into(), m.clone().into());
    }
    (!obj.is_empty()).then(|| serde_json::Value::Object(obj).to_string())
}

pub(crate) async fn append_message_impl<B: BackendCalls>(
    b: &B,
    input: &ChatAppendInput,
) -> Result<ChatMessageDto, String> {
    if input.session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    if input.role.trim().is_empty() {
        return Err("role must not be empty".to_string());
    }
    append_impl(
        b,
        &input.session_id,
        &input.role,
        &input.content,
        append_metadata_json(input),
    )
    .await
}

#[tauri::command]
pub async fn chat_append_message<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    input: ChatAppendInput,
    backend: State<'_, Arc<BackendState>>,
) -> Result<i64, String> {
    let msg = append_message_impl(&backend.current().await, &input).await?;

    // Secretary: detect actionable intent in user messages and *propose* it
    // to the user — it must NOT auto-submit to the orchestrator daemon task
    // graph (SUBMIT_TASK). Task 0.2 (harness parity plan): the secretary used
    // to fire SUBMIT_TASK itself here, silently turning any ≥10-word message
    // containing an action verb into a live task. That is now client-side
    // gated: this only emits the proposal toast; the actual SUBMIT_TASK call
    // happens in `secretary_confirm_task`, invoked when the user explicitly
    // clicks "confirm" on the toast (see `SecretaryToast.tsx`).
    if let Some(classified) =
        secretary_candidate(&input.role, &input.content, input.already_submitted)
    {
        // `item_id` here is a client-side proposal id (not a hopper/task id —
        // no task exists yet). `secretary_confirm_task` is keyed on it so the
        // eventual daemon submission uses the exact same session_id/intent
        // the user was shown in the toast.
        let proposal_id = uuid::Uuid::new_v4().to_string();
        crate::commands::orchestrator::emit_secretary_proposed(
            &app_handle,
            crate::commands::orchestrator::SecretaryProposedPayload {
                item_id: proposal_id,
                intent: classified.intent,
                confidence_pct: classified.confidence_pct,
                session_id: input.session_id.clone(),
            },
        );
    }

    Ok(msg.id)
}

/// Build the SUBMIT_TASK RPC params for a secretary-confirmed task.
fn build_submit_task_params(
    session_id: &str,
    intent: &str,
    active_skill: Option<&str>,
) -> serde_json::Value {
    serde_json::json!({
        "description": intent,
        "file_manifest": [],
        "priority": null,
        "session_id": session_id,
        "allow_duplicate": false,
        "model_hint": null,
        "dry_run": null,
        "active_skill": active_skill,
    })
}

/// Submit a secretary-proposed task to the orchestrator daemon (SUBMIT_TASK),
/// only ever called from the frontend when the user explicitly clicks
/// "confirm" on the `SecretaryToast` — this is the sole path by which a
/// secretary classification becomes a live task (Task 0.2: auto-dispatch →
/// propose-only).
#[tauri::command]
pub async fn secretary_confirm_task<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    session_id: String,
    intent: String,
    active_skill: Option<String>,
    backend: State<'_, Arc<BackendState>>,
) -> Result<Option<String>, String> {
    let params = build_submit_task_params(&session_id, &intent, active_skill.as_deref());
    let raw = backend
        .call(
            vox_foundation::protocol::orch_daemon_method::SUBMIT_TASK,
            params,
        )
        .await
        .map_err(String::from)?;

    let task_id = submitted_task_id(&raw);
    if task_id.is_some() {
        // Only ping the tasks list when something new was actually enqueued —
        // a deduped submission (task_id null, duplicate_of set) changed nothing.
        crate::commands::orchestrator::emit_tasks_changed(&app_handle);
    }
    Ok(task_id)
}

pub(crate) async fn rename_session_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    title: &str,
) -> Result<(), String> {
    let args = json!({ "session_id": session_id, "title": title });
    tool_data::<String>(RENAME_TOOL, b.call_tool(RENAME_TOOL, args).await?).map(|_| ())
}

pub(crate) async fn archive_session_impl<B: BackendCalls>(
    b: &B,
    session_id: &str,
    archived: bool,
) -> Result<(), String> {
    let args = json!({ "session_id": session_id, "archived": archived });
    tool_data::<String>(ARCHIVE_TOOL, b.call_tool(ARCHIVE_TOOL, args).await?).map(|_| ())
}

#[tauri::command]
pub async fn chat_rename_session(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    rename_session_impl(&backend.current().await, &session_id, &title).await
}

#[tauri::command]
pub async fn chat_archive_session(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
) -> Result<(), String> {
    archive_session_impl(&backend.current().await, &session_id, true).await
}

#[tauri::command]
pub async fn chat_unarchive_session(
    backend: State<'_, Arc<BackendState>>,
    session_id: String,
) -> Result<(), String> {
    archive_session_impl(&backend.current().await, &session_id, false).await
}

/// Secretary gate: never classify a message the composer already submitted
/// as a task — that path caused every actionable composer message to be
/// SUBMIT_TASK'd twice (explicit submit + secretary re-submit).
fn secretary_candidate(
    role: &str,
    content: &str,
    already_submitted: bool,
) -> Option<vox_orchestrator::secretary::ClassifyResult> {
    if already_submitted {
        return None;
    }
    vox_orchestrator::secretary::classify(role, content)
}

/// Task id from a `SUBMIT_TASK` daemon reply. `None` means the daemon
/// deduped the submission (`task_id: null` + `duplicate_of`): nothing new
/// was created, so no "Secretary proposed a task" toast may be emitted.
fn submitted_task_id(raw: &serde_json::Value) -> Option<String> {
    raw.get("task_id")
        .and_then(|v| v.as_u64())
        .map(|v| v.to_string())
}

#[derive(Debug, Deserialize)]
pub struct ChatSendInput {
    pub session_id: String,
    pub content: String,
    #[serde(default)]
    pub active_skill: Option<String>,
    /// Opt-in, non-blocking post-reply grounding/hallucination check (see
    /// `GroundingCheckToggle.tsx` and `vox_orchestrator::grounding::assess_reply_confidence`).
    /// Defaults to disabled when omitted (older frontends, /spawn, tests).
    #[serde(default)]
    pub grounding_check_enabled: Option<bool>,
}

/// Parsed reply extracted from a `vox_chat_message` `ToolResult` envelope.
#[derive(Debug)]
pub(crate) struct ParsedChatReply {
    pub(crate) content: String,
    pub(crate) model_id: Option<String>,
    pub(crate) latency_ms: Option<u64>,
    pub(crate) selection_reason: Option<String>,
    /// Turn events derived from tool results this turn (Phase E Task E1) —
    /// e.g. a skill-activation chip. Session-live only: not persisted into
    /// `payload` by `persist_assistant_reply`, so it does not survive a
    /// reload — reload persistence was not in the Task E1 Observable.
    pub(crate) events: Vec<serde_json::Value>,
}

/// Extracts a [`ParsedChatReply`] from a `vox_chat_message` `ToolResult`
/// envelope (`{"success", "data": {"message": {..., "content"}, "model_used"}}`
/// or `{"success": false, "error"}`), or reports a profile denial
/// (`{"error": {"code", "message"}}`) as `code: message`.
pub(crate) fn parse_chat_message_envelope(
    envelope: &serde_json::Value,
) -> Result<ParsedChatReply, String> {
    let success = envelope
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    if !success {
        return Err(match envelope.get("error") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(serde_json::Value::Object(o)) => format!(
                "{}: {}",
                o.get("code").and_then(|c| c.as_str()).unwrap_or("error"),
                o.get("message").and_then(|m| m.as_str()).unwrap_or("")
            ),
            _ => "vox_chat_message failed with no error detail".to_string(),
        });
    }
    let data = envelope
        .get("data")
        .ok_or_else(|| "vox_chat_message succeeded with no data".to_string())?;
    let content = data
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| "vox_chat_message response missing message.content".to_string())?
        .to_string();
    let model_id = data
        .get("model_used")
        .and_then(|m| m.as_str())
        .map(str::to_string);
    let latency_ms = data.get("latency_ms").and_then(|v| v.as_u64());
    let selection_reason = data
        .get("selection_reason")
        .and_then(|v| v.as_str())
        .map(str::to_string);
    let events = data
        .get("events")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(ParsedChatReply {
        content,
        model_id,
        latency_ms,
        events,
        selection_reason,
    })
}

/// Persists an already-parsed assistant reply into the GUI session's display
/// transcript (`vox_chat_session_append`) and returns the stored message, with
/// the server's real `created_at`.
///
/// That store is display-only, for rendering the transcript in this GUI's chat
/// panel — it is NOT what feeds the model's context. `vox_chat_message` itself
/// persists the turn into the separate "workspace" conversation keyed by
/// `(repository_id, session_id)`, which backs `chat_history:{session_id}`.
/// Both writes happen on every synchronous turn — that duplication is correct,
/// not a bug: do not remove either write thinking it duplicates the other.
pub(crate) async fn persist_assistant_reply<B: BackendCalls>(
    b: &B,
    session_id: &str,
    reply: &ParsedChatReply,
    grounding_flagged: Option<bool>,
) -> Result<ChatMessageDto, String> {
    let metadata = reply_metadata_json(
        reply.model_id.as_deref(),
        reply.latency_ms,
        reply.selection_reason.as_deref(),
        grounding_flagged,
    );
    append_impl(b, session_id, "assistant", &reply.content, metadata).await
}

/// Opt-in, non-blocking post-reply grounding check: runs after the reply is
/// already in hand (a cheap deterministic heuristic, not a second LLM call —
/// see `assess_reply_confidence`'s doc comment).
pub(crate) fn grounding_flag(enabled: Option<bool>, content: &str) -> Option<bool> {
    (enabled == Some(true))
        .then(|| vox_orchestrator::grounding::assess_reply_confidence(content).flagged)
}

pub(crate) async fn send_message_impl<B: BackendCalls>(
    b: &B,
    input: &ChatSendInput,
) -> Result<ChatMessageDto, String> {
    if input.session_id.trim().is_empty() {
        return Err("session_id must not be empty".to_string());
    }
    if input.content.trim().is_empty() {
        return Err("content must not be empty".to_string());
    }
    let mut args = json!({
        "prompt": input.content,
        "session_id": input.session_id,
    });
    if let Some(skill) = input.active_skill.as_ref() {
        args["active_skill"] = serde_json::Value::String(skill.clone());
    }
    let envelope = b.call_tool(CHAT_TOOL, args).await?;
    let reply = parse_chat_message_envelope(&envelope)?;
    let grounding_flagged = grounding_flag(input.grounding_check_enabled, &reply.content);
    persist_assistant_reply(b, &input.session_id, &reply, grounding_flagged).await
}

/// Synchronous chat reply: calls the real agent loop (`vox_chat_message`) and
/// persists the assistant's reply into the display transcript. The user's own
/// message is appended by the frontend (`chat_append_message`) beforehand.
#[tauri::command]
pub async fn chat_send_message<R: tauri::Runtime>(
    _app_handle: tauri::AppHandle<R>,
    input: ChatSendInput,
    backend: State<'_, Arc<BackendState>>,
) -> Result<ChatMessageDto, String> {
    send_message_impl(&backend.current().await, &input).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::remote::calls::fake::FakeBackend;

    fn keys(b: &FakeBackend) -> Vec<String> {
        b.calls().into_iter().map(|(k, _)| k).collect()
    }

    fn ok_tool(data: serde_json::Value) -> Result<serde_json::Value, String> {
        Ok(json!({ "success": true, "data": data }))
    }

    fn message(id: i64, role: &str, content: &str) -> serde_json::Value {
        json!({ "id": id, "role": role, "content": content, "created_at": "2026-10-09 00:00:00", "task_id": null })
    }

    fn session(id: &str) -> serde_json::Value {
        json!({
            "session_id": id, "title": "T", "updated_at": "now", "message_count": 0,
            "conversation_id": 1, "repository_id": "repo"
        })
    }

    fn append_input(session_id: &str, role: &str) -> ChatAppendInput {
        ChatAppendInput {
            session_id: session_id.into(),
            role: role.into(),
            content: "please fix the broken authentication flow in the login page it keeps failing"
                .into(),
            task_id: Some("7".into()),
            model_id: None,
            already_submitted: false,
        }
    }

    #[tokio::test]
    async fn each_session_command_issues_exactly_its_call() {
        let b = FakeBackend::new()
            .on_tool(CREATE_TOOL, ok_tool(session("s1")))
            .on_tool(RENAME_TOOL, ok_tool(json!("s1")))
            .on_tool(ARCHIVE_TOOL, ok_tool(json!("s1")))
            .on_get(
                "/chat/sessions?limit=200&include_archived=false",
                Ok(json!({ "v": 1, "data": [session("s1")] })),
            )
            .on_get(
                "/chat/sessions/a%2Fb/messages?limit=5",
                Ok(json!({ "v": 1, "data": [message(1, "user", "hi")] })),
            );

        let s = create_session_impl(&b, Some("T".into())).await.unwrap();
        assert_eq!(s.session_id, "s1");
        let all = list_sessions_impl(&b, Some(200), None).await.unwrap();
        assert_eq!(all.len(), 1);
        let msgs = get_messages_impl(&b, "a/b", Some(5)).await.unwrap();
        assert_eq!(msgs[0].content, "hi");
        rename_session_impl(&b, "s1", "New").await.unwrap();
        archive_session_impl(&b, "s1", true).await.unwrap();
        archive_session_impl(&b, "s1", false).await.unwrap();

        assert_eq!(
            keys(&b),
            vec![
                "tool:vox_chat_session_create",
                "get:/chat/sessions?limit=200&include_archived=false",
                "get:/chat/sessions/a%2Fb/messages?limit=5",
                "tool:vox_chat_session_rename",
                "tool:vox_chat_session_archive",
                "tool:vox_chat_session_archive",
            ]
        );
        let calls = b.calls();
        assert_eq!(calls[0].1, json!({ "title": "T" }));
        assert_eq!(calls[4].1["archived"], true);
        assert_eq!(calls[5].1["archived"], false);
    }

    #[tokio::test]
    async fn append_validates_locally_and_records_task_metadata() {
        let b = FakeBackend::new().on_tool(APPEND_TOOL, ok_tool(message(9, "user", "x")));
        for (sid, role, needle) in [("  ", "user", "session_id"), ("s", " ", "role")] {
            let err = append_message_impl(&b, &append_input(sid, role))
                .await
                .unwrap_err();
            assert!(err.contains(needle), "{err}");
        }
        assert!(
            b.calls().is_empty(),
            "invalid input must not reach the server"
        );

        let msg = append_message_impl(&b, &append_input("s", "user"))
            .await
            .unwrap();
        assert_eq!(msg.id, 9);
        let calls = b.calls();
        assert_eq!(keys(&b), vec!["tool:vox_chat_session_append"]);
        assert_eq!(calls[0].1["role"], "user");
        assert_eq!(calls[0].1["metadata_json"], r#"{"task_id":"7"}"#);
    }

    #[tokio::test]
    async fn send_message_calls_chat_then_persists_the_reply_with_metadata() {
        let b = FakeBackend::new()
            .on_tool(
                CHAT_TOOL,
                Ok(json!({
                    "success": true,
                    "data": {
                        "message": {"content": "Perhaps this is the cause. It might be unclear. This is probably wrong."},
                        "model_used": "m/x",
                        "latency_ms": 12
                    }
                })),
            )
            .on_tool(APPEND_TOOL, ok_tool(message(3, "assistant", "reply")));
        let input = ChatSendInput {
            session_id: "s".into(),
            content: "hello".into(),
            active_skill: Some("code-review".into()),
            grounding_check_enabled: Some(true),
        };
        let dto = send_message_impl(&b, &input).await.unwrap();
        assert_eq!(dto.id, 3);
        assert_eq!(
            keys(&b),
            vec!["tool:vox_chat_message", "tool:vox_chat_session_append"]
        );
        let calls = b.calls();
        assert_eq!(calls[0].1["active_skill"], "code-review");
        assert_eq!(calls[1].1["role"], "assistant");
        let meta: serde_json::Value =
            serde_json::from_str(calls[1].1["metadata_json"].as_str().unwrap()).unwrap();
        assert_eq!(meta["model_id"], "m/x");
        assert_eq!(meta["latency_ms"], 12);
        assert_eq!(meta["grounding_flagged"], true);
    }

    #[tokio::test]
    async fn send_message_rejects_empty_input_and_surfaces_chat_errors() {
        let b = FakeBackend::new().on_tool(
            CHAT_TOOL,
            Ok(json!({"error": {"code": "tool_not_in_profile", "message": "not served"}})),
        );
        let empty = ChatSendInput {
            session_id: String::new(),
            content: "hello".into(),
            active_skill: None,
            grounding_check_enabled: None,
        };
        assert!(
            send_message_impl(&b, &empty)
                .await
                .unwrap_err()
                .contains("session_id")
        );
        assert!(b.calls().is_empty());

        let input = ChatSendInput {
            session_id: "s".into(),
            ..empty
        };
        let err = send_message_impl(&b, &input).await.unwrap_err();
        assert_eq!(err, "tool_not_in_profile: not served");
        assert_eq!(keys(&b), vec!["tool:vox_chat_message"], "nothing persisted");
    }

    #[test]
    fn chat_module_has_no_direct_db_access() {
        let src = include_str!("chat.rs");
        let (body, _) = src.split_once("#[cfg(test)]").unwrap();
        assert!(!body.contains("vox_db"));
        assert!(!body.contains("GuiDbPool"));
        assert!(!body.contains("PersistentDaemon"));
    }

    #[test]
    fn secretary_classify_short_user_message_returns_none() {
        let result = vox_orchestrator::secretary::classify("user", "fix it");
        assert!(result.is_none(), "short message should return None");
    }

    #[test]
    fn secretary_classify_long_action_message_returns_some() {
        let result = vox_orchestrator::secretary::classify(
            "user",
            "fix the broken authentication flow in the login page it keeps redirecting users",
        );
        assert!(result.is_some());
    }

    #[test]
    fn secretary_skips_messages_the_composer_already_submitted() {
        let msg = "fix the broken authentication flow in the login page it keeps redirecting users";
        assert!(
            vox_orchestrator::secretary::classify("user", msg).is_some(),
            "precondition: classifier finds this actionable"
        );
        assert!(secretary_candidate("user", msg, true).is_none());
        assert!(secretary_candidate("user", msg, false).is_some());
    }

    #[test]
    fn submitted_task_id_is_none_when_daemon_dedupes() {
        assert_eq!(
            submitted_task_id(&serde_json::json!({"task_id": null, "duplicate_of": 7})),
            None
        );
        assert_eq!(submitted_task_id(&serde_json::json!({})), None);
        assert_eq!(
            submitted_task_id(&serde_json::json!({"task_id": 42})),
            Some("42".to_string())
        );
    }

    #[test]
    fn chat_append_input_already_submitted_defaults_to_false() {
        let input: ChatAppendInput = serde_json::from_str(
            r#"{"session_id":"s","role":"user","content":"hi","task_id":null}"#,
        )
        .expect("older frontends omit the field");
        assert!(!input.already_submitted);
    }

    #[test]
    fn parse_chat_message_envelope_extracts_content_and_model() {
        let envelope = serde_json::json!({
            "success": true,
            "data": {
                "message": {"id": "m1", "role": "assistant", "content": "Hi there"},
                "model_used": "openrouter/auto",
                "tokens": 42,
                "latency_ms": 913,
                "selection_reason": "Chosen by the model scorer as the best match"
            }
        });
        let reply = parse_chat_message_envelope(&envelope).expect("parse ok");
        assert_eq!(reply.content, "Hi there");
        assert_eq!(reply.model_id.as_deref(), Some("openrouter/auto"));
        assert_eq!(reply.latency_ms, Some(913));
        assert_eq!(
            reply.selection_reason.as_deref(),
            Some("Chosen by the model scorer as the best match")
        );
    }

    #[test]
    fn parse_chat_message_envelope_latency_ms_absent_is_none() {
        let envelope = serde_json::json!({
            "success": true,
            "data": {
                "message": {"id": "m1", "role": "assistant", "content": "Hi there"},
                "model_used": "openrouter/auto",
                "tokens": 42
            }
        });
        let reply = parse_chat_message_envelope(&envelope).expect("parse ok");
        assert_eq!(reply.latency_ms, None);
        assert_eq!(reply.selection_reason, None);
    }

    /// Proves the "honesty gate" (plan P3.2): `model_id` reports the
    /// server's actually-*served* model (`data.model_used`), never a
    /// silent fallback or an echo of anything else in the envelope.
    /// Case 1 plants a decoy `data.model` field and reply text naming a third
    /// model; case 2 omits `model_used` and expects `None`, not the decoy.
    #[test]
    fn chat_turn_reports_model_id_of_served_hub() {
        let envelope = serde_json::json!({
            "success": true,
            "data": {
                "message": {
                    "id": "m1",
                    "role": "assistant",
                    "content": "Sure, I'm gpt-4-turbo and happy to help."
                },
                "model": "requested/pinned-decoy",
                "model_used": "qwen/qwen3.8-27b-hub"
            }
        });
        let reply = parse_chat_message_envelope(&envelope).expect("parse ok");
        assert_eq!(reply.model_id.as_deref(), Some("qwen/qwen3.8-27b-hub"));

        let envelope_no_model_used = serde_json::json!({
            "success": true,
            "data": {
                "message": {"id": "m2", "role": "assistant", "content": "hi"},
                "model": "requested/pinned-decoy"
            }
        });
        let reply2 = parse_chat_message_envelope(&envelope_no_model_used).expect("parse ok");
        assert_eq!(reply2.model_id, None);
    }

    #[test]
    fn parse_chat_message_envelope_reports_tool_error() {
        let envelope = serde_json::json!({"success": false, "error": "model unavailable"});
        let err = parse_chat_message_envelope(&envelope).unwrap_err();
        assert_eq!(err, "model unavailable");
    }

    #[test]
    fn grounding_flag_runs_only_when_enabled() {
        let hedged = "Perhaps this is the cause. It might be unclear. This is probably wrong.";
        assert_eq!(grounding_flag(Some(true), hedged), Some(true));
        assert_eq!(grounding_flag(None, hedged), None);
        assert_eq!(grounding_flag(Some(false), hedged), None);
    }

    #[test]
    fn secretary_confirm_task_params_include_active_skill_when_provided() {
        let params = build_submit_task_params("sess-1", "do the thing", Some("code-review"));
        assert_eq!(params["active_skill"], serde_json::json!("code-review"));
    }

    #[test]
    fn secretary_confirm_task_params_active_skill_null_when_absent() {
        let params = build_submit_task_params("sess-1", "do the thing", None);
        assert_eq!(params["active_skill"], serde_json::Value::Null);
    }
}
