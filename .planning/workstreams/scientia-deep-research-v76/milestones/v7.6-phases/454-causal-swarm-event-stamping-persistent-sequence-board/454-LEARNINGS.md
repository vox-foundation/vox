---
phase: 454
phase_name: "causal-swarm-event-stamping-persistent-sequence-board"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 4
  lessons: 0
  patterns: 6
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 454 Learnings: causal-swarm-event-stamping-persistent-sequence-board

## Decisions

### Keep canonical pub mod concilium_watchdog; declaration in crates/vox-orc...
Keep canonical `pub mod concilium_watchdog;` declaration in `crates/vox-orchestrator/src/lib.rs` and avoid duplicate module exports.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 454-01-SUMMARY.md

---

### Emit zero-token caution alerts onto #seq without waking @monitor; emit @...
Emit zero-token caution alerts onto `#seq` without waking `@monitor`; emit `@monitor` doorbell ping only on `TripReason` circuit trips or stale exclusive `FileAffinityMap` locks.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 454-01-SUMMARY.md

---

### Mount super::concilium_api::router() inside http_gateway and proxy all v...
Mount `super::concilium_api::router()` inside `http_gateway` and proxy all `vox-gui` Concilium & Populi calls via `BackendCalls::api_v2_get` / HTTP RPC.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 454-02-SUMMARY.md

---

### Preserve #[rustfmt::skip] in concilium_api.rs (398 LoC) to stay strictly...
Preserve `#[rustfmt::skip]` in `concilium_api.rs` (`398` LoC) to stay strictly beneath the `400`-LoC child module ceiling.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 454-02-SUMMARY.md

## Lessons

## Patterns

### Concilium swarm events carry W3C traceparent and causal sequence ancestr...
Concilium swarm events carry W3C traceparent and causal sequence ancestry so UI swimlanes can render wakeup arcs deterministically.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-01-SUMMARY.md

---

### W3C Trace Context + causal sequence metadata (origin_seq, cause_seq, wor...
W3C Trace Context + causal sequence metadata (`origin_seq`, `cause_seq`, `workspace_mode`, `compute_tier`, `traceparent`) shared across L1 DTOs and L3 orchestrator events

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-01-SUMMARY.md

---

### Zero-token hibernation
Zero-token hibernation: parked workers remain at `0 tok/s` until `BulletinBoard` monotonic `#seq` doorbell matches `@mention` or `concilium_watchdog` trip rule

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-01-SUMMARY.md

---

### Hybrid local-MENS, QUIC peer, and cloud bandit placement telemetry and p...
Hybrid local-MENS, QUIC peer, and cloud bandit placement telemetry and pilot control actions flow uniformly through `concilium_api.rs`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-02-SUMMARY.md

---

### Single daemon-mediated HTTP v2 control plane (/api/v2/concilium/{snapsho...
Single daemon-mediated HTTP v2 control plane (`/api/v2/concilium/{snapshot,board,placement,pilot-action}`) keeping `vox-gui` completely free of direct `VoxDb` handles

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-02-SUMMARY.md

---

### Populi capability normalization using rec.capabilities.cpu_cores.unwrap_...
Populi capability normalization using `rec.capabilities.cpu_cores.unwrap_or(1).max(1)`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 454-02-SUMMARY.md

## Surprises

