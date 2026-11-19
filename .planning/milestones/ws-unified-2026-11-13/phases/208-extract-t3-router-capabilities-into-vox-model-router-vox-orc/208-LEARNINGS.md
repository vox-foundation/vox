---
phase: 208
phase_name: "extract-t3-router-capabilities-into-vox-model-router-vox-orc"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 5
  lessons: 2
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 208 Learnings: extract-t3-router-capabilities-into-vox-model-router-vox-orc

## Decisions

### Added ModelOperationalStatus (Operational, Degraded, Outage) in crates/v...
Added `ModelOperationalStatus` (`Operational`, `Degraded`, `Outage`) in `crates/vox-orchestrator/src/models/spec.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 208-01-SUMMARY.md

---

### Added operational_status registry state and set_operational_status, oper...
Added `operational_status` registry state and `set_operational_status`, `operational_status`, and `ingest_t3_status_indicator` methods on `ModelRegistry` (`crates/vox-orchestrator/src/models/registry.rs`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 208-01-SUMMARY.md

---

### Integrated operational status filtering into decide(), select_local_firs...
Integrated operational status filtering into `decide()`, `select_local_first`, `select_via_premium_alias`, and `select_via_scorer` (`crates/vox-orchestrator/src/models/select.rs`): `Outage` candidates are unconditionally excluded, and `Degraded` candidates are demoted out of the primary candidate pool whenever `Operational` alternatives exist (retaining `Degraded` as a graceful fallback tier if all candidates are `Degraded`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 208-01-SUMMARY.md

---

### Added ReasoningEffort (None, Low, Medium, High) and ModelSelectionReques...
Added `ReasoningEffort` (`None`, `Low`, `Medium`, `High`) and `ModelSelectionRequest::with_t3_controls(reasoning_effort, enable_web_search)` in `crates/vox-orchestrator/src/models/select.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 208-01-SUMMARY.md

---

### Added ReasoningEffortWire, T3WireOptions, normalize_egress_request_for_t...
Added `ReasoningEffortWire`, `T3WireOptions`, `normalize_egress_request_for_t3_options` (idempotent `:online` OpenRouter model slug normalization), and `build_request_with_t3_options` (`reasoning_effort` wire serialization) in `crates/vox-llm-egress/src/{lib.rs,wire.rs}`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 208-01-SUMMARY.md

## Lessons

### t3_operational_status_outage_excluded_and_degraded_demoted in crates/vox...
`t3_operational_status_outage_excluded_and_degraded_demoted` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).

**Context:** Verified in phase acceptance checks.
**Source:** 208-VERIFICATION.md

---

### t3_controls_attach_reasoning_and_web_search_capabilities in crates/vox-o...
`t3_controls_attach_reasoning_and_web_search_capabilities` in `crates/vox-orchestrator/src/models/select.rs`: PASSED (`ok`).

**Context:** Verified in phase acceptance checks.
**Source:** 208-VERIFICATION.md

## Patterns

## Surprises

