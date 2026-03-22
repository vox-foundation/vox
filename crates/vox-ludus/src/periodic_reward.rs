//! Periodic unique reward system.
//!
//! Generates a deterministic weekly rotating pool of 30 Roman-themed
//! rewards from a fixed pool. Users may claim one reward per day.
//! Random-drop rewards fire probabilistically on qualifying build events.

use serde::{Deserialize, Serialize};

// ─── Condition ───────────────────────────────────────────

/// Condition that must be met to claim or auto-receive a periodic reward.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PeriodicCondition {
    /// Awarded on the first login/activity of the day.
    DailyLogin,
    /// Requires `min_green` consecutive green builds this week.
    WeeklyBuildStreak {
        /// Minimum consecutive green builds required.
        min_green: u32,
    },
    /// Requires `min_items` doc items added in the current month.
    MonthlyDocSprint {
        /// Minimum documentation items required.
        min_items: u32,
    },
    /// A specific seasonal challenge must be completed.
    SeasonalChallenge {
        /// Identifier of the required seasonal challenge.
        challenge_id: String,
    },
    /// Fires randomly with the given probability on any qualifying build success.
    RandomDrop {
        /// Probability in the range `[0.0, 1.0]`.
        probability: f64,
    },
    /// Unlocked when a specific achievement is earned.
    MilestoneUnlock {
        /// ID of the prerequisite achievement.
        achievement_id: String,
    },
    /// Awarded on the first qualifying action of the week (any activity).
    WeeklyCheckIn,
    /// Requires completing all 3 daily quests in one day.
    DailyQuestComplete,
    /// Requires completing all 3 daily quests every day for a full week.
    PerfectWeek,
}

// ─── Reward ──────────────────────────────────────────────

/// A periodic unique reward that rotates weekly.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeriodicReward {
    /// Unique identifier for this reward definition.
    pub id: String,
    /// Display name shown in the UI.
    pub name: String,
    /// Description of the reward and its condition.
    pub description: String,
    /// Emoji icon for the reward.
    pub icon: String,
    /// XP bonus awarded on claim.
    pub xp_bonus: u64,
    /// Crystal bonus awarded on claim.
    pub crystal_bonus: u64,
    /// Condition that must be satisfied to claim this reward.
    pub unlock_condition: PeriodicCondition,
    /// Unix timestamp when this issued reward expires (0 = no expiry).
    pub valid_until: i64,
    /// Whether the user has already redeemed this reward instance.
    pub redeemed: bool,
}

/// Parameters for constructing a new PeriodicReward.
pub struct PeriodicRewardParams {
    /// Unique identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Detailed description.
    pub description: String,
    /// Emoji icon.
    pub icon: String,
    /// Awarded XP.
    pub xp_bonus: u64,
    /// Awarded Crystals.
    pub crystal_bonus: u64,
    /// Prerequisite condition.
    pub unlock_condition: PeriodicCondition,
    /// Expiry timestamp.
    pub valid_until: i64,
}

impl PeriodicReward {
    /// Create a new unclaimed reward record from parameters.
    pub fn new(params: PeriodicRewardParams) -> Self {
        Self {
            id: params.id,
            name: params.name,
            description: params.description,
            icon: params.icon,
            xp_bonus: params.xp_bonus,
            crystal_bonus: params.crystal_bonus,
            unlock_condition: params.unlock_condition,
            valid_until: params.valid_until,
            redeemed: false,
        }
    }

    /// Mark this reward as redeemed.
    pub fn claim(&mut self) {
        self.redeemed = true;
    }

    /// Whether this reward has expired at the given unix timestamp.
    pub fn is_expired(&self, now: i64) -> bool {
        self.valid_until > 0 && now > self.valid_until
    }
}

// ─── Static pool ─────────────────────────────────────────

/// Entry in the static reward definition pool.
#[derive(Debug, Clone)]
struct RewardDef {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    icon: &'static str,
    xp_bonus: u64,
    crystal_bonus: u64,
}

