---
phase: 449-extreme-data-streaming-and-epistemic-canvas
plan: 01
subsystem: streaming
tags: [rust, react, sse, websocket, chunk-spill, arrow, virtualizer, vox-search, vox-python-bridge, vox-orchestrator-mcp, vox-gui]

# Dependency graph
requires:
  - phase: 447-before-audit-baseline-and-continuum-contracts
    provides: Canonical L1 `SequencedResearchEvent` wire contracts (`vox-research-events`) and indexed `VoxDb` cursor pagination (`list_session_claims_page`, `list_session_sources_page`)
  - phase: 448-unified-continuum-socrates-and-kv-cache
    provides: Dynamic 1..=5 wave execution, scholarly domain mode, and continuum metadata emission
provides:
  - SHA3-256 disk-backed chunk spilling (`crates/vox-search/src/chunk_spill.rs`, 387 LoC) with 16 KiB inline threshold and 128 MiB oldest-first eviction
  - Bounded `mpsc(512)` `TelemetryBatcher` + `1024`-entry `ResearchFeedHub` replay ring (`scientia_feed.rs`) + SSE/WS `?from_seq=` / `resync_required` recovery (`ws.rs`, `research_stream.rs`)
  - Compact `GET /api/v2/research/sessions/{id}` detail payload + paginated `/claims`, `/sources`, and verified `/chunks/{hash}` endpoints (`research.rs`, `research_stream.rs`)
  - O(batch_size) Arrow IPC & CSV `FrameChunkCursor` streaming (`crates/vox-python-bridge/src/frame_stream.rs`, 391 LoC) + `GET /api/v2/research/frame-chunks/{stream_id}` (`gui_ops/frame_stream.rs`, 359 LoC)
  - Dynamic `MultiWaveProgressTimeline.tsx` (1..=5 waves), cluster-virtualized `ResearchDagCanvas.tsx` (>80 node wave clustering), and `@tanstack/react-virtual` `VirtualizedClaimsTable.tsx` & `VirtualizedFrameChunkTable.tsx` in `ResearchView.tsx`
affects:
  - Phase 450–452 (`VERI`, `DISC`, `PUBL`, `STUD`, `AUDT-02` paired `--stage after` diff)

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Spill-and-pointer IPC compaction: payloads >16 KiB spill to content-addressed chunks served via `/api/v2/research/sessions/{id}/chunks/{hash}` (`vox_crypto::compliance_hash`)"
    - "Sequenced SSE/WS replay ring: `ResearchFeedHub` (`1024` events) emits monotonic `seq: u64` frames and explicit `resync_required` recovery on cursor gap or `RecvError::Lagged`"
    - "Bounded streaming frames: `FrameChunkCursor` reads Arrow IPC & CSV in `O(batch_size)` RAM with sliding-window backpressure and `1..=500` page clamping"
    - "Pure `@tanstack/react-virtual` row windowing (`useVirtualizer` with JSDOM-safe `initialRect`) and wave-clustered SVG DAG (>80 nodes)"

key-files:
  created:
    - crates/vox-search/src/chunk_spill.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/research_stream.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs
    - crates/vox-python-bridge/src/frame_stream.rs
    - crates/vox-gui/ui/src/components/surfaces/Research/VirtualizedClaimsTable.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/VirtualizedFrameChunkTable.tsx
  modified:
    - crates/vox-search/src/lib.rs
    - crates/vox-python-bridge/src/lib.rs
    - crates/vox-orchestrator-mcp/src/http_gateway/mod.rs
    - crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs
    - crates/vox-orchestrator-mcp/src/http_gateway/ws.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/mod.rs
    - crates/vox-orchestrator-mcp/src/gui_ops/research.rs
    - crates/vox-gui/ui/src/components/surfaces/Research/MultiWaveProgressTimeline.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.test.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.tsx
    - crates/vox-gui/ui/src/components/surfaces/Research/ResearchView.test.tsx

