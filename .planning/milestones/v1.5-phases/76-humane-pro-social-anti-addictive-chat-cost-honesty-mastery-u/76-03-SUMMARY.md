---
phase: 76-humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u
plan: 03
subsystem: chat-loquela-ux
tags: [humane-ai, anti-sycophancy, cost-honesty, context-compaction, loquela]
dependency_graph:
  requires: [76-01]
  provides: [HUMANE-01, HUMANE-03]
  affects: [vox-cli, mens/config/system_prompt.txt, vox-gui]
tech_stack:
  added: []
  patterns: [anti-sycophancy-preamble, pre-send-cost-badge, context-compaction-nudge]
key_files:
  created: []
  modified:
    - crates/vox-cli/src/commands/chat.rs
    - mens/config/system_prompt.txt
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.test.tsx
decisions:
  - Enforce the Anti-Sycophancy & Craft Directness Preamble (`HUMANE-03`) in both `mens/config/system_prompt.txt` and `crates/vox-cli/src/commands/chat.rs`.
  - Surface live per-tier cost transparency (`$0.00 Local`, `$0.00 :free`, or `0% markup` BYOK), cumulative frontier savings, and a `> 24k`-token context-compaction nudge in `Loquela.tsx`.
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 76 Plan 03: Anti-Sycophancy Preamble & Pre-Send Cost/Context Transparency Summary

Wired the Anti-Sycophancy & Craft Directness Preamble into `vox chat` and `mens/config/system_prompt.txt` (`HUMANE-03`) and added live pre-send cost/savings/cache transparency plus the `> 24k`-token context-compaction nudge in `Loquela.tsx` (`HUMANE-01`, `HUMANE-03`).

## What Was Done

1. **Task 1 (`HUMANE-03` — Anti-Sycophancy & Craft Directness Preamble):**
   - Added `HUMANE_CRAFT_DIRECTNESS_PREAMBLE` in `crates/vox-cli/src/commands/chat.rs` and corresponding rules in `mens/config/system_prompt.txt` forbidding sycophantic filler, requiring architectural pushback when warranted, and prioritizing single-shot verified solutions.
2. **Task 2 (`HUMANE-01`, `HUMANE-03` — Pre-Send Cost/Savings/Cache Badge & `> 24k`-Token Session Compact Nudge in `Loquela.tsx`):**
   - Replaced `cost: null` tier placeholders in `Loquela.tsx` with transparent zero-markup cost badges (`$0.00 Local`, `$0.00 :free`, `~$0.001/1k (0% markup)`, `~$0.015/1k (0% markup)`), cumulative session savings vs frontier, and a `> 24,000`-token context-compaction chip.
   - Added Vitest coverage in `Loquela.test.tsx`.

## Verification

- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Loquela/Loquela.test.tsx` -> `41 passed (41)`
