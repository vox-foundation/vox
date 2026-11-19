---
phase: 267
phase_name: "agent-loop-checkpoints-and-resumable-streams"
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

# Phase 267 Learnings: agent-loop-checkpoints-and-resumable-streams

## Decisions

### Close GAP-05 by wiring normalize_terminal_output (\r), bound_snapshot_me...
Close `GAP-05` by wiring `normalize_terminal_output` (`\r`), `bound_snapshot_metadata` (`20 KiB` DOM shedding), automatic pre-write sparse Git checkpoints (`should_sparse_checkpoint`), `ResumableTurnStreamRegistry` (`vox_chat_stream_resume`), and mid-turn `ThreadRunQueue` steer draining (`vox_chat_queue_steer`) into `ServerState`, `agent_loop.rs`, and `salvage_and_stream.rs`.

**Rationale:** Primary phase objective established in plan.
**Source:** 267-01-PLAN.md

## Lessons

## Patterns

## Surprises

