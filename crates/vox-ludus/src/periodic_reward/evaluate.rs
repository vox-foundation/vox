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
            match db.gamify_periodic_profile_last_active(user_id).await {
                Ok(Some(last_active)) => {
                    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
                    last_active.starts_with(&today)
                }
                _ => false,
            }
        }
        PeriodicCondition::WeeklyCheckIn => {
            match db.gamify_periodic_profile_last_active(user_id).await {
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
            match db
                .gamify_periodic_daily_quests_completed_today_count(user_id)
                .await
            {
                Ok(count) => count >= 3,
                _ => false,
            }
        }
        PeriodicCondition::MilestoneUnlock { achievement_id } => db
            .gamify_periodic_has_achievement(user_id, achievement_id.as_str())
            .await
            .unwrap_or_default(),
        PeriodicCondition::RandomDrop { probability } => {
            let rand_val: f64 = rand::random();
            evaluate_random_drop(*probability, rand_val)
        }
        PeriodicCondition::WeeklyBuildStreak { min_green } => {
            match db.gamify_periodic_profile_streak_days(user_id).await {
                Ok(Some(streak)) => streak >= *min_green as i64,
                _ => false,
            }
        }
        PeriodicCondition::MonthlyDocSprint { min_items, .. } => {
            match db.gamify_periodic_doc_item_count_this_month(user_id).await {
                Ok(count) => count >= *min_items as i64,
                _ => false,
            }
        }
        PeriodicCondition::SeasonalChallenge { challenge_id } => db
            .gamify_periodic_has_completed_quest(user_id, challenge_id.as_str())
            .await
            .unwrap_or_default(),
        PeriodicCondition::PerfectWeek => {
            match db
                .gamify_periodic_perfect_week_completed_count(user_id)
                .await
            {
                Ok(count) => count >= 21,
                _ => false,
            }
        }
    }
}
