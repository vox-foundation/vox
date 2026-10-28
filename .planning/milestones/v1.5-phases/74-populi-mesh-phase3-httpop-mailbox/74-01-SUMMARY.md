---
phase: 74-populi-mesh-phase3-httpop-mailbox
plan: 01
subsystem: populi-mesh
tags: [populi, iroh, quic, workflow-runtime, a2a-mailbox]
dependency_graph:
  requires: [05]
  provides: [MESH-01, MESH-02, MESH-03]
  affects: [vox-mesh-transport, vox-workflow-runtime]
tech_stack:
  added: []
  patterns: [iroh-quic-endpoint, fail-closed-dispatch, a2a-mailbox]
key_files:
  created: []
  modified:
    - crates/vox-mesh-transport/src/endpoint.rs
    - crates/vox-mesh-transport/src/mailbox.rs
    - crates/vox-workflow-runtime/src/workflow/populi.rs
decisions:
  - Route `PopuliHttpOp` (`Noop`, `Snapshot`, `Join`, `Heartbeat`) over `vox-mesh-transport` iroh QUIC endpoints and ensure `Join` and `Heartbeat` never fabricate control-plane ACKs when unconfigured.
  - Ensure unconfigured `Dispatch` and `Wait` fail closed and A2A mailbox enforces strict inbox capacity boundaries across independent processes.
metrics:
  duration_minutes: 20
  completed_date: "2026-10-14"
---

# Phase 74 Plan 01: Populi Mesh Phase 3 (`PopuliHttpOp` over iroh QUIC & A2A Mailbox) Summary

Verified `PopuliHttpOp` (`Noop`, `Snapshot`, `Join`, `Heartbeat`, `Dispatch`, `Wait`) over `vox-mesh-transport` (iroh QUIC) in `crates/vox-workflow-runtime/src/workflow/populi.rs` and verified A2A mailbox capacity and delivery invariants in `crates/vox-mesh-transport/src/mailbox.rs` (`MESH-01..03`).

## What Was Done

1. **Task 1 (`MESH-01..03` — `PopuliHttpOp` over `vox-mesh-transport` & A2A Mailbox Verification):**
   - Verified `every_mesh_envelope_keeps_the_contract_keys`, `extra_fields_merge_into_the_envelope`, `join_and_heartbeat_never_claim_a_control_plane_ack`, `wait_is_inline_and_keeps_the_result_keys`, and `stalled_dispatch_times_out_and_cancels_the_same_job_id` in `vox-workflow-runtime`.
   - Verified `vox-mesh-transport` endpoint and A2A mailbox unit tests (`independent_processes_cannot_overbook_inbox_capacity`, etc.).

## Verification

- `cargo --config 'build.rustc-wrapper=""' test -p vox-mesh-transport --lib` -> `57 passed; 0 failed`
- `cargo --config 'build.rustc-wrapper=""' test -p vox-workflow-runtime --lib` -> `81 passed; 0 failed; 1 ignored`
