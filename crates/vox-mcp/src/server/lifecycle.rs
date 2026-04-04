//! [`ServerState`] construction, Populi polling, orchestrator event sinks, and optional DB wiring.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as SyncMutex;
use std::sync::RwLock;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::Mutex;

use vox_db::VoxDb;
use vox_orchestrator::{
    AgentEvent, AgentId, AgentTask, BudgetManager, CompletionAttestation, FileAffinity, Orchestrator,
    OrchestratorConfig, RemotePopuliSnapshot, TaskCapabilityHints, TaskEnqueueHints, TaskId,
    TaskPriority, SessionConfig, SessionManager, build_repo_scoped_orchestrator,
    build_repo_scoped_orchestrator_for_repository,
};
use vox_skills::{SkillRegistry, install_builtins, new_registry_arc};
use vox_socrates_policy::QuestioningPolicy;

/// When truthy (default if unset), MCP spawns [`vox_orchestrator::runtime::AgentFleet`] so queued
/// tasks receive `ProcessQueue` wakes from registered worker actors.
///
/// Disable with `VOX_MCP_AGENT_FLEET=0`, `false`, `no`, or `off`. Same gate as **`vox-orchestrator-d`** ([`vox_orchestrator::runtime::agent_fleet_env_enabled`]).
#[inline]
pub fn mcp_agent_fleet_env_enabled() -> bool {
    vox_orchestrator::runtime::agent_fleet_env_enabled()
}

fn spawn_embedded_agent_fleet_if_enabled(orchestrator: Arc<Orchestrator>) {
    vox_orchestrator::runtime::spawn_agent_fleet_if_enabled(orchestrator);
}

/// Chosen orchestrator backend for the current MCP operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrchestratorBackendMode {
    /// Use in-process `ServerState::orchestrator`.
    Embedded,
    /// Use aligned TCP `vox-orchestrator-d` via `OrchDaemonClient`.
    DaemonAlignedTcp,
}

/// Process-wide MCP server context: orchestrator, repository discovery, optional Codex, sessions, skills.
#[derive(Clone)]
pub struct ServerState {
    /// Snapshot of [`OrchestratorConfig`] used to construct the orchestrator (for rare re-rooting).
    pub orchestrator_config: OrchestratorConfig,
    /// Discovered repository root, stable id, and stack capabilities.
    pub repository: vox_repository::RepositoryContext,
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
    event_log_sink_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Last background fetch of `GET /v1/populi/nodes` (read-only federation; see mens SSOT).
    pub populi_remote_snapshot: Arc<RwLock<RemotePopuliSnapshot>>,
    /// Stops the federation poller when re-rooting (see [`Self::with_workspace_root`]).
    populi_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops [`Self::spawn_populi_remote_result_poller`] when re-rooting.
    populi_remote_result_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops [`Self::spawn_populi_remote_worker_poller`] when re-rooting.
    populi_remote_worker_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Stops the Codex `a2a_messages` clarification inbox poller when re-rooting.
    clarification_db_inbox_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
    /// Per–MCP-session wall-time analogue for Socrates clarification (`VOX_QUESTIONING_MAX_ATTENTION_MS`).
    questioning_attention_spent_ms: Arc<RwLock<HashMap<String, u64>>>,
    /// Set by [`Self::probe_external_orchestrator_daemon_if_configured`]: TCP **`orch.ping`** succeeded and returned a non-empty **`repository_id`** equal to [`Self::repository`].
    ///
    /// Cleared on workspace re-root. Used for optional IPC-first pilots (e.g. **`VOX_MCP_ORCHESTRATOR_TASK_STATUS_RPC`**).
    orch_daemon_repo_id_aligned: Arc<AtomicBool>,
}

impl ServerState {
    /// Best-effort mirror of [`vox_orchestrator::AttentionEvent`] into Codex via [`AttentionTracker`](vox_orchestrator::attention_tracker::AttentionTracker).
    ///
    /// This is **operator / budget-plane diagnostics** (attention ledger), not product “usage telemetry” or remote analytics.
    fn persist_attention_event_if_possible(&self, event: vox_orchestrator::AttentionEvent) {
        let Some(db) = self.db.as_ref().cloned() else {
            return;
        };
        tokio::spawn(async move {
            let tracker = vox_orchestrator::attention_tracker::AttentionTracker::new(&db);
            if let Err(e) = tracker.record_event(&event).await {
                tracing::debug!(error = %e, "attention tracker persistence failed");
            }
        });
    }

    /// Record an attention debit into the in-process orchestrator ledger and mirror to Codex when attached.
    ///
    /// Classification: **local operator diagnostics** (pilot attention budgeting), scoped by MCP/orchestrator policy —
    /// not general-purpose usage telemetry unless explicitly treated as such in SSOT (`docs/src/architecture/telemetry-trust-ssot.md`).
    pub fn record_attention_event(&self, mut event: vox_orchestrator::AttentionEvent) {
        let disable_mirror = std::env::var("VOX_QUESTIONING_MIRROR_GLOBAL_ATTENTION")
            .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
            .unwrap_or(false);
        if disable_mirror {
            event.cost_ms = 0;
        }
        let bm = self.orchestrator.budget_manager_handle();
        vox_orchestrator::sync_lock::rw_write(&*bm).record_attention(&event);
        self.persist_attention_event_if_possible(event);
    }

