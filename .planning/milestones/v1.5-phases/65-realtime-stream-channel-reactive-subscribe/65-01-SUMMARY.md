---
phase: 65-realtime-stream-channel-reactive-subscribe
plan: "01"
subsystem: runtime-and-codegen
status: completed
completed_at: "2026-10-15"
requirements-completed:
  - STREAM-01
  - STREAM-02
---

# Plan 65-01 Summary: Real-Time `Stream[T]`, `Channel[Send, Recv]` & Reactive `db.Table.subscribe`

## Accomplishments
- **Sequenced `Last-Event-ID` Replay (`STREAM-01`)**:
  - Extended [`crates/vox-actor-runtime/src/subscription.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/subscription.rs) with `SequencedStreamEvent`, `publish_sequenced`, and `replay_since(channel, last_event_id)` backed by a bounded 256-event replay ring per stream channel.
- **Axum SSE, WebSocket & Table Subscribe Route Emission (`STREAM-01`, `STREAM-02`)**:
  - Added `emit_realtime_stream_and_channel_routes(module: &HirModule)` in [`crates/vox-codegen/src/codegen_rust/emit/http.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen/src/codegen_rust/emit/http.rs) emitting `/api/stream/<name>` (with `last-event-id` header replay), `/api/channel/<name>` (`WebSocketUpgrade`), and `/api/subscribe/<table>`.
- **Reactive TanStack Query Client Hooks (`STREAM-02`)**:
  - Extended [`crates/vox-codegen-ts/src/tanstack_query_emit.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-codegen-ts/src/tanstack_query_emit.rs) with `useVoxReactiveTableQuery` (auto-invalidating `queryKey` on `/api/subscribe/${table}` SSE notifications) and `subscribeVoxEventStream`.

## Verification
- `cargo test -p vox-actor-runtime --lib subscription::tests`: `8 passed; 0 failed`.
- `cargo test -p vox-codegen --lib emit_realtime_routes_covers_stream_channel_and_table_subscribe`: `1 passed; 0 failed`.
