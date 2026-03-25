//! Database helpers shared across `crate::db` submodules.

/// Canonical user-identity normalisation.
///
/// Priority: non-empty `vox_db::paths::local_user_id()` > `DEFAULT_USER_ID`.
/// All reward/event write paths MUST call this instead of constructing IDs inline.
pub fn canonical_user_id() -> String {
    let from_db = vox_db::paths::local_user_id();
    if !from_db.is_empty() && from_db != "user" {
        from_db
    } else {
        crate::util::DEFAULT_USER_ID.to_string()
    }
}

/// Parse a quest-type string from DB without losing `agent_complete` / `collaborate`.
pub(super) fn parse_quest_type(s: &str) -> crate::quest::QuestType {
    use crate::quest::QuestType;
    match s {
        "create" => QuestType::Create,
        "review" => QuestType::Review,
        "battle" => QuestType::Battle,
        "improve" => QuestType::Improve,
        "agent_complete" => QuestType::AgentComplete,
        "collaborate" => QuestType::Collaborate,
        "ai_feedback" => QuestType::AiFeedback,
        "populi_contribute" => QuestType::PopuliContribute,
        "build_streak" => QuestType::BuildStreak,
        "doc_sprint" => QuestType::DocSprint,
        "toestub_fix" => QuestType::ToestubFix,
        "testing" => QuestType::Testing,
        "research" => QuestType::Research,
        "first_time" => QuestType::FirstTime,
        other => {
            tracing::warn!("unknown quest_type '{}' in DB, defaulting to Create", other);
            QuestType::Create
        }
    }
}
