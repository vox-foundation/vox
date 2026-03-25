//! MCP Server state and protocol handler implementation.

use rmcp::model::{
    CallToolRequestParams, CallToolResult, Content, Implementation, InitializeRequestParams,
    InitializeResult, ListToolsResult, PaginatedRequestParams, ProtocolVersion, ServerCapabilities,
    ToolsCapability,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, model as protocol};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Mutex as SyncMutex;
use std::sync::RwLock;
use tokio::sync::Mutex;

use vox_db::VoxDb;
use vox_orchestrator::{
    AffinityGroupRegistry, AgentEvent, BudgetManager, Orchestrator, OrchestratorConfig,
    PopuliNodeBrief, RemotePopuliRoutingHint, RemotePopuliSnapshot, SessionConfig, SessionManager,
    load_from_config,
};
use vox_skills::{SkillRegistry, install_builtins, new_registry_arc};

use crate::params::{SubmitTaskParams, TaskStatusParams, ToolResult};
use crate::tools;

/// When `Vox.toml` declares a non-empty `affinity_groups` array, use it; otherwise derive from repo layout.
fn affinity_groups_for_repository(
    repository: &vox_repository::RepositoryContext,
) -> AffinityGroupRegistry {
    repository
        .vox_toml
        .as_deref()
        .and_then(load_from_config)
        .unwrap_or_else(|| AffinityGroupRegistry::detect_from_repository_layout(&repository.root))
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
    /// Persists chat/session turns under `.sessions/<repository_id>/` when enabled.
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
    /// Stops [`Self::spawn_populi_federation_poller`] when re-rooting.
    populi_poll_join: Arc<SyncMutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl ServerState {
    /// Discover the repository from CWD, wire sessions under `.sessions/<repository_id>/`, and boot the orchestrator.
    pub fn new(config: OrchestratorConfig) -> Self {
        let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        let hint = vox_repository::find_project_manifest_root(&cwd).unwrap_or_else(|| cwd.clone());
        let repository = vox_repository::discover_repository_or_fallback(&hint);
        let workspace_root = Some(repository.root.clone());
        let groups = affinity_groups_for_repository(&repository);

        let session_cfg = SessionConfig {
            repository_id: Some(repository.repository_id.clone()),
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

        let mut orch_cfg = config;
        let mem_shard = repository
            .root
            .join(".vox")
            .join("cache")
            .join("repos")
            .join(&repository.repository_id);
        orch_cfg.memory.log_dir = mem_shard.join("memory");
        orch_cfg.memory.memory_md_path = mem_shard.join("memory").join("MEMORY.md");
        let orchestrator_config = orch_cfg.clone();

        let http_client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .expect("reqwest client for vox-mcp");

        let state = Self {
            orchestrator_config,
            repository: repository.clone(),
            orchestrator: Arc::new(Orchestrator::with_groups(orch_cfg, groups)),
            db: None,
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
        };
        state.spawn_orchestrator_event_log_sink();
        state.spawn_populi_federation_poller();
        state
    }

    /// Background poll of populi control plane when `populi_control_url` is set and `populi_poll_interval_secs` > 0.
    pub fn spawn_populi_federation_poller(&self) {
        let url = match self
            .orchestrator_config
            .populi_control_url
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            Some(u) => u.to_string(),
            None => return,
        };
        if self.orchestrator_config.populi_poll_interval_secs == 0 {
            return;
        }
        let interval_secs = self.orchestrator_config.populi_poll_interval_secs.max(1);
        let timeout_ms = self.orchestrator_config.populi_http_timeout_ms.max(500);
        let snap = self.populi_remote_snapshot.clone();
        let orch = self.orchestrator.clone();
        let mut guard = self
            .populi_poll_join
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(h) = guard.take() {
            h.abort();
        }
        let handle = tokio::spawn(async move {
            let mut tick = tokio::time::interval(std::time::Duration::from_secs(interval_secs));
            tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tick.tick().await;
                let timeout = std::time::Duration::from_millis(timeout_ms);
                let client =
                    vox_populi::http_client::PopuliHttpClient::new_with_timeout(&url, timeout)
                        .with_env_token();
                let now = vox_populi::wall_clock_unix_ms();
                match client.list_nodes().await {
                    Ok(f) => {
                        let brief: Vec<PopuliNodeBrief> = f
                            .nodes
                            .iter()
                            .map(|n| PopuliNodeBrief {
                                id: n.id.clone(),
                                last_seen_unix_ms: n.last_seen_unix_ms,
                            })
                            .collect();
                        let routing_hints: Vec<RemotePopuliRoutingHint> = f
                            .nodes
                            .iter()
                            .map(|n| RemotePopuliRoutingHint {
                                node_id: n.id.clone(),
                                capabilities: n.capabilities.clone(),
                                labels: n.capabilities.labels.clone(),
                                gpu_cuda: n.capabilities.gpu_cuda,
                                gpu_metal: n.capabilities.gpu_metal,
                            })
                            .collect();

                        orch.set_remote_populi_routing_hints(routing_hints);

                        let mut w = snap.write().unwrap();
                        *w = RemotePopuliSnapshot::success(now, f.schema_version, brief);
                    }
                    Err(e) => {
                        orch.set_remote_populi_routing_hints(Vec::new());
                        let mut w = snap.write().unwrap();
                        *w = RemotePopuliSnapshot::failure(now, e.to_string());
                    }
                }
            }
        });
        *guard = Some(handle);
    }

    /// Append JSON lines for every [`AgentEvent`] when **`VOX_ORCHESTRATOR_EVENT_LOG`** is set to a file path.
    pub fn spawn_orchestrator_event_log_sink(&self) {
        let Ok(raw) = std::env::var("VOX_ORCHESTRATOR_EVENT_LOG") else {
            return;
        };
        let path = PathBuf::from(raw);
        let orch = self.orchestrator.clone();
        let mut guard = self
            .event_log_sink_join
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(h) = guard.take() {
            h.abort();
        }
        let handle = tokio::spawn(async move {
            let mut rx = orch.event_bus().subscribe();
            use tokio::io::AsyncWriteExt;
            while let Ok(event) = rx.recv().await {
                if matches!(
                    event.kind,
                    vox_orchestrator::AgentEventKind::TokenStreamed { .. }
                ) {
                    continue;
                }
                let Ok(line) = serde_json::to_string(&event) else {
                    continue;
                };
                if let Ok(mut f) = tokio::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&path)
                    .await
                {
                    let _ = f.write_all(line.as_bytes()).await;
                    let _ = f.write_all(b"\n").await;
                }
            }
        });
        *guard = Some(handle);
    }

    /// Override the workspace root.
    pub fn with_workspace_root(mut self, path: PathBuf) -> Self {
        self.workspace_root = Some(path.clone());
        self.repository = vox_repository::discover_repository_or_fallback(&path);
        let groups = affinity_groups_for_repository(&self.repository);

        let session_cfg = SessionConfig {
            repository_id: Some(self.repository.repository_id.clone()),
            ..SessionConfig::default()
        };
        self.session_manager = Arc::new(Mutex::new(
            SessionManager::new(session_cfg.clone()).unwrap_or_else(|e| {
                panic!("in-memory session manager initialization failed: {}", e)
            }),
        ));

        let mem_base = vox_config::repo_memory_cache_dir(
            &self.repository.root,
            &self.repository.repository_id,
        );
        let mut orch_cfg = self.orchestrator_config.clone();
        orch_cfg.memory.log_dir = mem_base.clone();
        orch_cfg.memory.memory_md_path = mem_base.join("MEMORY.md");
        self.orchestrator_config = orch_cfg.clone();
        self.orchestrator = Arc::new(Orchestrator::with_groups(orch_cfg, groups));
        self.spawn_orchestrator_event_log_sink();
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
        }
    }

    /// Attach Codex, stream orchestrator events into Gamify tables, and enable skill persistence.
    pub fn with_db(mut self, db: VoxDb) -> Self {
        let db_arc = Arc::new(db);
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

                if let Some((agent_id, event_type)) = agent_and_type {
                    let mut kind_json = serde_json::to_value(&event.kind).unwrap_or_default();
                    if let Some(obj) = kind_json.as_object_mut() {
                        obj.insert(
                            "repository_id".to_string(),
                            serde_json::Value::String(repository_id.clone()),
                        );
                    }
                    let payload = serde_json::to_string(&kind_json).unwrap_or_default();
                    let _ = vox_ludus::db::insert_event(
                        &db_for_task,
                        &agent_id.to_string(),
                        event_type,
                        Some(&payload),
                    )
                    .await;

                    // Process rewards
                    let _ = vox_ludus::db::process_event_rewards(
                        &db_for_task,
                        vox_ludus::util::DEFAULT_USER_ID,
                        &kind_json,
                    )
                    .await;
                }
            }
        });

        // Wire DB into skill registry for persistence
        self.skill_registry.set_db(db_arc.clone());

        self
    }
}

