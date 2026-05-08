//! AI feedback persistence.

use anyhow::Result;
use vox_db::Codex;

use crate::feedback::AiFeedback;

/// Insert a piece of AI feedback.
pub async fn insert_feedback(db: &Codex, fb: &AiFeedback) -> Result<()> {
    db.insert_gamify_ai_feedback(
        fb.id.as_str(),
        fb.user_id.as_str(),
        fb.session_id.as_str(),
        fb.response_id.as_str(),
        fb.thumbs_up,
        fb.comment.as_deref().unwrap_or(""),
        fb.tokens_generated as i64,
        fb.example_code.as_deref().unwrap_or(""),
        fb.contributed_to_corpus,
        fb.created_at,
    )
    .await
    .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
