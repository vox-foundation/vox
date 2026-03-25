//! Gamify companion MCP tools: mood, status markdown, continuation tick, assessment, handoff payload.
//!
//! When [`ServerState::db`] is present, companion rows are read/written via Codex; otherwise
//! in-memory companions are synthesized per agent id.

use crate::{ServerState, ToolResult};
use schemars::JsonSchema;
use serde::Deserialize;
use vox_ludus::companion::{Companion, Interaction};
use vox_ludus::db::{list_companions, upsert_companion};

/// MCP arguments: load or bootstrap the gamify companion row for one orchestrator agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct CheckMoodParams {
    /// Orchestrator agent id backing the companion row.
    pub agent_id: u64,
}

/// Return JSON companion record (persisted when DB is wired).
pub async fn check_mood(state: &ServerState, params: CheckMoodParams) -> String {
    let id = format!("agent-{}", params.agent_id);
    let user_id = vox_db::paths::local_user_id();

    if let Some(db) = &state.db {
        match list_companions(db, &user_id).await {
            Ok(comps) => {
                if let Some(c) = comps.into_iter().find(|c: &Companion| c.id == id) {
                    return ToolResult::ok(c).to_json();
                }
            }
            Err(e) => tracing::warn!("failed to list companions from DB: {}", e),
        }
    }

    // Fallback/Initial create
    let companion = Companion::new(&id, &user_id, format!("Agent {}", params.agent_id), "vox");

    // Auto-save if DB exists
    if let Some(db) = &state.db {
        let _ = upsert_companion(db, &companion).await;
    }

    ToolResult::ok(companion).to_json()
}

/// MCP arguments: render queue-aware status markdown for one agent.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentStatusParams {
    /// Agent to describe in markdown.
    pub agent_id: u64,
}

/// Return markdown summarizing queue depth, completed tasks, and companion HP bar.
pub async fn agent_status(state: &ServerState, params: AgentStatusParams) -> String {
    let id = format!("agent-{}", params.agent_id);
    let user_id = vox_db::paths::local_user_id();
    let mut companion = if let Some(db) = &state.db {
        match list_companions(db, &user_id).await {
            Ok(comps) => comps.into_iter().find(|c: &Companion| c.id == id),
            Err(_) => None,
        }
    } else {
        None
    }
    .unwrap_or_else(|| Companion::new(&id, &user_id, format!("Agent {}", params.agent_id), "vox"));

    let orch = &state.orchestrator;

    if let Some(queue_arc) = orch.agent_queue(vox_orchestrator::AgentId(params.agent_id)) {
        let hp_bar = companion.render_status_bar(10);
        let (q_len, q_done, q_empty) = {
            let q = queue_arc.read().unwrap();
            (q.len(), q.completed_count(), q.is_empty())
        };
        let markdown = format!(
            "### 🤖 Agent {} Status\n\n**{}**\n\n**Stats:**\n- Queue Depth: `{}`\n- Tasks Done: `{}`\n\n**Activity:** {}",
            params.agent_id,
            hp_bar,
            q_len,
            q_done,
            if !q_empty {
                "Processing tasks... ⚙️"
            } else {
                "Idle 💤"
            }
        );
        ToolResult::ok(markdown).to_json()
    } else {
        ToolResult::<String>::err("Agent not found").to_json()
    }
}

/// MCP arguments: nudge orchestrator auto-continuations (idle agent wake-up path).
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentContinueParams {
    /// Agent mentioned in the confirmation string (tick is global).
    pub agent_id: u64,
}

/// Run one orchestrator tick then return a short confirmation string (JSON `ToolResult`).
pub async fn agent_continue(state: &ServerState, params: AgentContinueParams) -> String {
    let orch = &state.orchestrator;
    orch.tick().await; // Triggers auto-continuations for idle agents
    ToolResult::ok(format!(
        "Agent {} triggered for continuation",
        params.agent_id
    ))
    .to_json()
}

/// MCP arguments: estimate remaining wall time from queue depth and user preference `task.estimate_ms`.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentAssessParams {
    /// Agent whose queue depth is estimated.
    pub agent_id: u64,
}

/// Return human-readable pending/completed counts and rough ETA string.
pub async fn agent_assess(state: &ServerState, params: AgentAssessParams) -> String {
    let mut ms_per_task: usize = 45_000;
    let user_id = vox_db::paths::local_user_id();

    if let Some(db) = &state.db {
        if let Ok(Some(pref)) = db.get_user_preference(&user_id, "task.estimate_ms").await {
            if let Ok(val) = pref.parse::<usize>() {
                ms_per_task = val;
            }
        }
    }

    let orch = &state.orchestrator;

    if let Some(queue_arc) = orch.agent_queue(vox_orchestrator::AgentId(params.agent_id)) {
        let (active, completed) = {
            let q = queue_arc.read().unwrap();
            (q.len(), q.completed_count())
        };

        let estimate_s = (active * ms_per_task) / 1000;
        ToolResult::ok(format!(
            "Agent {} has {} pending and {} completed tasks. Est remaining time: {}s",
            params.agent_id, active, completed, estimate_s
        ))
        .to_json()
    } else {
        ToolResult::<String>::err("Agent not found").to_json()
    }
}

/// MCP arguments: structured plan handoff published on the orchestrator event bus.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct AgentHandoffParams {
    /// Handoff source agent.
    pub from_agent_id: u64,
    /// Handoff destination agent.
    pub to_agent_id: u64,
    /// High-level narrative inserted into [`vox_orchestrator::handoff::HandoffPayload`].
    pub plan_summary: String,
    #[serde(default)]
    /// Open work items the receiver should address.
    pub unresolved_objectives: Vec<String>,
    #[serde(default)]
    /// Checklist the receiver can use to validate completion.
    pub verification_criteria: Vec<String>,
}

/// Emit a [`vox_orchestrator::handoff::HandoffPayload`] (side effect: event bus + downstream listeners).
pub async fn agent_handoff(state: &ServerState, params: AgentHandoffParams) -> String {
    let orch = &state.orchestrator;
    let mut payload = vox_orchestrator::handoff::HandoffPayload::new(
        vox_orchestrator::AgentId(params.from_agent_id),
        Some(vox_orchestrator::AgentId(params.to_agent_id)),
        &params.plan_summary,
    );
    payload.unresolved_objectives = params.unresolved_objectives;
    payload.verification_criteria = params.verification_criteria;
    if let Err(e) = vox_orchestrator::handoff::execute_handoff(&payload, orch.event_bus()) {
        return ToolResult::<String>::err(e.to_string()).to_json();
    }

    ToolResult::ok(format!(
        "Handoff initiated from agent {} to agent {}: {}",
        params.from_agent_id, params.to_agent_id, params.plan_summary
    ))
    .to_json()
}
