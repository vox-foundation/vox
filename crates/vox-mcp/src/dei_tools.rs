//! Live DEI orchestrator inspection and lightweight control surfaces exposed as MCP tools.
//!
//! Covers agent queues, lock/budget summaries, Gamify-backed history, affinity graphs,
//! JSON config patch, task submission shims, session heartbeats, and cost recording.

use crate::{AgentInfo, ServerState, StatusResponse, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use vox_ludus::companion::Companion;
use vox_ludus::db::{list_companions, upsert_companion};
use vox_orchestrator::{AgentId, OrchestratorConfig, TaskId};

/// MCP arguments: serialize one agent's task queue as JSON.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct QueueStatusParams {
    /// Target agent id.
    pub agent_id: u64,
}

/// Return the queue snapshot for `params.agent_id`.
pub async fn queue_status(state: &ServerState, params: QueueStatusParams) -> String {
    let orch = &state.orchestrator;
    if let Some(queue_lock) = orch.agent_queue(AgentId(params.agent_id)) {
        let json = queue_lock.read().unwrap().to_json();
        ToolResult::ok(json).to_json()
    } else {
        ToolResult::<String>::err(format!("Agent {} not found", params.agent_id)).to_json()
    }
}

/// Count exclusive/read locks currently held across the orchestrator.
pub async fn lock_status(state: &ServerState) -> String {
    let orch = &state.orchestrator;
    let count = orch.lock_manager().active_lock_count();
    ToolResult::ok(format!("{} active locks", count)).to_json()
}

/// Aggregate token and USD spend tracked in agent budgets.
pub async fn budget_status(state: &ServerState) -> String {
    let orch = &state.orchestrator;

    // Total up from agents
    let mut total_tokens = 0;
    let mut total_cost = 0.0;

    let bh = orch.budget_handle();
    for agent_id in orch.agent_ids() {
        if let Some(budget) = bh.read().unwrap().check_budget(agent_id) {
            total_tokens += budget.tokens_used;
            total_cost += budget.cost_usd;
        }
    }

    ToolResult::ok(format!(
        "Total usage: {} tokens, cost: ${:.4}",
        total_tokens, total_cost
    ))
    .to_json()
}

/// MCP arguments: cancel helper used by some JSON-RPC shims (prefer [`crate::CancelTaskParams`]).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CancelTaskParams {
    /// Task id to cancel.
    pub task_id: u64,
}

/// Cancel a task by numeric id (wrapper around orchestrator APIs).
pub async fn cancel_task(state: &ServerState, params: crate::CancelTaskParams) -> String {
    let orch = &state.orchestrator;

    if let Err(e) = orch.cancel_task(TaskId(params.task_id)) {
        return ToolResult::<String>::err(format!("{}", e)).to_json();
    }
    ToolResult::ok(format!("Task {} cancelled", params.task_id)).to_json()
}

/// Change the priority of a queued task.
pub async fn reorder_task(state: &ServerState, params: crate::ReorderTaskParams) -> String {
    let orch = &state.orchestrator;

    let priority = match params.priority.as_str() {
        "urgent" => vox_orchestrator::TaskPriority::Urgent,
        "background" => vox_orchestrator::TaskPriority::Background,
        _ => vox_orchestrator::TaskPriority::Normal,
    };

    if let Err(e) = orch.reorder_task(TaskId(params.task_id), priority) {
        return ToolResult::<String>::err(format!("{}", e)).to_json();
    }
    ToolResult::ok(format!(
        "Task {} reordered to {:?}",
        params.task_id, priority
    ))
    .to_json()
}

/// Drop all queued (not running) tasks for an agent.
pub async fn drain_agent(state: &ServerState, params: crate::DrainAgentParams) -> String {
    let orch = &state.orchestrator;

    match orch.drain_agent(vox_orchestrator::AgentId(params.agent_id)) {
        Ok(tasks) => ToolResult::ok(format!(
            "Drained {} tasks from agent {}",
            tasks.len(),
            params.agent_id
        ))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("{}", e)).to_json(),
    }
}

/// Re-run the global task balancer and report how many tasks moved.
pub async fn rebalance(state: &ServerState) -> String {
    let orch = &state.orchestrator;
    let moved = orch.rebalance();
    ToolResult::ok(format!("Rebalanced {} tasks", moved)).to_json()
}

/// MCP arguments: fetch Gamify event rows for one agent id string.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentEventsParams {
    /// Agent id as u64 (stringified for DB lookup).
    pub agent_id: u64,
}

