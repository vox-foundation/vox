//! # vox-mcp binary
//!
//! MCP server entry point — runs on stdio for native Vox agent integration.
//!
//! Startup flow:
//! 1. Initialize logging → stderr (stdout reserved for MCP protocol)
//! 2. Load orchestrator config from `Vox.toml` (or defaults)
//! 3. Create shared `ServerState` with the orchestrator
//! 4. Start MCP server on stdio via `rmcp::transport::stdio()`

use std::path::PathBuf;

use tracing::info;
use vox_mcp::{ServerState, VoxMcpServer};
use vox_orchestrator::OrchestratorConfig;

use rmcp::ServiceExt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging to stderr (stdout is reserved for MCP JSON-RPC protocol)
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .try_init();

    info!("vox-mcp server starting...");

    // Load configuration
    let config = load_config();
    info!(?config, "orchestrator config loaded");

    // Create shared state and server
    let mut state = ServerState::new(config);

    // Same resolution policy as `vox-runtime` (`DbConfig::resolve_standalone`): canonical `VOX_DB_*`
    // with compatibility fallbacks and project default paths — not a hardcoded `vox.db` only.
    match vox_db::DbConfig::resolve_standalone() {
        Ok(db_config) => {
            info!(?db_config, "connecting to database...");
            match vox_db::VoxDb::connect(db_config).await {
                Ok(db) => {
                    state = state.with_db(db);
                    info!("database connected and linked to state");
                }
                Err(e) => {
                    tracing::warn!(
                        "failed to connect to database: {}. persistence disabled.",
                        e
                    );
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "database config resolution failed (resolve_standalone): {e}. persistence disabled."
            );
        }
    }

    vox_mcp::populi_startup::publish_mesh_on_mcp_start(&state).await;

    let server = VoxMcpServer::new(state);
    info!("server state initialized, starting stdio transport...");

    // Start the MCP server on stdio — this is the actual event loop.
    // MCP clients (vox CLI chat, VS Code extension) communicate via stdin/stdout
    // using JSON-RPC 2.0 messages per the MCP specification.
    let service = server
        .serve(rmcp::transport::stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("failed to start MCP server: {e}");
        })?;

    info!("vox-mcp server running on stdio");

    // Block until the service shuts down (client disconnects or EOF)
    service.waiting().await?;

    info!("vox-mcp server shutting down");
    Ok(())
}

/// Load orchestrator configuration with the following precedence:
/// 1. Nearest `Vox.toml` (manifest root), then CWD `Vox.toml` — each merges `[orchestrator]` + `[mens]`
/// 2. `VOX_ORCHESTRATOR_*` / `VOX_MESH_*` environment variables (see mens SSOT)
/// 3. Defaults
fn load_config() -> OrchestratorConfig {
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
                    info!(path = %toml_path.display(), "loaded orchestrator config from Vox.toml");
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
        info!("no readable Vox.toml found, using defaults");
    }

    config.merge_env_overrides();
    config
}
