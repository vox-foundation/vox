//! Wire-level MCP tool name aliases (same JSON args as canonical tools).

/// `(alias, canonical)` pairs accepted by [`super::handle_tool_call`] and [`super::input_schemas::tool_input_schema`].
pub const TOOL_WIRE_ALIASES: &[(&str, &str)] = &[
    ("vox_get_config", "vox_config_get"),
    ("vox_set_config", "vox_config_set"),
    ("vox_map_opencode_session", "vox_map_agent_session"),
    ("vox_map_vscode_session", "vox_map_agent_session"),
    ("vox_budget_history", "vox_cost_history"),
    ("vox_model_list", "vox_list_models"),
];

/// Resolve an incoming tool name to the canonical handler name.
#[must_use]
pub fn canonical_tool_name(name: &str) -> &str {
    for (alias, canonical) in TOOL_WIRE_ALIASES {
        if *alias == name {
            return canonical;
        }
    }
    name
}
