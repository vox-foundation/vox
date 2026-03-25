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
    db.connection()
        .execute(
            "INSERT OR IGNORE INTO gamify_notifications
             (id, user_id, notification_type, title, message, read, created_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                notif.id.clone(),
                notif.user_id.clone(),
                notif_type,
                notif.title.clone(),
                notif.message.clone(),
                if notif.read { 1i64 } else { 0i64 },
                notif.created_at,
                expires,
            ],
        )
        .await?;
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
        let notif_type_str: String = row.get(1)?;
        let notif_type = parse_notification_type(&notif_type_str);
        out.push(crate::notifications::Notification {
            id: row.get(0)?,
            user_id: user_id.to_string(),
            notification_type: notif_type,
            title: row.get(2)?,
            message: row.get(3)?,
            read: false,
            created_at: row.get::<i64>(4)?,
        });
    }
    Ok(out)
}

/// Mark a notification as read by ID.
pub async fn mark_notification_read(db: &Codex, notif_id: &str) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_notifications SET read = 1 WHERE id = ?1",
            params![notif_id],
        )
        .await?;
    Ok(())
}

/// Mark all unread notifications for a user as read.
pub async fn mark_all_notifications_read(db: &Codex, user_id: &str) -> Result<()> {
    db.connection()
        .execute(
            "UPDATE gamify_notifications SET read = 1 WHERE user_id = ?1 AND read = 0",
            params![user_id],
        )
        .await?;
    Ok(())
}

/// Delete notifications older than their `expires_at` timestamp (TTL cleanup).
pub async fn cleanup_expired_notifications(db: &Codex) -> Result<u64> {
    let now = crate::util::now_unix();
    let rows = db
        .connection()
        .execute(
            "DELETE FROM gamify_notifications WHERE expires_at > 0 AND expires_at < ?1",
            params![now],
        )
        .await?;
    Ok(rows)
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
