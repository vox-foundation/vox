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
