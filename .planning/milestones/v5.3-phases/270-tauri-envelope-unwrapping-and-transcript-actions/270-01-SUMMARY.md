---
phase: 270-tauri-envelope-unwrapping-and-transcript-actions
plan: 01
subsystem: vox-gui
tags: [t3, tauri-ipc, envelope-unwrapping, chat-transcript, vitest]
completed: 2026-10-25
---

# Phase 270 Plan 01 Summary: Tauri MCP Envelope Unwrapping, Transcript Regenerate/Edit UI & End-to-End Parity Gates

## Delivered Requirements (`T3-GUI-ENVELOPE-01..04`)

1. **`T3-GUI-ENVELOPE-01` — Recursive Tauri & MCP Envelope Unwrapping in `t3Wave3Rpc.ts` (`GAP-10`):**
   - Upgraded `unwrapMcpData<T>` in [`crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts) to recursively unwrap Tauri `{ is_error, result }` envelopes, JSON string payloads, standard MCP `{ content: [{ type: 'text', text }] }` arrays, and Vox `{ success, data, error }` `ToolResult` envelopes.

2. **`T3-GUI-ENVELOPE-02` — Typed Transport Wrappers for Regenerate, Edit-In-Place, Partial Compaction, Stream Resume & Queue Steer (`GAP-09`, `GAP-10`):**
   - Added `editChatMessageInPlace`, `regenerateChatTurn`, `compactChatUpTo`, `resumeChatStream`, and `steerChatQueue` to [`crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts) (re-exported via `transport.ts`).

3. **`T3-GUI-ENVELOPE-03` — Assistant Message In-Place Edit & Regenerate Controls in `ChatTranscript.tsx` (`GAP-09`):**
   - Added `✎ Edit` (with inline editor and `[Edited]` badge) and `↻ Regenerate` controls to `MessageBubble` and `ChatTranscript` in [`crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx).

4. **`T3-GUI-ENVELOPE-04` — Vitest Envelope Unwrapping & Live Integration Suite (`GAP-10`):**
   - Added [`crates/vox-gui/ui/src/transport/__tests__/t3Wave3Rpc.test.ts`](file:///Users/brbrainerd/dev/vox/crates/vox-gui/ui/src/transport/__tests__/t3Wave3Rpc.test.ts) verifying nested Tauri/MCP envelope unwrapping, error propagation, and Wave 4 transport wrappers.

## Verification Evidence
- `pnpm --dir crates/vox-gui/ui exec vitest run src/transport/__tests__/t3Wave3Rpc.test.ts src/components/surfaces/Chat/ChatTranscript.test.tsx` → **16 passed; 0 failed**