/// Return historical [`vox_ludus::db::AgentEventRecord`] rows when Codex is configured.
pub async fn agent_events(state: &ServerState, params: AgentEventsParams) -> String {
    if let Some(db) = &state.db {
        match vox_ludus::db::get_events(db, &params.agent_id.to_string(), None).await {
            Ok(events) => ToolResult::ok(events).to_json(),
            Err(e) => ToolResult::<String>::err(format!("DB error: {}", e)).to_json(),
        }
    } else {
        ToolResult::<String>::err("Database not configured, cannot fetch past events.").to_json()
    }
}

/// Bind `params.session_id` to `params.agent_id` inside the orchestrator.
pub async fn map_agent_session(
    state: &ServerState,
    params: crate::MapAgentSessionParams,
) -> String {
    let orch = &state.orchestrator;

    match orch.map_agent_session(
        vox_orchestrator::AgentId(params.agent_id),
        params.session_id.clone(),
    ) {
        Ok(_) => ToolResult::ok(format!(
            "Mapped agent session {} to agent {}",
            params.session_id, params.agent_id
        ))
        .to_json(),
        Err(e) => ToolResult::<String>::err(format!("{}", e)).to_json(),
    }
}

/// MCP arguments: cap rows pulled per pseudo-agent when listing spend history.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CostHistoryParams {
    /// Per-agent SQL `LIMIT` before global merge.
    pub limit_per_agent: Option<i64>,
}

/// Merge recent cost rows across orchestrator + agents (requires Codex).
pub async fn cost_history(state: &ServerState, params: CostHistoryParams) -> String {
    if let Some(db) = &state.db {
        let limit = params.limit_per_agent.unwrap_or(100);
        let mut all_records = Vec::new();

        let agent_ids = {
            let orch = &state.orchestrator;
            orch.agent_ids()
        };

        // Also add orchestrator
        let mut ids = vec![
            "vox-orchestrator".to_string(),
            "0".to_string(),
            "master".to_string(),
        ];
        for a in agent_ids {
            ids.push(a.0.to_string());
        }

        for id in ids {
            if let Ok(records) = vox_ludus::db::list_cost_records(db, &id, limit).await {
                all_records.extend(records);
            }
        }

        // Sort globally by timestamp descending
        all_records.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        // Take total limit
        all_records.truncate(limit as usize * 2); // just some bounding

        ToolResult::ok(all_records).to_json()
    } else {
        ToolResult::<String>::err("Database not configured, cannot fetch cost history.").to_json()
    }
}

/// Dump the affinity map as JSON (path → owner relationships).
/// Serialize the orchestrator affinity map to JSON (path keys → owning agent ids).
pub async fn file_graph(state: &ServerState) -> String {
    let orch = &state.orchestrator;

    let map = orch.affinity_map().as_json();
    ToolResult::ok(map).to_json()
}

/// Merge orchestrator config with on-disk `VoxConfig` toolchain map.
/// Return merged JSON: live `OrchestratorConfig` plus `VoxConfig` toolchain map from disk.
pub async fn config_get(state: &ServerState) -> String {
    let orch = &state.orchestrator;
    let orch_cfg = {
        let handle = orch.config_handle();
        let cfg = handle.read().unwrap();
        serde_json::to_value(&*cfg).unwrap_or_default()
    };

    // Load VoxConfig SSOT (toolchain settings) and merge on top
    let vox_cfg = vox_config::VoxConfig::load();
    let toolchain = vox_cfg.to_map();

    let mut merged = serde_json::Map::new();
    merged.insert("orchestrator".to_string(), orch_cfg);
    merged.insert(
        "toolchain".to_string(),
        serde_json::to_value(toolchain).unwrap_or_default(),
    );

    ToolResult::ok(serde_json::Value::Object(merged)).to_json()
}

/// Patch [`OrchestratorConfig`] by shallow-merging JSON keys into the live instance.
/// Deep-merge `params` into the current orchestrator JSON config (mutates in-memory orchestrator).
pub async fn config_set(state: &ServerState, params: serde_json::Value) -> String {
    let orch = &state.orchestrator;

    let mut current_json = {
        let handle = orch.config_handle();
        let cfg = handle.read().unwrap();
        serde_json::to_value(&*cfg).unwrap_or_default()
    };

    if let (serde_json::Value::Object(current), serde_json::Value::Object(patch)) =
        (&mut current_json, params)
    {
        for (k, v) in patch {
            current.insert(k, v);
        }
    }

    match serde_json::from_value::<vox_orchestrator::config::OrchestratorConfig>(current_json) {
        Ok(new_config) => {
            *orch.config_handle().write().unwrap() = new_config.clone();
            ToolResult::ok(new_config).to_json()
        }
        Err(e) => ToolResult::<String>::err(format!("invalid config fields: {e}")).to_json(),
    }
}