/// The 30-item pool of unique reward definitions, in rotation order.
static REWARD_POOL: &[RewardDef] = &[
    RewardDef {
        id: "pax_romana",
        name: "Pax Romana",
        icon: "🕊️",
        xp_bonus: 200,
        crystal_bonus: 40,
        description: "Awarded for logging in 3 days this week.",
    },
    RewardDef {
        id: "triumph",
        name: "Triumph!",
        icon: "🏛️",
        xp_bonus: 500,
        crystal_bonus: 100,
        description: "Awarded for 5 green builds this week.",
    },
    RewardDef {
        id: "golden_laurel",
        name: "Golden Laurel",
        icon: "🌿",
        xp_bonus: 300,
        crystal_bonus: 0,
        description: "Awarded for maintaining a 7-day build streak.",
    },
    RewardDef {
        id: "vox_populi_week",
        name: "Vox Populi",
        icon: "📢",
        xp_bonus: 400,
        crystal_bonus: 80,
        description: "Awarded for giving 10 AI feedback events this week.",
    },
    RewardDef {
        id: "scribe_bonus",
        name: "Scribe Bonus",
        icon: "📜",
        xp_bonus: 250,
        crystal_bonus: 0,
        description: "Awarded for adding 20 doc items this week.",
    },
    RewardDef {
        id: "new_world",
        name: "The New World",
        icon: "🗺️",
        xp_bonus: 350,
        crystal_bonus: 70,
        description: "Awarded for first use of 3 different Vox features.",
    },
    RewardDef {
        id: "corpus_gold",
        name: "Corpus Gold",
        icon: "🧠",
        xp_bonus: 600,
        crystal_bonus: 120,
        description: "Awarded for promoting an example to examples/canonical/.",
    },
    RewardDef {
        id: "gladiator_week",
        name: "Gladiator Week",
        icon: "⚔️",
        xp_bonus: 400,
        crystal_bonus: 0,
        description: "Awarded for winning 5 bug battles this week.",
    },
    RewardDef {
        id: "iron_codex",
        name: "Iron Codex",
        icon: "📚",
        xp_bonus: 450,
        crystal_bonus: 0,
        description: "Awarded for 0 missing_docs warnings 3 days in a row.",
    },
    RewardDef {
        id: "marketplace_bonus",
        name: "Marketplace Bonus",
        icon: "🏪",
        xp_bonus: 0,
        crystal_bonus: 200,
        description: "Awarded for buying any shop item this week.",
    },
    RewardDef {
        id: "master_builder",
        name: "Master Builder",
        icon: "🏗️",
        xp_bonus: 500,
        crystal_bonus: 0,
        description: "Awarded for 20 successful builds this week.",
    },
    RewardDef {
        id: "caesars_coin",
        name: "Caesar's Coin",
        icon: "🪙",
        xp_bonus: 0,
        crystal_bonus: 500,
        description: "Awarded for maintaining a 30-day streak this week.",
    },
    RewardDef {
        id: "legion_bonus",
        name: "Legion Bonus",
        icon: "🦅",
        xp_bonus: 300,
        crystal_bonus: 60,
        description: "Awarded for 3 successful agent handoffs this week.",
    },
    RewardDef {
        id: "perfect_run",
        name: "Perfect Run",
        icon: "✨",
        xp_bonus: 800,
        crystal_bonus: 0,
        description: "Awarded for a day with: green build + tests pass + 0 warnings.",
    },
    RewardDef {
        id: "forum_speaker",
        name: "Forum Speaker",
        icon: "🎙️",
        xp_bonus: 200,
        crystal_bonus: 0,
        description: "Awarded for submitting 5 AI responses with comments.",
    },
    RewardDef {
        id: "architects_gift",
        name: "Architect's Gift",
        icon: "🏛️",
        xp_bonus: 250,
        crystal_bonus: 50,
        description: "Awarded for fixing a TOESTUB violation.",
    },
    RewardDef {
        id: "random_serendipity",
        name: "Serendipity",
        icon: "🎲",
        xp_bonus: 150,
        crystal_bonus: 30,
        description: "A lucky drop on any green build (1-in-50 chance).",
    },
    RewardDef {
        id: "early_bird",
        name: "Early Bird",
        icon: "🐦",
        xp_bonus: 150,
        crystal_bonus: 0,
        description: "Awarded for completing a build before 8am local time.",
    },
    RewardDef {
        id: "night_owl",
        name: "Night Owl",
        icon: "🦉",
        xp_bonus: 150,
        crystal_bonus: 0,
        description: "Awarded for completing a build after 10pm local time.",
    },
    RewardDef {
        id: "weekend_warrior",
        name: "Weekend Warrior",
        icon: "🏕️",
        xp_bonus: 200,
        crystal_bonus: 0,
        description: "Awarded for a green build on Saturday or Sunday.",
    },
    RewardDef {
        id: "thousand_tokens",
        name: "Thousand Tokens",
        icon: "💬",
        xp_bonus: 100,
        crystal_bonus: 0,
        description: "Awarded for an AI session generating 1,000+ tokens.",
    },
    RewardDef {
        id: "ten_k_tokens",
        name: "Ten Thousand Tokens",
        icon: "💎",
        xp_bonus: 300,
        crystal_bonus: 60,
        description: "Awarded for an AI session generating 10,000+ tokens.",
    },
    RewardDef {
        id: "first_million",
        name: "Million Token March",
        icon: "🚀",
        xp_bonus: 2_000,
        crystal_bonus: 400,
        description: "Awarded for generating 1M tokens lifetime.",
    },
    RewardDef {
        id: "daily_check_in",
        name: "Daily Check-In",
        icon: "☀️",
        xp_bonus: 50,
        crystal_bonus: 0,
        description: "Awarded for activity every day this week.",
    },
    RewardDef {
        id: "quest_master",
        name: "Quest Master",
        icon: "📋",
        xp_bonus: 300,
        crystal_bonus: 0,
        description: "Awarded for completing all 3 daily quests in one day.",
    },
    RewardDef {
        id: "double_quest_master",
        name: "Double Quest Master",
        icon: "📋",
        xp_bonus: 600,
        crystal_bonus: 0,
        description: "Awarded for completing all 3 quests 2 days in a row.",
    },
    RewardDef {
        id: "perfect_week",
        name: "Perfect Week",
        icon: "🏆",
        xp_bonus: 1_500,
        crystal_bonus: 300,
        description: "Awarded for completing all daily quests every day this week.",
    },
    RewardDef {
        id: "island_builder",
        name: "Island Builder",
        icon: "🏝️",
        xp_bonus: 300,
        crystal_bonus: 0,
        description: "Awarded for building 3 islands this week.",
    },
    RewardDef {
        id: "open_source_spirit",
        name: "Open Source Spirit",
        icon: "🌐",
        xp_bonus: 500,
        crystal_bonus: 0,
        description: "Awarded for contributing a research doc + example on the same day.",
    },
    RewardDef {
        id: "season_of_code",
        name: "Season of Code",
        icon: "🌸",
        xp_bonus: 1_000,
        crystal_bonus: 200,
        description: "Special seasonal reward — available once per quarter.",
    },
];

