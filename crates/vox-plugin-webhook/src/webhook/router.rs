//! Axum HTTP router for the inbound webhook gateway.
//!
//! Exposes:
//! - POST `/webhooks/:source` — receive an inbound webhook
//! - GET  `/webhooks/health` — health check
//! - GET  `/webhooks/channels` — list registered channels

use std::sync::Arc;

use axum::{
    Router,
    extract::{Path, State},
    http::{HeaderMap, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Json, Response},
    routing::{get, post},
};
use serde::Serialize;
use serde_json::Value;
use tracing::{info, warn};

use super::{
    WebhookError,
    channel::ChannelManager,
    handler::{InboundPayload, WebhookHandler},
};

/// Shared state for the webhook router.
#[derive(Clone)]
pub struct WebhookState {
    pub handler: Arc<WebhookHandler>,
    pub channels: Arc<ChannelManager>,
    /// Sink for processed events (e.g. tokio broadcast channel)
    pub event_sink: Arc<tokio::sync::broadcast::Sender<super::handler::WebhookEvent>>,
    /// Bearer token every inbound request must carry. Required: the host resolves
    /// Clavis `WebhookIngressToken` and passes it to `start_listening`; there is no
    /// unauthenticated mode.
    pub ingress_token: Arc<str>,
}

impl WebhookState {
    pub fn new(handler: WebhookHandler, ingress_token: impl Into<String>) -> Self {
        let (tx, _) = tokio::sync::broadcast::channel(super::config::channel_cap_from_env());
        Self {
            handler: Arc::new(handler),
            channels: Arc::new(ChannelManager::new()),
            event_sink: Arc::new(tx),
            ingress_token: Arc::from(ingress_token.into()),
        }
    }
}

/// Build the Axum `Router` for the webhook gateway.
///
/// All routes except `/webhooks/health` require an
/// `Authorization: Bearer <token>` header matching `WebhookState.ingress_token`.
pub fn build_router(state: WebhookState) -> Router {
    Router::new()
        .route("/webhooks/health", get(health_check))
        .route("/webhooks/channels", get(list_channels))
        .route("/webhooks/{source}", post(receive_webhook))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            bearer_auth_middleware,
        ))
        .with_state(state)
}

/// Serve the webhook gateway on an already-bound listener (the caller binds, so a
/// bind failure is reported to it rather than lost in a background task).
pub async fn serve(
    state: WebhookState,
    listener: tokio::net::TcpListener,
) -> Result<(), WebhookError> {
    let router = build_router(state);
    if let Ok(addr) = listener.local_addr() {
        info!(%addr, "Webhook gateway listening");
    }
    axum::serve(listener, router)
        .await
        .map_err(|e| WebhookError::Http(e.to_string()))
}

// ---------------------------------------------------------------------------
// Middleware
// ---------------------------------------------------------------------------

/// Bearer token authentication middleware.
///
/// - `/webhooks/health` is always passed through (no auth required).
/// - Every other request must carry `Authorization: Bearer <token>` matching
///   `WebhookState.ingress_token` exactly; there is no pass-through.
async fn bearer_auth_middleware(
    State(state): State<WebhookState>,
    request: Request<axum::body::Body>,
    next: Next,
) -> Response {
    // Skip auth on the liveness probe.
    if request.uri().path() == "/webhooks/health" {
        return next.run(request).await;
    }

    let provided = request
        .headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix(vox_http_client::BEARER_PREFIX));

    if provided.is_some_and(|p| {
        super::signing::constant_time_eq(p.as_bytes(), state.ingress_token.as_bytes())
    }) {
        next.run(request).await
    } else {
        warn!("Webhook ingress: rejected request with missing/invalid bearer token");
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid or missing bearer token" })),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
}

async fn health_check() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

#[derive(Serialize)]
struct ChannelListResponse {
    channels: Vec<super::channel::Channel>,
}

async fn list_channels(State(state): State<WebhookState>) -> Json<ChannelListResponse> {
    Json(ChannelListResponse {
        channels: state.channels.list(),
    })
}

// `WebhookResponse` / `WebhookQuery` placeholders were removed (unused).
// `receive_webhook` returns a `serde_json::Value` directly until the response
// shape is needed; reintroduce typed structs at the same time as their fields.

async fn receive_webhook(
    State(state): State<WebhookState>,
    Path(source): Path<String>,
    headers: HeaderMap,
    body: String,
) -> (StatusCode, Json<serde_json::Value>) {
    // Extract event type from headers (X-Vox-Event, or X-GitHub-Event, etc.)
    let event_type = headers
        .get("x-vox-event")
        .or_else(|| headers.get("x-github-event"))
        // DEPRECATED (2026-06-03): GitLab is no longer supported; the
        // `x-gitlab-event` header is still read for backward compatibility only.
        .or_else(|| headers.get("x-gitlab-event"))
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Discord, Slack, and generic fallbacks.
    let signature = headers
        .get("x-signature-ed25519")
        .or_else(|| headers.get("x-slack-signature"))
        .or_else(|| headers.get("x-hub-signature-256"))
        .or_else(|| headers.get("x-vox-signature"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let req_timestamp = headers
        .get("x-signature-timestamp")
        .or_else(|| headers.get("x-slack-request-timestamp"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let parsed_body: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            warn!(source, "Failed to parse webhook body as JSON: {e}");
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({ "error": "invalid JSON body" })),
            );
        }
    };

    let payload = InboundPayload {
        source: source.clone(),
        event_type: event_type.clone(),
        body: parsed_body,
        signature,
        timestamp: req_timestamp,
    };

    match state.handler.handle(&payload) {
        Ok(event) => {
            info!(source, event_type, id = %event.id, "Webhook event accepted");
            let id = event.id.clone();
            let _ = state.event_sink.send(event);
            (
                StatusCode::ACCEPTED,
                Json(serde_json::json!({ "event_id": id.as_str(), "accepted": true })),
            )
        }
        Err(WebhookError::InvalidSignature) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid signature" })),
        ),
        Err(WebhookError::MissingTimestamp) => (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "missing timestamp" })),
        ),
        Err(WebhookError::TimestampOutOfWindow(ts)) => {
            warn!(
                source,
                ts, "Webhook rejected: timestamp outside replay window"
            );
            (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({ "error": "timestamp outside replay window" })),
            )
        }
        Err(e) => {
            warn!(source, "Webhook rejected: {e}");
            (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "error": e.to_string() })),
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_state_requires_and_keeps_the_ingress_token() {
        let state = WebhookState::new(WebhookHandler::new(), "t");
        assert_eq!(&*state.ingress_token, "t");
    }

    #[tokio::test]
    async fn bearer_check_rejects_missing_or_wrong_token_and_admits_the_right_one() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let app = build_router(WebhookState::new(WebhookHandler::new(), "s3cret"));
        tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });

        let client = vox_http_client::client();
        let channels = format!("http://{addr}/webhooks/channels");
        let status = |req: reqwest::RequestBuilder| async move {
            req.send().await.expect("request").status().as_u16()
        };
        assert_eq!(status(client.get(&channels)).await, 401);
        assert_eq!(
            status(client.get(&channels).bearer_auth("wrong")).await,
            401
        );
        assert_eq!(
            status(client.get(&channels).bearer_auth("s3cre")).await,
            401
        );
        assert_eq!(
            status(client.get(&channels).bearer_auth("s3cret")).await,
            200
        );
        let health = format!("http://{addr}/webhooks/health");
        assert_eq!(status(client.get(&health)).await, 200);
    }
}