/// Idempotent "fleet running" probe returning the current agent count.
/// Report agent count for the embedded orchestrator (no separate process spawn in this crate).
pub async fn orchestrator_start(state: &ServerState) -> String {
    let orch = &state.orchestrator;

    // In a real execution, we would start AgentFleet by moving Orchestrator into it.
    // However, since we hold it in ServerState inside a Mutex, we simply return
    // that the orchestrator is running and report the number of agents.
    let count = orch.agent_ids().len();
    ToolResult::ok(format!(
        "AgentFleet is running with {} active agents.",
        count
    ))
    .to_json()
}

/// Get a full snapshot of the orchestrator's state.
pub async fn orchestrator_status(state: &ServerState) -> String {
    let (
        status,
        scaling_profile,
        effective_scale_up_threshold,
        vcs_snapshot_count,
        vcs_oplog_count,
        vcs_active_conflicts,
        vcs_active_workspaces,
        vcs_active_changes,
        populi_control_url,
        mesh_http_timeout_ms,
    ) = {
        let orch = &state.orchestrator;
        let handle = orch.config_handle();
        let cfg = handle.read().unwrap();
        let effective = cfg.scaling_threshold as f64 * cfg.scaling_profile.threshold_multiplier();
        (
            orch.status(),
            Some(format!("{:?}", cfg.scaling_profile).to_lowercase()),
            Some(effective),
            orch.snapshot_store_handle().read().unwrap().count(),
            orch.oplog_handle().read().unwrap().count(),
            orch.conflict_manager_handle()
                .read()
                .unwrap()
                .active_count(),
            orch.workspace_manager_handle()
                .read()
                .unwrap()
                .list_workspaces()
                .len(),
            orch.workspace_manager_handle()
                .read()
                .unwrap()
                .list_changes(None, usize::MAX)
                .len(),
            cfg.populi_control_url.clone(),
            cfg.mesh_http_timeout_ms,
        )
    };

    let populi_federation_cache =
        serde_json::to_value(state.mesh_remote_snapshot.read().unwrap().clone()).ok();

    let max_stale_ms: Option<u64> = std::env::var("VOX_MESH_MAX_STALE_MS")
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .filter(|n| *n > 0);

    let mesh_snapshot = if let Some(url) = populi_control_url
        .as_ref()
        .filter(|s: &&String| !s.trim().is_empty())
    {
        let timeout = std::time::Duration::from_millis(mesh_http_timeout_ms.max(500_u64));
        let client = vox_populi::http_client::MeshHttpClient::new_with_timeout(url, timeout)
            .with_env_token();
        match client.list_nodes().await {
            Ok(f) => {
                let f = vox_populi::filter_registry_by_max_stale_ms(f, max_stale_ms);
                Some(serde_json::json!({
                    "ok": true,
                    "schema_version": f.schema_version,
                    "node_count": f.nodes.len(),
                    "nodes": f.nodes,
                }))
            }
            Err(e) => Some(serde_json::json!({
                "ok": false,
                "error": e.to_string(),
            })),
        }
    } else {
        None
    };

    if let Some(ref snap) = mesh_snapshot {
        persist_mesh_snapshot_codex_opt(state, snap).await;
    }

    let agents: Vec<AgentInfo> = status
        .agents
        .iter()
        .map(|a| AgentInfo {
            id: a.id.0,
            name: a.name.clone(),
            queued: a.queued,
            completed: a.completed,
            paused: a.paused,
        })
        .collect();

    let companion = {
        // Try to load from DB for persistence
        let id = "vox-dei";
        let mut comp = if let Some(db) = &state.db {
            match list_companions(db, "user").await {
                Ok(comps) => comps.into_iter().find(|c: &Companion| c.id == id),
                Err(_) => None,
            }
        } else {
            None
        }
        .unwrap_or_else(|| vox_ludus::companion::Companion::new(id, "user", "Vox DEI", "vox"));

        comp.ascii_sprite = Some("🧑‍💻".to_string());
        Some(comp)
    };

    let scaling_line = match (
        scaling_profile.as_ref().map(|s: &String| s.as_str()),
        effective_scale_up_threshold,
    ) {
        (Some(prof), Some(eff)) => format!(
            "**Scaling:** profile={}, effective scale-up threshold={:.1}\n\n",
            prof, eff
        ),
        _ => String::new(),
    };
    let mut markdown = format!(
        "### 🤖 Vox DEI Status\n\n**Agents Active:** {}\n**Tasks In Progress:** {}\n**Tasks Completed:** {}\n\n{}",
        status.agents.len(),
        status.agents.iter().map(|a| a.queued).sum::<usize>(),
        status.total_completed,
        scaling_line
    );

    if let Some(ref c) = companion {
        markdown.push_str("#### 🧬 Code Companion\n\n");
        markdown.push_str(&format!(
            "```\n{}\n```\n",
            c.ascii_sprite.as_deref().unwrap_or("")
        ));
        markdown.push_str(&format!("**{}** {}\n", c.name, c.mood.emoji()));
        markdown.push_str(&format!(
            "HP: `{}`\n",
            c.render_status_bar(15).split("HP: ").last().unwrap_or("")
        ));
    }
    let planning = if let Some(db) = &state.db {
        let mut active = 0_i64;
        let mut total = 0_i64;
        if let Ok(rows) = db
            .query_all(
                "SELECT
                    SUM(CASE WHEN status IN ('pending','queued','in_progress') THEN 1 ELSE 0 END) AS active,
                    COUNT(*) AS total
                 FROM plan_sessions",
                (),
            )
            .await
            && let Some(row) = rows.first()
        {
            active = row.get(0).unwrap_or(0);
            total = row.get(1).unwrap_or(0);
        }
        Some(serde_json::json!({
            "active_sessions": active,
            "total_sessions": total,
        }))
    } else {
        None
    };

    markdown.push_str("\n#### 📋 Agent Queue\n\n");
    for a in &agents {
        let status_icon = if a.paused {
            "⏸️"
        } else if a.queued > 0 {
            "⚙️"
        } else {
            "💤"
        };
        markdown.push_str(&format!(
            "- {} **{}** (Queued: {}, Done: {})\n",
            status_icon, a.name, a.queued, a.completed
        ));
    }

    let response = StatusResponse {
        agent_count: status.agents.len(),
        in_progress: status.agents.iter().map(|a| a.queued).sum(),
        completed: status.total_completed,
        agents,
        scaling_profile,
        effective_scale_up_threshold,
        companion,
        markdown_summary: Some(markdown),
        snapshot_count: vcs_snapshot_count,
        oplog_count: vcs_oplog_count,
        active_conflicts: vcs_active_conflicts,
        active_workspaces: vcs_active_workspaces,
        active_changes: vcs_active_changes,
        mesh_snapshot,
        populi_federation_cache,
        planning,
    };

    ToolResult::ok(response).to_json()
}

