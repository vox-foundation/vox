use super::rotation::{current_week_number, evaluate_random_drop};
use super::types::PeriodicCondition;
use turso::params;

/// Evaluates if a user meets a certain condition for a periodic reward.
///
/// Connects to the database via Codex to check profile stats, quest completion,
/// and achievement status.
pub async fn evaluate_condition(
    db: &vox_db::Codex,
    user_id: &str,
    cond: &PeriodicCondition,
) -> bool {
    match cond {
        PeriodicCondition::DailyLogin => {
            match profile_last_active(db, user_id).await {
                Ok(Some(last_active)) => {
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    last_active.starts_with(&today)
                }
                _ => false,
            }
        }
        PeriodicCondition::WeeklyCheckIn => {
            match profile_last_active(db, user_id).await {
                Ok(Some(last_active)) => {
                    let week_num = current_week_number();
                    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last_active) {
                        let ts = dt.timestamp() as u64;
                        let last_week = (ts + 4 * 86_400) / (7 * 86_400);
                        last_week == week_num
                    } else {
                        !last_active.is_empty()
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::DailyQuestComplete => {
            match daily_quests_completed_today_count(db, user_id).await {
                Ok(count) => count >= 3,
                _ => false,
            }
        }
        PeriodicCondition::MilestoneUnlock { achievement_id } => {
            has_achievement(db, user_id, achievement_id.as_str())
                .await
                .unwrap_or_default()
        }
        PeriodicCondition::RandomDrop { probability } => {
            let rand_val: f64 = rand::random();
            evaluate_random_drop(*probability, rand_val)
        }
        PeriodicCondition::WeeklyBuildStreak { min_green } => {
            match profile_streak_days(db, user_id).await {
                Ok(Some(streak)) => streak >= *min_green as i64,
                _ => false,
            }
        }
        PeriodicCondition::MonthlyDocSprint { min_items, .. } => {
            match doc_item_count_this_month(db, user_id).await {
                Ok(count) => count >= *min_items as i64,
                _ => false,
            }
        }
        PeriodicCondition::SeasonalChallenge { challenge_id } => {
            has_completed_quest(db, user_id, challenge_id.as_str())
                .await
                .unwrap_or_default()
        }
        PeriodicCondition::PerfectWeek => {
            match perfect_week_completed_count(db, user_id).await {
                Ok(count) => count >= 21,
                _ => false,
            }
        }
    }
}

async fn profile_last_active(
    db: &vox_db::Codex,
    user_id: &str,
) -> anyhow::Result<Option<String>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT last_active FROM gamify_profiles WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(None);
    };
    if let Ok(n) = row.get::<i64>(0) {
        return Ok(Some(n.to_string()));
    }
    if let Ok(s) = row.get::<String>(0) {
        return Ok(Some(s));
    }
    Ok(None)
}

async fn daily_quests_completed_today_count(
    db: &vox_db::Codex,
    user_id: &str,
) -> anyhow::Result<i64> {
    let mut rows = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', 'start of day')",
            params![user_id],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(0);
    };
    Ok(row.get::<i64>(0).unwrap_or(0))
}

async fn has_achievement(
    db: &vox_db::Codex,
    user_id: &str,
    achievement_id: &str,
) -> anyhow::Result<bool> {
    let mut rows = db
        .connection()
        .query(
            "SELECT 1 FROM gamify_achievements WHERE user_id = ?1 AND id = ?2 LIMIT 1",
            params![user_id, achievement_id],
        )
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn profile_streak_days(
    db: &vox_db::Codex,
    user_id: &str,
) -> anyhow::Result<Option<i64>> {
    let mut rows = db
        .connection()
        .query(
            "SELECT streak_days FROM gamify_profiles WHERE user_id = ?1",
            params![user_id],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(None);
    };
    Ok(Some(row.get::<i64>(0).unwrap_or(0)))
}

async fn doc_item_count_this_month(
    db: &vox_db::Codex,
    user_id: &str,
) -> anyhow::Result<i64> {
    let mut rows = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_policy_snapshots WHERE user_id = ?1 AND event_type = 'doc_item' AND created_at >= date('now', 'start of month')",
            params![user_id],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(0);
    };
    Ok(row.get::<i64>(0).unwrap_or(0))
}

async fn has_completed_quest(
    db: &vox_db::Codex,
    user_id: &str,
    quest_id: &str,
) -> anyhow::Result<bool> {
    let mut rows = db
        .connection()
        .query(
            "SELECT 1 FROM gamify_quests WHERE user_id = ?1 AND id = ?2 AND status = 'completed' LIMIT 1",
            params![user_id, quest_id],
        )
        .await?;
    Ok(rows.next().await?.is_some())
}

async fn perfect_week_completed_count(
    db: &vox_db::Codex,
    user_id: &str,
) -> anyhow::Result<i64> {
    let mut rows = db
        .connection()
        .query(
            "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', '-7 days')",
            params![user_id],
        )
        .await?;
    let Some(row) = rows.next().await? else {
        return Ok(0);
    };
    Ok(row.get::<i64>(0).unwrap_or(0))
}
