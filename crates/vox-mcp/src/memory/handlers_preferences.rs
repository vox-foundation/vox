use crate::{ServerState, ToolResult};

use super::params::{
    BehaviorRecordParams, BehaviorSummaryParams, LearnPatternParams, MemoryRecallDbParams,
    MemorySaveDbParams, PreferenceGetParams, PreferenceListParams, PreferenceSetParams,
};

/// Get a user preference from VoxDb.
pub async fn preference_get(state: &ServerState, params: PreferenceGetParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db.get_user_preference(&params.user_id, &params.key).await {
            Ok(Some(val)) => ToolResult::ok(val).to_json(),
            Ok(None) => ToolResult::<String>::err(format!("Preference '{}' not found", params.key))
                .to_json(),
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Set a user preference in VoxDb.
pub async fn preference_set(state: &ServerState, params: PreferenceSetParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .set_user_preference(&params.user_id, &params.key, &params.value)
            .await
        {
            Ok(()) => {
                if params.key == "socrates_gate_enforced" {
                    if let Ok(enforce) = params.value.parse::<bool>() {
                        let cfg_handle = state.orchestrator.config_handle();
                        let mut cfg = cfg_handle.write().unwrap();
                        cfg.socrates_gate_enforce = enforce;
                    }
                }
                ToolResult::ok(format!("Set '{}' = '{}'", params.key, params.value)).to_json()
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// List user preferences from VoxDb, optionally filtered by key prefix.
pub async fn preference_list(state: &ServerState, params: PreferenceListParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
        Some(db) => match db
            .list_user_preferences(&params.user_id, params.prefix.as_deref())
            .await
        {
            Ok(prefs) => {
                let lines: Vec<String> = prefs.iter().map(|(k, v)| format!("{k} = {v}")).collect();
                ToolResult::ok(lines.join("\n")).to_json()
            }
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Store a learned behavior pattern in VoxDb.
pub async fn learn_pattern(state: &ServerState, params: LearnPatternParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
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
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Record a user behavior event and get triggered suggestions.
pub async fn behavior_record(state: &ServerState, params: BehaviorRecordParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
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
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
    }
}

/// Analyze all behavior events for a user and return learned patterns summary.
pub async fn behavior_summary(state: &ServerState, params: BehaviorSummaryParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
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
                Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
            }
        }
    }
}

/// Persist a fact directly into VoxDb agent_memory table.
pub async fn memory_save_db(state: &ServerState, params: MemorySaveDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
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
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}

/// Recall facts from VoxDb agent_memory table.
pub async fn memory_recall_db(state: &ServerState, params: MemoryRecallDbParams) -> String {
    match &state.db {
        None => ToolResult::<String>::err("VoxDb not attached").to_json(),
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
            Err(e) => ToolResult::<String>::err(format!("{e}")).to_json(),
        },
    }
}