    /// Discover the repository from CWD, wire sessions under `.vox/sessions/<repository_id>/`, and boot the orchestrator.
    pub fn new(config: OrchestratorConfig) -> Self {
        let build = build_repo_scoped_orchestrator(config, None);
        let repository = build.repository.clone();
        vox_repository::migrate_legacy_sessions_into_vox(&repository.root, &repository.repository_id);
        vox_repository::migrate_legacy_memory_shard_into_vox_memory(
            &repository.root,
            &repository.repository_id,
        );
        let workspace_root = Some(repository.root.clone());

        let session_cfg = SessionConfig {
            repository_id: Some(repository.repository_id.clone()),
            sessions_dir: repository
                .root
                .join(vox_config::mcp_sessions_dir(&repository.repository_id)),
            ..SessionConfig::default()
        };
        let session_manager = SessionManager::new(session_cfg.clone())
            .unwrap_or_else(|e| panic!("in-memory session manager initialization failed: {}", e));
        let registry = new_registry_arc();

        // Auto-install built-in skills in the background
        let registry_for_builtins = registry.clone();
        tokio::spawn(async move {
            match install_builtins(&registry_for_builtins).await {
                Ok(n) if n > 0 => tracing::info!("Auto-installed {} built-in skill(s)", n),
                Ok(_) => {} // already installed
                Err(e) => tracing::warn!("Failed to auto-install built-in skills: {}", e),
            }
        });

        let orchestrator_config = build.config.clone();

        let http_client = vox_reqwest_defaults::client_builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client for vox-mcp");

        let state = Self {
            orchestrator_config,
            repository: repository.clone(),
            orchestrator: Arc::new(build.orchestrator),
            db: None,
            sqlite_capabilities: None,
            session_manager: Arc::new(Mutex::new(session_manager)),
            skill_registry: registry,
            transient_events: Arc::new(Mutex::new(Vec::new())),
            workspace_root,
            mcp_chat_model_override: Arc::new(RwLock::new(None)),
            budget_manager: Arc::new(BudgetManager::new()),
            http_client,
            mention_path_cache: Arc::new(SyncMutex::new(None)),
            event_log_sink_join: Arc::new(SyncMutex::new(None)),
            populi_remote_snapshot: Arc::new(RwLock::new(RemotePopuliSnapshot::default())),
            populi_poll_join: Arc::new(SyncMutex::new(None)),
            populi_remote_result_poll_join: Arc::new(SyncMutex::new(None)),
            populi_remote_worker_poll_join: Arc::new(SyncMutex::new(None)),
            clarification_db_inbox_poll_join: Arc::new(SyncMutex::new(None)),
            questioning_attention_spent_ms: Arc::new(RwLock::new(HashMap::new())),
            orch_daemon_repo_id_aligned: Arc::new(AtomicBool::new(false)),
        };
        state.spawn_orchestrator_event_log_sink();
        state.spawn_populi_federation_poller();
        state.spawn_populi_remote_result_poller();
        state.spawn_populi_remote_worker_poller();
        spawn_embedded_agent_fleet_if_enabled(state.orchestrator.clone());
        state
    }

    /// When **`VOX_ORCHESTRATOR_DAEMON_SOCKET`** names a reachable **`vox-orchestrator-d`** peer, log at INFO.
    ///
    /// Compares **`repository_id`** from **`orch.ping`** with this embed's [`vox_repository::RepositoryContext::repository_id`].
    /// Mismatch logs **WARN** (or **ERROR** when **`VOX_MCP_ORCHESTRATOR_DAEMON_REPOSITORY_ID_STRICT`** is truthy) to surface split-brain setups while MCP still uses an in-process [`Orchestrator`]. Full tool delegation over RPC remains ADR 022 **IPC-first** follow-up.
    pub async fn probe_external_orchestrator_daemon_if_configured(&self) {
        let Ok(raw) = std::env::var("VOX_ORCHESTRATOR_DAEMON_SOCKET") else {
            return;
        };
        let addr = raw.trim();
        if addr.is_empty() || addr == "0" || addr.eq_ignore_ascii_case("off") {
            return;
        }
        if vox_orchestrator::orch_daemon::is_stdio_transport(addr) {
            tracing::debug!(
                target: "vox_mcp::orch_daemon",
                "skip TCP probe: VOX_ORCHESTRATOR_DAEMON_SOCKET is stdio (supervisor-managed daemon)"
            );
            return;
        }
        let strict_repo = std::env::var("VOX_MCP_ORCHESTRATOR_DAEMON_REPOSITORY_ID_STRICT")
            .map(|v| {
                let t = v.trim();
                t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
            })
            .unwrap_or(false);
        let client =
            vox_orchestrator::orch_daemon::OrchDaemonClient::new(vox_orchestrator::orch_daemon::normalize_tcp_bind_addr(addr));
        match client.ping().await {
            Ok(v) => {
                let local = self.repository.repository_id.as_str();
                let remote = v
                    .get("repository_id")
                    .and_then(|x| x.as_str())
                    .unwrap_or("");
                let aligned = !remote.is_empty() && remote == local;
                self.orch_daemon_repo_id_aligned
                    .store(aligned, Ordering::SeqCst);
                if !remote.is_empty() && remote != local {
                    let msg = "external vox-orchestrator-d reachable but repository_id differs from MCP embed — two orchestrator instances; task/tool views can diverge until IPC-first MCP";
                    if strict_repo {
                        tracing::error!(
                            target: "vox_mcp::orch_daemon",
                            local_repository_id = %local,
                            remote_repository_id = %remote,
                            "{msg}"
                        );
                    } else {
                        tracing::warn!(
                            target: "vox_mcp::orch_daemon",
                            local_repository_id = %local,
                            remote_repository_id = %remote,
                            "{msg}"
                        );
                    }
                } else {
                    tracing::info!(
                        target: "vox_mcp::orch_daemon",
                        repository_id = %v["repository_id"],
                        "external vox-orchestrator-d reachable; MCP orchestrator remains in-process until RPC parity"
                    );
                }
            }
            Err(e) => {
                self.orch_daemon_repo_id_aligned.store(false, Ordering::SeqCst);
                tracing::warn!(
                    target: "vox_mcp::orch_daemon",
                    error = %e,
                    "VOX_ORCHESTRATOR_DAEMON_SOCKET set but orchestrator daemon did not respond to orch.ping"
                );
            }
        }
    }

