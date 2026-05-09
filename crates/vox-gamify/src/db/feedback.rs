//! AI feedback persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

use crate::feedback::AiFeedback;

/// Insert a piece of AI feedback.
#[allow(clippy::too_many_arguments)]
pub async fn insert_feedback(db: &Codex, fb: &AiFeedback) -> Result<()> {
    let id = fb.id.clone();
    let user_id = fb.user_id.clone();
    let session_id = fb.session_id.clone();
    let response_id = fb.response_id.clone();
    let thumbs: i64 = if fb.thumbs_up { 1 } else { 0 };
    let comment = fb.comment.clone().unwrap_or_default();
    let tokens_generated = fb.tokens_generated as i64;
    let example_code = fb.example_code.clone().unwrap_or_default();
    let corpus: i64 = if fb.contributed_to_corpus { 1 } else { 0 };
    let created_at = fb.created_at;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT INTO gamify_ai_feedback
                 (id, user_id, session_id, response_id, thumbs_up, comment, tokens_generated, example_code, contributed_to_corpus, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    id.as_str(), user_id.as_str(), session_id.as_str(), response_id.as_str(),
                    thumbs, comment.as_str(), tokens_generated, example_code.as_str(),
                    corpus, created_at,
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}
