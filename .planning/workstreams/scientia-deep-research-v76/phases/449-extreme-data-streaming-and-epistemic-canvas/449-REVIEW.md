---
phase: 449-extreme-data-streaming-and-epistemic-canvas
review_scope: plan 449-01 (Tasks 1, 2, 3 & Wave C wiring)
reviewed_at: 2026-11-15
reviewer: rust-verifier
status: approved_with_notes
---

# Phase 449 Code & Architecture Review

## Scope Reviewed

- **Chunk Spill & Bounded SSE/WS Streaming (`STRM-01`, `STRM-02`):** `crates/vox-search/src/{chunk_spill.rs,lib.rs}`, `crates/vox-orchestrator-mcp/src/http_gateway/{mod.rs,scientia_feed.rs,ws.rs}`, `crates/vox-orchestrator-mcp/src/gui_ops/{mod.rs,research_stream.rs,research.rs}`.
- **`FrameChunkCursor` & MCP Frame Endpoint (`STRM-02`, `STRM-03`):** `crates/vox-python-bridge/src/{frame_stream.rs,lib.rs}`, `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs`.
- **Axis Virtualized Research Canvas (`STRM-04`):** `crates/vox-gui/ui/src/components/surfaces/Research/{MultiWaveProgressTimeline.tsx,ResearchDagCanvas.tsx,ResearchDagCanvas.test.tsx,VirtualizedClaimsTable.tsx,VirtualizedFrameChunkTable.tsx,ResearchView.tsx,ResearchView.test.tsx}`.

## Findings Summary

- **Blocking Issues:** `0`
- **Phase 449 Architectural / Guard Violations:** `0`
- **Non-Blocking Findings:** `2` noted below (unrelated `pty/tests.rs:141` arch-check failure, 3 dead-code telemetry helper warnings in `scientia_feed.rs`)

## 1. Memory Bounds, Streaming Contracts & Security

- **Spill & Payload Safety (`T-447`/STRM):** `chunk_spill.rs` enforces digest hex validation (`is_valid_digest_hex`) and rejects path traversal digests (`rejects_invalid_and_traversal_digests`). `research_stream.rs` uses `vox_crypto::compliance_hash` + `vox_crypto::hex_encode` and returns `400 invalid_digest` on malformed hashes; `frame_stream.rs` uses `vox_crypto::secure_hash`, enforces `validate_stream_id` (`handler_serves_paginated_chunks_and_rejects_traversal_ids`), clamps `limit` to `1..=500`, and gates routes via `enforce_route(&gs, &connect.0, &headers, RESEARCH_SURFACES)`.
- **Bounded Memory & Backpressure:** `TelemetryBatcher` uses bounded `mpsc(512)` reusing `ServerState.db` (zero `VoxDb::connect_default()` polling). `ResearchFeedHub` bounds replay history at `1024` entries and emits explicit `resync_required` envelopes on ring wrap or `RecvError::Lagged`. `FrameChunkCursor` streams 10k+ Arrow IPC and CSV rows in `O(batch_size)` memory (`arrow_ipc_10k_rows_streams_and_enforces_sliding_window_backpressure`, `csv_10k_rows_streams_in_250_row_chunks_with_exact_termination`).
- **UI Virtualization & Source Guards:** `VirtualizedClaimsTable.tsx` & `VirtualizedFrameChunkTable.tsx` window 5,000+ rows cleanly using `@tanstack/react-virtual` (`< 50` mounted DOM rows verified in JSDOM), `ResearchDagCanvas.tsx` clusters waves above `80` nodes, and `noRawNativeControlsGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`, and `transportIpcGuard.test.ts` all pass (`exit 0`).

## 2. File Size & Dependency Discipline

| New Module | Observed LoC | Budget | Status |
|---|---|---|---|
| `crates/vox-search/src/chunk_spill.rs` | `387` | `<400` | Pass |
| `crates/vox-orchestrator-mcp/src/gui_ops/research_stream.rs` | `396` | `<400` | Pass |
| `crates/vox-orchestrator-mcp/src/gui_ops/frame_stream.rs` | `359` | `<400` | Pass |
| `crates/vox-python-bridge/src/frame_stream.rs` | `391` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Research/MultiWaveProgressTimeline.tsx` | `201` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Research/ResearchDagCanvas.tsx` | `370` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Research/VirtualizedClaimsTable.tsx` | `263` | `<400` | Pass |
| `crates/vox-gui/ui/src/components/surfaces/Research/VirtualizedFrameChunkTable.tsx` | `188` | `<400` | Pass |

- Zero new Cargo crates or npm packages added; zero banned `"/tmp|"/var|"/home` literals, zero direct `sha2`/`sha3`/`blake3` imports, and zero `VoxDb` handles in `vox-gui`.

## 3. Non-Blocking Observations

1. **`vox-arch-check` exited `1` on unrelated `pty` file:** `cargo run -p vox-arch-check` flagged `[no-hardcoded-abs-path] crates/vox-gui/src/commands/pty/tests.rs:141 — "/tmp` outside Phase 448/449 scope; zero violations exist in any Phase 449 file.
2. **Dead-code telemetry inspector warnings (`scientia_feed.rs`):** `TelemetryBatcher::dropped_count` (`line 140`), `TelemetryBatcher::flushed_count` (`line 144`), and `ResearchFeedHub::telemetry` (`line 276`) emit `#[warn(dead_code)]` notices in non-test builds because they are currently read in unit tests only.

## Verdict

**APPROVED** — Phase 449 delivers bounded chunk spilling, sequenced SSE/WS delta replay with `resync_required` recovery, `O(batch_size)` `FrameChunkCursor` streaming, and virtualized Axis multi-wave/DAG/table views with all scoped backend and frontend test suites passing.
