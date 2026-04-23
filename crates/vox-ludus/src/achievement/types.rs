//! Core achievement data types.

use serde::{Deserialize, Serialize};

/// Unique achievement identifier.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AchievementId(pub String);

impl std::fmt::Display for AchievementId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// An achievement that can be unlocked by agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    /// Unique identifier.
    pub id: AchievementId,
    /// Display name.
    pub name: String,
    /// Description of how to unlock.
    pub description: String,
    /// Emoji icon.
    pub icon: String,
    /// Category of achievement.
    pub category: AchievementCategory,
    /// XP reward for unlocking.
    pub xp_reward: u32,
    /// Crystal reward for unlocking.
    pub crystal_reward: u32,
    /// Sluggified icon name for high-fidelity UIs.
    pub icon_slug: String,
    /// Hidden until unlocked.
    pub secret: bool,
}

/// Achievement categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AchievementCategory {
    /// Task-related milestones.
    Tasks,
    /// Collaboration milestones.
    Collaboration,
    /// Efficiency milestones.
    Efficiency,
    /// Exploration milestones.
    Discovery,
    /// Streak milestones.
    Streaks,
    /// AI corpus and feedback milestones.
    AiCorpus,
    /// Build, test, and code quality milestones.
    BuildMastery,
    /// Vox language feature milestones.
    LangExplorer,
    /// Security and safety milestones.
    Security,
    /// Documentation milestones.
    Documentation,
    /// Human-in-the-loop and social agent milestones.
    Social,
    /// Workflow and process milestones.
    Workflow,
}

/// Record of an unlocked achievement.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockedAchievement {
    /// The identifier of the achievement that was unlocked.
    pub achievement_id: AchievementId,
    /// Identifier of the agent that earned the achievement.
    pub agent_id: String,
    /// Unix timestamp when the achievement was unlocked.
    pub unlocked_at: u64,
}
