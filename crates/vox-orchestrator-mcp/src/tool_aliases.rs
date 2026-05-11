//! Wire-level MCP tool name aliases (same JSON args as canonical tools).

/// `(alias, canonical)` pairs accepted by [`super::handle_tool_call`] and [`super::input_schemas::tool_input_schema`].
pub const TOOL_WIRE_ALIASES: &[(&str, &str)] = &[
    ("vox_get_config", "vox_config_get"),
    ("vox_set_config", "vox_config_set"),
    ("vox_map_opencode_session", "vox_map_agent_session"),
    ("vox_map_vscode_session", "vox_map_agent_session"),
    ("vox_budget_history", "vox_cost_history"),
    ("vox_model_list", "vox_list_models"),
    // Retired `vox-ludus` MCP prefix → canonical `vox_gamify_*` (see legacy remediation ledger).
    (
        "vox_ludus_notifications_list",
        "vox_gamify_notifications_list",
    ),
    (
        "vox_ludus_progress_snapshot",
        "vox_gamify_progress_snapshot",
    ),
    ("vox_ludus_notification_ack", "vox_gamify_notification_ack"),
    (
        "vox_ludus_notifications_ack_all",
        "vox_gamify_notifications_ack_all",
    ),
    ("vox_ludus_quest_list", "vox_gamify_quest_list"),
    ("vox_ludus_shop_catalog", "vox_gamify_shop_catalog"),
    ("vox_ludus_shop_buy", "vox_gamify_shop_buy"),
    ("vox_ludus_collegium_join", "vox_gamify_collegium_join"),
    ("vox_ludus_battle_start", "vox_gamify_battle_start"),
    ("vox_ludus_battle_submit", "vox_gamify_battle_submit"),
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

#[cfg(test)]
mod tests {
    use super::canonical_tool_name;

    #[test]
    fn ludus_wire_alias_maps_to_gamify() {
        assert_eq!(
            canonical_tool_name("vox_ludus_progress_snapshot"),
            "vox_gamify_progress_snapshot"
        );
        assert_eq!(
            canonical_tool_name("vox_gamify_progress_snapshot"),
            "vox_gamify_progress_snapshot"
        );
    }
}
