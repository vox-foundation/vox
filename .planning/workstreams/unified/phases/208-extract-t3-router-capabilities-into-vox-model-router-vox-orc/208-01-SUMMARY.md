# Phase 208 Plan 01 Summary: Extract T3 Router Capabilities into Vox Model Router (`vox-orchestrator`, `vox-llm-egress`)

## Accomplishments
- **Live Operational Health Gating (`VOX-ROUTER-T3-01`):**
  - Added `ModelOperationalStatus` (`Operational`, `Degraded`, `Outage`) in `crates/vox-orchestrator/src/models/spec.rs`.
  - Added `operational_status` registry state and `set_operational_status`, `operational_status`, and `ingest_t3_status_indicator` methods on `ModelRegistry` (`crates/vox-orchestrator/src/models/registry.rs`).
  - Integrated operational status filtering into `decide()`, `select_local_first`, `select_via_premium_alias`, and `select_via_scorer` (`crates/vox-orchestrator/src/models/select.rs`): `Outage` candidates are unconditionally excluded, and `Degraded` candidates are demoted out of the primary candidate pool whenever `Operational` alternatives exist (retaining `Degraded` as a graceful fallback tier if all candidates are `Degraded`).
- **Unified Reasoning Effort & Search-Grounding Controls (`VOX-ROUTER-T3-02`):**
  - Added `ReasoningEffort` (`None`, `Low`, `Medium`, `High`) and `ModelSelectionRequest::with_t3_controls(reasoning_effort, enable_web_search)` in `crates/vox-orchestrator/src/models/select.rs`.
  - Added `ReasoningEffortWire`, `T3WireOptions`, `normalize_egress_request_for_t3_options` (idempotent `:online` OpenRouter model slug normalization), and `build_request_with_t3_options` (`reasoning_effort` wire serialization) in `crates/vox-llm-egress/src/{lib.rs,wire.rs}`.
- **Exact Per-Turn Cost Delta & Provenance Reconciliation (`VOX-ROUTER-T3-03`):**
  - Added `CostProvenanceTier` (`PlatformCredits`, `Byok`, `FreeTier`, `LocalZero`) and `TurnCostDeltaReceipt` in `crates/vox-orchestrator/src/models/spec.rs`.
  - Added `ModelRegistry::reconcile_turn_cost_delta` in `crates/vox-orchestrator/src/models/registry.rs`, computing exact credit/USD deltas and feeding observed post-turn cost back into `record_cost_observation`.
- **Automated Verification (`VOX-ROUTER-T3-04`):**
  - Verified with `timeout 900s cargo test -p vox-llm-egress` (`31 passed` in unit tests, `16 passed` in `wire_mock`) and `timeout 900s cargo test -p vox-orchestrator --lib models::select::semcov_wave1c_tests::t3_` (`5 passed`).
