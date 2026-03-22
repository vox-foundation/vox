//! Bug battle system seeded from TOESTUB findings.

use crate::util::now_unix;
use serde::{Deserialize, Serialize};

// ─── Bug Type ────────────────────────────────────────────

/// Bug categories with associated reward tiers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BugType {
    /// Syntax or parsing error.
    Syntax,
    /// Logical or semantic error.
    Logic,
    /// Performance or efficiency issue.
    Performance,
    /// Security or safety vulnerability.
    Security,
}

impl BugType {
    /// Crystal reward for defeating this bug type.
    pub const fn crystal_reward(&self) -> u64 {
        match self {
            BugType::Syntax => 10,
            BugType::Logic => 25,
            BugType::Performance => 40,
            BugType::Security => 50,
        }
    }

    /// XP reward for defeating this bug type.
    pub const fn xp_reward(&self) -> u64 {
        match self {
            BugType::Syntax => 15,
            BugType::Logic => 30,
            BugType::Performance => 50,
            BugType::Security => 60,
        }
    }

    /// Emoji representation.
    pub const fn emoji(&self) -> &str {
        match self {
            BugType::Syntax => "🐛",
            BugType::Logic => "🧩",
            BugType::Performance => "🐢",
            BugType::Security => "🔒",
        }
    }

    /// Display name.
    pub const fn display(&self) -> &str {
        match self {
            BugType::Syntax => "Syntax Bug",
            BugType::Logic => "Logic Bug",
            BugType::Performance => "Performance Bug",
            BugType::Security => "Security Bug",
        }
    }

    /// Slug for DB storage.
    pub const fn as_str(&self) -> &str {
        match self {
            BugType::Syntax => "syntax",
            BugType::Logic => "logic",
            BugType::Performance => "performance",
            BugType::Security => "security",
        }
    }

    /// Base HP pool for this bug type.
    pub const fn base_hp(&self) -> i32 {
        match self {
            BugType::Syntax => 40,
            BugType::Logic => 70,
            BugType::Performance => 90,
            BugType::Security => 120,
        }
    }

    /// Base counter-attack damage per turn.
    pub const fn base_attack(&self) -> i32 {
        match self {
            BugType::Syntax => 5,
            BugType::Logic => 12,
            BugType::Performance => 18,
            BugType::Security => 20,
        }
    }

    /// Flavor text description.
    pub const fn description(&self) -> &'static str {
        match self {
            BugType::Syntax => "A malformed token lurks in the shadows of your code.",
            BugType::Logic => "Incorrect reasoning corrupts the execution path.",
            BugType::Performance => "Inefficient loops drain the system's vitality.",
            BugType::Security => "A malicious vector targets your application logic.",
        }
    }

    /// Map a TOESTUB rule_id prefix to a bug type.
    pub fn from_rule_id(rule_id: &str) -> Self {
        if rule_id.starts_with("stub/") || rule_id.starts_with("empty/") {
            BugType::Syntax
        } else if rule_id.starts_with("magic/") || rule_id.starts_with("dry/") {
            BugType::Logic
        } else if rule_id.starts_with("victory/")
            || rule_id.starts_with("unwired/")
            || rule_id.starts_with("clone/")
        {
            BugType::Performance
        } else if rule_id.starts_with("unresolved/")
            || rule_id.starts_with("ai/")
            || rule_id.starts_with("E04")
        {
            BugType::Security
        } else if rule_id.starts_with("E03") || rule_id.starts_with("vox-typeck") {
            BugType::Logic
        } else {
            BugType::Syntax // default
        }
    }
}

impl std::fmt::Display for BugType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display())
    }
}

/// Type effectiveness: Syntax > Performance > Security > Logic > Syntax.
/// Returns damage multiplier: 1.5 for super-effective, 0.5 for resisted, 1.0 for neutral.
pub fn type_effectiveness(attacker: BugType, defender: BugType) -> f64 {
    match (attacker, defender) {
        (BugType::Syntax, BugType::Performance) => 1.5,
        (BugType::Performance, BugType::Security) => 1.5,
        (BugType::Security, BugType::Logic) => 1.5,
        (BugType::Logic, BugType::Syntax) => 1.5,
        (BugType::Performance, BugType::Syntax) => 0.5,
        (BugType::Security, BugType::Performance) => 0.5,
        (BugType::Logic, BugType::Security) => 0.5,
        (BugType::Syntax, BugType::Logic) => 0.5,
        _ => 1.0,
    }
}

