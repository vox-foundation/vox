//! AI feedback persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::feedback::AiFeedback;

/// Insert a piece of AI feedback.
pub async fn insert_feedback(db: &Codex, fb: &AiFeedback) -> Result<()> {
    db.connection().execute(
        "INSERT INTO gamify_ai_feedback
             (id, user_id, session_id, response_id, thumbs_up, comment, tokens_generated, example_code, contributed_to_corpus, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            fb.id.clone(),
            fb.user_id.clone(),
            fb.session_id.clone(),
            fb.response_id.clone(),
            if fb.thumbs_up { 1i64 } else { 0i64 },
            fb.comment.clone(),
            fb.tokens_generated as i64,
            fb.example_code.clone(),
            if fb.contributed_to_corpus { 1i64 } else { 0i64 },
            fb.created_at,
        ],
    ).await?;
    Ok(())
}