fn mesh_codex_telemetry_enabled() -> bool {
    std::env::var("VOX_MESH_CODEX_TELEMETRY")
        .map(|v| {
            let v = v.trim();
            v == "1" || v.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

async fn persist_mesh_snapshot_codex_opt(state: &ServerState, snap: &serde_json::Value) {
    if !mesh_codex_telemetry_enabled() {
        return;
    }
    let Some(db) = state.db.as_ref() else {
        return;
    };
    let rid = state.repository.repository_id.clone();
    let details = serde_json::json!({
        "event": "orchestrator_status_mesh_snapshot",
        "ok": snap.get("ok"),
        "node_count": snap.get("node_count"),
        "schema_version": snap.get("schema_version"),
        "error": snap.get("error"),
    });
    if let Err(e) = db
        .record_populi_control_event(&rid, "orchestrator_status_mesh_snapshot", Some(details))
        .await
    {
        tracing::debug!(
            target: "vox.mesh_codex",
            error = %e,
            "record_populi_control_event failed (best-effort)"
        );
    }
}

/// Check which agent owns a given file path (async).
pub async fn check_file_owner(state: &ServerState, path: &str) -> String {
    let orch = &state.orchestrator;

    let affinity_map = orch.affinity_map();
    match affinity_map.lookup(&PathBuf::from(path)) {
        Some(agent_id) => ToolResult::ok(format!("owned by agent {agent_id}")).to_json(),
        None => ToolResult::ok("no owner assigned".to_string()).to_json(),
    }
}

/// Unified VCS status: snapshots, oplog, conflicts, workspaces, and changes.
pub async fn vcs_status(state: &ServerState) -> String {
    let orch = &state.orchestrator;

    let snapshot_count = crate::sync_lock::rw_read(&*orch.snapshot_store_handle()).count();
    let oplog_count = crate::sync_lock::rw_read(&*orch.oplog_handle()).count();
    let active_conflicts =
        crate::sync_lock::rw_read(&*orch.conflict_manager_handle()).active_count();
    let total_conflicts = crate::sync_lock::rw_read(&*orch.conflict_manager_handle()).total_count();
    let active_workspaces = crate::sync_lock::rw_read(&*orch.workspace_manager_handle())
        .list_workspaces()
        .len();
    let active_changes = crate::sync_lock::rw_read(&*orch.workspace_manager_handle())
        .list_changes(None, usize::MAX)
        .len();

    // Build workspace details
    let workspace_details: Vec<serde_json::Value> =
        crate::sync_lock::rw_read(&*orch.workspace_manager_handle())
            .list_workspaces()
            .iter()
            .map(|ws| {
                serde_json::json!({
                    "agent_id": ws.agent_id.0,
                    "base_snapshot": ws.base_snapshot.0,
                    "modified_files": ws.modified_count(),
                    "active_change": ws.active_change.map(|c| c.0),
                })
            })
            .collect();

    // Build recent oplog entries (last 10)
    let recent_ops: Vec<serde_json::Value> = crate::sync_lock::rw_read(&*orch.oplog_handle())
        .list(None, 10)
        .iter()
        .map(|op| {
            serde_json::json!({
                "id": op.id.to_string(),
                "agent_id": op.agent_id.0,
                "kind": format!("{:?}", op.kind),
                "description": op.description,
                "undone": op.undone,
            })
        })
        .collect();

    // Build active conflict details
    let conflict_details: Vec<serde_json::Value> =
        crate::sync_lock::rw_read(&*orch.conflict_manager_handle())
            .active_conflicts()
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id.to_string(),
                    "path": c.path.display().to_string(),
                    "sides": c.sides.len(),
                    "created_ms": c.created_ms,
                })
            })
            .collect();

    let result = serde_json::json!({
        "snapshots": snapshot_count,
        "oplog_entries": oplog_count,
        "recent_operations": recent_ops,
        "conflicts": {
            "active": active_conflicts,
            "total": total_conflicts,
            "details": conflict_details,
        },
        "workspaces": {
            "active": active_workspaces,
            "details": workspace_details,
        },
        "changes": active_changes,
    });

    ToolResult::ok(result).to_json()
}

