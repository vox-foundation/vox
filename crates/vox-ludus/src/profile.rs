//! Player profile with XP, leveling, energy, and crystals.
//!
//! ## Level Progression
//! Levels are infinite. The XP required to reach level `L` follows a smooth
//! quadratic curve: `Threshold(L) = 25L^2 + 25L - 50`.
//! This curve ensures a gradual increase in difficulty while remaining calculated
//! in O(1) time. Diminishing returns are built in as the gap between levels
//! increases linearly: `XP_for_level(L) = 50L + 50 (for L > 2)`.
//!
//! Players reach the Prestige threshold at level 1000.

use crate::streak::{StreakResult, StreakTracker};
use crate::util::now_unix;
use serde::{Deserialize, Serialize};

/// Community trust level based on verification and reputation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum TrustTier {
    /// Local-only play. No global synchronization or leaderboard access.
    Novice = 0,
    /// GitHub linked. Basic synchronization and community participation.
    Linked = 1,
    /// Verified consistent contributor with multiple successful builds.
    Proven = 2,
    /// Community veteran with high reputation and peer-vouched status.
    Master = 3,
}

impl Default for TrustTier {
    fn default() -> Self {
        Self::Novice
    }
}

impl TrustTier {
    /// Short label for CLI display.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Novice => "Novice",
            Self::Linked => "Linked",
            Self::Proven => "Proven",
            Self::Master => "Master",
        }
    }

    /// Color / emoji for the tier.
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Novice => "👤",
            Self::Linked => "🔗",
            Self::Proven => "🛡️",
            Self::Master => "👑",
        }
    }
}

// ─── Constants ───────────────────────────────────────────

/// Energy regenerated per regen tick.
const ENERGY_PER_REGEN: u64 = 1;

/// Seconds between energy regen ticks.
const REGEN_INTERVAL_SECS: u64 = 300; // 5 minutes

/// Starting crystals for a new profile.
const STARTING_CRYSTALS: u64 = 100;

/// Starting/base energy.
const BASE_ENERGY: u64 = 100;

/// Energy bonus per level (softcapped past L1000).
const ENERGY_PER_LEVEL: u64 = 5;

/// Prestige is triggered when the player reaches this level.
const PRESTIGE_THRESHOLD: u64 = 200;

// ─── XP Curve ────────────────────────────────────────────

/// Returns the XP required to advance *from* level `level-1` to `level`.
/// Each level takes exactly 50 XP more than the previous one.
pub fn xp_for_level(level: u64) -> u64 {
    if level <= 1 {
        0
    } else {
        100 + 50 * (level - 2)
    }
}

/// Returns the cumulative XP required to *reach* a given level.
/// sum_{i=2}^L (100 + 50(i-2)) = 25L^2 + 25L - 50 = 25(L^2 + L - 2).
pub fn xp_threshold_for_level(level: u64) -> u64 {
    if level <= 1 {
        return 0;
    }
    25 * (level * level + level - 2)
}

/// Computes the player level from raw XP using a closed-form inverse of the quadratic threshold.
/// L = (sqrt(9 + 0.16 * XP) - 1) / 2.
pub fn level_from_xp(xp: u64) -> u64 {
    let discriminant = 9.0 + (16.0 * xp as f64) / 100.0;
    let l_float = (discriminant.sqrt() - 1.0) / 2.0;
    (l_float + 1e-9).floor() as u64
}

// ─── Titles ──────────────────────────────────────────────

/// Roman rank title for a given level (infinite coverage).
pub fn title_for_level(level: u64) -> String {
    let title = match level {
        1..=3 => "Tiro",
        4..=7 => "Discipulus",
        8..=12 => "Librarius",
        13..=17 => "Scriba",
        18..=22 => "Milites",
        23..=27 => "Gregarius",
        28..=33 => "Duplicarius",
        34..=40 => "Decanus",
        41..=48 => "Tesserarius",
        49..=56 => "Optio",
        57..=65 => "Signifer",
        66..=74 => "Aquilifer",
        75..=85 => "Cornicen",
        86..=95 => "Imaginifer",
        96..=110 => "Centurion",
        111..=125 => "Princeps Prior",
        126..=140 => "Pilus Prior",
        141..=160 => "Tribunus Angusticlavius",
        161..=180 => "Tribunus Laticlavius",
        181..=205 => "Praefectus",
        206..=235 => "Legatus Legionis",
        236..=265 => "Praetor",
        266..=300 => "Propraetor",
        301..=340 => "Consul",
        341..=385 => "Proconsul",
        386..=430 => "Censor",
        431..=480 => "Dictator",
        481..=500 => "Princeps Senatus",
        501..=550 => "Augustus",
        551..=600 => "Caesar",
        601..=650 => "Dominus et Deus",
        651..=700 => "Pontifex Maximus",
        _ => return format!("Imperator L{}", level),
    };
    title.to_string()
}

