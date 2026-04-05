//! Agent topology and delegation metadata.
//!
//! This module provides a canonical snapshot shape for orchestrator agent topology.
//! It is intentionally lightweight: current runtime behavior is queue-centric, but
//! these structs let us persist and expose parent/child delegation relationships.

use serde::{Deserialize, Serialize};

use crate::types::{AgentId, TaskId, now_unix_ms};

/// High-level role attached to an agent node for delegation-aware orchestration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentRole {
    #[default]
    Generalist,
    Planner,
    Executor,
    Verifier,
    Synthesizer,
    Researcher,
    Observer,
}

/// Parent binding for a child agent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentDelegationBinding {
    pub parent_agent_id: AgentId,
    pub source_task_id: Option<TaskId>,
    pub reason: String,
}

/// Spawn provenance attached to dynamic agents even when no parent edge exists.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DynamicSpawnContext {
    pub source_task_id: Option<TaskId>,
    pub reason: String,
}

/// One directed edge in the agent-delegation graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DelegationEdge {
    pub parent_agent_id: AgentId,
    pub child_agent_id: AgentId,
    pub source_task_id: Option<TaskId>,
    pub reason: String,
}

/// One node in the orchestrator topology graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTopologyNode {
    pub agent_id: AgentId,
    pub name: String,
    pub role: AgentRole,
    pub dynamic: bool,
    pub parent_agent_id: Option<AgentId>,
    pub source_task_id: Option<TaskId>,
    pub spawn_reason: Option<String>,
    pub child_count: usize,
    pub queued: usize,
    pub in_progress: bool,
    pub paused: bool,
    pub agent_session_id: Option<String>,
}

/// Explicit limitations surfaced with a topology snapshot so operators know what is
/// currently modeled vs what still needs implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TopologyGap {
    pub code: String,
    pub description: String,
    pub suggested_state: String,
}

/// Current orchestrator topology with delegation edges and known modeling gaps.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentTopologySnapshot {
    pub generated_at_ms: u64,
    pub nodes: Vec<AgentTopologyNode>,
    pub delegation_edges: Vec<DelegationEdge>,
    pub known_gaps: Vec<TopologyGap>,
}

impl AgentTopologySnapshot {
    #[must_use]
    pub fn empty() -> Self {
        Self {
            generated_at_ms: now_unix_ms(),
            nodes: Vec::new(),
            delegation_edges: Vec::new(),
            known_gaps: default_topology_gaps(),
        }
    }

    /// Phase 2C: Generates a purely textual, screen-reader compliant tree-list
    /// representation of the node graph. This serves as the backing for the
    /// Visualizer ARIA Shadow DOM, ensuring node topology is accessible.
    #[must_use]
    pub fn aria_shadow_graph(&self) -> String {
        let mut out = String::from("Agent Topology Graph:\n");
        let mut visited = std::collections::HashSet::new();

        // Recursively build tree starting from roots
        for root in self.nodes.iter().filter(|n| n.parent_agent_id.is_none()) {
            Self::aria_append_node(&mut out, root, &self.nodes, 0, &mut visited);
        }

        // Append any orphaned/disconnected nodes flat at the bottom
        for node in &self.nodes {
            if !visited.contains(&node.agent_id) {
                out.push_str(&format!(
                    "- [Disconnected] Agent {} [Role: {:?}] - Status: {}\n",
                    node.name,
                    node.role,
                    if node.in_progress {
                        "working"
                    } else if node.paused {
                        "paused"
                    } else {
                        "idle"
                    }
                ));
            }
        }

        out
    }

    fn aria_append_node(
        out: &mut String,
        node: &AgentTopologyNode,
        all_nodes: &[AgentTopologyNode],
        depth: usize,
        visited: &mut std::collections::HashSet<AgentId>,
    ) {
        if visited.contains(&node.agent_id) {
            return;
        }
        visited.insert(node.agent_id);

        let indent = " ".repeat(depth * 2);
        let status = if node.in_progress {
            "working"
        } else if node.paused {
            "paused"
        } else {
            "idle"
        };
        let dynamic_str = if node.dynamic { " (Dynamic)" } else { "" };
        out.push_str(&format!(
            "{}- Agent {} [Role: {:?}]{} - Status: {}\n",
            indent, node.name, node.role, dynamic_str, status
        ));

        for child in all_nodes
            .iter()
            .filter(|n| n.parent_agent_id == Some(node.agent_id))
        {
            Self::aria_append_node(out, child, all_nodes, depth + 1, visited);
        }
    }
}

#[must_use]
pub fn default_topology_gaps() -> Vec<TopologyGap> {
    vec![
        TopologyGap {
            code: "topology.delegation_role_policy_missing".to_string(),
            description: "Delegation edges are tracked, but no policy engine currently assigns role-specific spawn templates.".to_string(),
            suggested_state:
                "Add role templates (planner/verifier/synthesizer) and spawn contracts by task class.".to_string(),
        },
        TopologyGap {
            code: "topology.consensus_cohort_missing".to_string(),
            description:
                "No first-class consensus cohort or cross-check quorum model is persisted.".to_string(),
            suggested_state:
                "Persist cohort membership and verifier vote outcomes per campaign/task.".to_string(),
        },
    ]
}
