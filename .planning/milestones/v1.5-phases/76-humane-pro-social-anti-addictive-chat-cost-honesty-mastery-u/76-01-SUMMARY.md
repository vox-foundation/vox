---
phase: 76-humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u
plan: 01
subsystem: orchestrator-budget
tags: [humane-ai, cost-honesty, turn-cost-receipt, no-auto-reload, budget]
dependency_graph:
  requires: [05, 75]
  provides: [HUMANE-01]
  affects: [vox-orchestrator, vox-cli]
tech_stack:
  added: []
  patterns: [zero-markup-cost-receipt, hard-no-auto-reload-cap]
key_files:
  created: []
  modified:
    - crates/vox-orchestrator/src/budget/mod.rs
    - crates/vox-cli/src/commands/chat.rs
decisions:
  - Enforce `vox_markup_usd: 0.0` in `TurnCostReceipt` and calculate `saved_vs_frontier_usd` against the frontier reference rate (`$15/M` input, `$75/M` output) so local MENS and `:free` OpenRouter usage transparently show developer budget preserved.
  - Enforce hard No-Auto-Reload on `BudgetSignal::CostExceeded` (`DowngradeToFreeOrLocal`) and honor `--budget-usd` in `vox chat`.
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 76 Plan 01: Zero-Markup `TurnCostReceipt` & Hard No-Auto-Reload Budget Cap Summary

Implemented zero-markup BYOK/local cost honesty (`TurnCostReceipt`) and hard No-Auto-Reload enforcement on `BudgetSignal::CostExceeded` in `crates/vox-orchestrator/src/budget/mod.rs` and `crates/vox-cli/src/commands/chat.rs` (`HUMANE-01`).

## What Was Done

1. **Task 1 (`HUMANE-01` — Zero-Markup `TurnCostReceipt` & Hard No-Auto-Reload Cap in `vox-orchestrator::budget`):**
   - Implemented `TurnCostReceipt` (`model_id`, `tier`, `prompt_tokens`, `cached_tokens`, `completion_tokens`, `upstream_cost_usd`, `vox_markup_usd: 0.0`, `saved_vs_frontier_usd`) and `BudgetOverrunAction::DowngradeToFreeOrLocal` in `crates/vox-orchestrator/src/budget/mod.rs`.
   - Added unit test `test_turn_cost_receipt_zero_markup_and_frontier_savings` verifying `vox_markup_usd == 0.0`, `saved_vs_frontier_usd > 0.0` on local/free tiers, and hard No-Auto-Reload downgrade recommendation.
2. **Task 2 (`HUMANE-01` — Per-Turn Cost/Savings Footer & `--budget-usd` Cap in `vox chat`):**
   - Wired `format_turn_cost_receipt_footer` and `--budget-usd` into `crates/vox-cli/src/commands/chat.rs`.

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-orchestrator --lib budget` -> `43 passed; 0 failed`