/// MCP arguments: cap merged event rows from Codex + transient buffer.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PollEventsParams {
    /// Maximum rows after sorting newest-first.
    pub limit: Option<i64>,
}

/// Pull recent Gamify rows for every live agent plus in-memory `transient_events`.
pub async fn poll_events(state: &ServerState, params: PollEventsParams) -> String {
    if let Some(db) = &state.db {
        let limit = params.limit.unwrap_or(50);
        let mut all_events = Vec::new();
        let agent_ids = {
            let orch = &state.orchestrator;
            orch.agent_ids()
        };

        for id in agent_ids {
            if let Ok(records) = vox_ludus::db::get_events(db, &id.0.to_string(), Some(limit)).await
            {
                all_events.extend(records);
            }
        }

        let mut transient = Vec::new();
        {
            let mut q = state.transient_events.lock().await;
            transient = std::mem::take(&mut *q);
        }

        let repo_id = state.repository.repository_id.clone();
        for ev in transient {
            let (agent_id, event_type) = match &ev.kind {
                vox_orchestrator::AgentEventKind::TokenStreamed { agent_id, .. } => {
                    (agent_id.0, "TokenStreamed")
                }
                _ => (0, "Unknown"),
            };
            let mut kind_json = serde_json::to_value(&ev.kind).unwrap_or_default();
            if let Some(obj) = kind_json.as_object_mut() {
                obj.insert(
                    "repository_id".to_string(),
                    serde_json::Value::String(repo_id.clone()),
                );
            }
            let payload = serde_json::to_string(&kind_json).unwrap_or_default();
            all_events.push(vox_ludus::db::AgentEventRecord {
                id: ev.id.0 as i64,
                agent_id: agent_id.to_string(),
                event_type: event_type.to_string(),
                payload: Some(payload),
                timestamp: ev.timestamp_ms.to_string(),
            });
        }

        all_events.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));
        all_events.truncate(limit as usize);
        ToolResult::ok(all_events).to_json()
    } else {
        ToolResult::<String>::err("DB not configured").to_json()
    }
}