key-decisions:
  - "Eliminated standalone `VoxDb::connect_default()` polling in `scientia_feed.rs` in favor of `ServerState.db` shared by `GatewayState` and bounded `mpsc(512)` `TelemetryBatcher`."
  - "Used existing `@tanstack/react-virtual` and `vox_crypto::compliance_hash`/`secure_hash` with zero new Rust or npm packages."
  - "Mounted `GET /research/frame-chunks/{stream_id}` on `gui_ops::research::api_v2_router()` alongside `super::research_stream::api_v2_router()` in Wave C."

patterns-established:
  - "Cursor-backed pagination + `?from_seq=` SSE replay recovery contract (`resync_required` -> snapshot reload) across daemon and Axis `ResearchView`."

requirements-completed: [STRM-01, STRM-02, STRM-03, STRM-04]

# Metrics
duration: 60min
completed: 2026-11-15
---

# Phase 449 Plan 01: Extreme-Data Streaming, `std.frame` Cursors & Virtualized Epistemic Canvas Summary

**Phase 449 Plan 01 landed sequenced SSE/WS research deltas (`resync_required` recovery), SHA3-256 chunk spilling & compact IPC payloads, `O(batch_size)` Arrow/CSV `FrameChunkCursor` streaming, and virtualized multi-wave/DAG/tabular views in Axis GUI.**

## Accomplishments

### Task 1: Disk-Backed Chunk Spill, Bounded Telemetry & Sequenced SSE/WS (`STRM-01`, `STRM-02`)
- Created `crates/vox-search/src/chunk_spill.rs` (`387` LoC, `< 400`) and exported `ChunkSpillStore`, `SPILL_THRESHOLD_BYTES` (`16 * 1024`), and `SpillPointer` from `crates/vox-search/src/lib.rs`, with `128 MiB` oldest-first eviction, digest validation, and zero extra dependencies.
- Updated `crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs` (`743` LoC), `http_gateway/mod.rs` (`pub(crate) mod scientia_feed` + `research_feed: Arc<ResearchFeedHub>` on `GatewayState`), and `http_gateway/ws.rs` (`RecvError::Lagged` -> `resync_required`, `"research.session.delta"` subscription replay), sharing `server_state.db` over a bounded `mpsc(512)` `TelemetryBatcher` and `1024`-entry replay ring.
- Created `crates/vox-orchestrator-mcp/src/gui_ops/research_stream.rs` (`396` LoC, `< 400`) serving `/research/sessions/{id}/events?from_seq=`, paginated `/claims` & `/sources`, and verified `/chunks/{hash}` (`vox_crypto::compliance_hash`), while compacting `get_research_session_detail` in `gui_ops/research.rs`.

### Task 2: Out-of-Core `FrameChunkCursor` & `/research/frame-chunks/{stream_id}` (`STRM-02`, `STRM-03`)
- Created `crates/vox-python-bridge/src/frame_stream.rs` (`391` LoC, `< 400`) and exported it in `crates/vox-python-bridge/src/lib.rs`, providing `FrameChunkCursor` (`from_arrow_ipc_reader`, `from_arrow_ipc_bytes`, `from_csv_reader`, `next_chunk`, `read_page`) with `1..=5_000` batch clamping (`DEFAULT_FRAME_BATCH_SIZE = 250`) and sliding-window backpressure.
- Created `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs` (`359` LoC, `< 400`) exporting `FrameStreamRegistry`, `global_frame_stream_registry`, `validate_stream_id`, and `get_research_frame_chunks` (`vox_crypto::secure_hash`, `limit` clamped `1..=500`, `RESEARCH_SURFACES` route enforcement).
- Wired `pub mod frame_stream;` in `gui_ops/mod.rs` and `.route("/research/frame-chunks/{stream_id}", get(super::frame_stream::get_research_frame_chunks))` into `gui_ops::research::api_v2_router()`.