    fn mcp_env_truthy(var: &str) -> bool {
        std::env::var(var)
            .map(|v| {
                let t = v.trim();
                t == "1" || t.eq_ignore_ascii_case("true") || t.eq_ignore_ascii_case("yes")
            })
            .unwrap_or(false)
    }

    /// **`VOX_MCP_ORCHESTRATOR_RPC_READS`**: umbrella to enable all repo-aligned daemon **read** RPC pilots (same effect as truthy **`VOX_MCP_ORCHESTRATOR_TASK_STATUS_RPC`**, **`START_RPC`**, **`STATUS_TOOL_RPC`** together).
    fn mcp_orch_daemon_reads_pilot_enabled(specific_flag: &str) -> bool {
        Self::mcp_env_truthy("VOX_MCP_ORCHESTRATOR_RPC_READS")
            || Self::mcp_env_truthy(specific_flag)
    }

    /// **`VOX_MCP_ORCHESTRATOR_RPC_WRITES`**: umbrella to enable aligned daemon write pilots.
    fn mcp_orch_daemon_writes_pilot_enabled(specific_flag: &str) -> bool {
        Self::mcp_env_truthy("VOX_MCP_ORCHESTRATOR_RPC_WRITES")
            || Self::mcp_env_truthy(specific_flag)
    }

    /// TCP client to **`vox-orchestrator-d`** when startup probe found matching non-empty **`repository_id`**.
    pub(crate) fn orch_daemon_tcp_client_when_repo_aligned(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !self
            .orch_daemon_repo_id_aligned
            .load(Ordering::SeqCst)
        {
            return None;
        }
        let raw = std::env::var("VOX_ORCHESTRATOR_DAEMON_SOCKET").ok()?;
        let addr = raw.trim();
        if addr.is_empty() || vox_orchestrator::orch_daemon::is_stdio_transport(addr) {
            return None;
        }
        Some(vox_orchestrator::orch_daemon::OrchDaemonClient::new(
            vox_orchestrator::orch_daemon::normalize_tcp_bind_addr(addr),
        ))
    }

    /// When **`VOX_MCP_ORCHESTRATOR_TASK_STATUS_RPC`** is truthy and the daemon probe confirmed matching **`repository_id`**, returns a TCP client for **`orch.task_status`** (IPC-first pilot). Otherwise **`None`** (embedded orchestrator only).
    pub fn orch_daemon_client_for_task_status_rpc(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !Self::mcp_orch_daemon_reads_pilot_enabled("VOX_MCP_ORCHESTRATOR_TASK_STATUS_RPC") {
            return None;
        }
        let c = self.orch_daemon_tcp_client_when_repo_aligned();
        if c.is_none() {
            tracing::debug!(
                target: "vox_mcp::orch_daemon",
                "VOX_MCP_ORCHESTRATOR_TASK_STATUS_RPC set but no TCP daemon client after probe alignment; using embedded orchestrator for task_status"
            );
        }
        c
    }