/// RMCP [`ServerHandler`] implementation listing tools and dispatching `call_tool`.
pub struct VoxMcpServer {
    state: ServerState,
}

impl VoxMcpServer {
    /// Wrap `state` for use with `rmcp` transport loops.
    pub fn new(state: ServerState) -> Self {
        Self { state }
    }
}

impl ServerHandler for VoxMcpServer {
    async fn initialize(
        &self,
        params: InitializeRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        let tool_count = tools::TOOL_REGISTRY.len();
        let vox_version = env!("CARGO_PKG_VERSION");
        Ok(InitializeResult {
            protocol_version: params.protocol_version.clone(),
            server_info: Implementation {
                name: "vox-mcp".to_string(),
                version: vox_version.to_string(),
                ..Default::default()
            },
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                experimental: {
                    let mut map = std::collections::BTreeMap::new();
                    let mut inner = serde_json::Map::new();
                    inner.insert("messagepack".to_string(), serde_json::Value::Bool(true));
                    inner.insert("inmem_transport".to_string(), serde_json::Value::Bool(true));
                    map.insert("transport_capabilities".to_string(), inner);
                    Some(map)
                },
                ..Default::default()
            },
            instructions: Some(format!(
                "vox-mcp v{} | tools: {} | protocol: {}",
                vox_version, tool_count, params.protocol_version,
            )),
        })
    }

    async fn list_tools(
        &self,
        _params: Option<PaginatedRequestParams>,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let mut tools = tools::tool_registry();

        // Auto-register tools from installed skills
        let skills = self.state.skill_registry.list(None);
        for skill in skills {
            for tool_name in &skill.tools {
                if !tools.iter().any(|t| t.name == *tool_name) {
                    tools.push(rmcp::model::Tool {
                        name: std::borrow::Cow::Owned(tool_name.clone()),
                        description: Some(std::borrow::Cow::Owned(format!(
                            "Instructional macro tool from skill: {}",
                            skill.name
                        ))),
                        input_schema: std::sync::Arc::new(serde_json::Map::new()),
                        output_schema: None,
                        meta: None,
                        annotations: None,
                        execution: None,
                        icons: None,
                        title: None,
                    });
                }
            }
        }

        Ok(ListToolsResult {
            meta: None,
            tools,
            next_cursor: None,
        })
    }
    async fn call_tool(
        &self,
        params: CallToolRequestParams,
        _ctx: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        self.state.orchestrator.record_activity();
        let args = params
            .arguments
            .map(serde_json::Value::Object)
            .unwrap_or_else(|| serde_json::Value::Object(Default::default()));
        let name_str = params.name.to_string();
        let (result_json, is_error) =
            match tools::handle_tool_call(&self.state, &name_str, args).await {
                Ok(json) => {
                    let is_err = tool_json_envelope_is_error(&json);
                    (json, is_err)
                }
                Err(e) => (
                    ToolResult::<serde_json::Value>::err(e.to_string()).to_json(),
                    true,
                ),
            };

        Ok(CallToolResult {
            content: vec![Content::text(result_json)],
            is_error: Some(is_error),
            meta: None,
            structured_content: None,
        })
    }
}

/// Returns true when JSON looks like [`ToolResult`] with `success: false` (MCP `is_error` signal).
pub fn tool_json_envelope_is_error(json: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get("success").and_then(|s| s.as_bool()))
        == Some(false)
}

#[cfg(test)]
mod call_tool_tests {
    use super::tool_json_envelope_is_error;

    #[test]
    fn envelope_error_when_success_false() {
        assert!(tool_json_envelope_is_error(
            r#"{"success":false,"error":"nope"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_success_true() {
        assert!(!tool_json_envelope_is_error(
            r#"{"success":true,"data":"x"}"#
        ));
    }

    #[test]
    fn envelope_ok_when_not_tool_result_shape() {
        assert!(!tool_json_envelope_is_error("not json"));
        assert!(!tool_json_envelope_is_error(r#"{"foo":1}"#));
    }
}
