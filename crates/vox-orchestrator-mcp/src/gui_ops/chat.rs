//! Chat-session routes and ops for Axis (plan 07 Task 4): the GUI's display-only transcript
//! store (`conversations` / `conversation_messages`, keyed by GUI session id). This is separate
//! from the workspace conversation `vox_chat_message` persists for model context; both writes
//! are intentional.
use std::net::SocketAddr;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::get;
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;

use crate::gui_ops::dto::chat::{ChatMessageDto, ChatSessionDto};
use crate::gui_ops::{api_db, enforce_dashboard_surface, require_db};
use crate::http_gateway::GatewayState;
use crate::http_gateway::dashboard_api::enforce_dashboard_read;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const REM_CHAT_DB: &str = "Verify Turso connectivity and vox-db migrations for conversations.";
const REM_CHAT_INPUT: &str = "Pass a non-empty session_id (and role for appends).";
const DEFAULT_SESSION_LIMIT: usize = 40;
const DEFAULT_MESSAGE_LIMIT: usize = 500;

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatSessionCreateParams {
    /// Session title; defaults to "New chat".
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatSessionAppendParams {
    pub session_id: String,
    /// `user`, `assistant`, ...
    pub role: String,
    pub content: String,
    /// Display metadata stored as the message's `payload_json` (a JSON object).
    #[serde(default)]
    pub metadata_json: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatSessionRenameParams {
    pub session_id: String,
    pub title: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ChatSessionArchiveParams {
    pub session_id: String,
    /// `true` archives, `false` restores an archived session.
    pub archived: bool,
}

fn fail<T: serde::Serialize>(msg: impl Into<String>, remediation: &str) -> String {
    ToolResult::<T>::err_with_remediation(msg.into(), remediation).to_json()
}

/// `vox_chat_session_create`: a new GUI chat session tagged with the server's repository.
pub async fn chat_session_create(state: &ServerState, p: ChatSessionCreateParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return fail::<ChatSessionDto>(e, REM_CHAT_DB),
    };
    let session_id = uuid::Uuid::new_v4().to_string();
    let title = p
        .title
        .filter(|t| !t.trim().is_empty())
        .unwrap_or_else(|| "New chat".to_string());
    let cwd = std::env::current_dir().unwrap_or_default();
    let repository_id = Some(vox_repository::discover_repository_or_fallback(&cwd).repository_id);
    match db
        .chat_ensure_gui_session_with_repo(&session_id, &title, repository_id.as_deref())
        .await
    {
        // SQLite `datetime('now')` format, so a fresh row sorts with DB-read rows.
        Ok(conversation_id) => ToolResult::ok(ChatSessionDto {
            session_id,
            title,
            updated_at: chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            message_count: 0,
            conversation_id,
            repository_id,
        })
        .to_json(),
        Err(e) => fail::<ChatSessionDto>(e.to_string(), REM_CHAT_DB),
    }
}

/// `vox_chat_session_append`: one message into a GUI session (created on first use).
pub async fn chat_session_append(state: &ServerState, p: ChatSessionAppendParams) -> String {
    if p.session_id.trim().is_empty() {
        return fail::<ChatMessageDto>("session_id must not be empty", REM_CHAT_INPUT);
    }
    if p.role.trim().is_empty() {
        return fail::<ChatMessageDto>("role must not be empty", REM_CHAT_INPUT);
    }
    if let Some(m) = p.metadata_json.as_deref()
        && !serde_json::from_str::<Value>(m).is_ok_and(|v| v.is_object())
    {
        return fail::<ChatMessageDto>(
            "metadata_json must be a JSON object",
            "Send display metadata as a JSON object string, or omit it.",
        );
    }
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return fail::<ChatMessageDto>(e, REM_CHAT_DB),
    };
    let appended = async {
        let conv_id = db.chat_ensure_gui_session(&p.session_id, "Chat").await?;
        let id = db
            .chat_append_message(conv_id, &p.role, &p.content, p.metadata_json.as_deref())
            .await?;
        let created_at = db.chat_message_created_at(id).await?;
        Ok::<_, vox_db::StoreError>((id, created_at))
    }
    .await;
    match appended {
        Ok((id, created_at)) => ToolResult::ok(ChatMessageDto::from_row(
            id,
            p.role,
            p.content,
            created_at,
            p.metadata_json.as_deref(),
        ))
        .to_json(),
        Err(e) => fail::<ChatMessageDto>(e.to_string(), REM_CHAT_DB),
    }
}

/// `vox_chat_session_rename`
pub async fn chat_session_rename(state: &ServerState, p: ChatSessionRenameParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return fail::<String>(e, REM_CHAT_DB),
    };
    match db.chat_find_gui_conversation_id(&p.session_id).await {
        Ok(Some(conv_id)) => match db.chat_rename_conversation(conv_id, &p.title).await {
            Ok(()) => ToolResult::ok(p.session_id).to_json(),
            Err(e) => fail::<String>(e.to_string(), REM_CHAT_DB),
        },
        Ok(None) => fail::<String>("session not found", REM_CHAT_INPUT),
        Err(e) => fail::<String>(e.to_string(), REM_CHAT_DB),
    }
}

