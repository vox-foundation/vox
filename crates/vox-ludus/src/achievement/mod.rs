//! Achievement system for gamifying agent activities.
//!
//! Tracks milestones like first task completion, first handoff,
//! error-free streaks, and cost efficiency. Achievements are
//! persisted and shown on the dashboard.

mod defaults;
mod thresholds;
mod tracker;
mod types;

pub use tracker::AchievementTracker;
pub use types::{Achievement, AchievementCategory, AchievementId, UnlockedAchievement};
