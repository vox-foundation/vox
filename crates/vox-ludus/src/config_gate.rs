//! Gamification config gate — thin wrapper around VoxConfig.
//!
//! All gamify subsystems must check `is_enabled()` before performing any
//! side-effectful operations (DB writes, notifications, coaching hints).
//! In `Serious` mode the system remains active but silent (no UI overlays,
//! no hint nudges). When `enabled = false` the system is fully bypassed.

use vox_config::{GamifyMode, VoxConfig};

/// Cached config accessor. Loads VoxConfig once per call site.
/// For long-running processes, reload on user config change.
pub fn load() -> VoxConfig {
    VoxConfig::load()
}

/// Whether gamification is active at all.
pub fn is_enabled() -> bool {
    load().gamify_enabled
}

/// The current gamification mode.
pub fn mode() -> GamifyMode {
    load().gamify_mode
}

/// Reward multiplier from the active mode.
pub fn reward_multiplier() -> f64 {
    mode().reward_multiplier()
}

/// Whether coaching hints should be shown.
pub fn hints_enabled() -> bool {
    let m = mode();
    m.hint_frequency() > 0.0 && is_enabled()
}

/// Whether celebration overlays (level-up banners, quest complete) should show.
pub fn overlays_enabled() -> bool {
    is_enabled() && mode().show_overlays()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        // Default config must have gamify enabled in balanced mode.
        let cfg = VoxConfig::default();
        assert!(cfg.gamify_enabled);
        assert_eq!(cfg.gamify_mode, GamifyMode::Balanced);
        assert!((cfg.gamify_mode.reward_multiplier() - 1.0).abs() < 0.01);
    }

    #[test]
    fn serious_mode_suppresses_overlays() {
        let cfg = VoxConfig {
            gamify_mode: GamifyMode::Serious,
            ..Default::default()
        };
        assert!(!cfg.gamify_mode.show_overlays());
        assert!((cfg.gamify_mode.hint_frequency() - 0.0).abs() < 0.01);
    }

    #[test]
    fn learning_mode_amplifies_rewards() {
        let cfg = VoxConfig {
            gamify_mode: GamifyMode::Learning,
            ..Default::default()
        };
        assert!(cfg.gamify_mode.reward_multiplier() > 1.0);
        assert!((cfg.gamify_mode.hint_frequency() - 1.0).abs() < 0.01);
    }
}