    /// When **`VOX_MCP_ORCHESTRATOR_START_RPC`** is truthy and repo ids aligned, returns a client for **`orch.status`** side-by-side telemetry on **`vox_orchestrator_start`**.
    pub(crate) fn orch_daemon_client_for_start_rpc(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !Self::mcp_orch_daemon_reads_pilot_enabled("VOX_MCP_ORCHESTRATOR_START_RPC") {
            return None;
        }
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    /// When **`VOX_MCP_ORCHESTRATOR_STATUS_TOOL_RPC`** (or read RPC umbrella) is truthy and repo ids align, returns a client to attach daemon **`orch.status`** to **`vox_orchestrator_status`**.
    pub(crate) fn orch_daemon_client_for_status_tool_rpc(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !Self::mcp_orch_daemon_reads_pilot_enabled("VOX_MCP_ORCHESTRATOR_STATUS_TOOL_RPC") {
            return None;
        }
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    /// When write pilots are enabled and repo ids align, returns a client for task lifecycle writes.
    pub(crate) fn orch_daemon_client_for_task_writes_rpc(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !Self::mcp_orch_daemon_writes_pilot_enabled("VOX_MCP_ORCHESTRATOR_TASK_WRITES_RPC") {
            return None;
        }
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    /// When write pilots are enabled and repo ids align, returns a client for agent lifecycle writes.
    pub(crate) fn orch_daemon_client_for_agent_writes_rpc(
        &self,
    ) -> Option<vox_orchestrator::orch_daemon::OrchDaemonClient> {
        if !Self::mcp_orch_daemon_writes_pilot_enabled("VOX_MCP_ORCHESTRATOR_AGENT_WRITES_RPC") {
            return None;
        }
        self.orch_daemon_tcp_client_when_repo_aligned()
    }

    /// Select write backend based on env + daemon alignment.
    pub fn orchestrator_backend_mode_for_writes(&self) -> OrchestratorBackendMode {
        if self
            .orch_daemon_tcp_client_when_repo_aligned()
            .is_some()
            && Self::mcp_env_truthy("VOX_MCP_ORCHESTRATOR_RPC_WRITES")
        {
            OrchestratorBackendMode::DaemonAlignedTcp
        } else {
            OrchestratorBackendMode::Embedded
        }
    }

    /// Submit task against daemon write backend (when enabled) or embedded orchestrator.
    pub async fn submit_task_with_agent_backend(
        &self,
        description: String,
        file_manifest: Vec<FileAffinity>,
        priority: Option<TaskPriority>,
        target_agent: Option<String>,
        capability_requirements: Option<TaskCapabilityHints>,
        enqueue_hints: Option<TaskEnqueueHints>,
        session_id: Option<String>,
    ) -> Result<TaskId, String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            let params = serde_json::json!({
                "description": description,
                "file_manifest": file_manifest,
                "priority": priority,
                "target_agent": target_agent,
                "capability_requirements": capability_requirements,
                "enqueue_hints": enqueue_hints,
                "session_id": session_id,
            });
            let resp = client.submit_task(params).await.map_err(|e| e.to_string())?;
            let task_id = resp
                .get("task_id")
                .and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.submit_task missing task_id".to_string())?;
            return Ok(TaskId(task_id));
        }
        self.orchestrator
            .submit_task_with_agent(
                description,
                file_manifest,
                priority,
                target_agent,
                capability_requirements,
                enqueue_hints,
                session_id,
            )
            .await
            .map_err(|e| e.to_string())
    }

    /// Complete task with attestation against selected write backend.
    pub async fn complete_task_with_attestation_backend(
        &self,
        task_id: TaskId,
        attestation: Option<CompletionAttestation>,
    ) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            let att = attestation
                .map(serde_json::to_value)
                .transpose()
                .map_err(|e| e.to_string())?;
            let _ = client
                .complete_task(task_id.0, att)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        self.orchestrator
            .complete_task_with_attestation(task_id, attestation)
            .await
            .map_err(|e| e.to_string())
    }

