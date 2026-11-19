---
phase: 65
phase_name: "realtime-stream-channel-reactive-subscribe"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 65 Learnings: realtime-stream-channel-reactive-subscribe

## Decisions

### Extended [crates/vox-actor-runtime/src/subscription.rs](file:///Users/br...
Extended [`crates/vox-actor-runtime/src/subscription.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/subscription.rs) with `SequencedStreamEvent`, `publish_sequenced`, and `replay_since(channel, last_event_id)` backed by a bounded 256-event replay ring per stream channel.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 65-01-SUMMARY.md

---

### Added emit_realtime_stream_and_channel_routes(module
Added `emit_realtime_stream_and_channel_routes(module: &HirModule)` in [`crates/vox-codegen/src/codegen_rust/emit/http.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/http.rs) emitting `/api/stream/<name>` (with `last-event-id` header replay), `/api/channel/<name>` (`WebSocketUpgrade`), and `/api/subscribe/<table>`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 65-01-SUMMARY.md

---

### Extended [crates/vox-codegen-ts/src/tanstack_query_emit.rs](file:///User...
Extended [`crates/vox-codegen-ts/src/tanstack_query_emit.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen-ts/src/tanstack_query_emit.rs) with `useVoxReactiveTableQuery` (auto-invalidating `queryKey` on `/api/subscribe/${table}` SSE notifications) and `subscribeVoxEventStream`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 65-01-SUMMARY.md

---

### Added ProcessFrameKind (Line, CarriageReturnProgress), ProcessFrame, Pro...
Added `ProcessFrameKind` (`Line`, `CarriageReturnProgress`), `ProcessFrame`, `ProcessRingBuffer` (`PROCESS_RUN_RING_CAPACITY = 5_000`), `ProcessRunOutcome`, and `vox_process_run_managed` in [`crates/vox-actor-runtime/src/builtins/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/builtins/mod.rs).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 65-02-SUMMARY.md

---

### Properly distinguishes \r in-place progress frames (tqdm, FFmpeg frame=....
Properly distinguishes `\r` in-place progress frames (`tqdm`, FFmpeg `frame=... fps=...`) from `\n` / `\r\n` log lines while bounding memory to 5,000 events.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 65-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

