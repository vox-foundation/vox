//! Crystal shop for unlocking abilities and consumables.
//!
//! Shop items are priced relative to the active GamifyMode multiplier
//! (Learning mode discounts, Serious mode neutral).

use serde::{Deserialize, Serialize};

use crate::ability::Ability;
use crate::profile::LudusProfile;

// ── Shop items ────────────────────────────────────────────

/// Available shop item categories.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ShopItem {
    /// Unlock a companion ability by id.
    AbilityUnlock {
        /// Identifier of the ability to unlock.
        ability_id: String,
        /// Display name of the ability.
        ability_name: String,
        /// Base crystal cost before mode adjustment.
        base_cost: u64,
    },
    /// Fully refill companion energy.
    EnergyRefill {
        /// Base crystal cost before mode adjustment.
        base_cost: u64,
    },
    /// Reroll the current daily quest set.
    QuestReroll {
        /// Base crystal cost before mode adjustment.
        base_cost: u64,
    },
    /// Unlock a new companion slot.
    CompanionSlot {
        /// 1-based index of the companion slot being unlocked.
        slot_index: u32,
        /// Base crystal cost before mode adjustment.
        base_cost: u64,
    },
    /// Protect the daily streak for 24 hours.
    StreakShield {
        /// Base crystal cost (high cost).
        base_cost: u64,
    },
}

impl ShopItem {
    /// Crystal cost adjusted by mode multiplier.
    ///
    /// Learning mode reduces cost by 30% to be more welcoming.
    /// Serious mode keeps base price.
    pub fn effective_cost(&self, mode_mult: f64) -> u64 {
        let base = match self {
            ShopItem::AbilityUnlock { base_cost, .. } => *base_cost,
            ShopItem::EnergyRefill { base_cost } => *base_cost,
            ShopItem::QuestReroll { base_cost } => *base_cost,
            ShopItem::CompanionSlot { base_cost, .. } => *base_cost,
            ShopItem::StreakShield { base_cost } => *base_cost,
        };
        // In Learning mode (mult 1.5), cost is 20% cheaper; Serious (0.5), same price.
        let discount = if mode_mult > 1.2 { 0.8 } else { 1.0 };
        ((base as f64) * discount).ceil() as u64
    }

    /// Human-readable name.
    pub fn name(&self) -> String {
        match self {
            ShopItem::AbilityUnlock { ability_name, .. } => {
                format!("Unlock: {}", ability_name)
            }
            ShopItem::EnergyRefill { .. } => "Energy Refill (Instauratio)".to_string(),
            ShopItem::QuestReroll { .. } => "Quest Reroll (Novum Agendum)".to_string(),
            ShopItem::CompanionSlot { slot_index, .. } => {
                format!("Companion Slot {}", slot_index)
            }
            ShopItem::StreakShield { .. } => "Streak Shield (Scutum)".to_string(),
        }
    }
}

// ── Purchase ──────────────────────────────────────────────

/// Result of a shop purchase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PurchaseResult {
    /// Whether the purchase was successful.
    pub success: bool,
    /// Human-readable outcome message.
    pub message: String,
    /// Crystals deducted; 0 if purchase failed.
    pub crystals_spent: u64,
    /// Crystal balance after the transaction.
    pub crystals_remaining: u64,
}

/// Attempt to purchase a shop item.
///
/// Mutates `profile` on success (deducts crystals).
/// Returns `Err` string if insufficient funds or item invalid.
pub fn purchase(
    profile: &mut LudusProfile,
    item: &ShopItem,
    mode_mult: f64,
    abilities: &mut [Ability],
) -> PurchaseResult {
    let cost = item.effective_cost(mode_mult);

    if !profile.spend_crystals(cost) {
        return PurchaseResult {
            success: false,
            message: format!(
                "Insufficient crystals. Need {}, have {}.",
                cost, profile.crystals
            ),
            crystals_spent: 0,
            crystals_remaining: profile.crystals,
        };
    }

    let msg = match item {
        ShopItem::AbilityUnlock {
            ability_id,
            ability_name,
            ..
        } => {
            if let Some(ability) = abilities.iter_mut().find(|a| &a.id == ability_id) {
                ability.unlocked = true;
                format!("'{}' unlocked. Ad victoriam!", ability_name)
            } else {
                // Refund if ability not found
                profile.add_crystals(cost);
                return PurchaseResult {
                    success: false,
                    message: format!("Ability '{}' not found.", ability_id),
                    crystals_spent: 0,
                    crystals_remaining: profile.crystals,
                };
            }
        }
        ShopItem::EnergyRefill { .. } => {
            profile.energy = profile.max_energy;
            "Energy fully restored. Instauratio complete!".to_string()
        }
        ShopItem::QuestReroll { .. } => {
            "Quest set will be rerolled on next refresh. Novum Agendum granted!".to_string()
        }
        ShopItem::CompanionSlot { slot_index, .. } => {
            format!("Companion slot {} unlocked.", slot_index)
        }
        ShopItem::StreakShield { .. } => {
            profile.earn_shield();
            "Streak Shield acquired. Your progress is guarded (Scutum paratum).".to_string()
        }
    };

    PurchaseResult {
        success: true,
        message: msg,
        crystals_spent: cost,
        crystals_remaining: profile.crystals,
    }
}

/// Build the default shop item list for display.
pub fn default_shop_items() -> Vec<ShopItem> {
    vec![
        ShopItem::EnergyRefill { base_cost: 30 },
        ShopItem::QuestReroll { base_cost: 50 },
        ShopItem::CompanionSlot {
            slot_index: 2,
            base_cost: 200,
        },
        ShopItem::CompanionSlot {
            slot_index: 3,
            base_cost: 400,
        },
        ShopItem::StreakShield { base_cost: 250 },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::profile::LudusProfile;

    #[test]
    fn energy_refill_purchase_succeeds() {
        let mut profile = LudusProfile::new_default("u");
        profile.energy = 10;
        let item = ShopItem::EnergyRefill { base_cost: 30 };
        let result = purchase(&mut profile, &item, 1.0, &mut []);
        assert!(result.success);
        assert_eq!(profile.energy, profile.max_energy);
        assert_eq!(profile.crystals, 70); // 100 - 30
    }

    #[test]
    fn insufficient_crystals_fails() {
        let mut profile = LudusProfile::new_default("u");
        profile.crystals = 5;
        let item = ShopItem::EnergyRefill { base_cost: 30 };
        let result = purchase(&mut profile, &item, 1.0, &mut []);
        assert!(!result.success);
        assert_eq!(profile.crystals, 5); // unchanged
    }

    #[test]
    fn learning_mode_discount() {
        let item = ShopItem::EnergyRefill { base_cost: 100 };
        let normal = item.effective_cost(1.0);
        let learning = item.effective_cost(1.5);
        assert!(learning < normal, "Learning mode should discount prices");
    }

    #[test]
    fn ability_unlock_purchase() {
        use crate::ability::{Archetype, default_abilities};
        let mut profile = LudusProfile::new_default("u");
        let mut abilities = default_abilities(Archetype::Centurion);
        let item = ShopItem::AbilityUnlock {
            ability_id: "pilum".to_string(),
            ability_name: "Pilum Throw".to_string(),
            base_cost: 50,
        };
        let result = purchase(&mut profile, &item, 1.0, &mut abilities);
        assert!(result.success);
        assert!(abilities.iter().find(|a| a.id == "pilum").unwrap().unlocked);
    }
}
