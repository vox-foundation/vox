//! AI response feedback tracking and XP calculation.
//!
//! Records thumbs-up/thumbs-down signals on AI responses.
//! These signals flow into the Mens training corpus and
//! award XP scaled by the richness of the feedback.

use serde::{Deserialize, Serialize};

// ─── Feedback record ─────────────────────────────────────

/// A single piece of feedback on an AI response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFeedback {
    /// Unique feedback identifier.
    pub id: String,
    /// Owning user identifier.
    pub user_id: String,
    /// Session in which the response was generated.
    pub session_id: String,
    /// Hash or ID of the AI response being rated.
    pub response_id: String,
    /// `true` = positive (thumbs up); `false` = negative (thumbs down).
    pub thumbs_up: bool,
    /// Optional free-text comment explaining the rating.
    pub comment: Option<String>,
    /// Number of tokens the rated response generated.
    pub tokens_generated: u64,
    /// Optional `.vox` example code attached to this feedback.
    pub example_code: Option<String>,
    /// Whether this feedback was forwarded to the Mens training corpus.
    pub contributed_to_corpus: bool,
    /// Unix timestamp of submission.
    pub created_at: i64,
}

impl AiFeedback {
    /// Create a new feedback record with default values.
    pub fn new(
        id: impl Into<String>,
        user_id: impl Into<String>,
        session_id: impl Into<String>,
        response_id: impl Into<String>,
        thumbs_up: bool,
        created_at: i64,
    ) -> Self {
        Self {
            id: id.into(),
            user_id: user_id.into(),
            session_id: session_id.into(),
            response_id: response_id.into(),
            thumbs_up,
            comment: None,
            tokens_generated: 0,
            example_code: None,
            contributed_to_corpus: false,
            created_at,
        }
    }

    /// Set the token count for the rated response.
    pub fn with_tokens(mut self, tokens: u64) -> Self {
        self.tokens_generated = tokens;
        self
    }

    /// Attach an optional code example.
    pub fn with_example(mut self, code: impl Into<String>) -> Self {
        self.example_code = Some(code.into());
        self
    }

    /// Attach an optional comment (truncated to 500 chars).
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        let c = comment.into();
        self.comment = Some(c.chars().take(500).collect());
        self
    }

    /// Mark this feedback as having been forwarded to the corpus.
    ///
    /// Auto-triggered when `thumbs_up` is `true` and `example_code` is present.
    pub fn mark_corpus_contributed(mut self) -> Self {
        self.contributed_to_corpus = true;
        self
    }
}

// ─── XP calculation ──────────────────────────────────────

/// Per-session cap on AI feedback XP credits.
pub const DAILY_FEEDBACK_CAP: u32 = 10;

/// Compute the XP earned for a single piece of AI feedback.
///
/// | Source | XP |
/// |---|---|
/// | Base thumbs-up | +5 |
/// | Base thumbs-down | +3 |
/// | Example code attached | +20 |
/// | Contributed to corpus | +30 |
pub fn xp_for_feedback(feedback: &AiFeedback) -> u64 {
    let base: u64 = if feedback.thumbs_up { 5 } else { 3 };
    let example_bonus: u64 = if feedback.example_code.is_some() {
        20
    } else {
        0
    };
    let corpus_bonus: u64 = if feedback.contributed_to_corpus {
        30
    } else {
        0
    };
    base + example_bonus + corpus_bonus
}

/// Whether this feedback should auto-contribute to the corpus.
///
/// True when the user gave a thumbs-up AND attached example code.
pub fn should_auto_contribute(feedback: &AiFeedback) -> bool {
    feedback.thumbs_up && feedback.example_code.is_some()
}

/// The canonical event slug for routing this feedback through `route_event`.
pub fn event_slug(feedback: &AiFeedback) -> &'static str {
    if feedback.thumbs_up {
        "ai_thumbs_up"
    } else {
        "ai_thumbs_down"
    }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_feedback(thumbs_up: bool) -> AiFeedback {
        AiFeedback::new("id-1", "user-1", "sess-1", "resp-1", thumbs_up, 0)
    }

    #[test]
    fn thumbs_up_base_xp() {
        let fb = make_feedback(true);
        assert_eq!(xp_for_feedback(&fb), 5);
    }

    #[test]
    fn thumbs_down_base_xp() {
        let fb = make_feedback(false);
        assert_eq!(xp_for_feedback(&fb), 3);
    }

    #[test]
    fn example_bonus_applies() {
        let fb = make_feedback(true).with_example("fn main() {}");
        assert_eq!(xp_for_feedback(&fb), 5 + 20);
    }

    #[test]
    fn corpus_bonus_stacks() {
        let fb = make_feedback(true)
            .with_example("fn main() {}")
            .mark_corpus_contributed();
        assert_eq!(xp_for_feedback(&fb), 5 + 20 + 30);
    }

    #[test]
    fn thumbs_down_with_corpus() {
        let fb = make_feedback(false).mark_corpus_contributed();
        assert_eq!(xp_for_feedback(&fb), 3 + 30);
    }

    #[test]
    fn auto_contribute_requires_example_and_thumbs_up() {
        assert!(!should_auto_contribute(&make_feedback(true)));
        assert!(!should_auto_contribute(&make_feedback(false)));
        let fb = make_feedback(true).with_example("let x = 1;");
        assert!(should_auto_contribute(&fb));
        let fb_down = make_feedback(false).with_example("let x = 1;");
        assert!(!should_auto_contribute(&fb_down));
    }

    #[test]
    fn comment_truncated_to_500() {
        let long = "x".repeat(600);
        let fb = make_feedback(true).with_comment(long);
        assert_eq!(fb.comment.unwrap().len(), 500);
    }

    #[test]
    fn event_slug_correct() {
        assert_eq!(event_slug(&make_feedback(true)), "ai_thumbs_up");
        assert_eq!(event_slug(&make_feedback(false)), "ai_thumbs_down");
    }
}
