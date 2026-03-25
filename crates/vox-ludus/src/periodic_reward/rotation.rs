use super::pool::REWARD_POOL;
use super::types::{PeriodicCondition, PeriodicReward, PeriodicRewardParams};

/// Returns the number of complete weeks since Unix epoch (Monday-aligned).
pub fn current_week_number() -> u64 {
    // Unix epoch was a Thursday; offset by 4 days so week starts on Monday
    let now_secs: u64 = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    (now_secs + 4 * 86_400) / (7 * 86_400)
}

/// Generate the weekly reward for a user deterministically.
///
/// Uses `(user_id hash XOR week_number)` modulo pool length to pick a
/// unique reward per user per week from the static pool.
pub fn generate_weekly_reward(user_id: &str, week_num: u64, valid_until: i64) -> PeriodicReward {
    let user_hash: u64 = user_id.bytes().enumerate().fold(0u64, |acc, (i, b)| {
        acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 31))
    });

    let idx = ((user_hash ^ week_num) as usize) % REWARD_POOL.len();
    let def = &REWARD_POOL[idx];

    PeriodicReward::new(PeriodicRewardParams {
        id: format!("{}-w{}", def.id, week_num),
        name: def.name.to_string(),
        description: def.description.to_string(),
        icon: def.icon.to_string(),
        xp_bonus: def.xp_bonus,
        crystal_bonus: def.crystal_bonus,
        unlock_condition: PeriodicCondition::WeeklyCheckIn,
        valid_until,
    })
}

/// Returns the current week's reward for a user.
pub fn current_weekly_reward(user_id: &str) -> PeriodicReward {
    let week = current_week_number();
    // Expires at start of next week
    let next_week_secs = ((week + 1) * 7 * 86_400) as i64 - 4 * 86_400;
    generate_weekly_reward(user_id, week, next_week_secs)
}

/// Evaluate whether a `RandomDrop` reward fires for a given random value.
///
/// `rand_val` should be in `[0.0, 1.0)`. Returns true if the drop fires.
pub fn evaluate_random_drop(probability: f64, rand_val: f64) -> bool {
    rand_val < probability.clamp(0.0, 1.0)
}
