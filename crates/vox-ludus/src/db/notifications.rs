//! In-app notifications persistence.

use anyhow::Result;
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
    db.insert_gamify_notification_ignore(
        notif.id.as_str(),
        notif.user_id.as_str(),
        notif_type.as_str(),
        notif.title.as_str(),
        notif.message.as_str(),
        notif.read,
        notif.created_at,
        expires,
    )
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
    let rows = db
        .list_gamify_unread_notifications(user_id, now, limit as i64)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let mut out = Vec::new();
    for (id, notif_type_str, title, message, created_at) in rows {
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
    db.mark_gamify_notification_read(notif_id)
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
    db.mark_gamify_notification_read_for_user(user_id, notif_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))
}

/// Mark all unread notifications for a user as read.
pub async fn mark_all_notifications_read(db: &Codex, user_id: &str) -> Result<()> {
    db.mark_all_gamify_notifications_read(user_id)
        .await
        .map_err(|e| anyhow::anyhow!("{}", e))?;
    Ok(())
}

/// Delete notifications older than their `expires_at` timestamp (TTL cleanup).
pub async fn cleanup_expired_notifications(db: &Codex) -> Result<u64> {
    let now = crate::util::now_unix();
    db.delete_expired_gamify_notifications(now)
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
