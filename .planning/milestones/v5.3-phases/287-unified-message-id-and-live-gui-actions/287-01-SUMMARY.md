---
phase: 287-unified-message-id-and-live-gui-actions
plan: 01
subsystem: gui
tags: [message-id, transcript, loquela, stream-watchdog, session-interop]
requires:
  - phase: 286-utf8-safety-consensus-and-session-discovery
    provides: [utf8-handoff, consensus-synthesis, session-discovery]
provides:
  - Unified message ID resolution (`find_transcript_message_index`) and `ParsedChatReply` / `ChatTurnDto` envelope fields (`T3-GUI-PIPELINE-01`)
  - Live transcript edit (user + assistant), regenerate with checkpoint restore, and partial compaction actions (`T3-GUI-PIPELINE-02`)
  - Loquela composer mid-turn `[⚡ Steer]` and `[+ Queue]` controls + `/compact` slash command (`T3-GUI-PIPELINE-03`)
  - Client stream watchdog (`FIRST_EVENT_TIMEOUT_MS = 30_000`, `STREAM_IDLE_TIMEOUT_MS = 120_000`) & Wave 5 RPC helpers (`T3-GUI-PIPELINE-04`)
affects: [vox-orchestrator-mcp, vox-gui]
tech-stack:
  added: []
  patterns: [unified-transcript-id-resolution, client-stream-watchdog, mid-turn-composer-steer]
key-files:
  created:
    - crates/vox-gui/ui/src/lib/chat/streamWatchdog.ts
    - crates/vox-gui/ui/src/lib/chat/__tests__/streamWatchdog.test.ts
  modified:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/history.rs
    - crates/vox-gui/src/commands/chat.rs
    - crates/vox-gui/src/commands/chat_turn.rs
    - crates/vox-gui/ui/src/transport/t3Wave3Rpc.ts
    - crates/vox-gui/ui/src/components/surfaces/Chat/ChatTranscript.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/ConsensusGridView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Chat/SessionInteropModal.tsx
    - crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx
    - crates/vox-gui/ui/src/App.tsx
key-decisions:
  - "Resolved transcript message indices by exact ID, stripped `:user`/`:asst` suffix, or 1-based GUI numeric row ID so edit, compact, and regenerate work regardless of ID origin."
  - "Preserved `backend_message_id`, `compaction_audit`, `posture_audit`, and `next_queued_follow_up` across `ParsedChatReply` and `ChatTurnDto`."
requirements-completed: [T3-GUI-PIPELINE-01, T3-GUI-PIPELINE-02, T3-GUI-PIPELINE-03, T3-GUI-PIPELINE-04]
completed: 2026-10-25
---

# Phase 287 Plan 01: Unified Message ID Pipeline, Live Transcript Actions & Composer Steer/Queue Summary

## Accomplishments
- **`T3-GUI-PIPELINE-01`**: Added `find_transcript_message_index` in `history.rs` supporting exact ID, stripped role suffix, and 1-based numeric row IDs; wired Git checkpoint restore into `chat_regenerate`; and preserved `backend_message_id`, `compaction_audit`, `posture_audit`, and `next_queued_follow_up` in `ParsedChatReply` and `ChatTurnDto`.
- **`T3-GUI-PIPELINE-02`**: Enabled in-place editing on both user and assistant messages, wired `execute: true` on regenerate, and added `✂ Compact Here` in `ChatTranscript.tsx` plus `/compact` in `App.tsx`.
- **`T3-GUI-PIPELINE-03`**: Added `[⚡ Steer]` (`promoteToSteer: true`) and `[+ Queue]` (`promoteToSteer: false`) controls and `Alt+Enter` mid-turn steering in `Loquela.tsx`.
- **`T3-GUI-PIPELINE-04`**: Created `streamWatchdog.ts` (`FIRST_EVENT_TIMEOUT_MS = 30_000`, `STREAM_IDLE_TIMEOUT_MS = 120_000`) and wired `vcsMusePoolAdopt`, `sessionImportBundle`, `sessionListExternal`, `guestShareMint`, `guestShareRedeem`, `guestShareSubmit`, `guestShareModerate`, `batteryStatus`, and `cancelChatStream` in `t3Wave3Rpc.ts`, `ConsensusGridView.tsx`, and `SessionInteropModal.tsx`.
