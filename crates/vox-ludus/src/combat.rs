//! Bug battle combat engine.
//!
//! Manages the turn-based state machine for a single bug battle.
//! Reward values are driven by the reward policy engine.

use serde::{Deserialize, Serialize};

use crate::ability::Ability;
use crate::battle::{Battle, BugType};
use crate::reward_policy::{SessionState, apply_policy, base_reward};

// ── Bug enemy ─────────────────────────────────────────────

/// An enemy bug encountered in a battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugEnemy {
    /// Display name of the bug.
    pub name: String,
    /// Category of the bug.
    pub bug_type: BugType,
    /// Human-readable description of the bug.
    pub description: String,
    /// Maximum HP pool.
    pub max_hp: i32,
    /// Current remaining HP.
    pub hp: i32,
    /// Base attack damage per turn.
    pub attack: i32,
    /// Optional file path where the bug was found.
    pub file_path: Option<String>,
}

impl BugEnemy {
    /// Create a bug enemy with stats derived from the given [`BugType`].
    pub fn new(bug_type: BugType, description: impl Into<String>) -> Self {
        let (max_hp, attack) = bug_stats(bug_type);
        Self {
            name: bug_type.display().to_string(),
            bug_type,
            description: description.into(),
            max_hp,
            hp: max_hp,
            attack,
            file_path: None,
        }
    }

    /// Is the bug defeated?
    pub fn is_defeated(&self) -> bool {
        self.hp <= 0
    }

    /// Apply player damage to this bug.
    pub fn take_damage(&mut self, amount: i32) {
        self.hp = (self.hp - amount).max(0);
    }

    /// Bug attacks the companion. Returns damage dealt.
    pub fn counter_attack(&self) -> i32 {
        self.attack
    }
}

fn bug_stats(bug_type: BugType) -> (i32, i32) {
    match bug_type {
        BugType::Syntax => (40, 5),
        BugType::Logic => (70, 12),
        BugType::Performance => (90, 18),
        BugType::Security => (120, 20),
    }
}

/// Type effectiveness multiplier (attack -> defense).
pub fn type_effectiveness(attacker: BugType, defender: BugType) -> f64 {
    match (attacker, defender) {
        (BugType::Syntax, BugType::Performance) => 2.0,
        (BugType::Performance, BugType::Security) => 2.0,
        (BugType::Security, BugType::Logic) => 2.0,
        (BugType::Logic, BugType::Syntax) => 2.0,
        _ => 1.0,
    }
}

// ── Combat state ──────────────────────────────────────────

/// Possible combat outcomes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CombatResult {
    /// Battle is still in progress.
    Ongoing,
    /// Companion defeated the bug.
    Victory,
    /// Companion's HP reached zero.
    Defeat,
    /// User chose to flee; treated as defeat for scoring.
    Fled,
}

/// Turn-by-turn battle state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatState {
    /// ID of the associated battle record.
    pub battle_id: String,
    /// The bug enemy being fought.
    pub bug: BugEnemy,
    /// Companion current HP.
    pub companion_hp: i32,
    /// Companion maximum HP.
    pub companion_max_hp: i32,
    /// Companion current energy.
    pub companion_energy: u32,
    /// Companion maximum energy.
    pub companion_max_energy: u32,
    /// Available abilities (may be pre-scaled by mode).
    pub abilities: Vec<Ability>,
    /// Per-ability cooldown counters (indexed by ability id).
    pub cooldowns: std::collections::HashMap<String, u32>,
    /// Turn number.
    pub turn: u32,
    /// Result if combat is over.
    pub result: CombatResult,
}

impl CombatState {
    /// Initialize a new combat.
    pub fn new(
        battle_id: impl Into<String>,
        bug: BugEnemy,
        companion_hp: i32,
        companion_max_hp: i32,
        companion_energy: u32,
        companion_max_energy: u32,
        abilities: Vec<Ability>,
    ) -> Self {
        Self {
            battle_id: battle_id.into(),
            bug,
            companion_hp,
            companion_max_hp,
            companion_energy,
            companion_max_energy,
            abilities,
            cooldowns: std::collections::HashMap::new(),
            turn: 0,
            result: CombatResult::Ongoing,
        }
    }

