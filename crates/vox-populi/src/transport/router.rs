use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::extract::DefaultBodyLimit;
use axum::http::{HeaderName, HeaderValue, Request, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use tower_http::request_id::{MakeRequestUuid, PropagateRequestIdLayer, SetRequestIdLayer};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};

use super::PopuliTransportState;
use super::auth::{PopuliAuthContext, PopuliMeshAuthRuntime};
use super::handlers::{
    a2a_ack, a2a_inbox, a2a_lease_renew, admin_exec_lease_revoke, admin_maintenance,
    admin_quarantine, bootstrap_exchange, deliver_a2a, dispatch_results_poll, dispatch_script, execute_on_worker,
    exec_lease_grant, exec_lease_list, exec_lease_release, exec_lease_renew, health, heartbeat,
    join_node, leave_node, list_nodes,
};

/// Default max JSON body size for control-plane POST routes (join, heartbeat, A2A, …).
const POPULI_DEFAULT_MAX_BODY_BYTES: usize = 512 * 1024;

fn populi_max_body_limit_bytes() -> usize {
    const MIN: usize = 2 * 1024;
    const MAX: usize = 8 * 1024 * 1024;
    std::env::var("VOX_MESH_HTTP_MAX_BODY_BYTES")
        .ok()
        .and_then(|s| s.parse().ok())
        .filter(|&n| (MIN..=MAX).contains(&n))
        .unwrap_or(POPULI_DEFAULT_MAX_BODY_BYTES)
}

/// Bearer authentication mode for [`populi_http_app_with_auth`].
#[derive(Clone, Debug)]
pub enum PopuliHttpAuth {
    /// Read mesh / role tokens once when building the router via Clavis (used by [`populi_http_app`] / [`serve`]).
    FromEnv,
    /// No bearer check (e.g. integration tests; explicit open control plane).
    Open,
    /// Require this bearer value; **ignores** the environment (tests or embedded callers).
    Bearer(String),
    /// Caller-built [`PopuliMeshAuthRuntime`] (tests and custom embedders).
    Custom(PopuliMeshAuthRuntime),
}

fn mesh_auth_runtime_for(auth: &PopuliHttpAuth) -> PopuliMeshAuthRuntime {
    match auth {
        PopuliHttpAuth::FromEnv => PopuliMeshAuthRuntime::from_env(),
        PopuliHttpAuth::Open => PopuliMeshAuthRuntime::default(),
        PopuliHttpAuth::Bearer(t) => PopuliMeshAuthRuntime::legacy_mesh_token_only(t),
        PopuliHttpAuth::Custom(rt) => rt.clone(),
    }
}

fn stamp_populi_feature_header<B>(res: &mut Response<B>) {
    let v = HeaderValue::from_static(
        "mesh-auth-v1,a2a-observe-v1,quarantine-v1,maintenance-v1,maintenance-deadline-v1,lease-renew-v1,exec-lease-v1,exec-lease-admin-revoke-v1,exec-lease-persist-v1,a2a-inbox-limit-v1,jwt-bearer-v1,result-attest-v1,detached-results-v1",
    );
    res.headers_mut()
        .insert(HeaderName::from_static("x-populi-feature"), v);
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
        .route("/v1/populi/exec/lease/grant", post(exec_lease_grant))
        .route("/v1/populi/exec/leases", get(exec_lease_list))
        .route("/v1/populi/exec/lease/renew", post(exec_lease_renew))
        .route("/v1/populi/exec/lease/release", post(exec_lease_release))
        .route("/v1/populi/a2a/deliver", post(deliver_a2a))
        .route("/v1/populi/a2a/inbox", post(a2a_inbox))
        .route("/v1/populi/a2a/ack", post(a2a_ack))
        .route("/v1/populi/a2a/lease-renew", post(a2a_lease_renew))
        .route("/v1/populi/admin/quarantine", post(admin_quarantine))
        .route("/v1/populi/admin/maintenance", post(admin_maintenance))
        .route("/v1/populi/dispatch", post(dispatch_script))
        .route("/v1/populi/dispatch/result/{dispatch_id}", get(dispatch_results_poll))
        .route("/v1/populi/worker/execute", post(execute_on_worker))
        .route(
            "/v1/populi/admin/exec-lease/revoke",
            post(admin_exec_lease_revoke),
        )
        .with_state(state)
}

/// Same as [`populi_http_app`] but with an explicit auth mode (avoids process-global env in tests).
///
/// The expected bearer value is **captured at build time** (not re-read on every request).
pub fn populi_http_app_with_auth(state: PopuliTransportState, auth: PopuliHttpAuth) -> Router {
    let mesh_replay = Arc::clone(&state.mesh_replay);
    let r = router(state);
    let runtime = Arc::new(mesh_auth_runtime_for(&auth));
    let runtime_cl = Arc::clone(&runtime);
    let mesh_replay_cl = Arc::clone(&mesh_replay);
    let r = r.layer(middleware::from_fn(
        move |mut req: Request<Body>, next: Next| {
            // Clone Arcs here so the inner `async move` does not capture `runtime_cl` /
            // `mesh_replay_cl` (which would make this middleware closure `FnOnce`).
            let runtime = Arc::clone(&runtime_cl);
            let mesh_replay = Arc::clone(&mesh_replay_cl);
            async move {
                let path = req.uri().path();
                if path == "/health" || path == "/v1/populi/bootstrap/exchange" {
                    req.extensions_mut().insert(PopuliAuthContext::FullAccess);
                    let mut res = next.run(req).await;
                    stamp_populi_feature_header(&mut res);
                    return res;
                }
                if !runtime.requires_bearer() {
                    req.extensions_mut().insert(PopuliAuthContext::FullAccess);
                    let mut res = next.run(req).await;
                    stamp_populi_feature_header(&mut res);
                    return res;
                }
                let token = req
                    .headers()
                    .get(header::AUTHORIZATION)
                    .and_then(|h| h.to_str().ok())
                    .and_then(|s| s.strip_prefix("Bearer "))
                    .map(str::trim)
                    .filter(|t| !t.is_empty());
                let Some(presented) = token else {
                    warn!(path = %path, "populi bearer auth missing");
                    let mut res = (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
                    stamp_populi_feature_header(&mut res);
                    return res;
                };
                if let Some(role) = runtime.classify_bearer(presented) {
                    req.extensions_mut().insert(PopuliAuthContext::Role(role));
                    let mut res = next.run(req).await;
                    stamp_populi_feature_header(&mut res);
                    return res;
                }
                if runtime.jwt_hmac.is_some() {
                    let now_sec = crate::now_ms() / 1000;
                    let mut maps = mesh_replay.maps().write().await;
                    if let Some(role) =
                        runtime.try_authorize_jwt(presented, now_sec, &mut maps.jwt_jti)
                    {
                        drop(maps);
                        mesh_replay.persist_if_configured().await;
                        req.extensions_mut().insert(PopuliAuthContext::Role(role));
                        let mut res = next.run(req).await;
                        stamp_populi_feature_header(&mut res);
                        return res;
                    }
                }
                warn!(path = %path, "populi mesh bearer rejected (unknown token)");
                let mut res = (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
                stamp_populi_feature_header(&mut res);
                res
            }
        },
    ));

    let r = r.layer(DefaultBodyLimit::max(populi_max_body_limit_bytes()));

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
        let app = populi_http_app_with_auth(PopuliTransportState::new(), PopuliHttpAuth::Open);
        let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
            .await
            .expect("bind");
        let addr = listener.local_addr().expect("local addr");
        let server = tokio::spawn(async move {
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .expect("serve");
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
