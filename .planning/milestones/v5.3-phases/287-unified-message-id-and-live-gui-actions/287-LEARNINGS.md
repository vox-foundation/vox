---
phase: 287
phase_name: "unified-message-id-and-live-gui-actions"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 6
  lessons: 0
  patterns: 3
  surprises: 0
missing_artifacts:
  - "VERIFICATION.md"
  - "UAT.md"
  - "STATE.md"
---

# Phase 287 Learnings: unified-message-id-and-live-gui-actions

## Decisions

### Resolved transcript message indices by exact ID, stripped :user/:asst su...
Resolved transcript message indices by exact ID, stripped `:user`/`:asst` suffix, or 1-based GUI numeric row ID so edit, compact, and regenerate work regardless of ID origin.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 287-01-SUMMARY.md

---

### Preserved backend_message_id, compaction_audit, posture_audit, and next_...
Preserved `backend_message_id`, `compaction_audit`, `posture_audit`, and `next_queued_follow_up` across `ParsedChatReply` and `ChatTurnDto`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 287-01-SUMMARY.md

---

### T3-GUI-PIPELINE-01
**`T3-GUI-PIPELINE-01`**: Added `find_transcript_message_index` in `history.rs` supporting exact ID, stripped role suffix, and 1-based numeric row IDs; wired Git checkpoint restore into `chat_regenerate`; and preserved `backend_message_id`, `compaction_audit`, `posture_audit`, and `next_queued_follow_up` in `ParsedChatReply` and `ChatTurnDto`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 287-01-SUMMARY.md

---

### T3-GUI-PIPELINE-02
**`T3-GUI-PIPELINE-02`**: Enabled in-place editing on both user and assistant messages, wired `execute: true` on regenerate, and added `✂ Compact Here` in `ChatTranscript.tsx` plus `/compact` in `App.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 287-01-SUMMARY.md

---

### T3-GUI-PIPELINE-03
**`T3-GUI-PIPELINE-03`**: Added `[⚡ Steer]` (`promoteToSteer: true`) and `[+ Queue]` (`promoteToSteer: false`) controls and `Alt+Enter` mid-turn steering in `Loquela.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 287-01-SUMMARY.md

---

### T3-GUI-PIPELINE-04
**`T3-GUI-PIPELINE-04`**: Created `streamWatchdog.ts` (`FIRST_EVENT_TIMEOUT_MS = 30_000`, `STREAM_IDLE_TIMEOUT_MS = 120_000`) and wired `vcsMusePoolAdopt`, `sessionImportBundle`, `sessionListExternal`, `guestShareMint`, `guestShareRedeem`, `guestShareSubmit`, `guestShareModerate`, `batteryStatus`, and `cancelChatStream` in `t3Wave3Rpc.ts`, `ConsensusGridView.tsx`, and `SessionInteropModal.tsx`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 287-01-SUMMARY.md

## Lessons

## Patterns

### unified-transcript-id-resolution
unified-transcript-id-resolution

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 287-01-SUMMARY.md

---

### client-stream-watchdog
client-stream-watchdog

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 287-01-SUMMARY.md

---

### mid-turn-composer-steer
mid-turn-composer-steer

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 287-01-SUMMARY.md

## Surprises

