//! Orchestrator agent adapter — event translation only.
//!
//! Translates `AgentEvent`s from `vox-orchestrator`'s `EventBus` into
//! `SessionEvent`s. Does NOT reimplement the agent loop.
//!
//! HITL note: `FeedbackStore::{open_needs_you, resolve}` will be wired here
//! once `claude/context-window-spine` merges to main (adds feedback module to
//! vox-orchestrator). See docs/superpowers/plans/…vox-terminal-warp-parity.md §HITL.

use vox_orchestrator::events::{AgentEvent, AgentEventKind};
use vox_orchestrator::models::{
    SelectionAxes, SelectionIntent, TaskCategory, infer_prompt_intents,
    select_with_default_registry,
};
use vox_orchestrator::types::AgentId;

use crate::session::SessionEvent;

/// Config for the adapter. `agent_id = Some(id)` filters events to one agent;
/// `None` passes through all agents (useful for single-agent sessions).
#[derive(Debug, Clone, Default)]
pub struct AgentAdapterConfig {
    pub agent_id: Option<AgentId>,
}

/// Structured model routing decision for a terminal agent turn (`?prompt` or `/ai prompt`).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct RoutedAgentTurn {
    pub prompt: String,
    pub task: String,
    pub complexity: u8,
    pub model_id: String,
    pub provider: String,
    pub tier: String,
    pub reason: String,
    pub efficiency_axis: u8,
    pub precision_axis: u8,
    pub latency_axis: u8,
}

impl RoutedAgentTurn {
    pub fn summary(&self) -> String {
        format!(
            "[routed model={} task={} tier={} axes=eff:{}/prec:{}/lat:{}] {}",
            self.model_id,
            self.task,
            self.tier,
            self.efficiency_axis,
            self.precision_axis,
            self.latency_axis,
            self.prompt
        )
    }
}

/// Intelligently classify a terminal prompt and select the optimal model via
/// `vox_orchestrator::models::select_with_default_registry`.
pub fn route_agent_prompt(prompt: &str) -> RoutedAgentTurn {
    let trimmed = prompt.trim();
    let lower = trimmed.to_ascii_lowercase();
    let inferred = infer_prompt_intents(trimmed);
    let words = trimmed.split_whitespace().count();

    let mut intent = if lower.contains("fix")
        || lower.contains("bug")
        || lower.contains("error")
        || lower.contains("fail")
        || lower.contains("panic")
        || lower.contains("crash")
        || lower.contains("debug")
        || lower.contains("exit code")
    {
        let mut i = SelectionIntent::for_task(TaskCategory::Debugging);
        i.axes = SelectionAxes::QUALITY_FIRST;
        i.complexity = 6;
        i
    } else if lower.contains("review")
        || lower.contains("audit")
        || lower.contains("security")
        || lower.contains("vulnerability")
    {
        SelectionIntent::review()
    } else if lower.contains("research")
        || lower.contains("compare")
        || lower.contains("investigate")
        || lower.contains("benchmark")
        || lower.starts_with("why ")
        || lower.starts_with("explain ")
    {
        SelectionIntent::research()
    } else if lower.contains("plan")
        || lower.contains("architect")
        || lower.contains("design")
        || lower.contains("roadmap")
        || lower.contains("milestone")
    {
        SelectionIntent::plan_mode()
    } else if lower.contains("parse")
        || lower.contains("extract")
        || lower.contains("classify")
        || lower.contains("triage")
        || lower.contains("summarize")
    {
        SelectionIntent::nli_classifier()
    } else if lower.contains("write")
        || lower.contains("implement")
        || lower.contains("refactor")
        || lower.contains("generate")
        || lower.contains("function")
        || lower.contains("test")
        || inferred
            .iter()
            .any(|p| format!("{p:?}").to_ascii_lowercase().contains("code"))
    {
        let mut i = SelectionIntent::for_task(TaskCategory::CodeGen);
        i.axes = SelectionAxes::BALANCED;
        i.complexity = 5;
        i
    } else {
        let mut i = SelectionIntent::for_task(TaskCategory::General);
        i.axes = SelectionAxes::FAST;
        i.complexity = 3;
        i
    };

    if words > 24 {
        intent.complexity = intent.complexity.saturating_add(2).clamp(1, 10);
    }

    if let Some(outcome) = select_with_default_registry(&intent) {
        RoutedAgentTurn {
            prompt: trimmed.to_string(),
            task: format!("{:?}", intent.task),
            complexity: intent.complexity,
            model_id: outcome.model_id,
            provider: outcome.model_spec.provider,
            tier: format!("{:?}", outcome.model_spec.capabilities.tier),
            reason: format!("{:?}", outcome.reason),
            efficiency_axis: outcome.effective_axes.efficiency,
            precision_axis: outcome.effective_axes.precision,
            latency_axis: outcome.effective_axes.latency,
        }
    } else {
        let prio = intent.axes.to_routing_priority(intent.prefer_local);
        RoutedAgentTurn {
            prompt: trimmed.to_string(),
            task: format!("{:?}", intent.task),
            complexity: intent.complexity,
            model_id: "mens-8b".to_string(),
            provider: "mens".to_string(),
            tier: "Local".to_string(),
            reason: "FallbackLocal".to_string(),
            efficiency_axis: prio.efficiency,
            precision_axis: prio.precision,
            latency_axis: prio.latency,
        }
    }
}

/// Pure translation function — maps one `AgentEvent` → `SessionEvent` or `None`.
///
/// Exposed as `pub` so tests can call it directly without a live orchestrator.
pub fn translate_event(ev: &AgentEvent, cfg: &AgentAdapterConfig) -> Option<SessionEvent> {
    // Filter by agent_id if configured.
    if let Some(filter_id) = cfg.agent_id {
        let ev_agent = agent_id_of(ev)?;
        if ev_agent != filter_id {
            return None;
        }
    }

    match &ev.kind {
        AgentEventKind::TokenStreamed { text, .. } => {
            Some(SessionEvent::AgentMessage { text: text.clone() })
        }
        // All other events are not surfaced to the terminal session.
        _ => None,
    }
}

fn agent_id_of(ev: &AgentEvent) -> Option<AgentId> {
    match &ev.kind {
        AgentEventKind::AgentSpawned { agent_id, .. }
        | AgentEventKind::AgentRetired { agent_id }
        | AgentEventKind::AgentHeartbeat { agent_id, .. }
        | AgentEventKind::ActivityChanged { agent_id, .. }
        | AgentEventKind::OperatingModeChanged { agent_id, .. }
        | AgentEventKind::TokenStreamed { agent_id, .. } => Some(*agent_id),
        _ => None,
    }
}
