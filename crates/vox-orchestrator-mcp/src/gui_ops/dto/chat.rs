//! Wire DTOs for the Axis chat-session surface (plan 07 Task 4).
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionDto {
    pub session_id: String,
    pub title: String,
    pub updated_at: String,
    pub message_count: i64,
    pub conversation_id: i64,
    pub repository_id: Option<String>,
    /// Parent session ID when this session was branched from an existing turn (`BRANCH-REWIND-01`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session_id: Option<String>,
    /// Source message ID in `parent_session_id` where the branch forked (`BRANCH-REWIND-01`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fork_from_message_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessageDto {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: String,
    pub task_id: Option<String>,
    /// Model that produced this message, if recorded at append time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    /// Turn latency in milliseconds, if recorded at append time (synchronous
    /// chat replies only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
    /// Human-readable reason the model was chosen, if recorded at append time
    /// (synchronous chat replies only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_reason: Option<String>,
    /// True when the opt-in post-reply grounding check flagged this reply as
    /// low-confidence. `None` when the check was not run.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grounding_flagged: Option<bool>,
    /// Snapshot VCS (`vox-vcs` / Jujutsu) checkpoint operation ID recorded at turn time (`BRANCH-REWIND-02`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vcs_checkpoint_op_id: Option<String>,
    /// Challenger model ID when this turn was chosen via Side-by-Side Arena comparison (`ARENA-COMPARE-01`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub arena_challenger_model: Option<String>,
}

/// One candidate arm in a Side-by-Side Arena turn comparison (`ARENA-COMPARE-01`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaCandidateTurnDto {
    pub arm: String,
    pub model_id: String,
    pub content: String,
    pub latency_ms: u64,
    pub ttft_ms: Option<u64>,
    pub selection_reason: Option<String>,
    pub estimated_tokens: usize,
    pub is_local_or_mens: bool,
}

/// Side-by-Side Arena turn comparison result (`ARENA-COMPARE-01`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaTurnComparisonDto {
    pub comparison_id: String,
    pub session_id: String,
    pub prompt: String,
    pub arm_a: ArenaCandidateTurnDto,
    pub arm_b: ArenaCandidateTurnDto,
}

/// Outcome of voting on a Side-by-Side Arena comparison (`ARENA-COMPARE-02`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ArenaVoteOutcomeDto {
    pub comparison_id: String,
    pub session_id: String,
    pub winner: String,
    pub chosen_model_id: String,
    pub persisted_message_id: i64,
    pub quality_a: f64,
    pub quality_b: f64,
    pub quality_delta_winner: f64,
    pub dpo_harvested: bool,
    pub telemetry_queued: bool,
}

/// Outcome of rewinding a chat session to an earlier turn checkpoint (`BRANCH-REWIND-02`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChatRewindOutcomeDto {
    pub session_id: String,
    pub rewound_to_message_id: i64,
    pub kept_messages: usize,
    pub dropped_messages: usize,
    pub vcs_checkpoint_op_id: String,
}

impl ChatMessageDto {
    /// A stored `conversation_messages` row, with display metadata read back out of its
    /// `payload_json` (unparseable payloads contribute nothing).
    pub fn from_row(
        id: i64,
        role: String,
        content: String,
        created_at: String,
        payload: Option<&str>,
    ) -> Self {
        let v = payload
            .and_then(|p| serde_json::from_str::<serde_json::Value>(p).ok())
            .unwrap_or(serde_json::Value::Null);
        let text = |k: &str| v.get(k).and_then(|x| x.as_str()).map(str::to_string);
        Self {
            id,
            role,
            content,
            created_at,
            task_id: text("task_id"),
            model_id: text("model_id"),
            latency_ms: v.get("latency_ms").and_then(|l| l.as_u64()),
            selection_reason: text("selection_reason"),
            grounding_flagged: v.get("grounding_flagged").and_then(|g| g.as_bool()),
            vcs_checkpoint_op_id: text("vcs_checkpoint_op_id"),
            arena_challenger_model: text("arena_challenger_model"),
        }
    }
}

/// `payload_json` for an assistant reply: only the fields that are present, or `None` when
/// nothing was recorded.
pub fn reply_metadata_json(
    model_id: Option<&str>,
    latency_ms: Option<u64>,
    selection_reason: Option<&str>,
    grounding_flagged: Option<bool>,
) -> Option<String> {
    let mut obj = serde_json::Map::new();
    if let Some(m) = model_id {
        obj.insert("model_id".into(), m.into());
    }
    if let Some(l) = latency_ms {
        obj.insert("latency_ms".into(), l.into());
    }
    if let Some(r) = selection_reason {
        obj.insert("selection_reason".into(), r.into());
    }
    if let Some(g) = grounding_flagged {
        obj.insert("grounding_flagged".into(), g.into());
    }
    (!obj.is_empty()).then(|| serde_json::Value::Object(obj).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_dto_reads_display_metadata_from_payload() {
        let payload = reply_metadata_json(Some("m/x"), Some(842), Some("scorer"), Some(true));
        let dto = ChatMessageDto::from_row(
            1,
            "assistant".into(),
            "hi".into(),
            "now".into(),
            payload.as_deref(),
        );
        assert_eq!(dto.model_id.as_deref(), Some("m/x"));
        assert_eq!(dto.latency_ms, Some(842));
        assert_eq!(dto.selection_reason.as_deref(), Some("scorer"));
        assert_eq!(dto.grounding_flagged, Some(true));
        assert_eq!(dto.task_id, None);
    }

    #[test]
    fn message_dto_tolerates_missing_or_garbage_payload() {
        for p in [None, Some("not json"), Some("[1,2]")] {
            let dto = ChatMessageDto::from_row(2, "user".into(), "x".into(), "t".into(), p);
            assert!(dto.model_id.is_none() && dto.task_id.is_none() && dto.latency_ms.is_none());
        }
    }

    #[test]
    fn reply_metadata_is_none_when_nothing_was_recorded() {
        assert_eq!(reply_metadata_json(None, None, None, None), None);
        let j = reply_metadata_json(None, Some(5), None, None).unwrap();
        assert_eq!(j, r#"{"latency_ms":5}"#);
    }

    #[test]
    fn absent_optionals_are_omitted_from_the_wire() {
        let dto = ChatMessageDto::from_row(3, "user".into(), "hello".into(), "now".into(), None);
        let j = serde_json::to_string(&dto).unwrap();
        for k in [
            "model_id",
            "latency_ms",
            "selection_reason",
            "grounding_flagged",
        ] {
            assert!(!j.contains(k), "{k} absent when None: {j}");
        }
        let back: ChatMessageDto = serde_json::from_str(&j).unwrap();
        assert_eq!(back.content, "hello");
    }
}
