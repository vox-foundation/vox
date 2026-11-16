---
phase: 449
title: "Extreme-Data Streaming and Epistemic Canvas"
slug: "extreme-data-streaming-and-epistemic-canvas"
milestone: "scientia-deep-research-v76"
status: "ready"
---

# Phase 449: Extreme-Data Streaming and Epistemic Canvas — Context

**Gathered:** 2026-11-15
**Status:** Ready for planning
**Requirements:** STRM-01, STRM-02, STRM-03, STRM-04

<domain>
## Phase Boundary

Phase 449 eliminates 5-second `VoxDb::connect_default()` polling, bloated multi-megabyte session detail IPC payloads, eager full-dataset `std.frame` loads, and non-virtualized research UI rendering across 4 tracks:
- **Telemetry Batching & Resumable Deltas (`STRM-01`):** Replace `spawn_scientia_queue_poller`'s `VoxDb::connect_default()` polling in `http_gateway/scientia_feed.rs` + `http_gateway/mod.rs` and per-event tokio task spawning with a bounded `mpsc(512)` telemetry batcher reusing `ServerState.db`, plus resumable `?from_seq=` SSE/WS deltas (`"research.session.delta"`) with `resync_required` on `RecvError::Lagged`.
- **Content-Addressed Chunk Spill & Paginated Session Detail (`STRM-02`):** Spill payloads `> 16 KiB` out-of-line in `crates/vox-search/src/chunk_spill.rs`, persist lightweight `content_hash` + `byte_len` pointers in `research_session_sources`, and paginate claims/sources/chunks so `GET /api/v2/research/sessions/{id}` stays strictly `< 256 KiB` (verified on 50 sources × 100 claims).
- **Bounded-RAM `std.frame` Chunk Streaming (`STRM-03`):** Add `O(batch_size)` chunk cursor over Arrow IPC `StreamReader` / line-bounded CSV files in `crates/vox-python-bridge/src/frame_stream.rs` and `GET /api/v2/research/frame-chunks/{stream_id}` in `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs`.
- **Virtualized Research Canvas & Tables (`STRM-04`):** Upgrade `MultiWaveProgressTimeline.tsx`, `ResearchDagCanvas.tsx`, `VirtualizedClaimsTable.tsx`, `VirtualizedFrameChunkTable.tsx`, and `ResearchView.tsx` using `@tanstack/react-virtual`, `> 80`-node wave clustering, dynamic wave count (`1..=5`), and all 4 domain modes (`general | shopping | codegen | scholarly`).

</domain>

<decisions>
## Implementation Decisions

