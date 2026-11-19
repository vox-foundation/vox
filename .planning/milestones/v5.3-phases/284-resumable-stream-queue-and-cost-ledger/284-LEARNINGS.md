---
phase: 284
phase_name: "resumable-stream-queue-and-cost-ledger"
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

# Phase 284 Learnings: resumable-stream-queue-and-cost-ledger

## Decisions

### Replace ServerState::resumable_streams tuple map with ResumableTurnStrea...
Replace `ServerState::resumable_streams` tuple map with `ResumableTurnStreamRegistry` (`SequencedTurnChunk`, multi-iteration accumulation across tool loops and direct streaming turns, `vox_chat_stream_resume` + `vox_chat_stream_cancel`).

**Rationale:** Primary phase objective established in plan.
**Source:** 284-01-PLAN.md

## Lessons

## Patterns

## Surprises

