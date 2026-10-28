---
phase: 53-scientia-publication-pipeline-closure
plan: 03
type: summary
status: complete
completed_at: "2026-10-14"
requirements:
  - PUB-03
files_modified:
  - crates/vox-scientia/src/claim_extractor/minicheck.rs
  - crates/vox-scientia/src/claim_extractor/pipeline.rs
  - crates/vox-cli/src/commands/scientia_phase_handlers.rs
---

# Summary 53-03: Calibrated Hybrid Claim Verifier (Lexical + Numeric Conflict + Model-Agnostic LLM Entailment)

## Delivered
1. **Calibrated Hybrid MiniCheck Verifier ([`minicheck.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/claim_extractor/minicheck.rs) & [`pipeline.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/claim_extractor/pipeline.rs))**:
   - Added [`MiniCheckBackend::CalibratedHybrid`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/claim_extractor/minicheck.rs#L65-L70) and [`MiniCheckVerifier::from_env_or_calibrated`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/claim_extractor/minicheck.rs#L118-L127).
   - Replaced raw token-overlap fallback with stopword-filtered lexical containment + numeric conflict detection + optional model-agnostic LLM entailment (`vox_actor_runtime::llm::llm_chat`) for borderline `[0.30, 0.70)` claims.
   - Added [`ExtractionPipeline::new_calibrated`](file:///Users/brbrainerd/dev/vox/crates/vox-scientia/src/claim_extractor/pipeline.rs#L71-L78) and wired `publication_extract_claims` in [`scientia_phase_handlers.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-cli/src/commands/scientia_phase_handlers.rs#L324-L355) and `/api/v2/scientia/publications/{id}/extract-claims` in [`scientia.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-orchestrator-mcp/src/gui_ops/scientia.rs#L894-L966).

## Verification
- `timeout 900s cargo test -p vox-scientia --lib claim_extractor` (`36 passed; 0 failed`)
