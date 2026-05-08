use serde::{Deserialize, Serialize};

// ─── Quest Modifier ──────────────────────────────────────

/// A roguelite modifier that adjusts a quest's behaviour.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuestModifier {
    /// No special modifier — standard quest.
    None,
    /// +25% XP reward.
    Blessed,
    /// Must complete within 2 hours or the quest forfeits.
    Timed,
    /// On completion, automatically generates a harder follow-up quest.
    Chains,
    /// No hint text shown (experienced players only).
    Silent,
    /// 5× XP, but can appear at most once per user ever.
    Legendary,
    /// Requires a peer's in-game confirmation to complete.
    Collaborative,
    /// Finishing below 50% energy gives −10% XP.
    Cursed,
    /// A second instance of the same task at ×1.5 XP.
    Echoed,
    /// XP is doubled but deadline is halved (1 hour).
    Frenzy,
}

impl QuestModifier {
    /// XP multiplier applied on top of the base reward.
    pub fn xp_multiplier(self) -> f64 {
        match self {
            QuestModifier::Blessed => 1.25,
            QuestModifier::Legendary => 5.0,
            QuestModifier::Echoed => 1.5,
            QuestModifier::Frenzy => 2.0,
            QuestModifier::Cursed => 0.9, // Penalty applied separately when low energy
            _ => 1.0,
        }
    }

    /// Override quest duration in seconds (None = use template default).
    pub fn duration_override_secs(self) -> Option<i64> {
        match self {
            QuestModifier::Timed => Some(7_200),  // 2 hours
            QuestModifier::Frenzy => Some(3_600), // 1 hour
            _ => None,
        }
    }

    /// Roll a modifier from a seed using a weighted distribution.
    pub fn roll(seed: u64) -> Self {
        // Distribution: 60% None, 15% Blessed, 8% Timed, 6% Echoed,
        //               4% Chains, 3% Silent, 2% Frenzy, 1.5% Cursed,
        //               0.5% Collaborative, 0.1% Legendary
        let v = seed % 1000;
        match v {
            0..=599 => QuestModifier::None,
            600..=749 => QuestModifier::Blessed,
            750..=829 => QuestModifier::Timed,
            830..=889 => QuestModifier::Echoed,
            890..=929 => QuestModifier::Chains,
            930..=959 => QuestModifier::Silent,
            960..=979 => QuestModifier::Frenzy,
            980..=994 => QuestModifier::Cursed,
            995..=998 => QuestModifier::Collaborative,
            _ => QuestModifier::Legendary,
        }
    }

    /// Display name.
    pub fn name(self) -> &'static str {
        match self {
            QuestModifier::None => "",
            QuestModifier::Blessed => "Blessed",
            QuestModifier::Timed => "Timed",
            QuestModifier::Chains => "Chains",
            QuestModifier::Silent => "Silent",
            QuestModifier::Legendary => "Legendary",
            QuestModifier::Collaborative => "Collaborative",
            QuestModifier::Cursed => "Cursed",
            QuestModifier::Echoed => "Echoed",
            QuestModifier::Frenzy => "Frenzy",
        }
    }
}