    /// Use an ability by id. Returns error message if invalid.
    pub fn use_ability(&mut self, ability_id: &str) -> Result<String, String> {
        if self.result != CombatResult::Ongoing {
            return Err("Combat is already over.".to_string());
        }

        let ability = self
            .abilities
            .iter()
            .find(|a| a.id == ability_id && a.unlocked)
            .cloned()
            .ok_or_else(|| format!("Ability '{}' not found or not unlocked.", ability_id))?;

        // Cooldown check
        if let Some(&cd) = self.cooldowns.get(&ability.id)
            && cd > 0
        {
            return Err(format!(
                "Ability '{}' is on cooldown ({} turns).",
                ability.id, cd
            ));
        }

        // Energy check
        if self.companion_energy < ability.energy_cost {
            return Err(format!(
                "Not enough energy ({} needed, {} available).",
                ability.energy_cost, self.companion_energy
            ));
        }

        // Apply ability
        self.companion_energy -= ability.energy_cost;
        let mut log = Vec::new();

        if ability.damage > 0 {
            // Offensive: deal damage with type effectiveness
            let mult = ability
                .effective_against
                .map(|t| type_effectiveness(t, self.bug.bug_type))
                .unwrap_or(1.0);
            let actual = (ability.damage as f64 * mult).round() as i32;
            self.bug.take_damage(actual);
            log.push(format!(
                "{} dealt {} damage to {}.",
                ability.name, actual, self.bug.name
            ));
        } else {
            // Heal
            let heal = (-ability.damage).max(0);
            self.companion_hp = (self.companion_hp + heal).min(self.companion_max_hp);
            log.push(format!("{} healed {} HP.", ability.name, heal));
        }

        // Set cooldown
        if ability.cooldown > 0 {
            self.cooldowns.insert(ability.id.clone(), ability.cooldown);
        }

        // Check win
        if self.bug.is_defeated() {
            self.result = CombatResult::Victory;
            log.push(format!("{} has been defeated!", self.bug.name));
        } else {
            // Bug counter-attacks
            let dmg = self.bug.counter_attack();
            self.companion_hp -= dmg;
            log.push(format!(
                "{} counter-attacked for {} damage!",
                self.bug.name, dmg
            ));
            if self.companion_hp <= 0 {
                self.companion_hp = 0;
                self.result = CombatResult::Defeat;
                log.push("Your companion has been defeated.".to_string());
            }
        }

        // Advance turn and tick cooldowns (at start of next use)
        self.turn += 1;
        for cd in self.cooldowns.values_mut() {
            *cd = cd.saturating_sub(1);
        }

        Ok(log.join("\n"))
    }

    /// Submit a code fix to instantly resolve the bug.
    /// Returns victory if fix is non-empty, defeat otherwise.
    pub fn submit_fix(&mut self, fix: &str) -> CombatResult {
        if fix.trim().is_empty() {
            self.result = CombatResult::Defeat;
        } else {
            self.result = CombatResult::Victory;
            self.bug.hp = 0;
        }
        self.result.clone()
    }

    /// Flee from combat — always results in defeat (but no companion death penalty).
    pub fn flee(&mut self) -> CombatResult {
        self.result = CombatResult::Fled;
        CombatResult::Fled
    }

    /// Calculate policy-driven rewards for this combat, if victorious.
    pub fn rewards(
        &self,
        mode_mult: f64,
        streak_days: u32,
        session: &mut SessionState,
    ) -> (u64, u64) {
        if self.result != CombatResult::Victory {
            return (0, 0);
        }
        let event = match self.bug.bug_type {
            BugType::Syntax => "bug_fix",
            BugType::Logic => "bug_fix",
            BugType::Performance => "bug_fix",
            BugType::Security => "bug_fix",
        };
        let base = base_reward(event);
        // Apply bug-type bonus on top of base
        let type_bonus_xp = self.bug.bug_type.xp_reward();
        let type_bonus_crystals = self.bug.bug_type.crystal_reward();
        let r = apply_policy(&base, mode_mult, streak_days, event, session);
        (r.xp + type_bonus_xp, r.crystals + type_bonus_crystals)
    }

    /// Convert to a `Battle` record for persistence.
    pub fn to_battle_record(
        &self,
        user_id: &str,
        companion_id: &str,
        xp_earned: u64,
        crystals_earned: u64,
        duration_secs: u64,
        submitted_code: Option<String>,
    ) -> Battle {
        Battle {
            id: self.battle_id.clone(),
            user_id: user_id.to_string(),
            companion_id: companion_id.to_string(),
            bug_type: self.bug.bug_type,
            bug_description: self.bug.description.clone(),
            bug_code: self.bug.file_path.clone(),
            submitted_code,
            success: self.result == CombatResult::Victory,
            crystals_earned,
            xp_earned,
            duration_secs,
            created_at: crate::util::now_unix(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ability::{Archetype, default_abilities};

    fn make_combat() -> CombatState {
        let bug = BugEnemy::new(BugType::Syntax, "A syntax error");
        let abilities = default_abilities(Archetype::Centurion);
        CombatState::new("test-battle", bug, 100, 100, 100, 100, abilities)
    }

    #[test]
    fn use_ability_deals_damage() {
        let mut state = make_combat();
        let result = state.use_ability("gladius");
        assert!(result.is_ok(), "gladius should succeed: {:?}", result);
    }

    #[test]
    fn flee_ends_combat() {
        let mut state = make_combat();
        let r = state.flee();
        assert_eq!(r, CombatResult::Fled);
        assert_eq!(state.result, CombatResult::Fled);
    }

    #[test]
    fn submit_empty_fix_is_defeat() {
        let mut state = make_combat();
        let r = state.submit_fix("");
        assert_eq!(r, CombatResult::Defeat);
    }

    #[test]
    fn submit_valid_fix_is_victory() {
        let mut state = make_combat();
        let r = state.submit_fix("fn fixed() {}");
        assert_eq!(r, CombatResult::Victory);
    }

    #[test]
    fn rewards_zero_on_defeat() {
        let mut state = make_combat();
        state.flee();
        let mut session = SessionState::default();
        let (xp, crystals) = state.rewards(1.0, 0, &mut session);
        assert_eq!(xp, 0);
        assert_eq!(crystals, 0);
    }

    #[test]
    fn type_effectiveness_double() {
        assert!((type_effectiveness(BugType::Syntax, BugType::Performance) - 2.0).abs() < 0.01);
    }
}
