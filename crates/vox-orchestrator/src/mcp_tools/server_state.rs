use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock, Mutex as SyncMutex};
use std::sync::atomic::{AtomicBool};
use tokio::sync::Mutex;

use vox_db::VoxDb;
use vox_repository::RepositoryContext;
use vox_skills::SkillRegistry;
use crate::{
    Orchestrator, OrchestratorConfig, BudgetManager, Observer, 
    RemotePopuliSnapshot, SessionManager, AgentEvent
};

/// Process-wide MCP server context: orchestrator, repository discovery, optional Codex, sessions, skills.
#[derive(Clone)]
pub struct ServerState {
    /// Snapshot of [`OrchestratorConfig`] used to construct the orchestrator (for rare re-rooting).
    pub orchestrator_config: OrchestratorConfig,
    /// Discovered repository root, stable id, and stack capabilities.
    pub repository: RepositoryContext,
    /// Live orchestrator (tasks, agents, VCS, event bus) - now fine-grained concurrent.
    pub orchestrator: Arc<Orchestrator>,
    /// Optional Turso/Codex handle for gamify, preferences, and knowledge graph tools.
    pub db: Option<Arc<VoxDb>>,
    /// Filled when the DB is attached via [`Self::with_db_initialized`] (PRAGMA snapshot for FTS/WAL/FK routing).
    pub sqlite_capabilities: Option<vox_db::capabilities::SqliteProbeSnapshot>,
    /// Persists chat/session turns under `.vox/sessions/<repository_id>/` when enabled.
    pub session_manager: Arc<Mutex<SessionManager>>,
    /// Installed vox-skills registry (also used for MCP skill tools).
    pub skill_registry: Arc<SkillRegistry>,
    /// In-memory buffer of recent token-stream events merged into `poll_events` responses.
    pub transient_events: Arc<Mutex<Vec<AgentEvent>>>,
    /// Root directory of the workspace, used for @mention resolution and PLAN.md writing.
    pub workspace_root: Option<PathBuf>,
    /// Sticky MCP chat model id override (empty string clears in tools that support it).
    pub mcp_chat_model_override: Arc<RwLock<Option<String>>>,
    /// In-memory token/cost caps for MCP LLM calls (paired with Codex usage when `db` is set).
    pub budget_manager: Arc<BudgetManager>,
    /// Shared HTTP client for OpenRouter/Gemini chat completions inside MCP tools.
    pub http_client: reqwest::Client,
    /// Cached basename → candidate paths map for `@file` mention resolution.
    pub mention_path_cache: Arc<SyncMutex<Option<(PathBuf, Arc<HashMap<String, Vec<PathBuf>>>)>>>,
    /// Aborted and replaced when the orchestrator is re-rooted so stale event sinks do not leak.
    /// Last background fetch of `GET /v1/populi/nodes` (read-only federation; see mens SSOT).
    pub populi_remote_snapshot: Arc<RwLock<RemotePopuliSnapshot>>,
    /// Stops the federation poller when re-rooting (see [`Self::with_workspace_root`]).
    pub populi_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops [`Self::spawn_populi_remote_result_poller`] when re-rooting.
    pub populi_remote_result_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops [`Self::spawn_populi_remote_worker_poller`] when re-rooting.
    pub populi_remote_worker_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops the Codex `a2a_messages` clarification inbox poller when re-rooting.
    pub clarification_db_inbox_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Per–MCP-session wall-time analogue for Socrates clarification (`VOX_QUESTIONING_MAX_ATTENTION_MS`).
    pub questioning_attention_spent_ms: Arc<RwLock<HashMap<String, u64>>>,
    /// Set by [`Self::probe_external_orchestrator_daemon_if_configured`]: TCP **`orch.ping`** succeeded and returned a non-empty **`repository_id`** equal to [`Self::repository`].
    pub orch_daemon_repo_id_aligned: Arc<AtomicBool>,
    /// Process-level observer for structural health evaluation.
    pub observer: Arc<Observer>,
    /// Cache for the resolved repo catalog to prevent disk/git reads on every query
    pub catalog_cache: Arc<tokio::sync::RwLock<Option<CachedCatalog>>>,
}

#[derive(Clone)]
pub struct CachedCatalog {
    pub resolved: vox_repository::ResolvedRepoCatalog,
    pub manifest_mtime: std::time::SystemTime,
}

/// Chosen orchestrator backend for the current MCP operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum OrchestratorBackendMode {
    /// Use in-process `ServerState::orchestrator`.
    Embedded,
    /// Use aligned TCP `vox-orchestrator-d` via `OrchDaemonClient`.
    DaemonAlignedTcp,
}

