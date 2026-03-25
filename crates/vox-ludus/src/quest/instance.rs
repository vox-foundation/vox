use crate::util::now_unix;
use serde::{Deserialize, Serialize};

use super::kind::QuestType;
use super::modifier::QuestModifier;
use super::templates::QuestTemplate;

const QUEST_DURATION_SECS: i64 = 86_400;
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Quest {
    /// Unique quest instance ID.
    pub id: String,
    /// User this quest belongs to.
    pub user_id: String,
    /// Category of the quest.
    pub quest_type: QuestType,
    /// Resolved description (slots filled in).
    pub description: String,
    /// Resolved hint (slots filled in; may be empty for Silent quests).
    pub hint: String,
    /// Completion target.
    pub target: u32,
    /// Current progress.
    pub progress: u32,
    /// XP reward after modifier applied.
    pub xp_reward: u64,
    /// Crystal reward after modifier applied.
    pub crystal_reward: u64,
    /// Roguelite modifier on this quest.
    pub modifier: QuestModifier,
    /// Whether the quest has been fully completed.
    pub completed: bool,
    /// Quest status for DB.
    pub status: String,
    /// Unix timestamp when this quest expires.
    pub expires_at: i64,
}

impl Quest {
    /// Generate a quest from a template, applying slot-fill and modifier.
    pub fn from_template(
        id: impl Into<String>,
        user_id: impl Into<String>,
        template: &QuestTemplate,
        seed: u64,
    ) -> Self {
        let modifier = QuestModifier::roll(seed.wrapping_mul(0xDEAD_BEEF));
        let xp_reward = (template.base_xp as f64 * modifier.xp_multiplier()).round() as u64;
        let crystal_reward =
            (template.base_crystals as f64 * modifier.xp_multiplier()).round() as u64;

        let duration = modifier
            .duration_override_secs()
            .unwrap_or(QUEST_DURATION_SECS);
        let now = now_unix();

        let hint = if modifier == QuestModifier::Silent {
            String::new()
        } else {
            template.hint(seed)
        };

        Self {
            id: id.into(),
            user_id: user_id.into(),
            quest_type: template.quest_type,
            description: template.description(seed),
            hint,
            target: template.target,
            progress: 0,
            xp_reward,
            crystal_reward,
            modifier,
            completed: false,
            status: "active".to_string(),
            expires_at: now + duration,
        }
    }

    /// Increment progress. Returns `true` if the quest just completed.
    pub fn increment(&mut self, amount: u32) -> bool {
        if self.completed {
            return false;
        }
        self.progress = (self.progress + amount).min(self.target);
        if self.progress >= self.target {
            self.completed = true;
            self.status = "completed".to_string();
            true
        } else {
            false
        }
    }

    /// Whether this quest has expired.
    pub fn is_expired(&self) -> bool {
        now_unix() > self.expires_at
    }

    /// Progress as a fraction (0.0–1.0).
    pub fn progress_pct(&self) -> f64 {
        if self.target == 0 {
            return 1.0;
        }
        self.progress as f64 / self.target as f64
    }

    /// Display label combining modifier prefix and description.
    pub fn display_title(&self) -> String {
        let m = self.modifier.name();
        if m.is_empty() {
            self.description.clone()
        } else {
            format!("[{m}] {}", self.description)
        }
    }
}
