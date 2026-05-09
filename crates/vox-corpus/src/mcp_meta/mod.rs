//! MCP project metadata: resource and tool descriptions.
//!
//! Local MCP metadata types for `vox-corpus` corpus generation.

use serde::{Deserialize, Serialize};

/// Metadata for a single MCP resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResource {
    pub name: String,
    pub description: String,
    pub uri_template: String,
}

/// Metadata for a single MCP tool.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDesc {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Aggregated MCP metadata for a project.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct McpMeta {
    pub project_name: String,
    pub version: String,
    pub resources: Vec<McpResource>,
    pub tools: Vec<McpToolDesc>,
}

/// List of core skill tool names for synthetic generation.
pub const SKILL_TOOLS: &[&str] = &[
    "vox_skill_install",
    "vox_skill_uninstall",
    "vox_skill_search",
    "vox_skill_info",
    "vox_skill_parse",
];

/// List of core orchestrator tool names for synthetic generation.
pub const ORCHESTRATOR_TOOLS: &[&str] = &[
    "vox_submit_task",
    "vox_task_status",
    "vox_complete_task",
    "vox_fail_task",
    "vox_cancel_task",
    "vox_orchestrator_start",
    "vox_orchestrator_status",
    "vox_spawn_agent",
    "vox_retire_agent",
    "vox_pause_agent",
    "vox_resume_agent",
    "vox_rebalance",
    "vox_lock_status",
    "vox_budget_status",
];