### Task 3: Virtualized Multi-Wave Timeline, Clustered DAG & Tables (`STRM-04`)
- Updated `MultiWaveProgressTimeline.tsx` (`201` LoC) for dynamic `1..=5` wave templates (`data-testid="wave-stage-*"`), `ResearchDagCanvas.tsx` (`370` LoC) with `CLUSTER_THRESHOLD = 80` automatic wave clustering, and `ResearchView.tsx` (`'scholarly'` mode, `1..=5` wave selector, SSE `?from_seq=` updates + `resync_required` snapshot reload).
- Created `VirtualizedClaimsTable.tsx` (`263` LoC) and `VirtualizedFrameChunkTable.tsx` (`188` LoC) powered by `@tanstack/react-virtual` `useVirtualizer` with JSDOM-safe `initialRect` and `#claim-<id>` highlight/scroll support.
- Expanded `ResearchDagCanvas.test.tsx` (`102` LoC, `4` tests) and `ResearchView.test.tsx` (`859` LoC, `33` tests) covering 250-node wave clustering, 5,000-claim DOM-bounded virtualization, 5,000-row frame chunk streaming, and `?from_seq=` / `resync_required` recovery.

## Verification Receipts (Observed Outputs)

1. **`vox-search --lib`:** `275 passed; 0 failed; 1 ignored; 0 measured; finished in 31.27s` (`chunk_spill::tests::*` `5/5` passed; `deep_lane_probe_searxng_vs_tavily_subquery` is pre-existing `#[ignore]` live network probe).
2. **`vox-python-bridge --lib`:** `15 passed; 0 failed; 0 ignored; 0 measured; finished in 0.02s` (both 10k-row `arrow_ipc` and `csv` `frame_stream::tests` passed).
3. **`vox-orchestrator-mcp --lib gui_ops`:** `151 passed; 0 failed; 0 ignored; 900 filtered out; finished in 36.92s` (includes `frame_stream`, `research_stream::sse_replay_resync_and_chunk_guard`, `detail_spills_50x100_artifact_under_32kb_and_serves_chunk_and_pages`, and `db_backed_claims_and_sources_paginate_with_offset`).
4. **`./target/debug/deps/vox_orchestrator_mcp-caf61828b67e474b http_gateway`:** `87 passed; 0 failed; 0 ignored; 964 filtered out; finished in 31.47s` (includes `scientia_feed::tests::*` and `ws::envelope_parity_tests::*`).
5. **Frontend TypeScript & Combined Vitest:**
   - `pnpm --dir crates/vox-gui/ui exec tsc --noEmit`: `exit 0`
   - Combined Vitest (`7` files): `64 passed; 0 failed` (`ResearchDagCanvas.test.tsx`: `4`, `ResearchView.test.tsx`: `33`, `noRawNativeControlsGuard.test.ts`: `2`, `zeroEmojiAndDingbat.test.ts`: `2`, `transportIpcGuard.test.ts`: `2`, plus Phase 448 UI tests `21`).
6. **Architecture & Static Guards:**
   - All new `.rs`/`.tsx` modules `< 400` LoC (`chunk_spill.rs`: `387`, `research_stream.rs`: `396`, `gui_ops/frame_stream.rs`: `359`, `vox-python-bridge/frame_stream.rs`: `391`, `MultiWaveProgressTimeline.tsx`: `201`, `ResearchDagCanvas.tsx`: `370`, `VirtualizedClaimsTable.tsx`: `263`, `VirtualizedFrameChunkTable.tsx`: `188`).
   - `rg` banned OS paths (`"/tmp|"/var|"/home`), direct `sha2`/`sha3`/`blake3` imports, and `crates/vox-gui/src/` DB handle check: `0` Phase 449 violations.
   - `cargo run -p vox-arch-check`: exited `1` solely due to 1 unrelated concurrent/pre-existing violation (`crates/vox-gui/src/commands/pty/tests.rs:141 — "/tmp`); `0` Phase 449 violations (`12` pre-existing release-delta LoC warnings).

---
*Phase: 449-extreme-data-streaming-and-epistemic-canvas*
*Completed: 2026-11-15*
