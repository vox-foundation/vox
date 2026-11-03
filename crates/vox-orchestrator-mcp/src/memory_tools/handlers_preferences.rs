use crate::params::ToolResult;
use crate::server_state::ServerState;

const REM_PREF_VOXDB: &str = "Attach VoxDb (`VOX_DB_PATH` / `VOX_DB_URL`) on the MCP server for preference and learner tools.";
const REM_PREF_KEY: &str =
    "List or set preferences first; the key may not exist for this `user_id`.";
const REM_PREF_DB: &str =
    "Verify Turso connectivity and vox-db migrations for preferences/learner tables.";

const OVERSIGHT_PREF_PREFIXES: &[&str] = &[
    "socrates_gate_enforce",
    "attention_",
    "oversight.",
    "oversight_",
    "approval",
    "trust_",
    "permission",
    "tier_gate",
];

fn is_oversight_preference_key(key: &str) -> bool {
    let normalized = key.trim().to_ascii_lowercase();
    OVERSIGHT_PREF_PREFIXES
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
}

use super::params::{
    BehaviorRecordParams, BehaviorSummaryParams, LearnPatternParams, MemoryRecallDbParams,
    MemorySaveDbParams, PreferenceGetParams, PreferenceListParams, PreferenceSetParams,
};

/// Get a user preference from VoxDb.
pub async fn preference_get(state: &ServerState, params: PreferenceGetParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db.get_user_preference(&params.user_id, &params.key).await {
            Ok(Some(val)) => ToolResult::ok(val).to_json(),
            Ok(None) => ToolResult::<String>::err_with_remediation(
                format!("Preference '{}' not found", params.key),
                REM_PREF_KEY,
            )
            .to_json(),
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

/// Set a user preference in VoxDb.
///
/// Refuses oversight-affecting keys (`socrates_gate_enforce*`, `attention_*`,
/// `oversight.*`, `oversight_*`, `approval*`, `trust_*`, `permission*`,
/// `tier_gate*`) with `OVERSIGHT_PREF_REFUSED`. Oversight-affecting preferences
/// are policy grants (FLOOR-03); human operators configure them via Vox.toml,
/// the GUI settings, or the `vox attention` CLI.
pub async fn preference_set(state: &ServerState, params: PreferenceSetParams) -> String {
    if is_oversight_preference_key(&params.key) {
        return ToolResult::<String>::err_with_remediation(
            format!(
                "OVERSIGHT_PREF_REFUSED: preference '{}' affects oversight and cannot be set through MCP",
                params.key
            ),
            "Configure oversight settings in Vox.toml, through the GUI settings, or via the 'vox attention' CLI.",
        )
        .to_json();
    }
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db
            .set_user_preference(&params.user_id, &params.key, &params.value)
            .await
        {
            Ok(()) => {
                ToolResult::ok(format!("Set '{}' = '{}'", params.key, params.value)).to_json()
            }
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

/// List user preferences from VoxDb, optionally filtered by key prefix.
pub async fn preference_list(state: &ServerState, params: PreferenceListParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db
            .list_user_preferences(&params.user_id, params.prefix.as_deref())
            .await
        {
            Ok(prefs) => {
                let lines: Vec<String> = prefs.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                ToolResult::ok(lines.join("\n")).to_json()
            }
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

/// Store a learned behavior pattern in VoxDb.
pub async fn learn_pattern(state: &ServerState, params: LearnPatternParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db
            .store_learned_pattern(
                &params.user_id,
                &params.pattern_type,
                &params.category,
                &params.description,
                params.confidence.unwrap_or(0.5),
                None,
            )
            .await
        {
            Ok(id) => ToolResult::ok(format!("Pattern stored with id={id}")).to_json(),
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

/// Record a user behavior event and get triggered suggestions.
pub async fn behavior_record(state: &ServerState, params: BehaviorRecordParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => {
            let learner = db.learner();
            match learner
                .observe(
                    &params.user_id,
                    &params.event_type,
                    params.context.as_deref(),
                    params.metadata.as_deref(),
                    None,
                )
                .await
            {
                Ok(suggestions) => {
                    if suggestions.is_empty() {
                        ToolResult::ok("Event recorded. No new patterns detected.".to_string())
                            .to_json()
                    } else {
                        let lines: Vec<String> = suggestions
                            .iter()
                            .map(|s| {
                                format!(
                                    "[{:.0}%] {}: {}",
                                    s.confidence * 100.0,
                                    s.title,
                                    s.description
                                )
                            })
                            .collect();
                        ToolResult::ok(format!(
                            "Event recorded. New patterns:\n{}",
                            lines.join("\n")
                        ))
                        .to_json()
                    }
                }
                Err(e) => ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB)
                    .to_json(),
            }
        }
    }
}

/// Analyze all behavior events for a user and return learned patterns summary.
pub async fn behavior_summary(state: &ServerState, params: BehaviorSummaryParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => {
            let learner = db.learner();
            match learner.analyze(&params.user_id, None).await {
                Ok(patterns) => {
                    if patterns.is_empty() {
                        ToolResult::ok("No patterns detected yet.".to_string()).to_json()
                    } else {
                        let lines: Vec<String> = patterns
                            .iter()
                            .map(|p| {
                                format!(
                                    "[{:.0}%] {} / {} — {}",
                                    p.confidence * 100.0,
                                    p.pattern_type,
                                    p.category,
                                    p.description
                                )
                            })
                            .collect();
                        ToolResult::ok(lines.join("\n")).to_json()
                    }
                }
                Err(e) => ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB)
                    .to_json(),
            }
        }
    }
}

/// Persist a fact directly into VoxDb agent_memory table.
pub async fn memory_save_db(state: &ServerState, params: MemorySaveDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db
            .save_memory(vox_db::MemoryParams {
                agent_id: &params.agent_id,
                session_id: &params.session_id,
                memory_type: &params.memory_type,
                content: &params.content,
                metadata: None,
                importance: params.importance.unwrap_or(1.0),
                vcs_snapshot_id: None,
            })
            .await
        {
            Ok(id) => ToolResult::ok(format!("Memory saved with id={id}")).to_json(),
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

/// Recall facts from VoxDb agent_memory table.
pub async fn memory_recall_db(state: &ServerState, params: MemoryRecallDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err_with_remediation("VoxDb not attached", REM_PREF_VOXDB)
            .to_json(),
        Some(db) => match db
            .recall_memory(
                &params.agent_id,
                params.memory_type.as_deref(),
                params.limit.unwrap_or(20),
                None,
            )
            .await
        {
            Ok(entries) => {
                if entries.is_empty() {
                    ToolResult::ok("No memories found.".to_string()).to_json()
                } else {
                    let lines: Vec<String> = entries
                        .iter()
                        .map(|e| format!("[{}] [{:.2}] {}", e.memory_type, e.importance, e.content))
                        .collect();
                    ToolResult::ok(lines.join("\n")).to_json()
                }
            }
            Err(e) => {
                ToolResult::<String>::err_with_remediation(format!("{e}"), REM_PREF_DB).to_json()
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::server_state::ServerState;

    #[test]
    fn oversight_key_predicate_table() {
        let positive = [
            "socrates_gate_enforce",
            "socrates_gate_enforced",
            "attention_enabled",
            "attention_budget_ms",
            "attention_alert_threshold",
            "oversight.level",
            "oversight_level",
            "approval_allowlist.repo.vox_run_shell",
            "trust_relax_gates",
            "permission_mode",
            "tier_gate_min",
            "  Attention_Enabled  ",
        ];
        for k in positive {
            assert!(
                is_oversight_preference_key(k),
                "key '{k}' should be recognized as an oversight preference key"
            );
        }

        let negative = [
            "ui_theme",
            "editor.font_size",
            "attentive_mode",
            "trusted_hosts",
            "user.name",
        ];
        for k in negative {
            assert!(
                !is_oversight_preference_key(k),
                "key '{k}' should not be recognized as an oversight preference key"
            );
        }
    }

    #[tokio::test]
    async fn preference_set_refuses_oversight_keys() {
        let state = ServerState::new_test().await;

        let initial_cfg = {
            let handle = state.orchestrator.config_handle();
            let cfg = handle.read().unwrap();
            (
                cfg.socrates_gate_enforce,
                cfg.attention_enabled,
                cfg.attention_budget_ms,
                cfg.attention_alert_threshold,
            )
        };

        let oversight_keys = [
            "socrates_gate_enforce",
            "socrates_gate_enforced",
            "attention_enabled",
            "attention_budget_ms",
            "attention_alert_threshold",
            "oversight.level",
            "oversight_level",
            "approval_allowlist.repo.vox_run_shell",
            "trust_relax_gates",
            "permission_mode",
            "tier_gate_min",
            "  Attention_Enabled  ",
        ];

        for key in oversight_keys {
            let out = crate::dispatch::handle_tool_call(
                &state,
                "vox_preference_set",
                serde_json::json!({
                    "user_id": "local_user",
                    "key": key,
                    "value": "false",
                }),
            )
            .await
            .unwrap();

            assert!(
                out.contains("OVERSIGHT_PREF_REFUSED"),
                "expected refusal for key '{key}', got: {out}"
            );
        }

        let post_cfg = {
            let handle = state.orchestrator.config_handle();
            let cfg = handle.read().unwrap();
            (
                cfg.socrates_gate_enforce,
                cfg.attention_enabled,
                cfg.attention_budget_ms,
                cfg.attention_alert_threshold,
            )
        };
        assert_eq!(
            initial_cfg, post_cfg,
            "config must remain unchanged after attempted lowering"
        );
    }

    #[tokio::test]
    async fn preference_set_allows_non_policy_keys() {
        let state = ServerState::new_test().await;

        for key in ["ui_theme", "editor.font_size"] {
            let out = crate::dispatch::handle_tool_call(
                &state,
                "vox_preference_set",
                serde_json::json!({
                    "user_id": "local_user",
                    "key": key,
                    "value": "dark",
                }),
            )
            .await
            .unwrap();

            assert!(
                !out.contains("OVERSIGHT_PREF_REFUSED"),
                "non-policy key '{key}' should not be refused as oversight"
            );
            assert!(
                out.contains("VoxDb not attached"),
                "expected 'VoxDb not attached' without DB for '{key}', got: {out}"
            );
        }
    }
}
