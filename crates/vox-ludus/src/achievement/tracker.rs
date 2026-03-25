//! Runtime achievement tracking (counters, unlocks).

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use super::defaults;
use super::thresholds;
use super::types::{Achievement, AchievementId, UnlockedAchievement};

/// Tracks achievements per agent.
#[derive(Debug, Default)]
pub struct AchievementTracker {
    /// All available achievements.
    definitions: Vec<Achievement>,
    /// Per-agent unlocked achievements.
    unlocked: HashMap<String, Vec<UnlockedAchievement>>,
    /// Per-agent counters for tracking progress.
    counters: HashMap<String, HashMap<String, u32>>,
}

impl AchievementTracker {
    /// Create a new tracker with the default achievement set.
    pub fn new() -> Self {
        let mut tracker = Self::default();
        tracker.register_defaults();
        tracker
    }

    /// Register the default set of achievements.
    pub fn register_defaults(&mut self) {
        self.definitions = defaults::all();
    }

    /// Increment a counter for an agent and check for unlocks.
    pub fn increment_counter(&mut self, agent_id: &str, counter: &str) -> Vec<Achievement> {
        let count = {
            let c = self
                .counters
                .entry(agent_id.to_string())
                .or_default()
                .entry(counter.to_string())
                .or_insert(0);
            *c += 1;
            *c
        };

        self.check_unlocks(agent_id, counter, count)
    }

    /// Check if any achievements should unlock based on a counter value.
    pub fn check_unlocks(&mut self, agent_id: &str, counter: &str, value: u32) -> Vec<Achievement> {
        let mut unlocked = Vec::new();

        let threshold_list = thresholds::for_counter(counter);

        for (achievement_id, threshold) in threshold_list {
            if value >= threshold
                && !self.has_achievement(agent_id, achievement_id)
                && let Some(achievement) =
                    self.definitions.iter().find(|a| a.id.0 == achievement_id)
            {
                let record = UnlockedAchievement {
                    achievement_id: AchievementId(achievement_id.to_string()),
                    agent_id: agent_id.to_string(),
                    unlocked_at: SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64,
                };
                self.unlocked
                    .entry(agent_id.to_string())
                    .or_default()
                    .push(record);
                unlocked.push(achievement.clone());
            }
        }

        unlocked
    }

    /// Check if an agent has a specific achievement.
    pub fn has_achievement(&self, agent_id: &str, achievement_id: &str) -> bool {
        self.unlocked
            .get(agent_id)
            .map(|list| list.iter().any(|a| a.achievement_id.0 == achievement_id))
            .unwrap_or(false)
    }

    /// Get all unlocked achievements for an agent.
    pub fn agent_achievements(&self, agent_id: &str) -> Vec<&Achievement> {
        let unlocked_ids: Vec<&str> = self
            .unlocked
            .get(agent_id)
            .map(|list| list.iter().map(|a| a.achievement_id.0.as_str()).collect())
            .unwrap_or_default();

        self.definitions
            .iter()
            .filter(|a| unlocked_ids.contains(&a.id.0.as_str()))
            .collect()
    }

    /// Get all available achievements.
    pub fn all_achievements(&self) -> &[Achievement] {
        &self.definitions
    }

    /// Get the counter value for an agent.
    pub fn counter_value(&self, agent_id: &str, counter: &str) -> u32 {
        self.counters
            .get(agent_id)
            .and_then(|c| c.get(counter))
            .copied()
            .unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_task_achievement() {
        let mut tracker = AchievementTracker::new();
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");

        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id.0, "first_task");
        assert!(tracker.has_achievement("agent-1", "first_task"));
    }

    #[test]
    fn no_duplicate_unlock() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("agent-1", "tasks_completed");
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");
        // Second increment should not re-unlock
        assert!(unlocked.is_empty());
    }

    #[test]
    fn multiple_achievements_at_threshold() {
        let mut tracker = AchievementTracker::new();
        for _ in 0..4 {
            tracker.increment_counter("agent-1", "tasks_completed");
        }
        let unlocked = tracker.increment_counter("agent-1", "tasks_completed");
        // At 5 tasks: "five_tasks" unlocks
        assert_eq!(unlocked.len(), 1);
        assert_eq!(unlocked[0].id.0, "five_tasks");

        assert_eq!(tracker.agent_achievements("agent-1").len(), 2);
    }

    #[test]
    fn counter_tracking() {
        let mut tracker = AchievementTracker::new();
        tracker.increment_counter("agent-1", "tasks_completed");
        assert_eq!(tracker.counter_value("agent-1", "tasks_completed"), 1);
    }
}
