---
phase: 65-realtime-stream-channel-reactive-subscribe
plan: "02"
subsystem: actor-runtime
status: completed
completed_at: "2026-10-15"
requirements-completed:
  - STREAM-03
  - STREAM-04
---

# Plan 65-02 Summary: Managed `ProcessRun` (`\r`/`\n` Progress Framing, 5,000-Event Ring Buffer & Process-Group Isolation)

## Accomplishments
- **Managed `ProcessRun` & Progress-Bar Frame Splitting (`STREAM-03`)**:
  - Added `ProcessFrameKind` (`Line`, `CarriageReturnProgress`), `ProcessFrame`, `ProcessRingBuffer` (`PROCESS_RUN_RING_CAPACITY = 5_000`), `ProcessRunOutcome`, and `vox_process_run_managed` in [`crates/vox-actor-runtime/src/builtins/mod.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-actor-runtime/src/builtins/mod.rs).
  - Properly distinguishes `\r` in-place progress frames (`tqdm`, FFmpeg `frame=... fps=...`) from `\n` / `\r\n` log lines while bounding memory to 5,000 events.

## Verification
- `cargo test -p vox-actor-runtime --lib process_ring_buffer_splits_cr_progress_and_evicts_past_capacity`: `1 passed; 0 failed`.
