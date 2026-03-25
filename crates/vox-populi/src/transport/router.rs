use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::Router;
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use super::auth::{bearer_token_eq, populi_control_token_from_env};
use super::handlers::{
    a2a_ack, a2a_inbox, bootstrap_exchange, deliver_a2a, health, heartbeat, join_node, leave_node,
    list_nodes,
};
use super::PopuliTransportState;

/// Bearer authentication mode for [`populi_http_app_with_auth`].
#[derive(Clone, Debug)]
pub enum PopuliHttpAuth {
    /// Read `VOX_MESH_TOKEN` once when building the router (used by [`populi_http_app`] / [`serve`]).
    FromEnv,
    /// No bearer check (e.g. integration tests; explicit open control plane).
    Open,
    /// Require this bearer value; **ignores** the environment (tests or embedded callers).
    Bearer(String),
}

/// Inner control-plane router (no auth layer). Prefer [`populi_http_app`] for serving.
pub fn router(state: PopuliTransportState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/v1/populi/nodes", get(list_nodes))
        .route("/v1/populi/join", post(join_node))
        .route("/v1/populi/heartbeat", post(heartbeat))
        .route("/v1/populi/leave", post(leave_node))
        .route("/v1/populi/bootstrap/exchange", post(bootstrap_exchange))
        .route("/v1/populi/a2a/deliver", post(deliver_a2a))
        .route("/v1/populi/a2a/inbox", post(a2a_inbox))
        .route("/v1/populi/a2a/ack", post(a2a_ack))
        .with_state(state)
}

/// Same as [`populi_http_app`] but with an explicit auth mode (avoids process-global env in tests).
///
/// The expected bearer value is **captured at build time** (not re-read on every request).
pub fn populi_http_app_with_auth(state: PopuliTransportState, auth: PopuliHttpAuth) -> Router {
    let r = router(state);
    let expected: Option<Arc<str>> = match auth {
        PopuliHttpAuth::FromEnv => populi_control_token_from_env().map(Arc::from),
        PopuliHttpAuth::Open => None,
        PopuliHttpAuth::Bearer(t) => {
            let t = t.trim().to_string();
            if t.is_empty() {
                None
            } else {
                Some(Arc::from(t))
            }
        }
    };
    let r = if let Some(expected) = expected {
        r.layer(middleware::from_fn(
            move |req: Request<Body>, next: Next| {
                let expected = Arc::clone(&expected);
                async move {
                    if req.uri().path() == "/health" {
                        return next.run(req).await;
                    }
                    let ok = req
                        .headers()
                        .get(header::AUTHORIZATION)
                        .and_then(|h| h.to_str().ok())
                        .and_then(|s| s.strip_prefix("Bearer "))
                        .is_some_and(|t| bearer_token_eq(expected.as_ref(), t));
                    if !ok {
                        warn!(path = %req.uri().path(), "populi bearer auth rejected request");
                        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
                    }
                    next.run(req).await
                }
            },
        ))
    } else {
        r
    };

    r.layer(PropagateRequestIdLayer::x_request_id())
        .layer(SetRequestIdLayer::x_request_id(MakeRequestUuid))
        .layer(TraceLayer::new_for_http())
}

/// Full app: same routes as [`router`], plus optional `VOX_MESH_TOKEN` bearer check (except `/health`).
pub fn populi_http_app(state: PopuliTransportState) -> Router {
    populi_http_app_with_auth(state, PopuliHttpAuth::FromEnv)
}

/// Bind and serve until error (Ctrl+C stops the process).
pub async fn serve(addr: SocketAddr, state: PopuliTransportState) -> Result<(), std::io::Error> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!(%addr, "vox-populi HTTP control plane listening");
    let app = populi_http_app(state);
    axum::serve(listener, app).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn populi_routes_exist_and_legacy_mens_routes_are_absent() {
        let app = router(PopuliTransportState::new());
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            axum::serve(listener, app).await.expect("serve");
        });

        let client = reqwest::Client::new();
        let ok = client
            .get(format!("http://{addr}/v1/populi/nodes"))
            .send()
            .await
            .expect("GET populi nodes");
        assert_eq!(ok.status(), StatusCode::OK);

        let missing = client
            .get(format!("http://{addr}/v1/mens/nodes"))
            .send()
            .await
            .expect("GET legacy mens nodes");
        assert_eq!(missing.status(), StatusCode::NOT_FOUND);

        server.abort();
    }
}
