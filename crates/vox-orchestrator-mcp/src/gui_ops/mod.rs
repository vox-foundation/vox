//! Daemon-side handlers backing Axis commands that used to open `vox_db` in the GUI process
//! (capability-profiles plan 07). Reads are `/api/v2` routes merged into
//! `http_gateway::dashboard_api::router()`; mutations are MCP tools dispatched from
//! `crate::dispatch` so `authorize_tool` gates them.

pub mod attention;
pub mod chat;
pub mod dto;
pub mod gamify;
pub mod harness;
pub mod inproc;
pub mod knowledge;
pub mod models;
pub mod orchestrator;
pub mod prefs;
pub mod research;
pub mod scientia;
pub mod search;

use std::sync::Arc;

use axum::Json;
use serde_json::Value;

use crate::http_gateway::GatewayState;
use crate::server_state::ServerState;
use crate::services::routes::err;

/// Codex user namespace the GUI has always used for its preferences.
pub const GUI_USER_ID: &str = "local_user";

const REM_GUI_DB: &str = "Attach VoxDb (`VOX_DB_PATH` / `VOX_DB_URL`) to the Vox server; Axis surfaces backed by the workspace DB need it.";

/// The daemon's attached DB, or a stable error string for a `ToolResult`.
pub fn require_db(state: &ServerState) -> Result<&Arc<vox_db::VoxDb>, String> {
    state
        .db
        .as_ref()
        .ok_or_else(|| format!("VoxDb not attached. {REM_GUI_DB}"))
}

pub(crate) fn api_db(gs: &GatewayState) -> Result<Arc<vox_db::VoxDb>, Json<Value>> {
    gs.server_state()
        .db
        .clone()
        .ok_or_else(|| err("db_unavailable", REM_GUI_DB))
}

/// Plan-07 routes are served only when the active profile lists one of the Axis surfaces
/// (`dev` lists `*`).
pub(crate) fn enforce_dashboard_surface(
    gs: &GatewayState,
    surfaces: &[&str],
) -> Result<(), Json<Value>> {
    let profile = &gs.server_state().active_profile;
    if profile.axis_surfaces.contains("*")
        || surfaces.iter().any(|s| profile.axis_surfaces.contains(s))
    {
        Ok(())
    } else {
        Err(err(
            "tool_not_in_profile",
            &format!(
                "surface {surfaces:?} is not served by profile {}",
                profile.id_string()
            ),
        ))
    }
}

/// Dashboard read auth (`enforce_dashboard_read`) plus the plan-07 surface gate.
pub(crate) fn enforce_route(
    gs: &GatewayState,
    peer: &std::net::SocketAddr,
    headers: &axum::http::HeaderMap,
    surfaces: &[&str],
) -> Result<(), Json<Value>> {
    crate::http_gateway::dashboard_api::enforce_dashboard_read(gs, peer, headers)?;
    enforce_dashboard_surface(gs, surfaces)
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::collections::HashSet;
    use std::sync::Arc;

    use crate::http_gateway::GatewayState;
    use crate::server_state::ServerState;

    pub(crate) async fn state_with_memory_db() -> (ServerState, Arc<vox_db::VoxDb>) {
        let db = Arc::new(
            vox_db::VoxDb::connect(vox_db::DbConfig::Memory)
                .await
                .expect("memory db"),
        );
        let state = ServerState::new_test()
            .await
            .with_db_initialized(db.clone())
            .await;
        (state, db)
    }

    pub(crate) async fn gateway_with_memory_db() -> (GatewayState, Arc<vox_db::VoxDb>) {
        let (state, db) = state_with_memory_db().await;
        (
            GatewayState::for_test_with_state(state, HashSet::new()).await,
            db,
        )
    }

    pub(crate) fn tool_json(out: &str) -> serde_json::Value {
        serde_json::from_str(out).expect("ToolResult JSON")
    }

    pub(crate) async fn body(resp: axum::Json<serde_json::Value>) -> serde_json::Value {
        use axum::response::IntoResponse;
        let bytes = axum::body::to_bytes(resp.into_response().into_body(), 1 << 20)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    pub(crate) fn peer() -> axum::extract::ConnectInfo<std::net::SocketAddr> {
        axum::extract::ConnectInfo("127.0.0.1:1".parse().unwrap())
    }

    #[tokio::test]
    async fn require_db_reports_missing_db() {
        let state = ServerState::new_test().await;
        let Err(msg) = super::require_db(&state) else {
            panic!("expected missing db");
        };
        assert!(msg.starts_with("VoxDb not attached"));
    }

    #[tokio::test]
    async fn dashboard_surface_gate_honours_dev_wildcard_and_named_profiles() {
        let gs = GatewayState::for_test().await;
        assert!(super::enforce_dashboard_surface(&gs, &["models"]).is_ok());
        let named = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
        );
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let err = super::enforce_dashboard_surface(&gs, &["models"]).unwrap_err();
        assert_eq!(err.0["error"]["code"], "tool_not_in_profile");
    }
}