impl ServerState {
    /// Best-effort mirror of [`vox_orchestrator::AttentionEvent`] into Codex via [`AttentionTracker`](crate::attention_tracker::AttentionTracker).
    fn persist_attention_event_if_possible(&self, event: crate::AttentionEvent) {
        let Some(db) = self.db.as_ref().cloned() else {
            return;
        };
        tokio::spawn(async move {
            let tracker = crate::attention_tracker::AttentionTracker::new(&db);
            if let Err(e) = tracker.record_event(&event).await {
                tracing::debug!(error = %e, "attention tracker persistence failed");
            }
        });
    }

    pub fn record_attention_event(&self, mut event: crate::AttentionEvent) {
        let disable_mirror_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxQuestioningMirrorGlobalAttention);
        let disable_mirror = disable_mirror_resolved.expose()
            .is_some_and(|v| v == "0" || v.eq_ignore_ascii_case("false"));
        if disable_mirror {
            event.cost_ms = 0;
        }
        let bm = self.orchestrator.budget_manager_handle();
        crate::sync_lock::rw_write(&*bm).record_attention(&event);
        self.persist_attention_event_if_possible(event);
    }

    pub fn record_clarification_interrupt(
        &self,
        session_key: &str,
        session_cost_ms: u64,
        event: crate::AttentionEvent,
    ) {
        if session_cost_ms > 0 {
            if let Ok(mut g) = self.questioning_attention_spent_ms.write() {
                *g.entry(session_key.to_string()).or_insert(0) += session_cost_ms;
            }
        }
        self.record_attention_event(event);
    }

    pub fn dogfood_trace_path_for(&self, name: &str) -> Option<std::path::PathBuf> {
        let root = self.workspace_root.clone()?;
        let path = root
            .join("target")
            .join("dogfood")
            .join(format!("{name}.jsonl"));
        Some(path)
    }

    pub fn rebalance_backend(&self) -> usize {
        self.orchestrator.rebalance()
    }

    pub async fn spawn_agent_backend(&self, params: crate::mcp_tools::params::SpawnAgentParams) -> Result<crate::AgentId, String> {
        if !params.dynamic.unwrap_or(false)
            && (params.parent_agent_id.is_some()
                || params.source_task_id.is_some()
                || params.delegation_reason.is_some())
        {
            return Err(
                "parent_agent_id / source_task_id / delegation_reason require dynamic=true"
                    .to_string(),
            );
        }
        if params.dynamic.unwrap_or(false) {
            self.orchestrator
                .spawn_dynamic_agent_with_parent(
                    &params.name,
                    params.parent_agent_id.map(crate::AgentId),
                    params.delegation_reason.as_deref(),
                    params.source_task_id.map(crate::TaskId),
                )
                .map_err(|e| e.to_string())
        } else {
            self.orchestrator
                .spawn_agent(&params.name)
                .map_err(|e| e.to_string())
        }
    }

    pub async fn retire_agent_backend(&self, agent_id: crate::AgentId) -> Result<usize, String> {
        self.orchestrator
            .retire_agent(agent_id)
            .await
            .map(|v| v.len())
            .map_err(|e| e.to_string())
    }

    pub async fn pause_agent_backend(&self, agent_id: crate::AgentId) -> Result<(), String> {
        self.orchestrator
            .pause_agent(agent_id)
            .map_err(|e| e.to_string())
    }

    pub async fn resume_agent_backend(&self, agent_id: crate::AgentId) -> Result<(), String> {
        self.orchestrator
            .resume_agent(agent_id)
            .map_err(|e| e.to_string())
    }

    pub fn mcp_agent_fleet_env_enabled() -> bool {
        let disable_fleet = std::env::var("VOX_MCP_AGENT_FLEET")
            .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
            .unwrap_or(false);
        !disable_fleet
    }

    pub fn record_questioning_attention_spend(&self, session_key: &str, cost_ms: u64) {
        if let Ok(mut g) = self.questioning_attention_spent_ms.write() {
            *g.entry(session_key.to_string()).or_insert(0) += cost_ms;
        }
    }

    pub fn questioning_attention_bounds(&self, session_key: &str) -> (u64, u64) {
        let spent = self.questioning_attention_spent_ms.read().ok()
            .and_then(|g| g.get(session_key).copied())
            .unwrap_or(0);
        let budget = self.orchestrator_config.question_attention_budget_ms;
        (spent, budget)
    }

    pub fn reset_all_questioning_attention(&self) {
        if let Ok(mut g) = self.questioning_attention_spent_ms.write() {
            g.clear();
        }
    }

    /// When **`VOX_ORCHESTRATOR_DAEMON_SOCKET`** names a reachable **`vox-orchestrator-d`** peer, log at INFO.
    pub async fn probe_external_orchestrator_daemon_if_configured(&self) {
        let raw_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOrchestratorDaemonSocket);
        let Some(raw) = raw_resolved.expose() else {
            return;
        };
        let addr = raw.trim();
        if addr.is_empty() || addr == "0" || addr.eq_ignore_ascii_case("off") {
            return;
        }
        if crate::orch_daemon::is_stdio_transport(addr) {
            return;
        }
        let client = crate::orch_daemon::OrchDaemonClient::new(
            crate::orch_daemon::normalize_tcp_bind_addr(addr),
        );
        match client.ping().await {
            Ok(v) => {
                let local = self.repository.repository_id.as_str();
                let remote = v.get("repository_id").and_then(|x| x.as_str()).unwrap_or("");
                let aligned = !remote.is_empty() && remote == local;
                self.orch_daemon_repo_id_aligned.store(aligned, std::sync::atomic::Ordering::SeqCst);
                if aligned {
                    tracing::info!(target: "vox_mcp::orch_daemon", "external vox-orchestrator-d reachable and repo aligned");
                }
            }
            Err(_) => {
                self.orch_daemon_repo_id_aligned.store(false, std::sync::atomic::Ordering::SeqCst);
            }
        }
    }

    pub fn orch_daemon_tcp_client_when_repo_aligned(&self) -> Option<crate::orch_daemon::OrchDaemonClient> {
        if !self.orch_daemon_repo_id_aligned.load(std::sync::atomic::Ordering::SeqCst) {
            return None;
        }
        let raw_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxOrchestratorDaemonSocket);
        let raw = raw_resolved.expose()?;
        let addr = raw.trim();
        if addr.is_empty() || crate::orch_daemon::is_stdio_transport(addr) {
            return None;
        }
        Some(crate::orch_daemon::OrchDaemonClient::new(
            crate::orch_daemon::normalize_tcp_bind_addr(addr),
        ))
    }

    pub fn orch_daemon_client_for_status_tool_rpc(&self) -> Option<crate::orch_daemon::OrchDaemonClient> {
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    pub fn orch_daemon_client_for_agent_writes_rpc(&self) -> Option<crate::orch_daemon::OrchDaemonClient> {
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    pub async fn spawn_agent_backend(&self, params: crate::mcp_tools::params::SpawnAgentParams) -> Result<crate::AgentId, String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            let resp = client.spawn_agent_ext(serde_json::json!({
                "name": params.name,
                "dynamic": params.dynamic.unwrap_or(false),
                "parent_agent_id": params.parent_agent_id,
                "delegation_reason": params.delegation_reason,
                "source_task_id": params.source_task_id,
            })).await.map_err(|e| e.to_string())?;
            let id = resp.get("agent_id").and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.spawn_agent_ext missing agent_id".to_string())?;
            return Ok(crate::AgentId(id));
        }
        if params.dynamic.unwrap_or(false) {
            self.orchestrator.spawn_dynamic_agent_with_parent(
                &params.name,
                params.parent_agent_id.map(crate::AgentId),
                params.delegation_reason.as_deref(),
                params.source_task_id.map(crate::TaskId),
            ).map_err(|e| e.to_string())
        } else {
            self.orchestrator.spawn_agent(&params.name).map_err(|e| e.to_string())
        }
    }

    pub async fn retire_agent_backend(&self, agent_id: crate::AgentId) -> Result<usize, String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            let v = client.retire_agent(agent_id.0).await.map_err(|e| e.to_string())?;
            let n = v.get("remaining_tasks").and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.retire_agent missing remaining_tasks".to_string())?;
            return Ok(n as usize);
        }
        self.orchestrator.retire_agent(agent_id).await.map(|v| v.len()).map_err(|e| e.to_string())
    }

    pub async fn pause_agent_backend(&self, agent_id: crate::AgentId) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            return client.pause_agent(agent_id.0).await.map(|_| ()).map_err(|e| e.to_string());
        }
        self.orchestrator.pause_agent(agent_id).map_err(|e| e.to_string())
    }

    pub async fn resume_agent_backend(&self, agent_id: crate::AgentId) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            return client.resume_agent(agent_id.0).await.map(|_| ()).map_err(|e| e.to_string());
        }
        self.orchestrator.resume_agent(agent_id).map_err(|e| e.to_string())
    }
}

/// Returns true when JSON looks like [`ToolResult`] with `success: false` (MCP `is_error` signal).
pub fn tool_json_envelope_is_error(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
        == Some(false)
}

