//! Optional network gateway for `vox-mcp` (HTTP + WebSocket).
//!
//! This gateway is disabled by default. When enabled, it exposes a bounded, authenticated
//! remote-control surface intended for mobile/browser clients that connect to a **remote** host
//! running the full Vox workspace and toolchain.

use anyhow::{Context, Result};
use axum::Json;
use axum::extract::DefaultBodyLimit;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Router, serve};
use governor::clock::DefaultClock;
use governor::state::keyed::DefaultKeyedStateStore;
use governor::{Quota, RateLimiter};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::num::NonZeroU32;
use std::sync::Arc;

use crate::params::ToolResult;
use crate::server::{ServerState, tool_json_envelope_is_error};
use crate::tools::{TOOL_REGISTRY, canonical_tool_name, handle_tool_call, tool_registry};

const DEFAULT_BIND_HOST: &str = "127.0.0.1";
const DEFAULT_BIND_PORT: u16 = 3921;
const DEFAULT_RATE_LIMIT_PER_MINUTE: u32 = 120;

type IdentityRateLimiter = RateLimiter<String, DefaultKeyedStateStore<String>, DefaultClock>;

fn new_identity_rate_limiter(calls_per_minute: u32) -> Arc<IdentityRateLimiter> {
    let n = NonZeroU32::new(calls_per_minute.max(1)).expect("rate limit min 1");
    Arc::new(RateLimiter::keyed(Quota::per_minute(n)))
}
const DEFAULT_ALLOWED_TOOLS: &[&str] = &[
    "vox_chat_message",
    "vox_plan",
    "vox_plan_status",
    "vox_inline_edit",
    "vox_validate_file",
    "vox_git_status",
    "vox_git_diff",
    "vox_workspace_modules",
    "vox_orchestrator_status",
    "vox_task_status",
    "vox_repo_index_status",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessRole {
    Read,
    Write,
}

#[derive(Clone)]
struct GatewayState {
    server_state: ServerState,
    bearer_token: Option<String>,
    read_bearer_token: Option<String>,
    allow_unauthenticated: bool,
    health_auth_required: bool,
    require_forwarded_https: bool,
    trust_forwarded_for: bool,
    allowed_tools: Arc<HashSet<String>>,
    read_role_eligible_tools: Arc<HashSet<String>>,
    read_role_tools_override: Option<Arc<HashSet<String>>>,
    calls_per_minute: u32,
    rate_limiter: Arc<IdentityRateLimiter>,
}

#[derive(Debug, Serialize)]
struct GatewayInfo {
    enabled: bool,
    bind_host: String,
    bind_port: u16,
    auth_required: bool,
    require_forwarded_https: bool,
    allowed_tools: Vec<String>,
    read_role_allowed_tools: Vec<String>,
    calls_per_minute: u32,
}

#[derive(Debug, Serialize)]
struct ToolDescriptor {
    name: String,
    description: String,
    input_schema: Value,
    product_lane: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ToolCallRequest {
    name: String,
    #[serde(default)]
    args: Value,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCallResponse {
    success: bool,
    is_error: bool,
    result: Value,
}

#[derive(Debug, Deserialize)]
struct WsMessageIn {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    msg_type: String,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    args: Option<Value>,
}

#[derive(Debug, Serialize)]
struct WsMessageOut {
    #[serde(default)]
    id: Option<String>,
    #[serde(rename = "type")]
    msg_type: String,
    success: bool,
    #[serde(default)]
    is_error: bool,
    #[serde(default)]
    data: Value,
    #[serde(default)]
    error: Option<String>,
}

/// Returns true when `VOX_MCP_HTTP_ENABLED` is truthy.
pub fn http_gateway_enabled() -> bool {
    read_bool_env("VOX_MCP_HTTP_ENABLED").unwrap_or(false)
}

/// Start the optional HTTP+WebSocket gateway in a background task.
pub fn spawn_http_gateway_if_enabled(
    state: ServerState,
) -> Result<Option<tokio::task::JoinHandle<()>>> {
    if !http_gateway_enabled() {
        return Ok(None);
    }

    let bind_host = std::env::var("VOX_MCP_HTTP_HOST")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BIND_HOST.to_string());
    let bind_port = std::env::var("VOX_MCP_HTTP_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_BIND_PORT);
    let bearer_token = std::env::var("VOX_MCP_HTTP_BEARER_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let read_bearer_token = std::env::var("VOX_MCP_HTTP_READ_BEARER_TOKEN")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    let allow_unauthenticated =
        read_bool_env("VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED").unwrap_or(false);
    if bearer_token.is_none() && read_bearer_token.is_none() && !allow_unauthenticated {
        anyhow::bail!(
            "VOX_MCP_HTTP_ENABLED=1 requires VOX_MCP_HTTP_BEARER_TOKEN or VOX_MCP_HTTP_READ_BEARER_TOKEN unless VOX_MCP_HTTP_ALLOW_UNAUTHENTICATED=1 is explicitly set."
        );
    }
    let require_forwarded_https =
        read_bool_env("VOX_MCP_HTTP_REQUIRE_FORWARDED_HTTPS").unwrap_or(false);
    let health_auth_required = read_bool_env("VOX_MCP_HTTP_HEALTH_AUTH").unwrap_or(false);
    let trust_forwarded_for = read_bool_env("VOX_MCP_HTTP_TRUST_X_FORWARDED_FOR").unwrap_or(false);
    let calls_per_minute = std::env::var("VOX_MCP_HTTP_RATE_LIMIT_PER_MINUTE")
        .ok()
        .and_then(|s| s.parse::<u32>().ok())
        .filter(|n| *n > 0)
        .unwrap_or(DEFAULT_RATE_LIMIT_PER_MINUTE);
    let allowed_tools = Arc::new(parse_allowed_tools()?);
    let read_role_eligible_tools = Arc::new(metadata_read_role_eligible_tools());
    let read_role_tools_override = parse_read_role_allowed_tools_override()?.map(Arc::new);

    let gateway_state = GatewayState {
        server_state: state,
        bearer_token: bearer_token.clone(),
        read_bearer_token,
        allow_unauthenticated,
        health_auth_required,
        require_forwarded_https,
        trust_forwarded_for,
        allowed_tools: allowed_tools.clone(),
        read_role_eligible_tools,
        read_role_tools_override,
        calls_per_minute,
        rate_limiter: new_identity_rate_limiter(calls_per_minute),
    };

    let app = Router::new()
        .route("/health", get(http_health))
        .route("/v1/info", get(http_info))
        .route("/v1/tools", get(http_tools))
        .route("/v1/tools/call", post(http_call_tool))
        .route("/v1/ws", get(http_ws))
        .route("/v1/mobile", get(http_mobile_workspace))
        .route("/v1/mobile/status", get(http_mobile_status))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .with_state(gateway_state.clone());

    let addr: SocketAddr = format!("{bind_host}:{bind_port}")
        .parse()
        .with_context(|| format!("invalid VOX_MCP_HTTP_HOST/PORT: {bind_host}:{bind_port}"))?;
    let listener = std::net::TcpListener::bind(addr)
        .with_context(|| format!("failed to bind MCP HTTP gateway at {addr}"))?;
    listener
        .set_nonblocking(true)
        .context("failed to set nonblocking gateway listener")?;
    let listener = tokio::net::TcpListener::from_std(listener)
        .context("failed to convert listener to tokio")?;

    let mut info_allowed_tools: Vec<String> = allowed_tools.iter().cloned().collect();
    info_allowed_tools.sort();
    let mut info_read_tools: Vec<String> = visible_tools_for_role(&gateway_state, AccessRole::Read)
        .into_iter()
        .collect();
    info_read_tools.sort();
    let info = GatewayInfo {
        enabled: true,
        bind_host: bind_host.clone(),
        bind_port,
        auth_required: (bearer_token.is_some() || gateway_state.read_bearer_token.is_some())
            && !allow_unauthenticated,
        require_forwarded_https,
        allowed_tools: info_allowed_tools,
        read_role_allowed_tools: info_read_tools,
        calls_per_minute,
    };
    tracing::info!(
        host = %info.bind_host,
        port = info.bind_port,
        auth_required = info.auth_required,
        tools = %info.allowed_tools.join(","),
        "VOX_MCP_HTTP_ENABLED: gateway started"
    );

    let handle = tokio::spawn(async move {
        if let Err(e) = serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        {
            tracing::error!("MCP HTTP gateway exited with error: {e}");
        }
    });
    Ok(Some(handle))
}

async fn http_health(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    let identity = request_identity(&state, &connect.0, &headers);
    if let Err(msg) = enforce_rate_limit(&state, &identity) {
        return (
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response();
    }
    if state.health_auth_required
        && let Err(msg) = enforce_auth(&state, &headers)
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response();
    }
    Json(serde_json::json!({ "status": "ok" })).into_response()
}

async fn http_info(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let bind_host = std::env::var("VOX_MCP_HTTP_HOST")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| DEFAULT_BIND_HOST.to_string());
    let bind_port = std::env::var("VOX_MCP_HTTP_PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(DEFAULT_BIND_PORT);
    let mut allowed_tools: Vec<String> = state.allowed_tools.iter().cloned().collect();
    allowed_tools.sort();
    let mut read_role_allowed_tools: Vec<String> = visible_tools_for_role(&state, AccessRole::Read)
        .into_iter()
        .collect();
    read_role_allowed_tools.sort();
    Json(GatewayInfo {
        enabled: true,
        bind_host,
        bind_port,
        auth_required: (state.bearer_token.is_some() || state.read_bearer_token.is_some())
            && !state.allow_unauthenticated,
        require_forwarded_https: state.require_forwarded_https,
        allowed_tools,
        read_role_allowed_tools,
        calls_per_minute: state.calls_per_minute,
    })
    .into_response()
}

async fn http_tools(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let role = match resolve_access_role(&state, &headers) {
        Ok(r) => r,
        Err(msg) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    let visible_tools = visible_tools_for_role(&state, role);
    let mut docs: Vec<ToolDescriptor> = Vec::new();
    for t in tool_registry() {
        let name = t.name.to_string();
        if !visible_tools.contains(&name) {
            continue;
        }
        let lane = t
            .meta
            .as_ref()
            .and_then(|m| m.0.get("vox_product_lane"))
            .and_then(|v| v.as_str())
            .map(ToString::to_string);
        docs.push(ToolDescriptor {
            name,
            description: t.description.map(|d| d.to_string()).unwrap_or_default(),
            input_schema: Value::Object((*t.input_schema).clone()),
            product_lane: lane,
        });
    }
    docs.sort_by(|a, b| a.name.cmp(&b.name));
    Json(serde_json::json!({ "success": true, "tools": docs })).into_response()
}

async fn http_call_tool(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(req): Json<ToolCallRequest>,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let role = match resolve_access_role(&state, &headers) {
        Ok(r) => r,
        Err(msg) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    call_tool_response(&state, req.name, req.args, Some(connect.0), role).await
}

async fn http_ws(
    State(state): State<GatewayState>,
    ws: WebSocketUpgrade,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let role = match resolve_access_role(&state, &headers) {
        Ok(r) => r,
        Err(msg) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": msg })),
            )
                .into_response();
        }
    };
    let identity = request_identity(&state, &connect.0, &headers);
    ws.on_upgrade(move |socket| async move {
        handle_ws(socket, state, connect.0, identity, role).await;
    })
}

async fn http_mobile_workspace(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }
    let mut resp = Html(mobile_workspace_html()).into_response();
    resp.headers_mut().insert(
        "content-security-policy",
        "default-src 'none'; style-src 'unsafe-inline'; script-src 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; base-uri 'none'; form-action 'none'"
            .parse()
            .expect("valid CSP header value"),
    );
    resp
}

async fn http_mobile_status(
    State(state): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = enforce_request_guards(&state, &connect.0, &headers).await {
        return resp;
    }

    let git = call_tool_json(
        &state.server_state,
        "vox_git_status",
        Value::Object(serde_json::Map::new()),
    )
    .await;
    let orch = call_tool_json(
        &state.server_state,
        "vox_orchestrator_status",
        Value::Object(serde_json::Map::new()),
    )
    .await;
    Json(serde_json::json!({
        "success": true,
        "git_status": git,
        "orchestrator_status": orch
    }))
    .into_response()
}

async fn handle_ws(
    mut socket: WebSocket,
    state: GatewayState,
    peer: SocketAddr,
    identity: String,
    role: AccessRole,
) {
    let mut rx = state.server_state.orchestrator.event_bus().subscribe();

    loop {
        tokio::select! {
            msg = socket.recv() => {
                let Some(Ok(msg)) = msg else { break };
                match msg {
                    Message::Text(text) => {
                        if let Err(msg) = enforce_rate_limit(&state, &identity) {
                            let _ = socket
                                .send(Message::Text(
                                    serde_json::json!({
                                        "type": "error",
                                        "success": false,
                                        "is_error": true,
                                        "error": msg
                                    })
                                    .to_string()
                                    .into(),
                                ))
                                .await;
                            break;
                        }
                        let parsed: Result<WsMessageIn, _> = serde_json::from_str(&text);
                        let reply = match parsed {
                            Ok(req) => ws_handle_message(&state, req, peer, role).await,
                            Err(e) => WsMessageOut {
                                id: None,
                                msg_type: "error".to_string(),
                                success: false,
                                is_error: true,
                                data: Value::Null,
                                error: Some(format!("invalid websocket payload: {e}")),
                            },
                        };
                        if socket
                            .send(Message::Text(
                                serde_json::to_string(&reply)
                                    .unwrap_or_else(|_| {
                                        "{\"type\":\"error\",\"success\":false,\"is_error\":true}"
                                            .to_string()
                                    })
                                    .into(),
                            ))
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                    Message::Ping(bytes) => {
                        if socket.send(Message::Pong(bytes)).await.is_err() {
                            break;
                        }
                    }
                    Message::Close(_) => break,
                    _ => {}
                }
            }
            Ok(event) = rx.recv() => {
                let out = WsMessageOut {
                    id: None,
                    msg_type: "agent_event".to_string(),
                    success: true,
                    is_error: false,
                    data: serde_json::to_value(&event).unwrap_or(Value::Null),
                    error: None,
                };
                if socket.send(Message::Text(
                    serde_json::to_string(&out).unwrap_or_default().into()
                )).await.is_err() {
                    break;
                }
            }
        }
    }
}

async fn ws_handle_message(
    state: &GatewayState,
    req: WsMessageIn,
    peer: SocketAddr,
    role: AccessRole,
) -> WsMessageOut {
    match req.msg_type.as_str() {
        "list_tools" => {
            let mut tools: Vec<String> = visible_tools_for_role(state, role).into_iter().collect();
            tools.sort();
            WsMessageOut {
                id: req.id,
                msg_type: "list_tools_result".to_string(),
                success: true,
                is_error: false,
                data: serde_json::json!({ "tools": tools }),
                error: None,
            }
        }
        "call_tool" => {
            let Some(name) = req.name else {
                return WsMessageOut {
                    id: req.id,
                    msg_type: "call_tool_result".to_string(),
                    success: false,
                    is_error: true,
                    data: Value::Null,
                    error: Some("missing tool name".to_string()),
                };
            };
            let args = req
                .args
                .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
            let result = call_tool_response_value(state, name, args, Some(peer), role).await;
            WsMessageOut {
                id: req.id,
                msg_type: "call_tool_result".to_string(),
                success: result.success,
                is_error: result.is_error,
                data: result.result,
                error: None,
            }
        }
        other => WsMessageOut {
            id: req.id,
            msg_type: "error".to_string(),
            success: false,
            is_error: true,
            data: Value::Null,
            error: Some(format!("unsupported message type: {other}")),
        },
    }
}

async fn call_tool_response(
    state: &GatewayState,
    name: String,
    args: Value,
    peer: Option<SocketAddr>,
    role: AccessRole,
) -> Response {
    Json(call_tool_response_value(state, name, args, peer, role).await).into_response()
}

async fn call_tool_response_value(
    state: &GatewayState,
    name: String,
    args: Value,
    peer: Option<SocketAddr>,
    role: AccessRole,
) -> ToolCallResponse {
    let canonical = canonical_tool_name(name.as_str()).to_string();
    if !visible_tools_for_role(state, role).contains(&canonical) {
        return ToolCallResponse {
            success: false,
            is_error: true,
            result: serde_json::json!({
                "success": false,
                "error": format!("tool is not allowed for current gateway role: {canonical}")
            }),
        };
    }

    if let Some(p) = peer {
        tracing::info!(tool = %canonical, peer = %p, "mcp-http call_tool");
    } else {
        tracing::info!(tool = %canonical, "mcp-http call_tool");
    }

    let result = call_tool_json(&state.server_state, &canonical, args).await;
    let is_error = result
        .as_str()
        .map(tool_json_envelope_is_error)
        .unwrap_or(false);
    let rendered = match result {
        Value::String(s) => serde_json::from_str::<Value>(&s).unwrap_or(Value::String(s)),
        other => other,
    };
    ToolCallResponse {
        success: !is_error,
        is_error,
        result: rendered,
    }
}

async fn call_tool_json(state: &ServerState, name: &str, args: Value) -> Value {
    match handle_tool_call(state, name, args).await {
        Ok(json) => Value::String(json),
        Err(e) => serde_json::to_value(ToolResult::<Value>::err_with_remediation(
            e.to_string(),
            "Verify tool args against /v1/tools schema and retry.",
        ))
        .unwrap_or_else(|_| {
            serde_json::json!({
                "success": false,
                "error": "failed to serialize tool error envelope"
            })
        }),
    }
}

async fn enforce_request_guards(
    state: &GatewayState,
    peer: &SocketAddr,
    headers: &HeaderMap,
) -> std::result::Result<(), Response> {
    let identity = request_identity(state, peer, headers);
    if let Err(msg) = enforce_auth(state, headers) {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response());
    }
    if let Err(msg) = enforce_https_requirement(state, headers) {
        return Err((
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response());
    }
    if let Err(msg) = enforce_rate_limit(state, &identity) {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(serde_json::json!({ "error": msg })),
        )
            .into_response());
    }
    Ok(())
}

fn enforce_auth(state: &GatewayState, headers: &HeaderMap) -> std::result::Result<(), String> {
    resolve_access_role(state, headers).map(|_| ())
}

fn enforce_https_requirement(
    state: &GatewayState,
    headers: &HeaderMap,
) -> std::result::Result<(), String> {
    if !state.require_forwarded_https {
        return Ok(());
    }
    let forwarded_proto = headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if forwarded_proto.eq_ignore_ascii_case("https") {
        Ok(())
    } else {
        Err("VOX_MCP_HTTP_REQUIRE_FORWARDED_HTTPS=1 requires X-Forwarded-Proto: https".to_string())
    }
}

fn enforce_rate_limit(state: &GatewayState, identity: &String) -> std::result::Result<(), String> {
    state.rate_limiter.check_key(identity).map_err(|_| {
        format!(
            "rate limit exceeded (max {} requests/minute)",
            state.calls_per_minute
        )
    })
}

fn parse_allowed_tools() -> Result<HashSet<String>> {
    parse_allowed_tools_from_value(std::env::var("VOX_MCP_HTTP_ALLOWED_TOOLS").ok().as_deref())
}

fn parse_read_role_allowed_tools_override() -> Result<Option<HashSet<String>>> {
    let explicit = std::env::var("VOX_MCP_HTTP_READ_ROLE_ALLOWED_TOOLS")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    if let Some(csv) = explicit {
        parse_allowed_tools_from_value(Some(csv.as_str())).map(Some)
    } else {
        Ok(None)
    }
}

fn metadata_read_role_eligible_tools() -> HashSet<String> {
    TOOL_REGISTRY
        .iter()
        .filter(|e| e.http_read_role_eligible)
        .map(|e| canonical_tool_name(e.name).to_string())
        .collect()
}

fn resolve_access_role(
    state: &GatewayState,
    headers: &HeaderMap,
) -> std::result::Result<AccessRole, String> {
    if state.allow_unauthenticated {
        return Ok(AccessRole::Write);
    }
    let auth = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    let got = auth.strip_prefix("Bearer ").unwrap_or_default().trim();
    if let Some(expected) = state.bearer_token.as_ref()
        && constant_time_eq(got.as_bytes(), expected.as_bytes())
    {
        return Ok(AccessRole::Write);
    }
    if let Some(expected_read) = state.read_bearer_token.as_ref()
        && constant_time_eq(got.as_bytes(), expected_read.as_bytes())
    {
        return Ok(AccessRole::Read);
    }
    Err("missing or invalid bearer token".to_string())
}

fn visible_tools_for_role(state: &GatewayState, role: AccessRole) -> HashSet<String> {
    match role {
        AccessRole::Write => (*state.allowed_tools).clone(),
        AccessRole::Read => {
            let mut visible: HashSet<String> = state
                .allowed_tools
                .intersection(&state.read_role_eligible_tools)
                .cloned()
                .collect();
            if let Some(override_tools) = &state.read_role_tools_override {
                visible = visible.intersection(override_tools).cloned().collect();
            }
            visible
        }
    }
}

fn parse_allowed_tools_from_value(raw: Option<&str>) -> Result<HashSet<String>> {
    let from_env = raw
        .map(|s| {
            s.split(',')
                .map(|v| canonical_tool_name(v.trim()).to_string())
                .filter(|v| !v.is_empty())
                .collect::<HashSet<_>>()
        })
        .filter(|set| !set.is_empty());

    let set = from_env.unwrap_or_else(|| {
        DEFAULT_ALLOWED_TOOLS
            .iter()
            .map(|n| canonical_tool_name(n).to_string())
            .collect()
    });
    let registry_names: HashSet<&str> = TOOL_REGISTRY.iter().map(|e| e.name).collect();
    for name in &set {
        if !registry_names.contains(name.as_str()) {
            anyhow::bail!("unknown tool in VOX_MCP_HTTP_ALLOWED_TOOLS: {name}");
        }
    }
    Ok(set)
}

fn request_identity(state: &GatewayState, peer: &SocketAddr, headers: &HeaderMap) -> String {
    if state.trust_forwarded_for
        && let Some(v) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok())
    {
        let first = v.split(',').next().map(str::trim).unwrap_or_default();
        if !first.is_empty() {
            return first.to_string();
        }
    }
    peer.ip().to_string()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    let mut diff = (a.len() ^ b.len()) as u8;
    for i in 0..a.len().max(b.len()) {
        let ai = *a.get(i).unwrap_or(&0);
        let bi = *b.get(i).unwrap_or(&0);
        diff |= ai ^ bi;
    }
    diff == 0
}

fn read_bool_env(name: &str) -> Option<bool> {
    std::env::var(name).ok().map(|v| {
        let t = v.trim();
        t == "1"
            || t.eq_ignore_ascii_case("true")
            || t.eq_ignore_ascii_case("yes")
            || t.eq_ignore_ascii_case("on")
    })
}

fn mobile_workspace_html() -> &'static str {
    r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1.0, viewport-fit=cover" />
  <title>Vox Mobile Workspace</title>
  <style>
    :root { color-scheme: dark light; font-family: Inter, system-ui, sans-serif; }
    body { margin: 0; background: #0f1117; color: #e8eaf0; }
    main { max-width: 860px; margin: 0 auto; padding: 1rem; display: grid; gap: 0.75rem; }
    h1 { margin: 0; font-size: 1.25rem; }
    .card { background: #1a1d27; border: 1px solid #2e3244; border-radius: 10px; padding: 0.75rem; }
    label { display: block; font-size: 0.9rem; margin-bottom: 0.25rem; color: #9ca3b4; }
    input, select, textarea, button {
      width: 100%; box-sizing: border-box; font: inherit;
      border-radius: 8px; border: 1px solid #2e3244; background: #252836; color: #e8eaf0;
      padding: 0.6rem; min-height: 44px;
    }
    textarea { min-height: 140px; }
    button { cursor: pointer; background: #6366f1; border-color: #6366f1; }
    .row { display: grid; gap: 0.5rem; grid-template-columns: 1fr 1fr; }
    pre { white-space: pre-wrap; overflow-wrap: anywhere; margin: 0; font-size: 0.8rem; }
    .hint { color: #9ca3b4; font-size: 0.82rem; }
  </style>
</head>
<body>
  <main>
    <h1>Vox Mobile Workspace (bounded)</h1>
    <p class="hint">This UI is intentionally scoped for safe remote operations: status, planning/chat, and allowlisted tool calls.</p>
    <section class="card">
      <label for="token">Bearer token</label>
      <input id="token" placeholder="VOX_MCP_HTTP_BEARER_TOKEN" />
      <p class="hint">Used for Authorization: Bearer &lt;token&gt; on each call.</p>
    </section>
    <section class="card">
      <div class="row">
        <button id="refresh-status">Refresh status</button>
        <button id="list-tools">List allowlisted tools</button>
      </div>
      <div class="row" style="margin-top:0.5rem;">
        <button id="workspace-modules">Workspace modules</button>
        <button id="git-diff">Git diff</button>
      </div>
    </section>
    <section class="card">
      <label for="tool-name">Tool name</label>
      <input id="tool-name" value="vox_chat_message" />
      <label for="tool-args">Tool args (JSON object)</label>
      <textarea id="tool-args">{}</textarea>
      <button id="call-tool">Call tool</button>
    </section>
    <section class="card">
      <label>Output</label>
      <pre id="output">Ready.</pre>
    </section>
  </main>
  <script>
    const output = document.getElementById("output");
    const API_BASE = (() => {
      const path = window.location.pathname;
      return path.endsWith("/mobile") ? path.slice(0, -"/mobile".length) : "/v1";
    })();
    const tokenInput = document.getElementById("token");
    tokenInput.value = localStorage.getItem("voxMcpHttpToken") || "";
    tokenInput.addEventListener("change", () => {
      localStorage.setItem("voxMcpHttpToken", tokenInput.value.trim());
    });
    function authHeaders() {
      const token = tokenInput.value.trim();
      const headers = { "Content-Type": "application/json" };
      if (token) headers["Authorization"] = `Bearer ${token}`;
      return headers;
    }
    async function showJson(resp) {
      const txt = await resp.text();
      try { output.textContent = JSON.stringify(JSON.parse(txt), null, 2); }
      catch { output.textContent = txt; }
    }
    document.getElementById("refresh-status").addEventListener("click", async () => {
      const resp = await fetch(`${API_BASE}/mobile/status`, { headers: authHeaders() });
      await showJson(resp);
    });
    document.getElementById("list-tools").addEventListener("click", async () => {
      const resp = await fetch(`${API_BASE}/tools`, { headers: authHeaders() });
      await showJson(resp);
    });
    document.getElementById("workspace-modules").addEventListener("click", async () => {
      const resp = await fetch(`${API_BASE}/tools/call`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ name: "vox_workspace_modules", args: {} }),
      });
      await showJson(resp);
    });
    document.getElementById("git-diff").addEventListener("click", async () => {
      const resp = await fetch(`${API_BASE}/tools/call`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ name: "vox_git_diff", args: {} }),
      });
      await showJson(resp);
    });
    document.getElementById("call-tool").addEventListener("click", async () => {
      const name = document.getElementById("tool-name").value.trim();
      let args = {};
      try { args = JSON.parse(document.getElementById("tool-args").value || "{}"); }
      catch (err) {
        output.textContent = `Invalid JSON args: ${err}`;
        return;
      }
      const resp = await fetch(`${API_BASE}/tools/call`, {
        method: "POST",
        headers: authHeaders(),
        body: JSON.stringify({ name, args }),
      });
      await showJson(resp);
    });
  </script>