/// Prestige rank title for a prestige level (0 = none, cycles every 12).
pub fn prestige_title(prestige: u32) -> &'static str {
    match prestige % 12 {
        0 => "",
        1 => "Vindex",        // Avenger
        2 => "Imperator",     // Supreme commander
        3 => "Divus",         // Deified
        4 => "Pater Patriae", // Father of the Fatherland
        5 => "Conservator",   // Protector
        6 => "Invictus",      // Unconquered
        7 => "Pacator",       // Bringer of peace
        8 => "Propagator",    // Extender of empire
        9 => "Maximus",       // The Greatest
        10 => "Sanctus",      // The Holy
        11 => "Aeternus",     // The Eternal
        _ => "Immortalis",    // Overflow
    }
}

/// Full decorated title: `"<PrestigeTitle> <RankTitle>"` or just `"<RankTitle>"`.
pub fn full_title(level: u64, prestige: u32) -> String {
    let rank = title_for_level(level);
    let p = prestige_title(prestige);
    if p.is_empty() {
        rank
    } else {
        format!("{p} {rank}")
    }
}

// ─── Profile ─────────────────────────────────────────────

/// A player's gamification profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LudusProfile {
    /// Unique user identifier.
    pub user_id: String,
    /// Current player level (1-based, infinite).
    pub level: u64,
    /// Current XP (resets to 10% bank on prestige).
    pub xp: u64,
    /// Lifetime XP ever earned (monotonically increasing).
    pub total_xp_earned: u64,
    /// Available crystal currency.
    pub crystals: u64,
    /// Current energy (0–max_energy).
    pub energy: u64,
    /// Maximum energy determined by level.
    pub max_energy: u64,
    /// Unix timestamp of the last energy regen tick.
    pub last_energy_regen: i64,
    /// Unix timestamp of the last recorded activity.
    pub last_active: i64,
    /// Daily streak tracker.
    #[serde(default)]
    pub streak: StreakTracker,
    /// Number of times the player has prestiged.
    #[serde(default)]
    pub prestige_level: u32,
    /// Lumens (unit of light) — social proof for quality work and pro-social actions.
    #[serde(default)]
    pub lumens: i64,
    /// Lumens earned by helping others (mentorship).
    #[serde(default)]
    pub generosity_lumens: i64,
    /// Number of streak shields available (protects against one day of inactivity).
    #[serde(default)]
    pub streak_shields: i32,
    /// Community trust tier (0=Novice, 1=Linked, 2=Proven, 3=Master).
    #[serde(default)]
    pub trust_tier: TrustTier,
    /// Flag indicating if rewards are currently suppressed due to a penalty.
    #[serde(default)]
    pub reward_suppressed: bool,
    /// Unix timestamp when the reward suppression expires.
    #[serde(default)]
    pub suppressed_until_ts: i64,
}

impl LudusProfile {
    /// Create a new default profile for a user.
    pub fn new_default(user_id: impl Into<String>) -> Self {
        let now = now_unix();
        Self {
            user_id: user_id.into(),
            level: 1,
            xp: 0,
            total_xp_earned: 0,
            crystals: STARTING_CRYSTALS,
            energy: BASE_ENERGY,
            max_energy: BASE_ENERGY,
            last_energy_regen: now,
            last_active: now,
            streak: StreakTracker::default(),
            prestige_level: 0,
            lumens: 0,
            generosity_lumens: 0,
            streak_shields: 0,
            trust_tier: TrustTier::Novice,
            reward_suppressed: false,
            suppressed_until_ts: 0,
        }
    }

    /// Record daily activity and return any streak result.
    pub fn record_daily_activity(&mut self) -> StreakResult {
        let result = self.streak.record_activity();
        match result {
            StreakResult::Continued { bonus_xp, .. }
            | StreakResult::SavedByGrace { bonus_xp, .. } => {
                self.add_xp(bonus_xp);
            }
            _ => {}
        }
        self.touch();
        result
    }

