# Requirements: Vox

**Defined:** 2026-09-22
**Core Value:** The compiler, orchestrator, and runtime that everything else depends on must keep building and passing CI throughout this cleanup — no crate disposition or architecture-decision closure is worth a broken workspace.

**Source:** Full-corpus ADR/SPEC/PRD ingest (`.planning/intel/SYNTHESIS.md`, `decisions.md`, `requirements.md`, `constraints.md`). The dead-crate-* requirements are extracted verbatim (per-fate grouping, not per-crate) from the one classified PRD (`docs/src/architecture/dead-crate-fate-plan-2026-05-08.md`). The GUI/MESH/TRUST/MODEL/ML requirements are mined from non-locked, "current but not yet closed" ADRs per the user's explicit BROAD-scope instruction — see PROJECT.md Key Decisions for rationale per ADR.

## v1 Requirements

### Dead Crate Disposition

- [x] **REQ-dead-crate-delete**: Delete workspace crates with zero consumers and no remaining conceptual fit, consolidating any worthwhile types into their true owning crate first.
  Acceptance: DELETE vox-schola (remove crate + workspace member; verify `cargo tree -p vox-cli` has no reference). DELETE vox-scientia-core and vox-scientia-social (pure pass-through facades over vox-publisher; verify no `vox_scientia_core::*`/`vox_scientia_social::*` imports remain). DELETE vox-scientia-ingest, but only after removing its mandatory `vox-cli` dependency first (audit usage, inline into vox-publisher's scholarly-external-jobs feature or gate behind a flag, then delete). DELETE vox-socrates-policy after migrating `ConfidencePolicy`/`ComplexityBand`/`RiskBand` types into vox-orchestrator-types. DELETE vox-spool (zero consumers; inline the JSONL helper elsewhere if needed). DELETE vox-tools (superseded by vox-capability-registry + vox-plugin-host dispatch pattern). DELETE vox-mcp-meta after migrating `A2A_MESSAGE_TYPES` constants into vox-orchestrator-types and wiring vox-mcp-registry directly. DELETE vox-browser and vox-audio-ingress (listed in executive summary; no consumers).
- [x] **REQ-dead-crate-wire-up**: Wire up functionally-complete but never-adopted crates into their intended call path instead of deleting working code.
  Acceptance: WIRE-UP vox-exec-grammar — add dependency from vox-container (or vox-cli-core) and call `vox_exec_grammar::risk::classify` inside the existing exec-policy gate per the ADR-026 contract (`contracts/terminal/exec-policy.v1.yaml`). WIRE-UP vox-mcp-registry — add dependency from vox-orchestrator, use `TOOL_REGISTRY` to validate/enumerate MCP tool names, then delete the now-redundant vox-mcp-meta wrapper. RECLASSIFY vox-search from DEAD to CORE (documentation-only correction: vox-cli and vox-orchestrator depend on it unconditionally; it is already wired). RECLASSIFY vox-doc-inventory from DEAD to CORE (vox-cli/Cargo.toml line 138 depends on it unconditionally via two binary targets).
- [x] **REQ-dead-crate-extract-to-plugin**: Move complete-but-CORE-inappropriate crates into the plugin architecture so their functionality survives without bloating the default CLI compile.
  Acceptance: Grammar export: amended 2026-09-25 (Phase 3 D-11) — vox-grammar-export stays a CORE library per crate-audit D-4/D-18, which deleted the plugin pass-through and its ABI extension and supersedes the PRD's EXTRACT; closed by removing its zero-consumer automaton module and vox-populi's unused dependency. Webhook: amended 2026-09-25 (Phase 3 D-10/D-13) — vox-plugin-webhook exposes a WebhookInbox poll extension; vox-orchestrator-mcp's opt-in poller loads it through vox-plugin-host and submits events to the hopper as IntakeSource::Webhook, re-deriving the OrchestratorInboxItem kind routing locally; no crate depends on the plugin.
- [x] **REQ-dead-crate-keep-frozen**: Explicitly preserve crates whose disposition is to remain as-is without further action in this plan's scope.
  Acceptance: KEEP-FROZEN vox-workflow-runtime, vox-integration-tests, vox-test-harness — no action required by this plan.
- [x] **REQ-dead-crate-misplaced**: Resolve the two crates flagged MISPLACED (wrong architectural tier) rather than DEAD (unused).
  Acceptance: vox-ssg: amended 2026-09-25 (Phase 3 D-05) — not a CORE crate; folded into vox-cli/src/utils/ssg (layer 4) by 9d385a60b. Complete the in-progress vox-oratio extraction (already underway; finishing it removes the last direct Candle dependency bleed from CORE).
- [x] **REQ-dead-crate-catalog-cleanup**: Confirm catalog-only ghost entries (already removed from the actual workspace) are fully reflected in catalog.toml with no further code action needed.
  Acceptance: Verify execution-api and stub-check are absent from the workspace and any remaining catalog.toml comments accurately describe them as already cleaned up.

### GUI/Dashboard Architecture

- [x] **GUI-01**: ADR-045 (Tauri GUI Replaces Axum Dashboard) is formally ratified — an explicit "Status: Accepted" line and locked-equivalent classification replace its current medium-confidence "current" status.
- [x] **GUI-02**: `vox-gui`'s command surface is verified fully sourced from `vox-cli`'s `CommandCatalog` SSOT, with no orphaned or duplicated command definitions between the two surfaces.
- [x] **GUI-03**: Vox-native and React/TanStack interop UI primitive tracks have an explicit, documented, enforced boundary per `external-frontend-interop-plan-2026.md` (ADR-027 is superseded as of 2026-05-03).
- [x] **GUI-04**: The Tauri 2 desktop-convergence clause of ADR-037 is confirmed complete and Accepted, independent of the already-superseded mobile clause (adr-NNN).

### Multi-Agent Trust

- [x] **MESH-01**: The locks subsystem is extended with `ResourceLockManager` for multi-agent resource coordination, lease propagation, and contention handling per ADR-025.
- [x] **TRUST-01**: Agent tool calls emit cryptographically verifiable HMAC receipts, checkable under the two-tier formal-intent verification system defined in ADR-029.

### Model Routing & ML Health

- [ ] **MODEL-01**: Model scoreboards present a Pareto-frontier view over reliability, cost, and latency (reporting-only — no routing behavior change) per ADR-046.
- [ ] **ML-01**: Candle, peft-rs, and qlora-rs dependency versions are unified across the workspace via a dedicated upgrade train and validated by fail-closed CUDA-toolchain compilation on GitHub-hosted `ubuntu-latest`; physical-GPU runtime coverage is explicitly out of scope for this closure, per ADR-034.

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Hosted Mens / BaaS

- **MENS-BAAS-01**: Managed, hosted Mens service with OAuth device-flow auth and multi-tenant, org-bound scoping (ADR-009). Explicitly scoped as future design in its own source doc; no concrete hosting driver yet.

### Compiler Track

- **WEBIR-01**: Internal Web IR as a standalone initiative (ADR-012). Direction already subsumed by locked ADR-036 (HIR core + WebIR projections); tracked as ongoing compiler-track work, not a discrete milestone deliverable.

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| SWC parser migration (ADR-035) | Evaluation-only; the ADR itself requires a fresh sign-off ADR before any migration — no committed direction to build toward. |
| Re-litigating ADR-030/031 "vox-dashboard" naming | Both ADRs' core decisions (state_machine as reactive-state SSoT; vox-vscode deprecation) remain valid and locked — only their prose still names the retired `vox-dashboard` surface. Operator-reviewed, no source edit required; downstream consumers treat `vox-gui` as current per synthesis notes. |
| Transport crypto provider collapse (`reqwest` 0.12→0.13, `ring`/`aws-lc-rs` dedup) | Real and unresolved (`contracts/crypto/transport-providers.v1.json`), but it's a codebase-audit (CONCERNS.md) finding, not an ADR/SPEC-sourced decision — outside this ingest's mining scope. Future milestone. |
| God-object file decomposition (549 findings, stale 2026-06-06 scan) | Codebase-audit tech debt, not an ADR/SPEC decision; re-scan and prioritize separately. |
| Mesh transport default posture (ADR-008/009/020) | Already superseded in practice by locked ADR-047 (iroh QUIC); no open work remains. |
| Stub durability/scheduling grammar removal (ADR-028) | Superseded by locked ADR-041, which closed the gap by shipping a working runtime instead of removing the grammar. |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| REQ-dead-crate-delete | Phase 1 | Complete |
| REQ-dead-crate-catalog-cleanup | Phase 1 | Complete |
| REQ-dead-crate-keep-frozen | Phase 1 | Complete |
| REQ-dead-crate-wire-up | Phase 2 | Complete |
| REQ-dead-crate-extract-to-plugin | Phase 3 | Complete |
| REQ-dead-crate-misplaced | Phase 3 | Complete |
| GUI-01 | Phase 4 | Complete |
| GUI-02 | Phase 4 | Complete |
| GUI-03 | Phase 4 | Complete |
| GUI-04 | Phase 4 | Complete |
| MESH-01 | Phase 5 | Complete |
| TRUST-01 | Phase 5 | Complete |
| MODEL-01 | Phase 6 | Pending |
| ML-01 | Phase 6 | Pending |

**Coverage:**

- v1 requirements: 14 total
- Mapped to phases: 14
- Unmapped: 0 ✓

---
*Requirements defined: 2026-09-22*
*Last updated: 2026-09-22 after initial roadmap creation*
