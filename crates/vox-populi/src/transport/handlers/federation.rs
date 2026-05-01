//! Federation handlers: directory listing and announcement.

use axum::Json;
use axum::extract::{Extension, State};
use axum::http::StatusCode;

use super::super::auth::{PopuliAuthContext, auth_allows_worker_plane};
use super::super::{FederationAnnounceRequest, FederationDirectoryResponse, PopuliTransportState};
use super::nodes::ResponseErr;

pub(crate) async fn federation_directory(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
) -> Result<Json<FederationDirectoryResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: token required for federation/directory".into(),
        ));
    }

    let entries = {
        let g = st.federated_meshes.read().await;
        g.clone()
    };

    Ok(Json(FederationDirectoryResponse { entries }))
}

pub(crate) async fn federation_announce(
    State(st): State<PopuliTransportState>,
    Extension(ctx): Extension<PopuliAuthContext>,
    Json(req): Json<FederationAnnounceRequest>,
) -> Result<Json<FederationDirectoryResponse>, ResponseErr> {
    if !auth_allows_worker_plane(&ctx) {
        return Err(ResponseErr(
            StatusCode::FORBIDDEN,
            "populi: token required for federation/announce".into(),
        ));
    }

    // Optional Security: Verify entry signature if provided
    if let (Some(sig), Some(pk)) = (&req.entry.signature, &req.entry.public_key) {
        let msg = req.entry.canonical_bytes();
        let mut sig_arr = [0u8; 64];
        if sig.len() == 64 {
            sig_arr.copy_from_slice(sig);
            if let Ok(vk) = vox_crypto::facades::verifying_key_from_bytes(pk) {
                if !vox_crypto::facades::verify(&vk, &msg, &sig_arr) {
                    return Err(ResponseErr(
                        StatusCode::BAD_REQUEST,
                        "populi: invalid federation announcement signature".into(),
                    ));
                }
            } else {
                return Err(ResponseErr(
                    StatusCode::BAD_REQUEST,
                    "populi: invalid federation public key".into(),
                ));
            }
        } else {
            return Err(ResponseErr(
                StatusCode::BAD_REQUEST,
                "populi: invalid signature length (expected 64 bytes)".into(),
            ));
        }
    } else if req.entry.public {
        // Policy: Public meshes MUST sign their announcements in production
        // For now, we log a warning but allow it for backward compatibility or simple LAN use.
        tracing::warn!(scope_id = %req.entry.scope_id, "Received unsigned public mesh announcement");
    }

    let entries = {
        let mut g = st.federated_meshes.write().await;

        if let Some(i) = g.iter().position(|e| e.scope_id == req.entry.scope_id) {
            g[i] = req.entry;
        } else {
            g.push(req.entry);
        }
        g.clone()
    };

    Ok(Json(FederationDirectoryResponse { entries }))
}