/// MCP arguments: lightweight task submit (string description + optional affinities).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct SubmitTaskParams {
    /// Raw task description (not canonicalized here).
    pub description: String,
    /// Optional file path hints (`write(...)` affinity strings).
    pub affinites: Option<Vec<String>>,
    /// Optional forced routing target.
    pub agent_id: Option<u64>,
    /// Optional session link (for chat/workflow grouping in Mens).
    pub session_id: Option<String>,
}

/// Submit a task through the orchestrator (simpler shape than [`crate::params::SubmitTaskParams`]).
pub async fn submit_task(state: &ServerState, params: SubmitTaskParams) -> String {
    let orch = &state.orchestrator;

    let affinities = params
        .affinites
        .unwrap_or_default()
        .into_iter()
        .map(vox_orchestrator::FileAffinity::write)
        .collect();
    let agent_id = params.agent_id.map(vox_orchestrator::AgentId);

    match orch
        .submit_task_with_agent(
            &params.description,
            affinities,
            None,
            None,
            None,
            params.session_id,
        )
        .await
    {
        Ok(task_id) => ToolResult::ok(format!("Submitted task {}", task_id.0)).to_json(),
        Err(e) => ToolResult::<String>::err(format!("Submit failed: {}", e)).to_json(),
    }
}

/// MCP arguments: correlate IDE session ids with orchestrator agents.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct HeartbeatParams {
    /// Client session string previously mapped via [`map_agent_session`].
    pub session_id: String,
}

/// Emit a synthetic busy event for the agent mapped to `session_id`.
pub async fn heartbeat(state: &ServerState, params: HeartbeatParams) -> String {
    let orch = &state.orchestrator;

    // Try finding the agent mapped to this session
    let mut target = None;
    for agent in orch.status().agents {
        if agent.agent_session_id.as_deref() == Some(params.session_id.as_str()) {
            target = Some(vox_orchestrator::AgentId(agent.id.0));
            break;
        }
    }

    if let Some(id) = target {
        // We simulate a heartbeat by emitting a basic event or just resetting their heartbeat timer
        orch.event_bus()
            .emit(vox_orchestrator::AgentEventKind::AgentBusy { agent_id: id });
        ToolResult::ok(format!("Heartbeat received for agent {}", id.0)).to_json()
    } else {
        ToolResult::<String>::err("No agent mapped to this session").to_json()
    }
}

/// MCP arguments: attribute spend to the agent tied to a session id.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct RecordCostParams {
    /// Session key used to resolve the target agent.
    pub session_id: String,
    /// LLM provider slug (`openrouter`, ...).
    pub provider: String,
    /// Concrete model name/id.
    pub model: String,
    /// Total USD charged for the call.
    pub cost_usd: f64,
    /// Prompt tokens billed.
    pub input_tokens: u32,
    /// Completion tokens billed.
    pub output_tokens: u32,
}

/// Persist a cost row (when DB present) and emit `CostIncurred` on the orchestrator bus.
pub async fn record_cost(state: &ServerState, params: RecordCostParams) -> String {
    let (target_id, event_bus) = {
        let orch = &state.orchestrator;

        let mut target = None;
        for agent in orch.status().agents {
            if agent.agent_session_id.as_deref() == Some(params.session_id.as_str()) {
                target = Some(vox_orchestrator::AgentId(agent.id.0));
                break;
            }
        }
        (target, orch.event_bus().clone())
    };

    if let Some(id) = target_id {
        if let Some(db) = &state.db {
            let _ = vox_ludus::db::insert_cost_record(
                db,
                &id.0.to_string(),
                Some(&params.session_id),
                &params.provider,
                Some(&params.model),
                params.input_tokens as i64,
                params.output_tokens as i64,
                params.cost_usd,
            )
            .await;
        }

        event_bus.emit(vox_orchestrator::AgentEventKind::CostIncurred {
            agent_id: id,
            provider: params.provider,
            model: params.model,
            input_tokens: params.input_tokens,
            output_tokens: params.output_tokens,
            cost_usd: params.cost_usd,
            temporal_context: None,
        });
        ToolResult::ok(format!(
            "Cost {:.4} recorded for agent {}",
            params.cost_usd, id.0
        ))
        .to_json()
    } else {
        ToolResult::<String>::err("No agent mapped to this session").to_json()
    }
}
