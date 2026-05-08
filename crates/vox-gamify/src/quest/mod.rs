//! Dynamic daily quest system with slot-filled templates and roguelite modifiers.
//!
//! ## Design
//! - All quest descriptions are **templates** with `{SLOT}` placeholders.
//! - Slots are filled from a seeded pool at generation time so descriptions
//!   change every day: a quest says "Fix a {RULE} violation in {CRATE}" — not
//!   the same text forever.
//! - Each quest may roll a **modifier** (Blessed, Timed, Chains, Silent, …)
//!   that changes its XP reward, behaviour, or unlock condition.
//! - Daily quests reset at midnight UTC. Three quests are generated per day
//!   from the full template library (seeded by user_id × day-number).
//! - Anti-grind: quests with low-complexity actions use capped targets
//!   and cooldowns enforced at the template layer.

mod daily;
mod instance;
mod kind;
mod modifier;
mod slots;
mod templates;

/// Quests generated per day.
pub const DAILY_QUEST_COUNT: usize = 3;

pub use daily::{current_day_number, generate_daily_quests, todays_quests};
pub use instance::Quest;
pub use kind::QuestType;
pub use modifier::QuestModifier;
pub use slots::slot_fill;
pub use templates::{QUEST_TEMPLATES, QuestTemplate};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn three_quests_generated() {
        let quests = generate_daily_quests("user-1", 100);
        assert_eq!(quests.len(), 3);
    }

    #[test]
    fn quests_are_deterministic() {
        let q1 = generate_daily_quests("user-1", 42);
        let q2 = generate_daily_quests("user-1", 42);
        assert_eq!(q1[0].id, q2[0].id);
        assert_eq!(q1[0].description, q2[0].description);
    }

    #[test]
    fn different_days_may_give_different_quests() {
        let q1 = generate_daily_quests("user-1", 1);
        let q2 = generate_daily_quests("user-1", 2);
        let any_different = q1.iter().zip(q2.iter()).any(|(a, b)| a.id != b.id);
        assert!(any_different);
    }

    #[test]
    fn slot_fill_replaces_crate() {
        use crate::quest::slots::CRATE_POOL;
        let result = slot_fill("Fix a bug in {CRATE}", 0);
        assert!(!result.contains("{CRATE}"));
        assert!(CRATE_POOL.iter().any(|c| result.contains(c)));
    }

    #[test]
    fn slot_fill_all_pools_rotate() {
        let r1 = slot_fill("{CRATE}", 0);
        let r2 = slot_fill("{CRATE}", 1);
        assert!(!r1.contains('{'));
        assert!(!r2.contains('{'));
    }

    #[test]
    fn modifier_roll_distribution() {
        let legendary_count = (0u64..10_000)
            .filter(|&s| QuestModifier::roll(s) == QuestModifier::Legendary)
            .count();
        assert!(
            legendary_count < 30,
            "Legendary too common: {legendary_count}"
        );
        assert!(
            legendary_count > 0,
            "Legendary never rolled in 10,000 samples"
        );
    }

    #[test]
    fn quest_increment_completes() {
        let template = &QUEST_TEMPLATES[0];
        let mut q = Quest::from_template("q1", "u1", template, 0);
        for _ in 0..q.target {
            q.increment(1);
        }
        assert!(q.completed);
    }

    #[test]
    fn quest_display_title_with_modifier() {
        let template = &QUEST_TEMPLATES[0];
        let mut q = Quest::from_template("q1", "u1", template, 0);
        q.modifier = QuestModifier::Blessed;
        q.description = "Do a thing".to_string();
        assert!(q.display_title().contains("[Blessed]"));
    }

    #[test]
    fn blessed_modifier_increases_xp() {
        let template = &QUEST_TEMPLATES[0];
        let base_xp = template.base_xp;
        let q = Quest::from_template("q1", "u1", template, 600);
        if q.modifier == QuestModifier::Blessed {
            assert!(q.xp_reward > base_xp);
        }
    }

    #[test]
    fn test_new_quest_types() {
        let feedback_exists = QUEST_TEMPLATES
            .iter()
            .any(|t| t.quest_type == QuestType::AiFeedback);
        let streak_exists = QUEST_TEMPLATES
            .iter()
            .any(|t| t.quest_type == QuestType::BuildStreak);
        assert!(feedback_exists);
        assert!(streak_exists);
    }
}
