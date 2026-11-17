---
phase: 454-causal-swarm-event-stamping-persistent-sequence-board
plan: 01
subsystem: orchestrator-concilium-core
tags: [a2a, w3c-traceparent, bulletin-seq, vox-db, watchdog, circuit-breaker]
requires:
  - phase: 453-deep-research-reasoning-budget-separation-citation-scoped-au
    provides: Baseline orchestrator bulletin board, A2A envelope, and VoxDb A2A store
provides:
  - W3C `traceparent`/`tracestate` & `CausalSwarmStamp` DTOs in `vox-orchestrator-types::concilium` (`concilium.rs`, `327` LoC)
  - Causal propagation across `RemoteTaskEnvelope`, `AgentEventKind`, and `BulletinBoard` `#seq` doorbell
  - Persistent `concilium_seq_board` storage in `crates/vox-db/src/store/ops_a2a.rs` (`append_concilium_seq_entry`, `read_concilium_seq_board`)
  - Zero-token deterministic `concilium_watchdog.rs` (`361` LoC) posting structured `#seq` telemetry and gating `@monitor` wakeups on tripped rules
affects: [vox-orchestrator-types, vox-orchestrator, vox-db]
tech-stack:
  added: []
  patterns:
    - "W3C Trace Context + causal sequence metadata (`origin_seq`, `cause_seq`, `workspace_mode`, `compute_tier`, `traceparent`) shared across L1 DTOs and L3 orchestrator events"
    - "Zero-token hibernation: parked workers remain at `0 tok/s` until `BulletinBoard` monotonic `#seq` doorbell matches `@mention` or `concilium_watchdog` trip rule"
key-files:
  created:
    - crates/vox-orchestrator-types/src/concilium.rs
    - crates/vox-orchestrator/src/concilium_watchdog.rs
  modified:
    - crates/vox-orchestrator/src/lib.rs
    - crates/vox-orchestrator/src/bulletin.rs
    - crates/vox-orchestrator/src/a2a/envelope.rs
    - crates/vox-orchestrator/src/events.rs
    - crates/vox-db/src/store/ops_a2a.rs
key-decisions:
  - "Keep canonical `pub mod concilium_watchdog;` declaration in `crates/vox-orchestrator/src/lib.rs` and avoid duplicate module exports."
  - "Emit zero-token caution alerts onto `#seq` without waking `@monitor`; emit `@monitor` doorbell ping only on `TripReason` circuit trips or stale exclusive `FileAffinityMap` locks."
patterns-established:
  - "Concilium swarm events carry W3C traceparent and causal sequence ancestry so UI swimlanes can render wakeup arcs deterministically."
requirements-completed: [CONC-01, CONC-02, CONC-04]
duration: 45min
completed: 2026-11-16
---

# Phase 454 Plan 01: Causal Swarm Event Stamping & Persistent Sequence Board Summary

**Phase 454 Plan 01 introduced canonical `vox-orchestrator-types::concilium` causal DTOs (`CONC-01`), monotonic `#seq` doorbell persistence in `vox-db` (`CONC-02`), and zero-token deterministic `concilium_watchdog.rs` (`361` LoC).**

## Accomplishments

### `CONC-01`: Canonical W3C Causal DTOs & Event Stamping
- Created [`crates/vox-orchestrator-types/src/concilium.rs`](crates/vox-orchestrator-types/src/concilium.rs) (`327` LoC) defining `CausalSwarmStamp` (`traceparent`, `tracestate`, `origin_seq`, `cause_seq`, `compute_tier`, etc.), `SequenceBoardEntry`, `ConciliumSnapshotDto`, `PopuliPlacementDto`, and `ConciliumPilotAction` types.
- Propagated `causal_stamp` into [`crates/vox-orchestrator/src/a2a/envelope.rs`](crates/vox-orchestrator/src/a2a/envelope.rs) (`RemoteTaskEnvelope`), [`crates/vox-orchestrator/src/events.rs`](crates/vox-orchestrator/src/events.rs), and [`crates/vox-orchestrator/src/bulletin.rs`](crates/vox-orchestrator/src/bulletin.rs).

### `CONC-02`: Persistent `#seq` Board & Zero-Token Deterministic Watchdog
- Extended [`crates/vox-db/src/store/ops_a2a.rs`](crates/vox-db/src/store/ops_a2a.rs) (`append_concilium_seq_entry`, `read_concilium_seq_board`) so `#seq` board entries persist across orchestrator restarts.
- Created [`crates/vox-orchestrator/src/concilium_watchdog.rs`](crates/vox-orchestrator/src/concilium_watchdog.rs) (`361` LoC, `< 400` LoC budget) evaluating `CircuitBreakerState`, `AttentionBudget`, lease TTL staleness, and `FileAffinityMap` lock contention, waking `@monitor` only on actual trip/stall conditions.

## Verification
- `cargo test -p vox-orchestrator-types`: `61 passed; 0 failed` (`51` unit + `3` `providers_and_routes` + `5` `routing_and_ids_smoke` + `2` doc-tests).
- `cargo test -p vox-orchestrator --lib bulletin`: `2 passed; 0 failed` (`publish_and_receive`, `monotonic_seq_park_wakeup_and_pilot_actions`).
- `cargo test -p vox-orchestrator --lib concilium_watchdog`: `2 passed; 0 failed` (`circuit_trip_and_lock_stall_wake_monitor_via_seq_doorbell`, `caution_alert_posts_zero_token_telemetry_without_waking_monitor`).

---
*Phase: 454-causal-swarm-event-stamping-persistent-sequence-board*
*Completed: 2026-11-16*