</body>
</html>
"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;
    use axum::http::{HeaderValue, header::AUTHORIZATION};

    async fn make_state(calls_per_minute: u32) -> GatewayState {
        let mut allowed = HashSet::new();
        allowed.insert("vox_orchestrator_status".to_string());
        GatewayState {
            server_state: ServerState::new_test().await,
            bearer_token: Some("secret-token".to_string()),
            read_bearer_token: Some("read-token".to_string()),
            allow_unauthenticated: false,
            health_auth_required: false,
            require_forwarded_https: false,
            trust_forwarded_for: false,
            allowed_tools: Arc::new(allowed),
            read_role_eligible_tools: Arc::new(
                ["vox_orchestrator_status".to_string()]
                    .into_iter()
                    .collect(),
            ),
            read_role_tools_override: None,
            calls_per_minute,
            rate_limiter: new_identity_rate_limiter(calls_per_minute),
        }
    }

    #[test]
    fn constant_time_eq_matches_expected() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"abcd"));
    }

    #[tokio::test]
    async fn auth_accepts_matching_bearer() {
        let state = make_state(5).await;
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_static("Bearer secret-token"),
        );
        assert!(enforce_auth(&state, &headers).is_ok());
    }

    #[tokio::test]
    async fn auth_rejects_bad_bearer() {
        let state = make_state(5).await;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer nope"));
        assert!(enforce_auth(&state, &headers).is_err());
    }

    #[tokio::test]
    async fn auth_accepts_read_bearer_as_read_role() {
        let state = make_state(5).await;
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer read-token"));
        assert_eq!(
            resolve_access_role(&state, &headers).expect("read token accepted"),
            AccessRole::Read
        );
    }

    #[test]
    fn parse_allowed_tools_default_has_safe_entries() {
        let set = parse_allowed_tools_from_value(None).expect("default allowlist should parse");
        assert!(set.contains("vox_orchestrator_status"));
        assert!(set.contains("vox_validate_file"));
    }

    #[test]
    fn parse_allowed_tools_rejects_unknown() {
        let err = parse_allowed_tools_from_value(Some("definitely_not_a_real_tool"))
            .expect_err("unknown tool should fail");
        assert!(err.to_string().contains("unknown tool"));
    }

    #[tokio::test]
    async fn rate_limit_blocks_after_budget() {
        let state = make_state(2).await;
        let key = "127.0.0.1".to_string();
        assert!(enforce_rate_limit(&state, &key).is_ok());
        assert!(enforce_rate_limit(&state, &key).is_ok());
        assert!(enforce_rate_limit(&state, &key).is_err());
    }

    #[tokio::test]
    async fn request_identity_prefers_forwarded_when_enabled() {
        let mut state = make_state(3).await;
        state.trust_forwarded_for = true;
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            HeaderValue::from_static("203.0.113.10, 10.0.0.1"),
        );
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        assert_eq!(
            request_identity(&state, &peer, &headers),
            "203.0.113.10".to_string()
        );
    }

    #[tokio::test]
    async fn ws_list_tools_filtered_to_allowlist() {
        let state = make_state(5).await;
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        let msg = WsMessageIn {
            id: Some("1".to_string()),
            msg_type: "list_tools".to_string(),
            name: None,
            args: None,
        };
        let out = ws_handle_message(&state, msg, peer, AccessRole::Write).await;
        assert!(out.success);
        let tools = out
            .data
            .get("tools")
            .and_then(|v| v.as_array())
            .expect("tools array");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].as_str(), Some("vox_orchestrator_status"));
    }

    #[tokio::test]
    async fn ws_call_tool_requires_name() {
        let state = make_state(5).await;
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        let msg = WsMessageIn {
            id: Some("x".to_string()),
            msg_type: "call_tool".to_string(),
            name: None,
            args: Some(Value::Null),
        };
        let out = ws_handle_message(&state, msg, peer, AccessRole::Write).await;
        assert!(!out.success);
        assert!(out.is_error);
        assert_eq!(out.error.as_deref(), Some("missing tool name"));
    }

    #[tokio::test]
    async fn read_role_is_limited_to_read_allowlist() {
        let mut state = make_state(5).await;
        state.allowed_tools = Arc::new(
            [
                "vox_orchestrator_status".to_string(),
                "vox_inline_edit".to_string(),
            ]
            .into_iter()
            .collect(),
        );
        state.read_role_eligible_tools = Arc::new(
            ["vox_orchestrator_status".to_string()]
                .into_iter()
                .collect(),
        );
        let visible = visible_tools_for_role(&state, AccessRole::Read);
        assert!(visible.contains("vox_orchestrator_status"));
        assert!(!visible.contains("vox_inline_edit"));
    }

    #[test]
    fn metadata_read_role_eligible_contains_expected_tools() {
        let set = metadata_read_role_eligible_tools();
        assert!(set.contains("vox_orchestrator_status"));
        assert!(set.contains("vox_task_status"));
        assert!(set.contains("vox_git_status"));
    }

    #[tokio::test]
    async fn http_tools_read_role_hides_write_only_tools() {
        let mut state = make_state(5).await;
        state.allowed_tools = Arc::new(
            [
                "vox_orchestrator_status".to_string(),
                "vox_inline_edit".to_string(),
            ]
            .into_iter()
            .collect(),
        );
        state.read_role_eligible_tools = Arc::new(
            ["vox_orchestrator_status".to_string()]
                .into_iter()
                .collect(),
        );
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer read-token"));
        let resp = http_tools(State(state), ConnectInfo(peer), headers).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let parsed: Value = serde_json::from_slice(&body).expect("json response");
        let tools = parsed
            .get("tools")
            .and_then(|v| v.as_array())
            .expect("tools array");
        let names: HashSet<String> = tools
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect();
        assert!(names.contains("vox_orchestrator_status"));
        assert!(!names.contains("vox_inline_edit"));
    }

    #[tokio::test]
    async fn http_call_tool_read_role_denies_write_only_tool() {
        let mut state = make_state(5).await;
        state.allowed_tools = Arc::new(
            [
                "vox_orchestrator_status".to_string(),
                "vox_inline_edit".to_string(),
            ]
            .into_iter()
            .collect(),
        );
        state.read_role_eligible_tools = Arc::new(
            ["vox_orchestrator_status".to_string()]
                .into_iter()
                .collect(),
        );
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer read-token"));
        let resp = http_call_tool(
            State(state),
            ConnectInfo(peer),
            headers,
            Json(ToolCallRequest {
                name: "vox_inline_edit".to_string(),
                args: Value::Object(serde_json::Map::new()),
            }),
        )
        .await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let parsed: ToolCallResponse = serde_json::from_slice(&body).expect("json response");
        assert!(!parsed.success);
        assert!(parsed.is_error);
        assert!(
            parsed
                .result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .contains("not allowed for current gateway role")
        );
    }

    #[tokio::test]
    async fn http_info_exposes_effective_read_role_tools() {
        let mut state = make_state(5).await;
        state.allowed_tools = Arc::new(
            [
                "vox_orchestrator_status".to_string(),
                "vox_inline_edit".to_string(),
            ]
            .into_iter()
            .collect(),
        );
        state.read_role_eligible_tools = Arc::new(
            ["vox_orchestrator_status".to_string()]
                .into_iter()
                .collect(),
        );
        let peer: SocketAddr = "127.0.0.1:1234".parse().expect("socket parse");
        let mut headers = HeaderMap::new();
        headers.insert(AUTHORIZATION, HeaderValue::from_static("Bearer read-token"));
        let resp = http_info(State(state), ConnectInfo(peer), headers).await;
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), 1024 * 1024)
            .await
            .expect("read body");
        let parsed: Value = serde_json::from_slice(&body).expect("json response");
        let read_tools = parsed
            .get("read_role_allowed_tools")
            .and_then(|v| v.as_array())
            .expect("read_role_allowed_tools array");
        let names: HashSet<String> = read_tools
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect();
        assert!(names.contains("vox_orchestrator_status"));
        assert!(!names.contains("vox_inline_edit"));
    }

    #[tokio::test]
    async fn router_level_read_token_filters_tools_and_denies_write_only_call() {
        let mut state = make_state(10).await;
        state.allowed_tools = Arc::new(
            [
                "vox_orchestrator_status".to_string(),
                "vox_inline_edit".to_string(),
            ]
            .into_iter()
            .collect(),
        );
        state.read_role_eligible_tools = Arc::new(
            ["vox_orchestrator_status".to_string()]
                .into_iter()
                .collect(),
        );
        let app = Router::new()
            .route("/v1/tools", get(http_tools))
            .route("/v1/tools/call", post(http_call_tool))
            .with_state(state.clone());

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test listener");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            let _ = serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await;
        });
        let client = reqwest::Client::new();
        let tools_resp = client
            .get(format!("http://{addr}/v1/tools"))
            .header("authorization", "Bearer read-token")
            .send()
            .await
            .expect("tools request");
        assert_eq!(tools_resp.status(), reqwest::StatusCode::OK);
        let tools_json: Value = tools_resp.json().await.expect("tools json");
        let names: HashSet<String> = tools_json["tools"]
            .as_array()
            .expect("tools array")
            .iter()
            .filter_map(|v| v.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect();
        assert!(names.contains("vox_orchestrator_status"));
        assert!(!names.contains("vox_inline_edit"));

        let call_resp = client
            .post(format!("http://{addr}/v1/tools/call"))
            .header("authorization", "Bearer read-token")
            .json(&serde_json::json!({
                "name": "vox_inline_edit",
                "args": {}
            }))
            .send()
            .await
            .expect("call request");
        assert_eq!(call_resp.status(), reqwest::StatusCode::OK);
        let call_json: ToolCallResponse = call_resp.json().await.expect("call json");
        assert!(!call_json.success);
        assert!(call_json.is_error);
        assert!(
            call_json
                .result
                .get("error")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .contains("not allowed for current gateway role")
        );
        server.abort();
    }
}
