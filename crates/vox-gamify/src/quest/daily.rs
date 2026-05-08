use crate::util::now_unix;

use super::DAILY_QUEST_COUNT;
use super::instance::Quest;
use super::kind::QuestType;
use super::templates::{QUEST_TEMPLATES, QuestTemplate};

/// Returns the number of complete days since Unix epoch (UTC).
pub fn current_day_number() -> u64 {
    now_unix().max(0) as u64 / 86_400
}

/// Generate three daily quests for a user.
///
/// Uses `(user_id_hash × day_number)` as a deterministic seed, varied per
/// quest slot so each of the three quests draws a different template type.
pub fn generate_daily_quests(user_id: &str, day: u64) -> Vec<Quest> {
    let user_hash: u64 = user_id.bytes().enumerate().fold(0u64, |acc, (i, b)| {
        acc.wrapping_add((b as u64).wrapping_mul(i as u64 + 31))
    });

    let base_seed = user_hash.wrapping_mul(day.wrapping_add(1));

    // Spread across quest types to ensure variety each day
    let type_count = QuestType::ALL.len() as u64;

    (0..DAILY_QUEST_COUNT)
        .map(|slot| {
            let slot_seed = base_seed.wrapping_add(slot as u64 * 7919);
            // Select a quest type different for each slot
            let type_idx = ((slot_seed / type_count) ^ slot_seed) % type_count;
            let target_type = QuestType::ALL[type_idx as usize];

            // Filter templates by type, pick one via seed
            let candidates: Vec<&QuestTemplate> = QUEST_TEMPLATES
                .iter()
                .filter(|t| t.quest_type == target_type)
                .collect();

            // Fallback to any template if type has no candidates
            let template = if candidates.is_empty() {
                &QUEST_TEMPLATES[slot_seed as usize % QUEST_TEMPLATES.len()]
            } else {
                candidates[slot_seed as usize % candidates.len()]
            };

            let id = format!("quest-{user_id}-{day}-{slot}");
            Quest::from_template(id, user_id, template, slot_seed)
        })
        .collect()
}

/// Generate daily quests for today.
pub fn todays_quests(user_id: &str) -> Vec<Quest> {
    generate_daily_quests(user_id, current_day_number())
}
