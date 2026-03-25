use super::rotation::{current_week_number, evaluate_random_drop};
use super::types::PeriodicCondition;

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
            // Check if last_active is today
            let sql = "SELECT last_active FROM gamify_profiles WHERE user_id = ?1";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Text(last_active)) = rows[0].get_value(0) {
                        let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                        last_active.starts_with(&today)
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::WeeklyCheckIn => {
            // Check if user has any activity this week
            let sql = "SELECT last_active FROM gamify_profiles WHERE user_id = ?1";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Text(last_active)) = rows[0].get_value(0) {
                        let week_num = current_week_number();
                        // Naive check: if we can parse the date and calculate its week number
                        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&last_active) {
                            let ts = dt.timestamp() as u64;
                            let last_week = (ts + 4 * 86_400) / (7 * 86_400);
                            last_week == week_num
                        } else {
                            // If it's not RFC3339 (SQLite datetime('now') is YYYY-MM-DD HH:MM:SS)
                            // We attempt a simpler check or just return true if it exists
                            !last_active.is_empty()
                        }
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::DailyQuestComplete => {
            // All 3 daily quests completed today
            let sql = "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', 'start of day')";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Integer(count)) = rows[0].get_value(0) {
                        count >= 3
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::MilestoneUnlock { achievement_id } => {
            let sql = "SELECT 1 FROM gamify_achievements WHERE user_id = ?1 AND id = ?2";
            match db
                .query_all(
                    sql,
                    [
                        turso::Value::Text(user_id.to_string()),
                        turso::Value::Text(achievement_id.clone()),
                    ],
                )
                .await
            {
                Ok(rows) => !rows.is_empty(),
                _ => false,
            }
        }
        PeriodicCondition::RandomDrop { probability } => {
            let rand_val: f64 = rand::random();
            evaluate_random_drop(*probability, rand_val)
        }
        PeriodicCondition::WeeklyBuildStreak { min_green } => {
            let sql = "SELECT streak_days FROM gamify_profiles WHERE user_id = ?1";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Integer(streak)) = rows[0].get_value(0) {
                        streak >= *min_green as i64
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::MonthlyDocSprint { min_items, .. } => {
            let sql = "SELECT COUNT(*) FROM gamify_policy_snapshots WHERE user_id = ?1 AND event_type = 'doc_item' AND created_at >= date('now', 'start of month')";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Integer(count)) = rows[0].get_value(0) {
                        count >= *min_items as i64
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
        PeriodicCondition::SeasonalChallenge { challenge_id } => {
            let sql = "SELECT 1 FROM gamify_quests WHERE user_id = ?1 AND id = ?2 AND status = 'completed'";
            match db
                .query_all(
                    sql,
                    [
                        turso::Value::Text(user_id.to_string()),
                        turso::Value::Text(challenge_id.clone()),
                    ],
                )
                .await
            {
                Ok(rows) => !rows.is_empty(),
                _ => false,
            }
        }
        PeriodicCondition::PerfectWeek => {
            // Check if 21 daily quests were completed in the last 7 days
            let sql = "SELECT COUNT(*) FROM gamify_quests WHERE user_id = ?1 AND status = 'completed' AND created_at >= date('now', '-7 days')";
            match db
                .query_all(sql, [turso::Value::Text(user_id.to_string())])
                .await
            {
                Ok(rows) if !rows.is_empty() => {
                    if let Ok(turso::Value::Integer(count)) = rows[0].get_value(0) {
                        count >= 21
                    } else {
                        false
                    }
                }
                _ => false,
            }
        }
    }
}
