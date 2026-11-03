---
phase: 53-scientia-publication-pipeline-closure
plan: 04
type: summary
status: complete
completed_at: "2026-10-14"
requirements:
  - PUB-04
files_modified:
  - crates/vox-scientia/src/review_flow.rs
  - crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs
  - crates/vox-cli/src/commands/scientia_phase_handlers.rs
---

# Summary 53-04: Honest Scientia Dashboard Queue Snapshot, Real Confidence Signals & Replay Write-Back

## Delivered
1. **Shared DB-Backed Queue Snapshot & Cost Rollup SSOT ([`review_flow.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs))**:
   - Added [`assemble_scientia_queue_from_db`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs#L1150-L1254) merging `publication_manifests` and unpromoted `scientia_finding_candidates`, computing real confidence scores from `confidence_json` / `scientia_evidence` / verified claim ratios, and populating `manifests_in_reply_window` using `reply_window_days_for`.
   - Added [`record_replay_for_publication`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs#L1116-L1147) so `replay_execute` and `/api/v2/scientia/publications/{id}/replay` write `artifact_replayability` into both `publication_status_events` and `metadata_json.scientia_evidence.artifact_replayability`.
2. **Unified CLI & HTTP Gateway Wiring ([`dashboard_api.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/http_gateway/dashboard_api.rs), [`scientia.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs), [`scientia_phase_handlers.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/scientia_phase_handlers.rs))**:
   - Wired `GET /api/v2/scientia/queue`, `GET /api/v2/scientia/cost`, `GET /api/v2/scientia/dashboard`, and `vox scientia dashboard` to the shared SSOT functions.

## Verification
- `timeout 900s cargo test -p vox-orchestrator-mcp --lib gui_ops::scientia`
