---
phase: 285
phase_name: "worktree-lease-checkpoints-and-muse-pool"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 1
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 285 Learnings: worktree-lease-checkpoints-and-muse-pool

## Decisions

### Hold an async tokio::sync::MutexGuard across .await in DelegationWorktre...
Hold an async `tokio::sync::MutexGuard` across `.await` in `DelegationWorktree::create` (`agy_worktree.rs`) and persist `WebhookDedupeAndRateGate` on `ServerState` (`webhook_intake.rs`).

**Rationale:** Primary phase objective established in plan.
**Source:** 285-01-PLAN.md

## Lessons

## Patterns

## Surprises

