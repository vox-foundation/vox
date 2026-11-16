---
phase: 76-humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u
plan: 02
subsystem: gamify-humane
tags: [humane-ai, gamify, anti-addictive, rest-day-policy, economy]
dependency_graph:
  requires: [05]
  provides: [HUMANE-02]
  affects: [vox-gamify, contracts/gamify/economy.v1.yaml]
tech_stack:
  added: []
  patterns: [zero-turn-grind-xp, weekends-exempt-streaks, milestone-earned-grace]
key_files:
  created: []
  modified:
    - crates/vox-gamify/src/reward_policy.rs
    - contracts/gamify/economy.v1.yaml
    - crates/vox-gamify/src/shop.rs
    - crates/vox-gamify/src/streak.rs
decisions:
  - Set `message_sent` XP and crystals to `0` in `reward_policy.rs` and `contracts/gamify/economy.v1.yaml` so chat turn volume never farms progression, and reward `single_shot_verified` (`35 XP / 8 crystals`) and `local_mens_verified` (`25 XP / 6 crystals`).
  - Remove purchasable `StreakShield` from `shop::default_catalog()` and default `StreakTracker` to `RestDayPolicy::WeekendsExempt` with `rest_mode_until_ts` support so weekends and declared rest windows never break streaks or consume grace periods.
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 76 Plan 02: Anti-Addictive Mastery & Rest-Day Streak Policy Summary

Eliminated dark-pattern engagement hooks in `vox-gamify` (`HUMANE-02`): zeroed out `message_sent` XP/crystals, rewarded `single_shot_verified` and `local_mens_verified` craft outcomes, removed purchasable `StreakShield` from `default_catalog()`, and implemented `RestDayPolicy::WeekendsExempt` + `rest_mode_until_ts` vacation freeze in `StreakTracker`.

## What Was Done

1. **Task 1 (`HUMANE-02` — Zero `message_sent` XP & Reward Single-Shot Verified Craft):**
   - Set `message_sent` base XP and crystals to `0` in `crates/vox-gamify/src/reward_policy.rs` and `contracts/gamify/economy.v1.yaml`.
   - Added `single_shot_verified` (`base_xp: 35, base_crystals: 8`, grind-exempt) and `local_mens_verified` (`base_xp: 25, base_crystals: 6`).
2. **Task 2 (`HUMANE-02` — Remove Purchasable `StreakShield` & Add `RestDayPolicy`):**
   - Removed `ShopItem::StreakShield` from `default_catalog()` in `crates/vox-gamify/src/shop.rs`.
   - Added `RestDayPolicy` (`WeekendsExempt` default) and `rest_mode_until_ts` in `crates/vox-gamify/src/streak.rs` with unit tests proving weekend gaps and rest-mode windows preserve streaks without consuming grace shields.

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-gamify --lib` -> all tests pass (`0 failed`)
