---
phase: 449
phase_name: "extreme-data-streaming-and-epistemic-canvas"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 3
  lessons: 3
  patterns: 5
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 449 Learnings: extreme-data-streaming-and-epistemic-canvas

## Decisions

### Eliminated standalone VoxDb::connect_default() polling in scientia_feed....
Eliminated standalone `VoxDb::connect_default()` polling in `scientia_feed.rs` in favor of `ServerState.db` shared by `GatewayState` and bounded `mpsc(512)` `TelemetryBatcher`.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 449-01-SUMMARY.md

---

### Used existing @tanstack/react-virtual and vox_crypto::compliance_hash/se...
Used existing `@tanstack/react-virtual` and `vox_crypto::compliance_hash`/`secure_hash` with zero new Rust or npm packages.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 449-01-SUMMARY.md

---

### Mounted GET /research/frame-chunks/{stream_id} on gui_ops::research::api...
Mounted `GET /research/frame-chunks/{stream_id}` on `gui_ops::research::api_v2_router()` alongside `super::research_stream::api_v2_router()` in Wave C.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 449-01-SUMMARY.md

## Lessons

### Blocking Issues
**Blocking Issues:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 449-REVIEW.md

---

### Phase 449 Architectural / Guard Violations
**Phase 449 Architectural / Guard Violations:** `0`

**Context:** Surfaced in structured phase code review.
**Source:** 449-REVIEW.md

---

### Non-Blocking Findings
**Non-Blocking Findings:** `2` noted below (unrelated `pty/tests.rs:141` arch-check failure, 3 dead-code telemetry helper warnings in `scientia_feed.rs`)

**Context:** Surfaced in structured phase code review.
**Source:** 449-REVIEW.md

## Patterns

### Cursor-backed pagination + ?from_seq= SSE replay recovery contract (resy...
Cursor-backed pagination + `?from_seq=` SSE replay recovery contract (`resync_required` -> snapshot reload) across daemon and Axis `ResearchView`.

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 449-01-SUMMARY.md

---

### Spill-and-pointer IPC compaction
Spill-and-pointer IPC compaction: payloads >16 KiB spill to content-addressed chunks served via `/api/v2/research/sessions/{id}/chunks/{hash}` (`vox_crypto::compliance_hash`)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 449-01-SUMMARY.md

---

### Sequenced SSE/WS replay ring
Sequenced SSE/WS replay ring: `ResearchFeedHub` (`1024` events) emits monotonic `seq: u64` frames and explicit `resync_required` recovery on cursor gap or `RecvError::Lagged`

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 449-01-SUMMARY.md

---

### Bounded streaming frames
Bounded streaming frames: `FrameChunkCursor` reads Arrow IPC & CSV in `O(batch_size)` RAM with sliding-window backpressure and `1..=500` page clamping

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 449-01-SUMMARY.md

---

### Pure @tanstack/react-virtual row windowing (useVirtualizer with JSDOM-sa...
Pure `@tanstack/react-virtual` row windowing (`useVirtualizer` with JSDOM-safe `initialRect`) and wave-clustered SVG DAG (>80 nodes)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 449-01-SUMMARY.md

## Surprises

