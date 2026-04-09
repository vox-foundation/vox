//! Long-lived orchestrator owner: TCP JSON-line RPC (ADR 022 Phase B).
//!
//! Requires **`VOX_ORCHESTRATOR_DAEMON_SOCKET`**: TCP bind (`127.0.0.1:9745`) or **`stdio`** / **`-`** for line JSON on stdin/stdout.

use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use vox_orchestrator::{
    OrchestratorConfig, RemotePopuliSnapshot, a2a, build_repo_scoped_orchestrator,
    clarification_db_inbox_poll, mesh_federation_poll, orch_daemon, runtime,
};

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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .try_init();

    let bind_raw = std::env::var("VOX_ORCHESTRATOR_DAEMON_SOCKET").map_err(|_| {
        anyhow::anyhow!("VOX_ORCHESTRATOR_DAEMON_SOCKET is required (e.g. 127.0.0.1:9745 or stdio)")
    })?;

    let cfg = load_config();
    let build = build_repo_scoped_orchestrator(cfg, None);
    let orch_config = build.config.clone();
    let repository_id = build.repository.repository_id.clone();
    let orch = Arc::new(build.orchestrator);

    let mut db_holder: Option<Arc<vox_db::VoxDb>> = None;
    if let Some(db) =
        vox_db::connect_workspace_journey_optional(vox_db::DbConnectSurface::Runtime, false).await
    {
        let db = Arc::new(db);
        db_holder = Some(db.clone());
        if let Err(e) = orch.init_db(db).await {
            tracing::warn!(error = %e, "orchestrator init_db failed; continuing without persisted Codex");
            db_holder = None;
        } else {
            tracing::info!("Codex attached and orchestrator schema synced");
        }
    }

    runtime::spawn_agent_fleet_if_enabled(orch.clone());

    // MCP parity: mesh federation snapshot, remote task pollers, event log, clarification inbox.
    let populi_remote_snapshot = Arc::new(RwLock::new(RemotePopuliSnapshot::default()));
    let populi_poll_join = Arc::new(Mutex::new(None));
    mesh_federation_poll::spawn_populi_federation_poller(
        &orch_config,
        repository_id.clone(),
        db_holder.clone(),
        orch.clone(),
        Arc::clone(&populi_remote_snapshot),
        Arc::clone(&populi_poll_join),
    );
    a2a::spawn_populi_remote_result_poller(orch.clone(), Arc::new(Mutex::new(None)));
    a2a::spawn_populi_remote_worker_poller(orch.clone(), Arc::new(Mutex::new(None)));

    if let Some(db) = db_holder.as_ref() {
        clarification_db_inbox_poll::spawn_clarification_db_inbox_poller(
            db.clone(),
            repository_id.clone(),
            Arc::new(Mutex::new(None)),
        );
    }
    vox_orchestrator::socrates::spawn_socrates_research_poller(orch.clone());

    if orch_daemon::is_stdio_transport(&bind_raw) {
        return orch_daemon::run_stdio_server(repository_id, orch).await;
    }

    let bind = orch_daemon::normalize_tcp_bind_addr(&bind_raw);
    if bind.is_empty() {
        anyhow::bail!("VOX_ORCHESTRATOR_DAEMON_SOCKET is empty after normalization");
    }

    orch_daemon::run_tcp_server(&bind, repository_id, orch).await
}
