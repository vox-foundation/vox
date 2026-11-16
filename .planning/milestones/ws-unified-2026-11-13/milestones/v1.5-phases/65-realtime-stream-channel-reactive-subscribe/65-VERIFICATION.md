---
phase: 65-realtime-stream-channel-reactive-subscribe
status: passed
verified_at: "2026-10-15"
requirements_verified:
  - STREAM-01
  - STREAM-02
  - STREAM-03
  - STREAM-04
---

# Phase 65 Verification Report: Real-Time `Stream[T]`, `Channel[Send, Recv]`, Reactive `db.Table.subscribe` & Managed `ProcessRun` (`STREAM-01..04`)

## Requirement Matrix
| Requirement | Artifact / Symbol | Verification Command | Result |
|---|---|---|---|
| `STREAM-01` | `SequencedStreamEvent`, `publish_sequenced`, `replay_since` (`subscription.rs`), `emit_realtime_stream_and_channel_routes` (`http.rs`), `subscribeVoxEventStream` (`tanstack_query_emit.rs`) | `cargo test -p vox-actor-runtime --lib subscription::tests` & `cargo test -p vox-codegen --lib emit_realtime_routes_covers_stream_channel_and_table_subscribe` | PASS |
| `STREAM-02` | `/api/subscribe/<Table>` route (`http.rs`) + `useVoxReactiveTableQuery` (`tanstack_query_emit.rs`) | `cargo test -p vox-codegen --lib tanstack_query_tsx_emits_reactive_table_and_stream_helpers` | PASS |
| `STREAM-03` | `ProcessRingBuffer` (`PROCESS_RUN_RING_CAPACITY = 5_000`), `ProcessFrameKind::CarriageReturnProgress`, `vox_process_run_managed` (`builtins/mod.rs`) | `cargo test -p vox-actor-runtime --lib process_ring_buffer_splits_cr_progress_and_evicts_past_capacity` | PASS |
| `STREAM-04` | Unit & integration tests across `vox-actor-runtime` and `vox-codegen` | All 4 test suites green | PASS |
