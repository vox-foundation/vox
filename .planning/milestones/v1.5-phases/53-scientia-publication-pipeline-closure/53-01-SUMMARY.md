---
phase: 53-scientia-publication-pipeline-closure
plan: 01
type: summary
status: complete
completed_at: "2026-10-14"
requirements:
  - PUB-01
files_modified:
  - crates/vox-research-shim/src/research/discovery_bridge.rs
  - crates/vox-scientia/src/review_flow.rs
  - crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs
---

# Summary 53-01: Discovery Bridge Class & Title Inference + Candidate-to-Publication Promotion Bridge

## Delivered
1. **Rich Candidate Class, Title Hint, Contradiction Risk & Novelty Bundle Linking ([`discovery_bridge.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/discovery_bridge.rs))**:
   - Added [`infer_candidate_class`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/discovery_bridge.rs#L56-L91) to classify research runs into `AlgorithmicImprovement`, `ReproducibilityInfra`, `TelemetryTrust`, `PolicyGovernance`, or `Other`.
   - Added [`extract_title_hint`](file:///Users/brbrainerd/dev/vox/crates/vox-research-shim/src/research/discovery_bridge.rs#L95-L133) to extract human-meaningful headings or lead claims from `result.answer`.
   - Computed real `contradiction_risk` (`contradicted_claims / total_verifiable_claims`) and deterministic `novelty_evidence_bundle_id` (`format!("novelty-session-{}", session_id)`).
2. **Candidate-to-Publication Promotion Bridge ([`review_flow.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs) & [`scientia.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs))**:
   - Added [`promote_candidate_to_publication`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs#L738-L923) and [`resolve_manifest_or_promote`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/review_flow.rs#L714-L735) so `finding-*` candidate IDs and research sessions automatically promote into `publication_manifests`, seed `scientia_claims` + `scientia_claim_verdicts`, and acknowledge the `scientia_discovery_inbox` row.
   - Exposed `GET`/`POST /api/v2/scientia/candidates/{id}/promote` in [`scientia.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs#L693-L721).

## Verification
- `timeout 900s cargo test -p vox-research-shim --lib discovery_bridge` (`5 passed; 0 failed`)
- `timeout 900s cargo test -p vox-scientia --lib review_flow` (`14 passed; 0 failed`)
