---
phase: 286
phase_name: "utf8-safety-consensus-and-session-discovery"
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

# Phase 286 Learnings: utf8-safety-consensus-and-session-discovery

## Decisions

### Fix UTF-8 byte-slice panics in context_handoff.rs and mcp_tool_access.rs...
Fix UTF-8 byte-slice panics in `context_handoff.rs` and `mcp_tool_access.rs` using `floor_char_boundary`, upgrade `HandoffStrategy::Summarized` to generate a structured digest of earlier turns prior to the verbatim tail, and mint a `VoxIdAllocatorV2` `message_id` on persisted handoff entries.

**Rationale:** Primary phase objective established in plan.
**Source:** 286-01-PLAN.md

## Lessons

## Patterns

## Surprises