/// `vox_chat_session_archive`: archive (`archived: true`) or restore (`false`) a session.
pub async fn chat_session_archive(state: &ServerState, p: ChatSessionArchiveParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return fail::<String>(e, REM_CHAT_DB),
    };
    let found = if p.archived {
        db.chat_find_gui_conversation_id(&p.session_id).await
    } else {
        db.chat_find_gui_conversation_id_including_archived(&p.session_id)
            .await
    };
    let conv_id = match found {
        Ok(Some(id)) => id,
        Ok(None) => {
            return fail::<String>(
                format!("session {} not found", p.session_id),
                REM_CHAT_INPUT,
            );
        }
        Err(e) => return fail::<String>(e.to_string(), REM_CHAT_DB),
    };
    let res = if p.archived {
        db.chat_archive_conversation(conv_id).await
    } else {
        db.chat_unarchive_conversation(conv_id).await
    };
    match res {
        Ok(()) => ToolResult::ok(p.session_id).to_json(),
        Err(e) => fail::<String>(e.to_string(), REM_CHAT_DB),
    }
}

#[derive(Debug, Deserialize)]
pub struct SessionsQuery {
    pub limit: Option<usize>,
    pub include_archived: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct MessagesQuery {
    pub limit: Option<usize>,
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/chat/sessions", get(get_chat_sessions))
        .route(
            "/chat/sessions/{session_id}/messages",
            get(get_chat_messages),
        )
}

fn guard_chat(
    gs: &GatewayState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> Result<(), Json<Value>> {
    enforce_dashboard_read(gs, peer, headers)?;
    enforce_dashboard_surface(gs, &["chat"])
}

/// GET /api/v2/chat/sessions?limit=&include_archived=
pub async fn get_chat_sessions(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SessionsQuery>,
) -> Json<Value> {
    if let Err(e) = guard_chat(&gs, &connect.0, &headers) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .chat_list_gui_sessions(
            q.limit.unwrap_or(DEFAULT_SESSION_LIMIT),
            q.include_archived.unwrap_or(false),
        )
        .await
    {
        Ok(rows) => ok(rows
            .into_iter()
            .map(
                |(conversation_id, title, session_id, updated_at, message_count, repository_id)| {
                    ChatSessionDto {
                        session_id,
                        title,
                        updated_at,
                        message_count,
                        conversation_id,
                        repository_id,
                    }
                },
            )
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/chat/sessions/{session_id}/messages?limit=
pub async fn get_chat_messages(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
    Query(q): Query<MessagesQuery>,
) -> Json<Value> {
    if let Err(e) = guard_chat(&gs, &connect.0, &headers) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .chat_get_gui_messages(&session_id, q.limit.unwrap_or(DEFAULT_MESSAGE_LIMIT))
        .await
    {
        Ok(rows) => ok(rows
            .into_iter()
            .map(|(id, role, content, created_at, payload)| {
                ChatMessageDto::from_row(id, role, content, created_at, payload.as_deref())
            })
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use axum::extract::{Path, Query, State};
    use axum::http::HeaderMap;
    use serde_json::Value;

    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = super::api_v2_router();
    }

    async fn sessions(gs: &crate::http_gateway::GatewayState, include_archived: bool) -> Value {
        let q = Query(super::SessionsQuery {
            limit: Some(200),
            include_archived: Some(include_archived),
        });
        body(super::get_chat_sessions(State(gs.clone()), peer(), HeaderMap::new(), q).await).await
    }

    fn listed(v: &Value, session_id: &str) -> bool {
        v["data"]
            .as_array()
            .expect("sessions array")
            .iter()
            .any(|s| s["session_id"] == session_id)
    }

    #[tokio::test]
    async fn session_lifecycle_round_trips_through_ops_and_routes() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();

        let created = tool_json(
            &super::chat_session_create(
                st,
                super::ChatSessionCreateParams {
                    title: Some("Remote".into()),
                },
            )
            .await,
        );
        assert_eq!(created["success"], true, "{created}");
        assert!(created["data"]["repository_id"].is_string());
        let sid = created["data"]["session_id"].as_str().unwrap().to_string();

        for (role, content, meta) in [
            ("user", "hello", None),
            (
                "assistant",
                "hi there",
                Some(r#"{"model_id":"m/x","latency_ms":7}"#),
            ),
        ] {
            let out = tool_json(
                &super::chat_session_append(
                    st,
                    super::ChatSessionAppendParams {
                        session_id: sid.clone(),
                        role: role.into(),
                        content: content.into(),
                        metadata_json: meta.map(str::to_string),
                    },
                )
                .await,
            );
            assert_eq!(out["success"], true, "{out}");
            assert!(!out["data"]["created_at"].as_str().unwrap().is_empty());
        }

        assert!(listed(&sessions(&gs, false).await, &sid));

        let q = Query(super::MessagesQuery { limit: None });
        let msgs = body(
            super::get_chat_messages(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path(sid.clone()),
                q,
            )
            .await,
        )
        .await;
        let msgs = msgs["data"].as_array().expect("messages array").clone();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["content"], "hello");
        assert_eq!(msgs[1]["content"], "hi there");
        assert_eq!(msgs[1]["model_id"], "m/x");
        assert_eq!(msgs[1]["latency_ms"], 7);

        let renamed = tool_json(
            &super::chat_session_rename(
                st,
                super::ChatSessionRenameParams {
                    session_id: sid.clone(),
                    title: "Renamed".into(),
                },
            )
            .await,
        );
        assert_eq!(renamed["success"], true, "{renamed}");
        let all = sessions(&gs, false).await;
        let row = all["data"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["session_id"] == sid.as_str())
            .unwrap()
            .clone();
        assert_eq!(row["title"], "Renamed");

        let archive = |archived| super::ChatSessionArchiveParams {
            session_id: sid.clone(),
            archived,
        };
        assert_eq!(
            tool_json(&super::chat_session_archive(st, archive(true)).await)["success"],
            true
        );
        assert!(!listed(&sessions(&gs, false).await, &sid));
        assert!(listed(&sessions(&gs, true).await, &sid));
        assert_eq!(
            tool_json(&super::chat_session_archive(st, archive(false)).await)["success"],
            true
        );
        assert!(listed(&sessions(&gs, false).await, &sid));
    }

    #[tokio::test]
    async fn append_rejects_blank_ids_and_non_object_metadata() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let append =
            |session_id: &str, role: &str, meta: Option<&str>| super::ChatSessionAppendParams {
                session_id: session_id.into(),
                role: role.into(),
                content: "x".into(),
                metadata_json: meta.map(str::to_string),
            };
        for (p, needle) in [
            (append("  ", "user", None), "session_id"),
            (append("s", " ", None), "role"),
            (append("s", "user", Some("[1]")), "metadata_json"),
            (append("s", "user", Some("nope")), "metadata_json"),
        ] {
            let out = tool_json(&super::chat_session_append(st, p).await);
            assert_eq!(out["success"], false, "{out}");
            assert!(out["error"].as_str().unwrap().contains(needle), "{out}");
        }
    }

    #[tokio::test]
    async fn rename_and_archive_report_unknown_sessions() {
        let (gs, _db) = gateway_with_memory_db().await;
        let st = gs.server_state();
        let out = tool_json(
            &super::chat_session_rename(
                st,
                super::ChatSessionRenameParams {
                    session_id: "missing".into(),
                    title: "t".into(),
                },
            )
            .await,
        );
        assert_eq!(out["success"], false);
        let out = tool_json(
            &super::chat_session_archive(
                st,
                super::ChatSessionArchiveParams {
                    session_id: "missing".into(),
                    archived: false,
                },
            )
            .await,
        );
        assert_eq!(out["success"], false);
    }

    #[tokio::test]
    async fn chat_routes_report_missing_db_and_are_profile_gated() {
        let gs = crate::http_gateway::GatewayState::for_test().await;
        assert_eq!(
            sessions(&gs, false).await["error"]["code"],
            "db_unavailable"
        );

        let named = crate::server_state::ServerState::new_test()
            .await
            .with_profile(
                crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
            );
        let gs = crate::http_gateway::GatewayState::for_test_with_state(
            named,
            std::collections::HashSet::new(),
        )
        .await;
        assert_eq!(
            sessions(&gs, false).await["error"]["code"],
            "tool_not_in_profile"
        );
    }
}
