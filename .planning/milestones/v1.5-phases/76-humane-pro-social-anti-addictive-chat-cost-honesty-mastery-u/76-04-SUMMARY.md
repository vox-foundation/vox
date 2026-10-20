---
phase: 76-humane-pro-social-anti-addictive-chat-cost-honesty-mastery-u
plan: 04
subsystem: fatigue-session-closure
tags: [humane-ai, fatigue-monitor, session-closure, chat-surface, wellbeing]
dependency_graph:
  requires: [76-02, 76-03]
  provides: [HUMANE-04]
  affects: [vox-orchestrator, vox-gui]
tech_stack:
  added: []
  patterns: [session-closure-summary, calm-wrap-up-card]
key_files:
  created: []
  modified:
    - crates/vox-orchestrator/src/fatigue_monitor.rs
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.test.tsx
decisions:
  - Attach `SessionClosureSummary` (`active_duration_mins`, `verified_tasks_completed`, `saved_vs_frontier_usd`, `rest_mode_preserved_streak: true`) to `FatigueMonitor` so `> 4h` continuous work or `> 25` context switches produce an accomplishment-focused wrap-up rather than a nagging alert.
  - Render a calm, dismissible **Session Wrap-Up & Step Away** card in `ChatSurface.tsx` with 1-click **"Pause & Enable Rest Mode (Streak Protected)"** and **"Dismiss"** actions.
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 76 Plan 04: Session Closure Summary & Calm "Session Wrap-Up & Step Away" Card Summary

Implemented `SessionClosureSummary` in `crates/vox-orchestrator/src/fatigue_monitor.rs` and wired the calm **Session Wrap-Up & Step Away** card into `crates/vox-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx` (`HUMANE-04`).

## What Was Done

1. **Task 1 (`HUMANE-04` — `SessionClosureSummary` in `FatigueMonitor`):**
   - Added `SessionClosureSummary` and `FatigueEvent::with_closure_summary` in `crates/vox-orchestrator/src/fatigue_monitor.rs` with unit tests verifying accomplishment and streak-preservation metadata.
2. **Task 2 (`HUMANE-04` — Calm "Session Wrap-Up & Step Away" Card in `ChatSurface.tsx`):**
   - Rendered the dismissible **Session Wrap-Up & Step Away** banner (`data-testid="session-wrap-up-card"`) in `ChatSurface.tsx` with **"Pause & Enable Rest Mode (Streak Protected)"** (`data-testid="session-wrap-up-rest-btn"`) and **"Dismiss"** (`data-testid="session-wrap-up-dismiss-btn"`).
   - Added Vitest tests in `ChatSurface.test.tsx` (`71 passed`).

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-orchestrator --lib fatigue_monitor` -> `7 passed; 0 failed`
- `pnpm --dir crates/vox-gui/ui exec vitest run src/components/surfaces/Chat/ChatSurface.test.tsx` -> `71 passed (71)`