### BLAKE3 & `crate-edges` Compliance for `chunk_spill.rs` (`STRM-02`)
- **Critical repo rule discovery:** `vox-search/Cargo.toml` currently lists `blake3` in `[dependencies]` (line 39) and `vox-crypto` ONLY in `[dev-dependencies]` (line 58), and `contracts/ci/crate-edges.allow.v1.json` does **not** have a `["vox-search", "vox-crypto"]` normal-dependency edge (adding an allowlist entry is USER-AUTHORIZED-ONLY). Meanwhile, `AGENTS.md` bans direct `blake3` imports outside `crates/vox-crypto/`, and `vox-orchestrator-mcp/Cargo.toml` (line 103) **already** depends on `vox-crypto` in `[dependencies]`.
- **Resolution without Cargo/allowlist changes:**
  - `crates/vox-search/src/chunk_spill.rs` (`< 400` LoC) implements `ChunkSpillStore` / `SpillPointer` keyed by pre-computed `[u8; 32]` / 64-char hex BLAKE3 digest (`SpillKey` / `spill_with_digest(&[u8], &[u8; 32])` or accepting a `hash_fn: fn(&[u8]) -> [u8; 32]` callback) — zero `blake3` imports in `vox-search`, zero edits to `Cargo.toml` or `contracts/ci/crate-edges.allow.v1.json`!
  - Callers in `vox-orchestrator-mcp` (`gui_ops/research.rs` / `research_stream.rs`) invoke `vox_crypto::secure_hash` and `vox_crypto::hex_encode` (using `vox-crypto`'s canonical BLAKE3 API) when spilling `raw_content` / `artifact_json` chunks `> 16 KiB` into `ChunkSpillStore`, and `vox-search` unit tests use `vox_crypto::secure_hash` directly via `[dev-dependencies]`.
- **Path hygiene:** Spill root defaults to `std::env::temp_dir().join("vox-research-spill")` (or caller-provided workspace path via `vox_bounded_fs`) — NEVER literal `"/tmp"`, `"/var"`, or `"/home"`.

### Bounded Telemetry Batcher & Resumable SSE/WS (`STRM-01`)
- **Zero `VoxDb::connect_default()` polling:** Remove `VoxDb::connect_default()` from `scientia_feed.rs` (`spawn_scientia_queue_poller`) and reuse `state.db` (`Option<Arc<VoxDb>>`) with a bounded `tokio::sync::mpsc::channel(512)` writer batcher + `tokio::sync::Notify` / event-driven wakeups.
- **Resumable replay buffer:** Keep a bounded ring buffer (e.g., `VecDeque<SequencedResearchEvent>`, capacity 1024 per active session or global ring) paired with `state.research_events` (`broadcast::Sender`).
  - `GET /api/v2/research/sessions/{id}/events?from_seq=N` (SSE) replays events `seq > N` if still in buffer; if `N` precedes the oldest retained event (`oldest_seq - 1`), emit an immediate `resync_required` SSE event (`{ type: "resync_required", session_id, oldest_available_seq, latest_seq }`) before streaming live events.
  - `/v1/ws` topic `"research.session.delta"` (`ws.rs` + `scientia_feed.rs`) supports `from_seq` replay and maps `broadcast::error::RecvError::Lagged(skipped)` to a `resync_required` frame instead of silently dropping events.

### Paginated Session Detail (`< 256 KiB` Cap, `STRM-02`)
- `get_research_session_detail` (`GET /api/v2/research/sessions/{id}`) returns bounded summary fields: `session`, capped preview claims/citations (or page 0 summary metadata + `claims_total` / `sources_total` + `has_spilled_artifact`), omitting unbounded `raw_content` and full `artifact_json` when payload exceeds threshold so `probe_session_detail_ipc_payload_bytes` (50 sources × 100 claims) measures strictly `< 256 * 1024` bytes (`< 256 KiB`).
- Dedicated paginated endpoints in `gui_ops/research_stream.rs`:
  - `GET /api/v2/research/sessions/{id}/claims?offset=0&limit=100` (backed by `VoxDb::list_session_claims_page`)
  - `GET /api/v2/research/sessions/{id}/sources?offset=0&limit=50` (backed by `VoxDb::list_session_sources_page`)
  - `GET /api/v2/research/sessions/{id}/chunks/{hash}` (serves spilled content-addressed chunks via `ChunkSpillStore` / `vox-bounded-fs`)

### `std.frame` Streaming Without New Cargo Deps (`STRM-03`)
- `vox-python-bridge/Cargo.toml` already has `arrow-array = "=60.1.0"`, `arrow-ipc = "=60.1.0"`, `serde`, `serde_json`, `tokio`. Implement `FrameChunkCursor` in `crates/vox-python-bridge/src/frame_stream.rs` (`< 400` LoC) supporting:
  - Arrow IPC stream incremental batch reading (`arrow_ipc::reader::StreamReader` yielding one `RecordBatch` / `batch_size` slice per `next_chunk()` in `O(batch_size)` RAM)
  - Bounded line-chunked CSV reader (`std::io::BufReader` + `lines()` with RFC-4180 quote handling)
- Expose `GET /api/v2/research/frame-chunks/{stream_id}?offset=&limit=` in `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs` (`< 400` LoC). Leave the 2 router/module wiring lines in `gui_ops/mod.rs` & `gui_ops/research.rs` to Wave C (`rust-verifier`) so `p449-stream` and `p449-frame` never touch the same file.

### Epistemic Canvas & Virtualized UI (`STRM-04`)
- Reuse existing `@tanstack/react-virtual`, `radix-ui`, `lucide-react`, and internal `<Icon.*>` / `<BentoScrollRegion>` — zero new npm dependencies.
- `ResearchDagCanvas.tsx`: when `nodes.length > 80`, group nodes by `wave` into collapsible wave cluster cards with aggregate verified/contested/contradicted counts and expand/collapse toggles (so initial collapsed DOM node count stays well under 150 even at 1,000+ nodes).
- `VirtualizedClaimsTable.tsx` (`< 400` LoC) and `VirtualizedFrameChunkTable.tsx` (`< 400` LoC): virtualized scrolling via `useVirtualizer` from `@tanstack/react-virtual` capable of rendering 5,000+ rows smoothly.
- `MultiWaveProgressTimeline.tsx` & `ResearchView.tsx`: dynamic wave count `1..=5` driven by session `wave_count` / live SSE delta stream (replacing hardcoded 3-wave array and `currentWave={1}`), all 4 domain modes `['general', 'shopping', 'codegen', 'scholarly']`, and `resync_required` recovery banner/refetch.
- Strict UI guards: zero raw `<select>`, zero `<input type="range">`, zero emojis/dingbats (`noRawNativeControlsGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`).

</decisions>

<code_context>
## Existing Code Insights

### Reusable Pieces from Phase 447
- `crates/vox-research-events/src/events.rs` (392 LoC): `SequencedResearchEvent`, `ResearchEventSequencer`, `WaveStarted`, `WaveCompleted`, `SubqueryDispatched`, `ContinuumEscalated`, `EpistemicDagDelta`.
- `crates/vox-db/src/research_session_pages.rs` (391 LoC): `upsert_session_source`, `list_session_claims_page`, `list_session_sources_page`.
- `crates/vox-crypto/src/facades.rs`: `vox_crypto::secure_hash(&[u8]) -> [u8; 32]`, `vox_crypto::hex_encode(&[u8]) -> String` (BLAKE3).

### Verified Pain Points in Current Code
- `crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs:59,122`: calls `VoxDb::connect_default()` inside `spawn_scientia_queue_poller` every 5s.
- `crates/vox-orchestrator-mcp/src/gui_ops/research.rs:1133-1187`: `get_research_session_detail` returns full `artifact_json` + all claims + full `epistemic_dag` synchronously.
- `crates/vox-gui/ui/src/components/surfaces/Research/MultiWaveProgressTimeline.tsx:26-64`: hardcoded 3-wave array; `ResearchView.tsx:695,745,787`: missing `'scholarly'`, hardcoded `[1, 2, 3]`, hardcoded `currentWave={1}`.
- `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx`: renders every node and SVG `<path>` unconditionally.

</code_context>

<specifics>
## Specific Ideas & Parallel Wave Boundaries

- **Wave B parallel file-disjoint workers:**
  1. `p449-stream`: `crates/vox-search/src/chunk_spill.rs`, `crates/vox-search/src/lib.rs`, `crates/vox-orchestrator-mcp/src/http_gateway/scientia_feed.rs`, `crates/vox-orchestrator-mcp/src/http_gateway/mod.rs`, `crates/vox-orchestrator-mcp/src/http_gateway/ws.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/research.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/research_stream.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/mod.rs`.
  2. `p449-frame`: `crates/vox-python-bridge/src/frame_stream.rs`, `crates/vox-python-bridge/src/lib.rs`, `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs` (does NOT touch `gui_ops/mod.rs` or `gui_ops/research.rs`).
  3. `p449-ui`: `MultiWaveProgressTimeline.tsx`, `ResearchDagCanvas.tsx`, `ResearchDagCanvas.test.tsx`, `VirtualizedClaimsTable.tsx`, `VirtualizedFrameChunkTable.tsx`, `ResearchView.tsx`, `ResearchView.test.tsx`.
- **Wave C (`rust-verifier`)**: adds the 2 wiring lines in `gui_ops/mod.rs` (`pub mod frame_stream;`) and `gui_ops/research.rs` (`.route("/research/frame-chunks/{stream_id}", ...)`), and runs serialized Cargo tests + UI TypeScript/Vitest/guard checks.

</specifics>

<deferred>
## Deferred Ideas

- Client-side IndexedDB offline cache for spilled chunks (Phase 449 serves paginated chunks on-demand over local HTTP/IPC).
- Cross-node cluster replication of spilled chunks over Populi mesh.

</deferred>

---

*Phase: 449-extreme-data-streaming-and-epistemic-canvas*
*Context gathered: 2026-11-15*