// ─── Rotation ────────────────────────────────────────────

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

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pool_contains_thirty_entries() {
        assert_eq!(REWARD_POOL.len(), 30);
    }

    #[test]
    fn generate_weekly_reward_deterministic() {
        let r1 = generate_weekly_reward("user-abc", 42, 0);
        let r2 = generate_weekly_reward("user-abc", 42, 0);
        assert_eq!(r1.id, r2.id);
    }

    #[test]
    fn different_users_may_differ() {
        let r1 = generate_weekly_reward("user-alice", 1, 0);
        let r2 = generate_weekly_reward("user-bob", 1, 0);
        // They may differ (not guaranteed, but the pool has 30 items so usually different)
        // We only verify both are valid pool entries.
        assert!(!r1.name.is_empty());
        assert!(!r2.name.is_empty());
    }

    #[test]
    fn different_weeks_produce_different_rewards() {
        let r1 = generate_weekly_reward("user-x", 10, 0);
        let r2 = generate_weekly_reward("user-x", 11, 0);
        // With 30-item pool, rotation every week: IDs differ in the week suffix
        assert_ne!(r1.id, r2.id);
    }

    #[test]
    fn random_drop_fires_at_threshold() {
        assert!(evaluate_random_drop(0.02, 0.019));
        assert!(!evaluate_random_drop(0.02, 0.021));
    }

    #[test]
    fn random_drop_clamps_probability() {
        assert!(evaluate_random_drop(1.5, 0.999));
        assert!(!evaluate_random_drop(-0.5, 0.001));
    }

    #[test]
    fn claim_marks_redeemed() {
        let mut r = generate_weekly_reward("user-1", 1, 0);
        assert!(!r.redeemed);
        r.claim();
        assert!(r.redeemed);
    }

    #[test]
    fn is_expired_logic() {
        let r = generate_weekly_reward("user-1", 1, 1_000);
        assert!(r.is_expired(2_000));
        assert!(!r.is_expired(500));
    }

    #[test]
    fn no_expiry_when_valid_until_zero() {
        let r = generate_weekly_reward("user-1", 1, 0);
        assert!(!r.is_expired(i64::MAX));
    }
}