    /// Fail task against selected write backend.
    pub async fn fail_task_backend(&self, task_id: TaskId, reason: String) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            let _ = client
                .fail_task(task_id.0, reason)
                .await
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        self.orchestrator
            .fail_task(task_id, reason)
            .await
            .map_err(|e| e.to_string())
    }

    /// Cancel task against selected write backend.
    pub async fn cancel_task_backend(&self, task_id: TaskId) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            return client
                .cancel_task(task_id.0)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        self.orchestrator.cancel_task(task_id).map_err(|e| e.to_string())
    }

    /// Reorder task against selected write backend.
    pub async fn reorder_task_backend(
        &self,
        task_id: TaskId,
        priority: TaskPriority,
    ) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            let p = match priority {
                TaskPriority::Urgent => "urgent",
                TaskPriority::Background => "background",
                TaskPriority::Normal => "normal",
                _ => "normal",
            };
            return client
                .reorder_task(task_id.0, p)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        self.orchestrator
            .reorder_task(task_id, priority)
            .map_err(|e| e.to_string())
    }

    /// Drain agent against selected write backend.
    pub async fn drain_agent_backend(&self, agent_id: AgentId) -> Result<usize, String> {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            let v = client.drain_agent(agent_id.0).await.map_err(|e| e.to_string())?;
            let n = v
                .get("drained_count")
                .and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.drain_agent missing drained_count".to_string())?;
            return Ok(n as usize);
        }
        self.orchestrator
            .drain_agent(agent_id)
            .map(|v: Vec<AgentTask>| v.len())
            .map_err(|e| e.to_string())
    }

    /// Rebalance against selected write backend.
    pub async fn rebalance_backend(&self) -> usize {
        if let Some(client) = self.orch_daemon_client_for_task_writes_rpc() {
            if let Ok(v) = client.rebalance().await {
                if let Some(n) = v.get("rebalanced").and_then(|x| x.as_u64()) {
                    return n as usize;
                }
            }
        }
        self.orchestrator.rebalance()
    }

    /// Spawn agent against selected write backend.
    pub async fn spawn_agent_backend(
        &self,
        params: crate::params::SpawnAgentParams,
    ) -> Result<AgentId, String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            let resp = client
                .spawn_agent_ext(serde_json::json!({
                    "name": params.name,
                    "dynamic": params.dynamic.unwrap_or(false),
                    "parent_agent_id": params.parent_agent_id,
                    "delegation_reason": params.delegation_reason,
                    "source_task_id": params.source_task_id,
                }))
                .await
                .map_err(|e| e.to_string())?;
            let id = resp
                .get("agent_id")
                .and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.spawn_agent_ext missing agent_id".to_string())?;
            return Ok(AgentId(id));
        }
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
                    params.parent_agent_id.map(AgentId),
                    params.delegation_reason.as_deref(),
                    params.source_task_id.map(TaskId),
                )
                .map_err(|e| e.to_string())
        } else {
            self.orchestrator
                .spawn_agent(&params.name)
                .map_err(|e| e.to_string())
        }
    }

    /// Retire agent against selected write backend.
    pub async fn retire_agent_backend(&self, agent_id: AgentId) -> Result<usize, String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            let v = client
                .retire_agent(agent_id.0)
                .await
                .map_err(|e| e.to_string())?;
            let n = v
                .get("remaining_tasks")
                .and_then(|x| x.as_u64())
                .ok_or_else(|| "orch.retire_agent missing remaining_tasks".to_string())?;
            return Ok(n as usize);
        }
        self.orchestrator
            .retire_agent(agent_id)
            .map(|v| v.len())
            .map_err(|e| e.to_string())
    }

    /// Pause agent against selected write backend.
    pub async fn pause_agent_backend(&self, agent_id: AgentId) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            return client
                .pause_agent(agent_id.0)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        self.orchestrator
            .pause_agent(agent_id)
            .map_err(|e| e.to_string())
    }

    /// Resume agent against selected write backend.
    pub async fn resume_agent_backend(&self, agent_id: AgentId) -> Result<(), String> {
        if let Some(client) = self.orch_daemon_client_for_agent_writes_rpc() {
            return client
                .resume_agent(agent_id.0)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string());
        }
        self.orchestrator
            .resume_agent(agent_id)
            .map_err(|e| e.to_string())
    }

    /// Add LLM / clarification-estimated milliseconds toward the session attention analogue.
    ///
    /// When `VOX_QUESTIONING_MIRROR_GLOBAL_ATTENTION` is unset or truthy, also debits
    /// the **embedded orchestrator** [`BudgetManager`] global attention (`attention_snapshot`).
    pub fn record_questioning_attention_spend(&self, session_key: &str, delta_ms: u64) {
        if delta_ms == 0 {
            return;
        }
        let Ok(mut g) = self.questioning_attention_spent_ms.write() else {
            return;
        };
        *g.entry(session_key.to_string()).or_insert(0) += delta_ms;
        let disable_mirror = std::env::var("VOX_QUESTIONING_MIRROR_GLOBAL_ATTENTION")
            .map(|v| v == "0" || v.eq_ignore_ascii_case("false"))
            .unwrap_or(false);
        if !disable_mirror {
            let bm = self.orchestrator.budget_manager_handle();
            vox_orchestrator::sync_lock::rw_write(&*bm)
                .add_questioning_attention_debit_ms(delta_ms);
        }
    }

    /// Session wall-time tally + orchestrator [`vox_orchestrator::AttentionEvent`] (interrupt EWMA and global spent when mirrored).
    pub fn record_clarification_interrupt(
        &self,
        session_key: &str,
        session_cost_ms: u64,
        event: vox_orchestrator::AttentionEvent,
    ) {
        if session_cost_ms > 0 {
            if let Ok(mut g) = self.questioning_attention_spent_ms.write() {
                *g.entry(session_key.to_string()).or_insert(0) += session_cost_ms;
            }
        }
        self.record_attention_event(event);
    }

    pub fn questioning_attention_spent(&self, session_key: &str) -> u64 {
        self.questioning_attention_spent_ms
            .read()
            .ok()
            .and_then(|g| g.get(session_key).copied())
            .unwrap_or(0)
    }

    pub fn questioning_attention_bounds(&self, session_key: &str) -> (u64, u64) {
        let spent = self.questioning_attention_spent(session_key);
        let max = std::env::var("VOX_QUESTIONING_MAX_ATTENTION_MS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or_else(|| QuestioningPolicy::default().max_clarification_attention_ms);
        (spent, max)
    }

    fn spawn_clarification_db_inbox_poller_if_db(&self) {
        let Some(db) = self.db.as_ref() else {
            return;
        };
        vox_orchestrator::clarification_db_inbox_poll::spawn_clarification_db_inbox_poller(
            db.clone(),
            self.repository.repository_id.clone(),
            self.clarification_db_inbox_poll_join.clone(),
        );
    }

    /// Background poll of populi control plane when `populi_control_url` is set and `populi_poll_interval_secs` > 0.
    pub fn spawn_populi_federation_poller(&self) {
        vox_orchestrator::mesh_federation_poll::spawn_populi_federation_poller(
            &self.orchestrator_config,
            self.repository.repository_id.clone(),
            self.db.clone(),
            self.orchestrator.clone(),
            Arc::clone(&self.populi_remote_snapshot),
            Arc::clone(&self.populi_poll_join),
        );
    }

    /// Drain populi **`remote_task_result`** inbox rows (experimental remote execute).
    ///
    /// Runs on its own interval ([`OrchestratorConfig::populi_remote_result_poll_interval_secs`]) so
    /// results are picked up even when federation polling (`populi_poll_interval_secs`) is `0` or very slow.
    pub fn spawn_populi_remote_result_poller(&self) {
        vox_orchestrator::a2a::spawn_populi_remote_result_poller(
            self.orchestrator.clone(),
            Arc::clone(&self.populi_remote_result_poll_join),
        );
    }

    /// Drain populi `remote_task_envelope` rows and emit correlated `remote_task_result` responses.
    pub fn spawn_populi_remote_worker_poller(&self) {
        vox_orchestrator::a2a::spawn_populi_remote_worker_poller(
            self.orchestrator.clone(),
            Arc::clone(&self.populi_remote_worker_poll_join),
        );
    }

    /// Append JSON lines for every [`AgentEvent`] when **`VOX_ORCHESTRATOR_EVENT_LOG`** is set to a file path.
    pub fn spawn_orchestrator_event_log_sink(&self) {
        vox_orchestrator::orchestrator_event_log::spawn_orchestrator_event_log_sink(
            self.orchestrator.clone(),
            Some(self.event_log_sink_join.clone()),
        );
    }

    /// Override the workspace root.
    pub fn with_workspace_root(mut self, path: PathBuf) -> Self {
        self.workspace_root = Some(path.clone());
        self.repository = vox_repository::discover_repository_or_fallback(&path);
        vox_repository::migrate_legacy_sessions_into_vox(
            &self.repository.root,
            &self.repository.repository_id,
        );
        vox_repository::migrate_legacy_memory_shard_into_vox_memory(
            &self.repository.root,
            &self.repository.repository_id,
        );
        self.orch_daemon_repo_id_aligned
            .store(false, Ordering::SeqCst);
        let build = build_repo_scoped_orchestrator_for_repository(
            self.orchestrator_config.clone(),
            &self.repository,
        );

        let session_cfg = SessionConfig {
            repository_id: Some(self.repository.repository_id.clone()),
            sessions_dir: self
                .repository
                .root
                .join(vox_config::mcp_sessions_dir(&self.repository.repository_id)),
            ..SessionConfig::default()
        };
        self.session_manager = Arc::new(Mutex::new(
            SessionManager::new(session_cfg.clone()).unwrap_or_else(|e| {
                panic!("in-memory session manager initialization failed: {}", e)
            }),
        ));

        self.orchestrator_config = build.config.clone();
        self.orchestrator = Arc::new(build.orchestrator);
        self.spawn_orchestrator_event_log_sink();
        if let Ok(mut g) = self.populi_poll_join.lock() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
        if let Ok(mut g) = self.populi_remote_result_poll_join.lock() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
        if let Ok(mut g) = self.populi_remote_worker_poll_join.lock() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
        if let Ok(mut g) = self.clarification_db_inbox_poll_join.lock() {
            if let Some(h) = g.take() {
                h.abort();
            }
        }
        self.spawn_populi_federation_poller();
        self.spawn_populi_remote_result_poller();
        self.spawn_populi_remote_worker_poller();
        self.spawn_clarification_db_inbox_poller_if_db();
        self
    }

    /// Minimal `ServerState` for integration and unit tests.
    ///
    /// Sets **`OrchestratorConfig::toestub_gate`** to **false** so completing a task that touches `*.rs`
    /// does not run post-task validation’s nested **`cargo check --workspace`** (large wall time, nested
    /// file locks on Windows). [`Self::new`] leaves the default gate **on** for real runs.
    pub async fn new_test() -> Self {
        // `for_testing()` disables `toestub_gate` and tightens limits; avoids nested `cargo check --workspace`
        // in `complete_task` when tasks touch `*.rs` (see `vox_orchestrator::validation::post_task_validate`).
        Self::new(OrchestratorConfig::for_testing())
    }

    /// Assemble state for unit tests without running repository discovery.
    #[cfg(test)]
    pub(crate) fn test_stub(
        orchestrator_config: OrchestratorConfig,
        repository: vox_repository::RepositoryContext,
        orchestrator: Arc<Orchestrator>,
        session_manager: Arc<Mutex<SessionManager>>,
        skill_registry: Arc<SkillRegistry>,
    ) -> Self {
        let workspace_root = Some(repository.root.clone());
        Self {
            orchestrator_config,
            repository,
            orchestrator,
            db: None,
            sqlite_capabilities: None,
            session_manager,
            skill_registry,
            transient_events: Arc::new(Mutex::new(Vec::new())),
            workspace_root,
            mcp_chat_model_override: Arc::new(RwLock::new(None)),
            budget_manager: Arc::new(BudgetManager::new()),
            http_client: reqwest::Client::new(),
            mention_path_cache: Arc::new(SyncMutex::new(None)),
            event_log_sink_join: Arc::new(SyncMutex::new(None)),
            populi_remote_snapshot: Arc::new(RwLock::new(RemotePopuliSnapshot::default())),
            populi_poll_join: Arc::new(SyncMutex::new(None)),
            populi_remote_result_poll_join: Arc::new(SyncMutex::new(None)),
            populi_remote_worker_poll_join: Arc::new(SyncMutex::new(None)),
            clarification_db_inbox_poll_join: Arc::new(SyncMutex::new(None)),
            questioning_attention_spent_ms: Arc::new(RwLock::new(HashMap::new())),
            orch_daemon_repo_id_aligned: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Attach Codex after syncing orchestrator schema into the same [`VoxDb`] handle.
    ///
    /// Ensures [`Orchestrator::db`] is populated for reputation routing, A2A mailboxes, and other
    /// orchestrator features that read `self.db()` — not only MCP tool `state.db`.
    pub async fn with_db_initialized(self, db: VoxDb) -> Self {
        let db_arc = Arc::new(db);
        if let Err(e) = self.orchestrator.init_db(db_arc.clone()).await {
            tracing::warn!(
                "orchestrator.init_db failed (orchestrator DB features may be disabled): {}",
                e
            );
        }
        let probe = match db_arc.sqlite_capabilities_snapshot().await {
            Ok(p) => {
                tracing::info!(
                    journal_mode = %p.journal_mode,
                    foreign_keys_on = p.foreign_keys_on,
                    fts5_reported = p.fts5_reported,
                    "sqlite capability probe"
                );
                Some(p)
            }
            Err(e) => {
                tracing::warn!(error = %e, "sqlite capability probe failed");
                None
            }
        };
        let mut next = self.with_db_arc(db_arc);
        next.sqlite_capabilities = probe;
        next
    }

    /// Attach Codex, stream orchestrator events into Gamify tables, and enable skill persistence.
    pub fn with_db(self, db: VoxDb) -> Self {
        self.with_db_arc(Arc::new(db))
    }

    fn with_db_arc(mut self, db_arc: Arc<VoxDb>) -> Self {
        self.db = Some(db_arc.clone());

        let mut session_cfg = self.orchestrator_config.session.clone();
        session_cfg.repository_id = Some(self.repository.repository_id.clone());
        self.session_manager = Arc::new(Mutex::new(
            SessionManager::new(session_cfg.clone())
                .unwrap_or_else(|e| panic!("session manager initialization failed: {}", e))
                .with_db(db_arc.clone()),
        ));

        let mut rx = self.orchestrator.event_bus().subscribe();

        let db_for_task = db_arc.clone();
        let transient = self.transient_events.clone();
        let repository_id = self.repository.repository_id.clone();
        tokio::spawn(async move {
            while let Ok(event) = rx.recv().await {
                let agent_and_type: Option<(u64, &'static str)> = match &event.kind {
                    vox_orchestrator::AgentEventKind::AgentSpawned { agent_id, .. } => {
                        Some((agent_id.0, "AgentSpawned"))
                    }
                    vox_orchestrator::AgentEventKind::AgentRetired { agent_id } => {
                        Some((agent_id.0, "AgentRetired"))
                    }
                    vox_orchestrator::AgentEventKind::ActivityChanged { agent_id, .. } => {
                        Some((agent_id.0, "ActivityChanged"))
                    }
                    vox_orchestrator::AgentEventKind::TaskSubmitted { agent_id, .. } => {
                        Some((agent_id.0, "TaskSubmitted"))
                    }
                    vox_orchestrator::AgentEventKind::TaskStarted { agent_id, .. } => {
                        Some((agent_id.0, "TaskStarted"))
                    }
                    vox_orchestrator::AgentEventKind::TaskCompleted { agent_id, .. } => {
                        Some((agent_id.0, "TaskCompleted"))
                    }
                    vox_orchestrator::AgentEventKind::TaskFailed { agent_id, .. } => {
                        Some((agent_id.0, "TaskFailed"))
                    }
                    vox_orchestrator::AgentEventKind::LockAcquired { agent_id, .. } => {
                        Some((agent_id.0, "LockAcquired"))
                    }
                    vox_orchestrator::AgentEventKind::LockReleased { agent_id, .. } => {
                        Some((agent_id.0, "LockReleased"))
                    }
                    vox_orchestrator::AgentEventKind::AgentIdle { agent_id } => {
                        Some((agent_id.0, "AgentIdle"))
                    }
                    vox_orchestrator::AgentEventKind::AgentBusy { agent_id } => {
                        Some((agent_id.0, "AgentBusy"))
                    }
                    vox_orchestrator::AgentEventKind::MessageSent { from, .. } => {
                        Some((from.0, "MessageSent"))
                    }
                    vox_orchestrator::AgentEventKind::CostIncurred { agent_id, .. } => {
                        Some((agent_id.0, "CostIncurred"))
                    }
                    vox_orchestrator::AgentEventKind::ContinuationTriggered {
                        agent_id, ..
                    } => Some((agent_id.0, "ContinuationTriggered")),
                    vox_orchestrator::AgentEventKind::PlanHandoff { from, .. } => {
                        Some((from.0, "PlanHandoff"))
                    }
                    vox_orchestrator::AgentEventKind::AgentHandoffAccepted { agent_id, .. } => {
                        Some((agent_id.0, "AgentHandoffAccepted"))
                    }
                    vox_orchestrator::AgentEventKind::AgentHandoffRejected { from, .. } => {
                        Some((from.0, "AgentHandoffRejected"))
                    }
                    vox_orchestrator::AgentEventKind::ScopeViolation { agent_id, .. } => {
                        Some((agent_id.0, "ScopeViolation"))
                    }
                    vox_orchestrator::AgentEventKind::PromptConflictDetected { .. } => {
                        Some((0, "PromptConflictDetected"))
                    }
                    vox_orchestrator::AgentEventKind::InjectionDetected { .. } => {
                        Some((0, "InjectionDetected"))
                    }
                    vox_orchestrator::AgentEventKind::CompactionTriggered { agent_id, .. } => {
                        Some((agent_id.0, "CompactionTriggered"))
                    }
                    vox_orchestrator::AgentEventKind::MemoryFlushed { agent_id, .. } => {
                        Some((agent_id.0, "MemoryFlushed"))
                    }
                    vox_orchestrator::AgentEventKind::SessionCreated { agent_id, .. } => {
                        Some((agent_id.0, "SessionCreated"))
                    }
                    vox_orchestrator::AgentEventKind::SessionReset { agent_id, .. } => {
                        Some((agent_id.0, "SessionReset"))
                    }
                    vox_orchestrator::AgentEventKind::WorkflowStarted { .. } => {
                        Some((0, "WorkflowStarted"))
                    }
                    vox_orchestrator::AgentEventKind::WorkflowCompleted { .. } => {
                        Some((0, "WorkflowCompleted"))
                    }
                    vox_orchestrator::AgentEventKind::WorkflowFailed { .. } => {
                        Some((0, "WorkflowFailed"))
                    }
                    vox_orchestrator::AgentEventKind::ActivityStarted { .. } => {
                        Some((0, "ActivityStarted"))
                    }
                    vox_orchestrator::AgentEventKind::ActivityCompleted { .. } => {
                        Some((0, "ActivityCompleted"))
                    }
                    vox_orchestrator::AgentEventKind::ActivityRetried { .. } => {
                        Some((0, "ActivityRetried"))
                    }
                    // JJ-inspired VCS events
                    vox_orchestrator::AgentEventKind::SnapshotCaptured { agent_id, .. } => {
                        Some((agent_id.0, "SnapshotCaptured"))
                    }
                    vox_orchestrator::AgentEventKind::OperationUndone { agent_id, .. } => {
                        Some((agent_id.0, "OperationUndone"))
                    }
                    vox_orchestrator::AgentEventKind::OperationRedone { agent_id, .. } => {
                        Some((agent_id.0, "OperationRedone"))
                    }
                    vox_orchestrator::AgentEventKind::ConflictDetected { agent_ids, .. } => Some((
                        agent_ids.first().map(|a| a.0).unwrap_or(0),
                        "ConflictDetected",
                    )),
                    vox_orchestrator::AgentEventKind::ConflictResolved { .. } => {
                        Some((0, "ConflictResolved"))
                    }
                    vox_orchestrator::AgentEventKind::WorkspaceCreated { agent_id, .. } => {
                        Some((agent_id.0, "WorkspaceCreated"))
                    }
                    vox_orchestrator::AgentEventKind::UrgentRebalanceTriggered { .. } => {
                        Some((0, "UrgentRebalanceTriggered"))
                    }
                    vox_orchestrator::AgentEventKind::TokenStreamed { .. } => {
                        // Keep transient events in memory
                        if let Ok(mut q) = transient.try_lock() {
                            q.push(event.clone());
                        }
                        None
                    }
                    _ => None,
                };

                if let Some((_agent_id, _event_type)) = agent_and_type {
                    let mut kind_json = serde_json::to_value(&event.kind).unwrap_or_default();
                    if let Some(obj) = kind_json.as_object_mut() {
                        obj.insert(
                            "repository_id".to_string(),
                            serde_json::Value::String(repository_id.clone()),
                        );
                        obj.insert("ludus_dedupe_id".to_string(), serde_json::json!(event.id.0));
                    }
                    let _ =
                        vox_ludus::event_router::route_event_auto_user(&db_for_task, &kind_json)
                            .await;
                }
            }
        });

        // Wire DB into skill registry for persistence
        self.skill_registry.set_db(db_arc.clone());

        self.spawn_clarification_db_inbox_poller_if_db();

        self
    }
}