    /// Add XP, apply prestige bonus, and check for level-up / prestige.
    /// Returns `true` if at least one level-up occurred.
    pub fn add_xp(&mut self, amount: u64) -> bool {
        // Prestige XP bonus: +5% per prestige level, capped at +60%.
        let bonus_pct = (self.prestige_level as u64 * 5).min(60);
        let effective = amount + (amount * bonus_pct / 100);
        self.xp += effective;
        self.total_xp_earned += effective;

        let new_level = level_from_xp(self.xp);

        if new_level >= PRESTIGE_THRESHOLD && self.level < PRESTIGE_THRESHOLD {
            // Prestige: bank 10% of current XP, reset level
            self.prestige_level += 1;
            self.xp /= 10;
            self.level = level_from_xp(self.xp).max(1);
        } else if new_level > self.level {
            self.level = new_level;
        } else {
            return false;
        }

        // Energy scales with level, softcapped at PRESTIGE_THRESHOLD
        let eff = self.level.min(PRESTIGE_THRESHOLD);
        self.max_energy = BASE_ENERGY + eff * ENERGY_PER_LEVEL;
        self.energy = self.max_energy;
        true
    }

    /// Roman rank title for the current level.
    pub fn title(&self) -> String {
        title_for_level(self.level)
    }

    /// Prestige title string (empty if not yet prestiged).
    pub fn prestige_title_str(&self) -> &'static str {
        prestige_title(self.prestige_level)
    }

    /// Full decorated title combining prestige + rank.
    pub fn full_title(&self) -> String {
        full_title(self.level, self.prestige_level)
    }

    /// Add crystals.
    pub fn add_crystals(&mut self, amount: u64) {
        self.crystals += amount;
    }

    /// Spend crystals. Returns `false` if insufficient.
    pub fn spend_crystals(&mut self, amount: u64) -> bool {
        if self.crystals >= amount {
            self.crystals -= amount;
            true
        } else {
            false
        }
    }

    /// Spend energy. Returns `false` if insufficient.
    pub fn spend_energy(&mut self, amount: u64) -> bool {
        if self.energy >= amount {
            self.energy -= amount;
            true
        } else {
            false
        }
    }

    /// Regenerate energy based on elapsed time since last regen.
    pub fn regen_energy(&mut self) {
        let now = now_unix();
        let elapsed = (now - self.last_energy_regen).max(0) as u64;
        let ticks = elapsed / REGEN_INTERVAL_SECS;
        if ticks > 0 {
            let gained = ticks * ENERGY_PER_REGEN;
            self.energy = (self.energy + gained).min(self.max_energy);
            self.last_energy_regen = now;
        }
    }

    /// XP needed to reach the next level.
    pub fn xp_to_next_level(&self) -> u64 {
        xp_threshold_for_level(self.level + 1).saturating_sub(self.xp)
    }

    /// XP progress as a fraction within the current level (0.0–1.0).
    pub fn xp_progress(&self) -> f64 {
        let lo = xp_threshold_for_level(self.level);
        let hi = xp_threshold_for_level(self.level + 1);
        let span = hi.saturating_sub(lo);
        if span == 0 {
            return 1.0;
        }
        let within = self.xp.saturating_sub(lo);
        (within as f64 / span as f64).min(1.0)
    }

    /// Touch the last_active timestamp.
    pub fn touch(&mut self) {
        self.last_active = now_unix();
    }

    /// Add lumens to the profile (uncapped in struct; capped in DB-layer/policy).
    pub fn add_lumens(&mut self, delta: i64) {
        self.lumens += delta;
    }

    /// Add generosity lumens.
    pub fn add_generosity_lumens(&mut self, delta: i64) {
        self.generosity_lumens += delta;
    }

    /// Earn a new streak shield.
    pub fn earn_shield(&mut self) {
        self.streak_shields += 1;
    }

    /// Spend a streak shield. Returns true if successful.
    pub fn spend_shield(&mut self) -> bool {
        if self.streak_shields > 0 {
            self.streak_shields -= 1;
            true
        } else {
            false
        }
    }
}

