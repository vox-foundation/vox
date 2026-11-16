---
phase: 208
status: passed
verified_at: 2026-10-25T00:52:30Z
requirements_verified:
  - VOX-ROUTER-T3-01
  - VOX-ROUTER-T3-02
  - VOX-ROUTER-T3-03
  - VOX-ROUTER-T3-04
---

# Phase 208 Verification: Extract T3 Router Capabilities into Vox Model Router (`vox-orchestrator`, `vox-llm-egress`)

## Verification Evidence
1. **`VOX-ROUTER-T3-01` (Operational Status Gating):**
   - `t3_operational_status_outage_excluded_and_degraded_demoted` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).
2. **`VOX-ROUTER-T3-02` (Reasoning Effort & `:online` Grounding):**
   - `t3_controls_attach_reasoning_and_web_search_capabilities` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).
   - `t3_wire_options_normalizes_reasoning_effort_and_openrouter_online_suffix` in `crates/vox-llm-egress/src/lib.rs`: PASSED (`ok`).
3. **`VOX-ROUTER-T3-03` (Per-Turn Cost Delta & Tier Provenance):**
   - `t3_reconcile_turn_cost_delta_tracks_credits_and_provenance` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).
4. **`VOX-ROUTER-T3-04` (Router Verification Suite):**
   - `timeout 900s cargo test -p vox-llm-egress` → `31 passed; 0 failed` (lib) + `16 passed; 0 failed` (`wire_mock`).
   - `timeout 900s cargo test -p vox-orchestrator --lib models::select::semcov_wave1c_tests::t3_` → `5 passed; 0 failed`.
