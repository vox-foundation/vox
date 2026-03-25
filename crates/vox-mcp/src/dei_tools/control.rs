use super::params::{AgentEventsParams, CostHistoryParams, QueueStatusParams};
use crate::{ServerState, ToolResult};
use vox_orchestrator::{AgentId, TaskId};

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
