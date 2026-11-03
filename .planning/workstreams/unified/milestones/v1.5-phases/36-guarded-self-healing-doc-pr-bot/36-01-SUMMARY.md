---
phase: 36-guarded-self-healing-doc-pr-bot
plan: 01
subsystem: vox-doc-pipeline
tags: [self-heal-bot, draft-pr, doc-freshness]
provides:
  - Human-reviewed draft-PR specification with `[path:Lstart-Lend]` evidence citations (`BOT-01`)
  - Strict `docs/src/**` path-scope guard rejecting `.rs`, `.vox`, `Cargo.toml`, `contracts/**`, `*.generated.md`, and `docs/src/archive/**` (`BOT-02`)
  - Pre-PR verification gates for frontmatter, code fences, Mermaid blocks, and citations (`BOT-03`)
  - Concurrency cap (`<= 3`), in-place branch updates, and rolling precision auto-pause (`BOT-04`)
key-files:
  created:
    - crates/vox-doc-pipeline/src/claims/self_heal_bot.rs
  modified:
    - crates/vox-doc-pipeline/src/claims/mod.rs
requirements-completed: [BOT-01, BOT-02, BOT-03, BOT-04]
completed: 2026-10-15
---

# Phase 36 Plan 01: Guarded Self-Healing Doc Draft-PR Bot Summary

## Accomplishments
- Implemented `validate_bot_target_path` and `validate_bot_patch_scope_and_gates` in [`crates/vox-doc-pipeline/src/claims/self_heal_bot.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/self_heal_bot.rs) (`BOT-01..03`).
- Implemented `BotMetricsLedger::plan_or_update_draft_pr` enforcing `MAX_CONCURRENT_BOT_DRAFT_PRS = 3`, in-place PR updates per subsystem, and precision auto-pause (`BOT-04`).

## Verification
- `timeout 900s cargo --config 'build.rustc-wrapper=""' test -p vox-doc-pipeline --lib` — **84 passed, 0 failed**.
