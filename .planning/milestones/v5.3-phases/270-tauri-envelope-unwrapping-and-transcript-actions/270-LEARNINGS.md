---
phase: 270
phase_name: "tauri-envelope-unwrapping-and-transcript-actions"
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

# Phase 270 Learnings: tauri-envelope-unwrapping-and-transcript-actions

## Decisions

### Close GAP-10 by fixing unwrapMcpData in crates/vox-gui/ui/src/transport/...
Close `GAP-10` by fixing `unwrapMcpData` in `crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts` to unwrap nested Tauri `{ is_error, result: { success, data } }` and stringified MCP `{ content: [{ type: 'text', text: '...' }] }` envelopes, exposing typed RPC helpers (`editChatMessageInPlace`, `regenerateChatTurn`, `compactChatUpTo`, `resumeChatStream`, `steerChatQueue`), wiring in-place assistant editing and posture-aware regeneration into `ChatTranscript.tsx`, and verifying all Rust, Vitest, TypeScript, and audit gates.

**Rationale:** Primary phase objective established in plan.
**Source:** 270-01-PLAN.md

## Lessons

## Patterns

## Surprises

