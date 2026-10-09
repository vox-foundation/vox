//! MCP tool registry built from `contracts/mcp/tool-registry.canonical.yaml`.
//!
//! Consume [`TOOL_REGISTRY`] instead of duplicating name/description tables.

mod boot;
mod classification;
mod exposure;
mod profile;
mod subsystem;

pub use boot::{BootReport, ProfileDenial};
pub use classification::{ToolClassification, classify_tool_row};
pub use exposure::{Exposure, SideEffectClass};
pub use profile::{
    ActiveProfile, CallContext, CallerRole, DEV_PROFILE_ID, PROFILES, ProfileError,
    ProfileManifest, authorize_tool,
};
pub use subsystem::{Subsystem, subsystem_closure};

/// One MCP tool row: identity, bell-curve [`product_lane`](McpToolRegistryEntry::product_lane),
/// and capability-profile exposure metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McpToolRegistryEntry {
    pub name: &'static str,
    pub description: &'static str,
    pub product_lane: &'static str,
    pub http_read_role_eligible: bool,
    pub exposure: Exposure,
    pub side_effect_class: SideEffectClass,
    pub requires: &'static [Subsystem],
}

include!(concat!(env!("OUT_DIR"), "/tool_registry.rs"));

/// Row for an exact canonical tool name; wire aliases are not resolved here.
pub fn registry_entry(name: &str) -> Option<&'static McpToolRegistryEntry> {
    TOOL_REGISTRY.iter().find(|e| e.name == name)
}

mod semcov_wave46_tests;

/// All A2A message type strings.
pub const A2A_MESSAGE_TYPES: &[&str] = &[
    "plan_handoff",
    "scope_request",
    "scope_grant",
    "progress_update",
    "help_request",
    "completion_notice",
    "error_report",
    "conflict_detected",
    "conflict_resolved",
    "vcs_event",
    "cancel_request",
    "snapshot_share",
    "free_form",
    "file_lock_request",
    "file_lock_release",
    "multi_file_synthesis",
    "time_threshold_breach",
    "model_regression_hint",
    "socrates_gate_bypass",
];

/// Skill-oriented MCP tools.
pub const SKILL_TOOLS: &[&str] = &[
    "vox_skill_install",
    "vox_skill_uninstall",
    "vox_skill_add",
    "vox_skill_remove",
    "vox_skill_list",
    "vox_skill_search",
    "vox_skill_info",
    "vox_skill_parse",
    "vox_skill_use",
    "vox_skill_discover",
];

/// Orchestrator lifecycle tools.
pub const ORCHESTRATOR_TOOLS: &[&str] = &[
    "vox_submit_task",
    "vox_task_status",
    "vox_orchestrator_status",
    "vox_orchestrator_start",
    "vox_spawn_agent",
    "vox_retire_agent",
    "vox_pause_agent",
    "vox_resume_agent",
    "vox_complete_task",
    "vox_fail_task",
    "vox_cancel_task",
    "vox_rebalance",
    "vox_reorder_task",
    "vox_drain_agent",
    "vox_queue_status",
    "vox_lock_status",
    "vox_budget_status",
];

#[cfg(test)]
mod registry_entry_tests {
    use super::*;

    #[test]
    fn registry_entry_finds_every_row_by_name() {
        for e in TOOL_REGISTRY {
            assert_eq!(registry_entry(e.name), Some(e));
        }
    }

    #[test]
    fn registry_entry_rejects_unknown_and_alias_names() {
        assert_eq!(registry_entry("vox_no_such_tool"), None);
        assert_eq!(
            registry_entry("vox_model_list"),
            None,
            "wire aliases resolve in vox-orchestrator-mcp, not here"
        );
    }
}