/// A bug enemy with HP, type resistances, and counter-attack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BugEnemy {
    /// Bug category determining rewards and effectiveness.
    pub bug_type: BugType,
    /// Display name for the enemy.
    pub name: String,
    /// Current remaining HP.
    pub hp: i32,
    /// Maximum HP pool.
    pub max_hp: i32,
    /// Base counter-attack damage per turn.
    pub attack_power: i32,
    /// Description of the bug.
    pub description: String,
    /// Source file where the bug was found.
    pub file_path: String,
}

impl BugEnemy {
    /// Create a bug enemy from a TOESTUB-style finding.
    pub fn from_finding(
        rule_id: &str,
        name: impl Into<String>,
        description: impl Into<String>,
        file_path: impl Into<String>,
    ) -> Self {
        let bug_type = BugType::from_rule_id(rule_id);
        let max_hp = bug_type.base_hp();
        let attack_power = bug_type.base_attack();
        Self {
            bug_type,
            name: name.into(),
            hp: max_hp,
            max_hp,
            attack_power,
            description: description.into(),
            file_path: file_path.into(),
        }
    }

    /// Apply damage, accounting for type effectiveness. Returns actual damage dealt.
    pub fn take_damage(&mut self, base_damage: i32, attacker_type: BugType) -> i32 {
        let mult = type_effectiveness(attacker_type, self.bug_type);
        let actual = (base_damage as f64 * mult).round() as i32;
        self.hp = (self.hp - actual).max(0);
        actual
    }

    /// Counter-attack damage dealt to the companion.
    pub fn counter_attack(&self) -> i32 {
        self.attack_power
    }

    /// Whether the bug is defeated.
    pub fn is_defeated(&self) -> bool {
        self.hp <= 0
    }
}

// ─── Battle ──────────────────────────────────────────────

/// A bug battle instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Battle {
    /// Unique battle identifier.
    pub id: String,
    /// Owning user ID.
    pub user_id: String,
    /// ID of the companion used in this battle.
    pub companion_id: String,
    /// Category of the bug fought.
    pub bug_type: BugType,
    /// Human-readable description of the bug.
    pub bug_description: String,
    /// Optional code context where the bug was found.
    pub bug_code: Option<String>,
    /// Code submitted as a fix, if any.
    pub submitted_code: Option<String>,
    /// Whether the user won the battle.
    pub success: bool,
    /// Crystals earned from this battle.
    pub crystals_earned: u64,
    /// XP earned from this battle.
    pub xp_earned: u64,
    /// Time taken to complete the battle in seconds.
    pub duration_secs: u64,
    /// Unix timestamp when the battle was created.
    pub created_at: i64,
}

impl Battle {
    /// Create a new battle from a TOESTUB-style finding.
    pub fn from_finding(
        id: impl Into<String>,
        user_id: impl Into<String>,
        companion_id: impl Into<String>,
        rule_id: &str,
        description: impl Into<String>,
        code_context: Option<String>,
    ) -> Self {
        let now = now_unix();
        let bug_type = BugType::from_rule_id(rule_id);
        Self {
            id: id.into(),
            user_id: user_id.into(),
            companion_id: companion_id.into(),
            bug_type,
            bug_description: description.into(),
            bug_code: code_context,
            submitted_code: None,
            success: false,
            crystals_earned: 0,
            xp_earned: 0,
            duration_secs: 0,
            created_at: now,
        }
    }

    /// Record a battle result.
    pub fn record_result(&mut self, success: bool, duration_secs: u64) {
        self.success = success;
        self.duration_secs = duration_secs;
        if success {
            self.crystals_earned = self.bug_type.crystal_reward();
            self.xp_earned = self.bug_type.xp_reward();
        } else {
            self.crystals_earned = 0;
            self.xp_earned = 0;
        }
    }

