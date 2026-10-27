//! Structured 5-strategy cross-agent context handoff (`T3-HANDOFF-MCP-01`).

use serde::{Deserialize, Serialize};

/// Default token budget for a cross-agent handoff payload (`16_000` tokens).
pub const DEFAULT_HANDOFF_TOKEN_CAP: usize = 16_000;
/// Hard byte cap (`~4 chars/token`, `64_000` bytes).
pub const HANDOFF_BYTE_CAP: usize = 64_000;

/// Context extraction strategy for `ContextHandoffService` (`T3-HANDOFF-MCP-01`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "strategy", rename_all = "snake_case")]
pub enum HandoffStrategy {
    /// Include full transcript up to the `64_000`-byte (`16_000`-token) ceiling.
    FullContext,
    /// Summarize transcript, retaining the most recent 2 verbatim exchanges.
    Summarized,
    /// Retain only the last `n` user-assistant exchanges.
    RecentMessages { count: usize },
    /// Retain only architectural plans, file diffs, and tool error summaries.
    CodeAndPlansOnly,
    /// User-supplied custom focus directive + recent context.
    CustomPrompt { directive: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffTranscriptTurn {
    pub role: String,
    pub content: String,
    pub is_plan_or_code: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HandoffPackage {
    pub source_thread_id: String,
    pub source_provider: String,
    pub target_provider: String,
    pub strategy: HandoffStrategy,
    pub formatted_prompt: String,
    pub truncated: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ContextHandoffService;

impl ContextHandoffService {
    pub fn build_package(
        source_thread_id: impl Into<String>,
        source_provider: impl Into<String>,
        target_provider: impl Into<String>,
        strategy: HandoffStrategy,
        turns: &[HandoffTranscriptTurn],
    ) -> HandoffPackage {
        let selected: Vec<&HandoffTranscriptTurn> = match &strategy {
            HandoffStrategy::FullContext => turns.iter().collect(),
            HandoffStrategy::Summarized | HandoffStrategy::CustomPrompt { .. } => {
                let keep = turns.len().min(4);
                turns[turns.len().saturating_sub(keep)..].iter().collect()
            }
            HandoffStrategy::RecentMessages { count } => {
                let keep = (*count).saturating_mul(2).min(turns.len());
                turns[turns.len().saturating_sub(keep)..].iter().collect()
            }
            HandoffStrategy::CodeAndPlansOnly => {
                turns.iter().filter(|t| t.is_plan_or_code).collect()
            }
        };

        let mut body = String::from("<context_handoff>\n");
        if let HandoffStrategy::CustomPrompt { directive } = &strategy {
            body.push_str("<directive>");
            body.push_str(directive);
            body.push_str("</directive>\n");
        }
        if matches!(strategy, HandoffStrategy::Summarized) && turns.len() > 4 {
            let earlier = &turns[..turns.len().saturating_sub(4)];
            body.push_str("<earlier_context_summary>\n");
            for (idx, t) in earlier.iter().enumerate() {
                let first_line = t.content.lines().next().unwrap_or("");
                let cut = first_line.floor_char_boundary(first_line.len().min(120));
                body.push_str(&format!(
                    "- Turn {} [{}]: {}\n",
                    idx + 1,
                    t.role,
                    &first_line[..cut]
                ));
            }
            body.push_str("</earlier_context_summary>\n");
        }
        for turn in selected {
            body.push_str(&format!("[{}]: {}\n", turn.role, turn.content));
        }
        body.push_str("</context_handoff>");

        let mut truncated = false;
        const SUFFIX: &str = "\n...[truncated]</context_handoff>";
        if body.len() > HANDOFF_BYTE_CAP {
            truncated = true;
            let safe_cut = body.floor_char_boundary(HANDOFF_BYTE_CAP - SUFFIX.len());
            body.truncate(safe_cut);
            body.push_str(SUFFIX);
        }

        HandoffPackage {
            source_thread_id: source_thread_id.into(),
            source_provider: source_provider.into(),
            target_provider: target_provider.into(),
            strategy,
            formatted_prompt: body,
            truncated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn context_handoff_filters_strategies_and_enforces_64kb_cap() {
        let turns = vec![
            HandoffTranscriptTurn {
                role: "user".into(),
                content: "chatting".into(),
                is_plan_or_code: false,
            },
            HandoffTranscriptTurn {
                role: "assistant".into(),
                content: "diff --git a/foo.rs b/foo.rs".into(),
                is_plan_or_code: true,
            },
        ];
        let pkg = ContextHandoffService::build_package(
            "t-1",
            "claude",
            "codex",
            HandoffStrategy::CodeAndPlansOnly,
            &turns,
        );
        assert!(pkg.formatted_prompt.contains("diff --git"));
        assert!(!pkg.formatted_prompt.contains("chatting"));
        assert!(!pkg.truncated);

        let huge_turns = vec![HandoffTranscriptTurn {
            role: "assistant".into(),
            content: "x".repeat(HANDOFF_BYTE_CAP + 5_000),
            is_plan_or_code: true,
        }];
        let capped = ContextHandoffService::build_package(
            "t-2",
            "claude",
            "gemini",
            HandoffStrategy::FullContext,
            &huge_turns,
        );
        assert!(capped.truncated);
        assert!(capped.formatted_prompt.len() <= HANDOFF_BYTE_CAP);
    }
}