// ─── Tests ───────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_profile_defaults() {
        let p = LudusProfile::new_default("user-1");
        assert_eq!(p.level, 1);
        assert_eq!(p.xp, 0);
        assert_eq!(p.total_xp_earned, 0);
        assert_eq!(p.crystals, 100);
        assert_eq!(p.energy, 100);
        assert_eq!(p.prestige_level, 0);
    }

    #[test]
    fn level_up_basic() {
        let mut p = LudusProfile::new_default("u1");
        assert!(!p.add_xp(50));
        assert_eq!(p.level, 1);
        assert!(p.add_xp(50)); // 100 total → L2
        assert_eq!(p.level, 2);
    }

    #[test]
    fn xp_threshold_round_trip() {
        assert_eq!(xp_threshold_for_level(1), 0);
        assert_eq!(xp_threshold_for_level(2), 100);
        assert_eq!(xp_threshold_for_level(3), 250);
        assert_eq!(xp_threshold_for_level(4), 450);
        assert_eq!(xp_threshold_for_level(10), 2700);
    }

    #[test]
    fn level_from_xp_round_trip() {
        for level in [1u64, 2, 3, 4, 10, 100, 1000] {
            let xp = xp_threshold_for_level(level);
            assert_eq!(level_from_xp(xp), level, "round trip failed at L{level}");
        }
    }

    #[test]
    fn title_coverage() {
        assert_eq!(title_for_level(1), "Tiro");
        assert_eq!(title_for_level(100), "Centurion");
        assert_eq!(title_for_level(500), "Princeps Senatus");
        assert_eq!(title_for_level(700), "Pontifex Maximus");
        assert_eq!(title_for_level(701), "Imperator L701");
        assert_eq!(title_for_level(99_999), "Imperator L99999");
    }

    #[test]
    fn prestige_titles_cycle() {
        assert_eq!(prestige_title(0), "");
        assert_eq!(prestige_title(1), "Vindex");
        assert_eq!(prestige_title(12), ""); // cycle
        assert_eq!(prestige_title(13), "Vindex");
    }

    #[test]
    fn full_title_combines() {
        assert_eq!(full_title(1, 0), "Tiro");
        assert_eq!(full_title(100, 1), "Vindex Centurion");
    }

    #[test]
    fn energy_scales_with_level() {
        let mut p = LudusProfile::new_default("u1");
        // Threshold(5) = 25 * (25 + 5 - 2) = 25 * 28 = 700
        p.add_xp(700);
        assert_eq!(p.level, 5);
        assert_eq!(p.max_energy, 100 + 5 * 5); // 125
    }

    #[test]
    fn prestige_bonus_applies() {
        let mut p = LudusProfile::new_default("u1");
        p.prestige_level = 2; // +10%
        let before = p.total_xp_earned;
        p.add_xp(100);
        assert_eq!(p.total_xp_earned - before, 110);
    }

    #[test]
    fn crystal_ops() {
        let mut p = LudusProfile::new_default("u1");
        assert!(p.spend_crystals(50));
        assert_eq!(p.crystals, 50);
        assert!(!p.spend_crystals(100));
    }

    #[test]
    fn energy_regen() {
        let mut p = LudusProfile::new_default("u1");
        p.energy = 50;
        p.last_energy_regen = now_unix() - 900; // 3 ticks
        p.regen_energy();
        assert_eq!(p.energy, 53);
    }

    #[test]
    fn xp_progress_fraction() {
        let mut p = LudusProfile::new_default("u1");
        p.xp = 50; // Lo=0, Hi=100
        assert!((p.xp_progress() - 0.5).abs() < 0.01);
        assert_eq!(p.xp_to_next_level(), 50);

        p.xp = 175; // Lo=100 (L2), Hi=250 (L3)
        p.level = 2;
        // span = 150. within = 75. fraction = 0.5
        assert!((p.xp_progress() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_xp_curve_tiers() {
        // Verify specifically the quadratic thresholds at Wave 2 boundaries
        // Threshold(L) = 25 * (L^2 + L - 2)
        assert_eq!(xp_threshold_for_level(10), 2700);
        assert_eq!(xp_threshold_for_level(25), 16200);
        assert_eq!(xp_threshold_for_level(50), 63700);
        assert_eq!(xp_threshold_for_level(100), 252450);
        assert_eq!(xp_threshold_for_level(200), 1004950);
    }

    #[test]
    fn test_lumens_and_shields() {
        let mut p = LudusProfile::new_default("tester");
        p.add_lumens(10);
        assert_eq!(p.lumens, 10);
        p.add_generosity_lumens(5);
        assert_eq!(p.generosity_lumens, 5);

        assert_eq!(p.streak_shields, 0);
        p.earn_shield();
        assert_eq!(p.streak_shields, 1);
        assert!(p.spend_shield());
        assert_eq!(p.streak_shields, 0);
        assert!(!p.spend_shield());
    }
}