    /// Check if a battle has been completed (code has been submitted).
    pub fn is_completed(&self) -> bool {
        self.submitted_code.is_some()
    }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bug_type_rewards() {
        assert_eq!(BugType::Syntax.crystal_reward(), 10);
        assert_eq!(BugType::Security.crystal_reward(), 50);
        assert_eq!(BugType::Logic.xp_reward(), 30);
        assert_eq!(BugType::Performance.xp_reward(), 50);
    }

    #[test]
    fn bug_type_from_rule_id() {
        assert_eq!(BugType::from_rule_id("stub/todo"), BugType::Syntax);
        assert_eq!(BugType::from_rule_id("empty/body"), BugType::Syntax);
        assert_eq!(BugType::from_rule_id("magic/value"), BugType::Logic);
        assert_eq!(BugType::from_rule_id("dry/violation"), BugType::Logic);
        assert_eq!(BugType::from_rule_id("victory/claim"), BugType::Performance);
        assert_eq!(
            BugType::from_rule_id("unwired/module"),
            BugType::Performance
        );
        assert_eq!(BugType::from_rule_id("clone/abuse"), BugType::Performance);
        assert_eq!(BugType::from_rule_id("unresolved/ref"), BugType::Security);
        assert_eq!(BugType::from_rule_id("ai/dead-code"), BugType::Security);
        assert_eq!(BugType::from_rule_id("unknown/rule"), BugType::Syntax); // default
    }

    #[test]
    fn battle_from_finding() {
        let b = Battle::from_finding(
            "b-1",
            "u-1",
            "c-1",
            "stub/todo",
            "Found todo!() macro",
            Some("fn foo() { todo!() }".to_string()), // toestub-ignore(stub)
        );
        assert_eq!(b.bug_type, BugType::Syntax);
        assert!(!b.success);
        assert_eq!(b.crystals_earned, 0);
    }

    #[test]
    fn battle_record_success() {
        let mut b = Battle::from_finding("b-1", "u-1", "c-1", "magic/value", "Hardcoded 42", None);
        b.record_result(true, 30);
        assert!(b.success);
        assert_eq!(b.crystals_earned, 25); // Logic bug reward
        assert_eq!(b.xp_earned, 30);
        assert_eq!(b.duration_secs, 30);
    }

    #[test]
    fn battle_record_failure() {
        let mut b = Battle::from_finding(
            "b-1",
            "u-1",
            "c-1",
            "unresolved/ref",
            "Missing import",
            None,
        );
        b.record_result(false, 60);
        assert!(!b.success);
        assert_eq!(b.crystals_earned, 0);
        assert_eq!(b.xp_earned, 0);
    }

    #[test]
    fn type_effectiveness_super_effective() {
        assert!((type_effectiveness(BugType::Syntax, BugType::Performance) - 1.5).abs() < 0.01);
        assert!((type_effectiveness(BugType::Logic, BugType::Syntax) - 1.5).abs() < 0.01);
    }

    #[test]
    fn type_effectiveness_resisted() {
        assert!((type_effectiveness(BugType::Performance, BugType::Syntax) - 0.5).abs() < 0.01);
    }

    #[test]
    fn bug_enemy_take_damage() {
        let mut bug = BugEnemy::from_finding("stub/todo", "TestBug", "A bug", "src/main.rs");
        assert_eq!(bug.hp, BugType::Syntax.base_hp());
        let dmg = bug.take_damage(10, BugType::Logic); // Logic vs Syntax = 1.5x
        assert_eq!(dmg, 15);
        assert_eq!(bug.hp, bug.max_hp - 15);
    }

    #[test]
    fn bug_enemy_counter_attack() {
        // ai/dead-code maps to Security per bug_type_from_rule_id test
        let bug = BugEnemy::from_finding("ai/dead-code", "SecBug", "Security bug", "src/lib.rs");
        assert_eq!(bug.bug_type, BugType::Security);
        assert_eq!(bug.counter_attack(), 20);
    }

    #[test]
    fn battle_completion_check() {
        let mut b = Battle::from_finding("b-1", "u-1", "c-1", "stub/todo", "Test", None);
        assert!(!b.is_completed());
        b.submitted_code = Some("fn foo() { 42 }".to_string());
        assert!(b.is_completed());
    }
}
