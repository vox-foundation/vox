---
phase: 36
phase_name: "guarded-self-healing-doc-pr-bot"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 36 Learnings: guarded-self-healing-doc-pr-bot

## Decisions

### Implemented validate_bot_target_path and validate_bot_patch_scope_and_ga...
Implemented `validate_bot_target_path` and `validate_bot_patch_scope_and_gates` in [`crates/vox-doc-pipeline/src/claims/self_heal_bot.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-doc-pipeline/src/claims/self_heal_bot.rs) (`BOT-01..03`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 36-01-SUMMARY.md

---

### Implemented BotMetricsLedger::plan_or_update_draft_pr enforcing MAX_CONC...
Implemented `BotMetricsLedger::plan_or_update_draft_pr` enforcing `MAX_CONCURRENT_BOT_DRAFT_PRS = 3`, in-place PR updates per subsystem, and precision auto-pause (`BOT-04`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 36-01-SUMMARY.md

## Lessons

## Patterns

## Surprises

