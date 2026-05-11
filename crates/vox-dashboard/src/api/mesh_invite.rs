//! "Add a Node" wizard backend — one-shot bearer mint with TTL ≤ 10 minutes.
//!
//! ## Anti-goals reminder (per plan §Anti-goals)
//!
//! - The install command is printed to the user, never auto-executed.
//! - The bearer is bound to a single peer_id and expires in ≤ 600 seconds.
//! - The bearer-URL scheme is `vox+invite://<host>?b=<token>`.

use axum::extract::State;
use axum::response::Json;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::time::Duration;

use crate::api::mesh_topology::MeshState;

const MAX_BEARER_TTL_SECS: u64 = 600;

#[derive(Debug, Deserialize)]
pub struct MintRequest {
    pub slot_kind: String,
    pub ttl_secs: u64,
    #[serde(default)]
    pub label: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MintResponse {
    pub peer_id: String,
    pub bearer_url: String,
    pub install_command: String,
    pub install_command_print: String,
    pub qr_svg: String,
    pub expires_in_secs: u64,
}

pub async fn mint(
    State(state): State<MeshState>,
    Json(req): Json<MintRequest>,
) -> Result<Json<Value>, axum::http::StatusCode> {
    // 1. Cap the TTL — hard maximum of 10 minutes regardless of request.
    let ttl = req.ttl_secs.min(MAX_BEARER_TTL_SECS);

    // 2. Derive a peer_id from a UUID v4.
    let peer_id = format!("peer-{}", uuid::Uuid::new_v4().simple());

    // 3. Mint a bearer token bound to (peer_id, slot_kind, expiry).
    let bearer = state
        .registry
        .mint_invite_bearer(&peer_id, &req.slot_kind, Duration::from_secs(ttl))
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    // 4. Build the URL forms.
    let host_port = state.registry.public_host_port().await;
    let bearer_url = format!("vox+invite://{host_port}?b={bearer}");
    let install_command = format!("vox populi join {bearer_url}");
    let install_command_print = format!("{install_command} --print");

    // 5. Generate the QR SVG (server-authoritative — the client renders from the SVG string).
    let qr_svg = generate_qr_svg(&bearer_url);

    // 6. Log the issuance (peer_id only — never the bearer token itself).
    tracing::info!(
        peer_id = %peer_id,
        slot_kind = %req.slot_kind,
        ttl_secs = ttl,
        label = ?req.label,
        "vox.mesh.invite.minted"
    );

    Ok(Json(json!({
        "v": 1,
        "data": {
            "peer_id":               peer_id,
            "bearer_url":            bearer_url,
            "install_command":       install_command,
            "install_command_print": install_command_print,
            "qr_svg":                qr_svg,
            "expires_in_secs":       ttl,
        }
    })))
}

fn generate_qr_svg(content: &str) -> String {
    use qrcode::QrCode;
    use qrcode::render::svg;

    QrCode::new(content.as_bytes())
        .map(|code| {
            code.render::<svg::Color<'_>>()
                .min_dimensions(180, 180)
                .build()
        })
        .unwrap_or_else(|_| {
            // Fallback: a placeholder SVG square so the UI doesn't break on encoding failures.
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="180" height="180"><rect width="180" height="180" fill="#18181b"/><text x="90" y="95" fill="#71717a" font-family="monospace" font-size="11" text-anchor="middle">QR unavailable</text></svg>"##.into()
        })
}
