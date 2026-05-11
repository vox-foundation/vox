//! Inverse of mesh_invite — accepts a bearer URL, decodes it, joins the mesh.
//!
//! Phase 4, P4-T11.
//!
//! Routes:
//!   POST /api/v2/mesh/invite/preview — decode bearer, return policy preview (no side-effect)
//!   POST /api/v2/mesh/join           — consume bearer and register as worker node

use axum::extract::State;
use axum::response::Json;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::api::mesh_topology::MeshState;

#[derive(Deserialize)]
pub struct JoinRequest {
    pub bearer_url: String,
}

/// Decode and preview an invite bearer without consuming it.
/// Returns the inviter host, slot list, and expiry.
pub async fn preview(
    State(state): State<MeshState>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let parsed =
        parse_bearer_url(&req.bearer_url).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    // Check the bearer exists in the registry (without consuming it).
    let exists = state.registry.peek_bearer(&parsed.token).await;
    if !exists {
        return Err(axum::http::StatusCode::GONE);
    }

    Ok(Json(json!({
        "v": 1,
        "data": {
            "inviter":     parsed.host,
            "expires_in":  parsed.expires_in_hint,
            "slots": [
                { "kind": "gpu", "max_concurrent": 1 }
            ],
        }
    })))
}

/// Consume the bearer and register this machine as a worker node.
pub async fn join(
    State(state): State<MeshState>,
    Json(req): Json<JoinRequest>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    let parsed =
        parse_bearer_url(&req.bearer_url).map_err(|_| axum::http::StatusCode::BAD_REQUEST)?;

    let (peer_id, _slot_kind) = state
        .registry
        .consume_bearer(&parsed.token)
        .await
        .map_err(|_| axum::http::StatusCode::FORBIDDEN)?;

    tracing::info!(
        peer_id = %peer_id,
        host    = %parsed.host,
        "vox.mesh.join.succeeded"
    );

    Ok(Json(json!({
        "v": 1,
        "data": {
            "joined_as": peer_id,
            "inviter":   parsed.host,
        }
    })))
}

// ── URL parsing ───────────────────────────────────────────────────────────────

struct ParsedBearer {
    host: String,
    token: String,
    expires_in_hint: u64,
}

fn parse_bearer_url(url: &str) -> Result<ParsedBearer, ()> {
    // Expected format: vox+invite://<host>?b=<token>
    let rest = url.strip_prefix("vox+invite://").ok_or(())?;
    let (host, query) = rest.split_once('?').ok_or(())?;
    let token = query
        .split('&')
        .find_map(|kv| kv.strip_prefix("b="))
        .ok_or(())?
        .to_string();
    Ok(ParsedBearer {
        host: host.to_string(),
        token,
        expires_in_hint: 600,
    })
}
