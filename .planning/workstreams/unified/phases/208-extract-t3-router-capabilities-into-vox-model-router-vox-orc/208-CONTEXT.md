# Phase 208: Extract T3 Router Capabilities into Vox Model Router (vox-orchestrator::models & vox-llm-egress) - Context

**Gathered:** 2026-10-24
**Status:** Ready for planning
**Mode:** Autonomous (`/gsd-autonomous`)

<domain>
## Phase Boundary

Implement live operational status gating (`ModelOperationalStatus`), unified `ReasoningEffort` & search-grounding wire normalization (`T3WireOptions`, `with_t3_controls`), and per-turn credit/cost delta & `:free` vs. BYOK tier provenance (`CostProvenanceTier`, `TurnCostDeltaReceipt`) across `vox-orchestrator::models` and `vox-llm-egress`.

</domain>

<decisions>
## Implementation Decisions

- `VOX-ROUTER-T3-01`: `ModelOperationalStatus` (`Operational`, `Degraded`, `Outage`) in `models/spec.rs` and `models/registry.rs`, gated in `models/select.rs` (`decide`, `select_local_first`, `select_via_premium_alias`, `select_via_scorer`).
- `VOX-ROUTER-T3-02`: `ReasoningEffort` + `ModelSelectionRequest::with_t3_controls` in `vox-orchestrator` and `T3WireOptions` + `normalize_egress_request_for_t3_options` + `build_request_with_t3_options` in `vox-llm-egress`.
- `VOX-ROUTER-T3-03`: `CostProvenanceTier` (`LocalZeroCost`, `FreeTier`, `Byok`, `PlatformCredits`) + `TurnCostDeltaReceipt` + `ModelRegistry::reconcile_turn_cost_delta`.

</decisions>
