//! In-app notifications persistence.

use anyhow::Result;
use turso::params;
use vox_db::Codex;

/// Default TTL for notifications: 7 days in seconds.
const NOTIF_TTL_SECS: i64 = 7 * 24 * 3600;

/// Persist a new notification. Expired_at is set to now + 7 days by default.
pub async fn insert_notification(
    db: &Codex,
    notif: &crate::notifications::Notification,
) -> Result<()> {
    let expires = notif.created_at + NOTIF_TTL_SECS;
    let notif_type = format!("{:?}", notif.notification_type);
    let id = notif.id.clone();
    let user_id = notif.user_id.clone();
    let title = notif.title.clone();
    let message = notif.message.clone();
    let read_flag: i64 = if notif.read { 1 } else { 0 };
    let created_at = notif.created_at;
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "INSERT OR IGNORE INTO gamify_notifications
                 (id, user_id, notification_type, title, message, read, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id.as_str(), user_id.as_str(), notif_type.as_str(),
                    title.as_str(), message.as_str(), read_flag, created_at, expires,
                ],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// List unread notifications for a user (up to `limit`).
pub async fn list_unread_notifications(
    db: &Codex,
    user_id: &str,
    limit: u32,
) -> Result<Vec<crate::notifications::Notification>> {
    let now = crate::util::now_unix();
    let mut rows = db
        .connection()
        .query(
            "SELECT id, notification_type, title, message, created_at
             FROM gamify_notifications
             WHERE user_id = ?1 AND read = 0 AND (expires_at = 0 OR expires_at > ?2)
             ORDER BY created_at DESC
             LIMIT ?3",
            params![user_id, now, limit as i64],
        )
        .await?;
    let mut out = Vec::new();
    while let Some(row) = rows.next().await? {
        let id: String = row.get(0)?;
        let notif_type_str: String = row.get(1)?;
        let title: String = row.get(2)?;
        let message: String = row.get(3)?;
        let created_at: i64 = row.get(4)?;
        let notif_type = parse_notification_type(&notif_type_str);
        out.push(crate::notifications::Notification {
            id,
            user_id: user_id.to_string(),
            notification_type: notif_type,
            title,
            message,
            read: false,
            created_at,
        });
    }
    Ok(out)
}

/// Mark a notification as read by ID (any user row — prefer [`mark_notification_read_for_user`] from MCP).
pub async fn mark_notification_read(db: &Codex, notif_id: &str) -> Result<()> {
    let notif_id = notif_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_notifications SET read = 1 WHERE id = ?1",
                params![notif_id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Mark a notification as read for a specific user (prevents cross-user ACK by id).
pub async fn mark_notification_read_for_user(
    db: &Codex,
    user_id: &str,
    notif_id: &str,
) -> Result<u64> {
    let notif_id = notif_id.to_string();
    let user_id = user_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            let n = conn
                .execute(
                    "UPDATE gamify_notifications SET read = 1 WHERE id = ?1 AND user_id = ?2",
                    params![notif_id.as_str(), user_id.as_str()],
                )
                .await?;
            Ok::<_, vox_db::StoreError>(n)
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Mark all unread notifications for a user as read.
pub async fn mark_all_notifications_read(db: &Codex, user_id: &str) -> Result<()> {
    let user_id = user_id.to_string();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            conn.execute(
                "UPDATE gamify_notifications SET read = 1 WHERE user_id = ?1 AND read = 0",
                params![user_id.as_str()],
            )
            .await?;
            Ok::<(), vox_db::StoreError>(())
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Delete notifications older than their `expires_at` timestamp (TTL cleanup).
pub async fn cleanup_expired_notifications(db: &Codex) -> Result<u64> {
    let now = crate::util::now_unix();
    let breaker = db.breaker().clone();
    let conn = db.connection().clone();
    breaker
        .call(|| async move {
            let n = conn
                .execute(
                    "DELETE FROM gamify_notifications WHERE expires_at > 0 AND expires_at < ?1",
                    params![now],
                )
                .await?;
            Ok::<_, vox_db::StoreError>(n)
        })
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

fn parse_notification_type(s: &str) -> crate::notifications::NotificationType {
    use crate::notifications::NotificationType;
    match s {
        "LevelUp" => NotificationType::LevelUp,
        "AchievementUnlocked" => NotificationType::AchievementUnlocked,
        "StreakContinued" => NotificationType::StreakContinued,
        "StreakLost" => NotificationType::StreakLost,
        "ChallengeCompleted" => NotificationType::ChallengeCompleted,
        "CompanionStatus" => NotificationType::CompanionStatus,
        "QuestCompleted" => NotificationType::QuestCompleted,
        _ => NotificationType::CompanionStatus,
    }
}
