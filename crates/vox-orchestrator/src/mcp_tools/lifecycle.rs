//! [`ServerState`] construction, Populi polling, orchestrator event sinks, and optional DB wiring.

use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    Orchestrator, OrchestratorConfig, 
};
use crate::mcp_tools::server_state::ServerState;
// No longer used in this module after refactor

/// When truthy (default if unset), MCP spawns [`crate::runtime::AgentFleet`] so queued
/// tasks receive `ProcessQueue` wakes from registered worker actors.
#[inline]
pub fn mcp_agent_fleet_env_enabled() -> bool {
    crate::runtime::agent_fleet_env_enabled()
}

fn spawn_embedded_agent_fleet_if_enabled(orchestrator: Arc<Orchestrator>) {
    crate::runtime::spawn_agent_fleet_if_enabled(orchestrator.clone());

    // NOTE: ResolutionAgent (formerly vox_dei) functionality is now integrated 
    // into the main orchestrator dispatch and verification loops.
}

pub fn load_config() -> OrchestratorConfig {
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let mut candidates = Vec::new();
    if let Some(root) = vox_repository::find_project_manifest_root(&cwd) {
        candidates.push(root.join("Vox.toml"));
    }
    candidates.push(PathBuf::from("Vox.toml"));

    let mut config = OrchestratorConfig::default();
    let mut loaded = false;
    for toml_path in candidates {
        if toml_path.is_file() {
            match OrchestratorConfig::load_from_toml(&toml_path) {
                Ok(cfg) => {
                    tracing::info!(path = %toml_path.display(), "loaded orchestrator config from Vox.toml");
                    config = cfg;
                    loaded = true;
                    break;
                }
                Err(e) => tracing::warn!(
                    path = %toml_path.display(),
                    "failed to load Vox.toml: {e}, trying next candidate"
                ),
            }
        }
    }
    if !loaded {
        tracing::info!("no readable Vox.toml found, using defaults");
    }

    config.merge_env_overrides();
    config
}

pub async fn run_stdio_server_blocking() -> anyhow::Result<()> {
    tracing::info!("vox native mcp server starting...");

    // Load configuration
    let config = load_config();
    tracing::info!(?config, "orchestrator config loaded");

    // Create shared state and server
    let mut state = ServerState::new_full(config);
    spawn_embedded_agent_fleet_if_enabled(state.orchestrator.clone());

    // Degraded optional: MCP must keep serving stdio even if Codex is misconfigured
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::Mcp, false).await
    {
        state = state.with_db_initialized(Arc::new(db)).await;
        tracing::info!(
            "workspace journey database connected and linked to state (orchestrator schema synced)"
        );
    }

    crate::mcp_tools::populi_startup::publish_mesh_on_mcp_start(&state).await;

    state
        .probe_external_orchestrator_daemon_if_configured()
        .await;

    // Optional remote/mobile control plane (HTTP + WebSocket).
    let _http_gateway = crate::mcp_tools::http_gateway::spawn_http_gateway_if_enabled(state.clone())?;

    // Flywheel automation: Monitor diversity and trigger training
    let flywheel = crate::services::flywheel::FlywheelMonitor::new(state.orchestrator.clone());
    flywheel.spawn().await;

    let server = crate::mcp_tools::server::VoxMcpServer::new(state);
    tracing::info!("server state initialized, starting stdio transport...");

    // Start the MCP server on stdio via RMCP
    let service = rmcp::ServiceExt::serve(server, rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("failed to start MCP server: {e}");
        })?;

    tracing::info!("vox native mcp server running on stdio");

    // Block until the service shuts down
    service.waiting().await?;

    tracing::info!("vox native mcp server shutting down");
    Ok(())
}
