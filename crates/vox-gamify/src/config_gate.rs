//! Gamification config gate — thin wrapper around VoxConfig.
//!
//! All gamify subsystems must check `is_enabled()` before performing any
//! side-effectful operations (DB writes, notifications, coaching hints).
//! In `Serious` mode the system remains active but silent (no UI overlays,
//! no hint nudges). When `enabled = false` the system is fully bypassed.
//!
//! ## Overrides (non-persistent)
//!
//! - **`VOX_LUDUS_EMERGENCY_OFF=1`**: hard-off all Ludus side effects (rollout kill-switch).
//! - **`VOX_LUDUS_SESSION_ENABLED`**: `true` / `false` — session-only enable toggle.
//! - **`VOX_LUDUS_SESSION_MODE`**: `balanced` | `serious` | `learning` | `off` (off disables for session).
//! - **`VOX_LUDUS_MCP_TOOL_ARGS`**: `full` (default) \| `hash` \| `omit` — canonical table in **`docs/src/reference/env-vars.md`**
//!   (behavior: [`mcp_tool_args_storage`], consumer: [`crate::mcp_privacy::prepare_mcp_tool_args_for_storage`]).

use vox_config::{GamifyMode, VoxConfig};

/// UX channel for Ludus output: inline celebrations vs digest-first behavior.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LudusChannel {
    /// Ludus disabled or kill-switch active.
    Off,
    /// Match [`GamifyMode::Serious`] — minimal surface.
    Serious,
    /// Default interactive channel.
    Balanced,
    /// Prefer weekly/session digests; suppress unsolicited CLI celebration lines.
    DigestPriority,
}

/// Effective Ludus UX channel (`VOX_LUDUS_CHANNEL` overrides, else derived from mode).
pub fn ludus_channel() -> LudusChannel {
    if let Some(raw) = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusChannel).expose() {
        match raw.to_lowercase().as_str() {
            "off" => return LudusChannel::Off,
            "serious" => return LudusChannel::Serious,
            "balanced" => return LudusChannel::Balanced,
            "digest-priority" | "digest_priority" | "digest" => {
                return LudusChannel::DigestPriority;
            }
            _ => {}
        }
    }
    if !is_enabled() {
        return LudusChannel::Off;
    }
    match mode() {
        GamifyMode::Serious => LudusChannel::Serious,
        GamifyMode::Balanced | GamifyMode::Learning => LudusChannel::Balanced,
    }
}

/// How to store MCP tool `args` in Ludus-routed `mcp_tool_called` / raw `tool_call` telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpToolArgsStorage {
    /// Full JSON args (default).
    Full,
    /// Replace args with `xxh3:<hex>` of the serialized payload.
    Hash,
    /// Drop args (`null` in JSON).
    Omit,
}

/// `VOX_LUDUS_MCP_TOOL_ARGS`: `full` (default) | `hash` | `omit`.
#[must_use]
pub fn mcp_tool_args_storage() -> McpToolArgsStorage {
    match vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusMcpToolArgs)
        .expose()
        .unwrap_or_default()
        .to_lowercase()
        .as_str()
    {
        "hash" | "redact" => McpToolArgsStorage::Hash,
        "omit" | "none" => McpToolArgsStorage::Omit,
        _ => McpToolArgsStorage::Full,
    }
}

/// When `VOX_LUDUS_EXPERIMENT` is set, scales teaching hint frequency for measurable A/B arms (see policy snapshots).
pub fn experiment_hint_frequency_multiplier() -> f64 {
    let exp_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusExperiment);
    let Some(exp) = exp_resolved.expose() else {
        return 1.0;
    };
    if exp.trim().is_empty() {
        return 1.0;
    }
    let arm = exp
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_add(u32::from(b)))
        % 2;
    if arm == 0 { 0.85 } else { 1.15 }
}

/// On-disk / standard [`VoxConfig::load()`] (no session env overlay). Use before `save()`.
pub fn load_disk() -> VoxConfig {
    VoxConfig::load()
}

/// Back-compat alias for [`load_disk`].
pub fn load() -> VoxConfig {
    load_disk()
}

/// Effective config: disk + session env overrides + emergency kill-switch.
pub fn load_effective() -> VoxConfig {
    let mut c = VoxConfig::load();
    let emergency_resolved = vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusEmergencyOff);
    if matches!(
        emergency_resolved
            .expose()
            .unwrap_or_default()
            .to_lowercase()
            .as_str(),
        "1" | "true" | "yes"
    ) {
        c.gamify_enabled = false;
        return c;
    }
    let session_enabled_resolved =
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusSessionEnabled);
    if let Some(v) = session_enabled_resolved.expose() {
        let low = v.to_lowercase();
        c.gamify_enabled = matches!(low.as_str(), "1" | "true" | "yes");
    }
    let session_mode_resolved =
        vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusSessionMode);
    if let Some(v) = session_mode_resolved.expose() {
        match v.to_lowercase().as_str() {
            "serious" => c.gamify_mode = GamifyMode::Serious,
            "learning" => c.gamify_mode = GamifyMode::Learning,
            "balanced" => c.gamify_mode = GamifyMode::Balanced,
            "off" => c.gamify_enabled = false,
            _ => {}
        }
    }
    c
}

/// Whether gamification is active at all.
pub fn is_enabled() -> bool {
    load_effective().gamify_enabled
}

/// The current gamification mode.
pub fn mode() -> GamifyMode {
    load_effective().gamify_mode
}

/// Label persisted on policy snapshots; appends non-empty `VOX_LUDUS_EXPERIMENT` for A/B tagging.
pub fn policy_snapshot_mode_label() -> String {
    let base = format!("{:?}", mode());
    match vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusExperiment).expose() {
        Some(exp) => {
            let t = exp.trim();
            if t.is_empty() {
                base
            } else {
                format!("{base}:{t}")
            }
        }
        None => base,
    }
}

/// Reward multiplier from the active mode.
pub fn reward_multiplier() -> f64 {
    mode().reward_multiplier()
}

/// Second experiment knob: multiply policy XP/crystals when `VOX_LUDUS_EXPERIMENT_REWARD_MULT` is a finite positive number (default `1.0`).
#[must_use]
pub fn experiment_reward_multiplier() -> f64 {
    match vox_clavis::resolve_secret(vox_clavis::SecretId::VoxLudusExperimentRewardMult).expose() {
        Some(s) => {
            let t = s.trim();
            if t.is_empty() {
                return 1.0;
            }
            let v: f64 = t.parse().unwrap_or(1.0);
            if v.is_finite() && v > 0.0 { v } else { 1.0 }
        }
        None => 1.0,
    }
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
