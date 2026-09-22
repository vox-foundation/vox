# Roadmap: Vox

## Overview

Vox is a mature, working system; this roadmap is not a build-from-zero journey but a closure pass. It starts with the lowest-risk work (deleting confirmed-dead crates and confirming catalog hygiene), moves through progressively more invasive crate surgery (activating dormant code, then extracting misplaced crates into the plugin architecture), and finishes by formally closing out four clusters of "current but not yet locked" architecture decisions that a full-corpus ADR/SPEC ingest surfaced as open: GUI/dashboard architecture, multi-agent trust, and model/ML routing health.

## Phases

**Phase Numbering:**
- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [ ] **Phase 1: Dead Crate Cleanup — Remove & Confirm** - Delete zero-consumer crates, confirm frozen crates need no action, verify catalog hygiene
- [ ] **Phase 2: Wire Up & Reclassify Dormant Crates** - Activate functionally-complete but never-adopted crates in their intended call path
- [ ] **Phase 3: Extract Misplaced Crates to Plugin Architecture** - Move CORE-inappropriate crates into the plugin system
- [ ] **Phase 4: GUI/Dashboard Architecture Consolidation** - Ratify ADR-045, verify CommandCatalog SSOT alignment, enforce the Vox-native/React interop UI boundary, confirm Tauri desktop convergence
- [ ] **Phase 5: Multi-Agent Coordination & Trust Hardening** - Ship ResourceLockManager and HMAC tool-call receipts
- [ ] **Phase 6: Model Routing Transparency & ML Dependency Health** - Ship Pareto-frontier model reporting and unify the Candle/QLoRA dependency stack

## Phase Details

### Phase 1: Dead Crate Cleanup — Remove & Confirm
**Goal**: The workspace no longer carries zero-consumer crates or stale catalog entries; frozen crates are explicitly documented as intentionally inactive.
**Depends on**: Nothing (first phase)
**Requirements**: REQ-dead-crate-delete, REQ-dead-crate-catalog-cleanup, REQ-dead-crate-keep-frozen
**Success Criteria** (what must be TRUE):
  1. `cargo tree -p vox-cli` and workspace-wide `cargo metadata` show no reference to vox-schola, vox-scientia-core, vox-scientia-social, vox-scientia-ingest, vox-socrates-policy, vox-spool, vox-tools, vox-mcp-meta, vox-browser, or vox-audio-ingress.
  2. `ConfidencePolicy`/`ComplexityBand`/`RiskBand` types live in vox-orchestrator-types with zero remaining `vox_socrates_policy::*` imports anywhere in the workspace.
  3. vox-scientia-ingest's functionality (scholarly-external-jobs) is reachable from vox-publisher with no crate requiring a `vox-cli` dependency to use it.
  4. `catalog.toml` shows execution-api and stub-check as already-removed, and vox-workflow-runtime/vox-integration-tests/vox-test-harness remain present, unmodified, and annotated as intentionally frozen.
**Plans**: TBD

### Phase 2: Wire Up & Reclassify Dormant Crates
**Goal**: Functionally-complete crates that were never adopted are active in their intended call path, and crate classification matches actual usage.
**Depends on**: Phase 1
**Requirements**: REQ-dead-crate-wire-up
**Success Criteria** (what must be TRUE):
  1. The exec-policy gate (vox-container or vox-cli-core) calls `vox_exec_grammar::risk::classify`, matching the ADR-026 contract in `contracts/terminal/exec-policy.v1.yaml`.
  2. vox-orchestrator depends on vox-mcp-registry and validates/enumerates MCP tool names via `TOOL_REGISTRY`; vox-mcp-meta no longer exists in the workspace.
  3. The crate catalog lists vox-search and vox-doc-inventory as CORE, matching their existing unconditional use by vox-cli and vox-orchestrator.
**Plans**: TBD

### Phase 3: Extract Misplaced Crates to Plugin Architecture
**Goal**: Crates that don't belong in the CORE compile graph move to the plugin architecture without losing functionality, and CORE loses its last direct Candle dependency bleed.
**Depends on**: Phase 2
**Requirements**: REQ-dead-crate-extract-to-plugin, REQ-dead-crate-misplaced
**Success Criteria** (what must be TRUE):
  1. vox-grammar-export's functionality ships as `vox-plugin-grammar-export`, implementing the `GrammarExportPlugin` ABI and dispatched from vox-constrained-gen via the plugin host.
  2. vox-webhook ships as `vox-plugin-webhook`; vox-orchestrator calls it through `WebhookOrchestratorBridge`/`OrchestratorInboxItem` dispatch instead of a direct crate dependency.
  3. vox-ssg's functionality ships as a plugin rather than a CORE crate.
  4. The vox-oratio extraction is complete, with no direct Candle dependency remaining in any CORE (L0-L3) crate.
**Plans**: TBD

### Phase 4: GUI/Dashboard Architecture Consolidation
**Goal**: The Tauri GUI is the ratified, sole orchestration surface, with a clear, enforced boundary between Vox-native and React/TanStack interop UI code.
**Depends on**: Nothing (independent of Phases 1-3; can run in parallel)
**Requirements**: GUI-01, GUI-02, GUI-03, GUI-04
**Success Criteria** (what must be TRUE):
  1. `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` carries an explicit "Status: Accepted" line and its classification changes from `locked: false`/medium-confidence to locked.
  2. Every command `vox-gui` exposes traces back to `vox-cli`'s `CommandCatalog` SSOT — an audit finds no orphaned or duplicated command surface between the two.
  3. A documented rule (per ADR-027) states which UI primitives are Vox-native vs React/TanStack interop, and existing components are checked against it with no undocumented crossovers.
  4. ADR-037's desktop-convergence clause is confirmed complete and its own status reflects "Accepted" for that clause, independent of the already-superseded mobile clause.
**Plans**: TBD

### Phase 5: Multi-Agent Coordination & Trust Hardening
**Goal**: Multiple concurrent agents can safely contend for shared resources, and every tool call they make is independently auditable.
**Depends on**: Nothing (independent of Phases 1-4; can run in parallel)
**Requirements**: MESH-01, TRUST-01
**Success Criteria** (what must be TRUE):
  1. `ResourceLockManager` exists and is used for multi-agent resource coordination, correctly handling lease expiration and contention per ADR-025.
  2. Agent tool calls produce HMAC receipts that the two-tier formal-intent verification system (ADR-029) can independently check for authenticity.
**Plans**: TBD

### Phase 6: Model Routing Transparency & ML Dependency Health
**Goal**: Model selection is observable as a cost/latency/reliability tradeoff, and the local ML training stack runs on a unified, GPU-CI-verified dependency set.
**Depends on**: Nothing (independent of Phases 1-5; can run in parallel)
**Requirements**: MODEL-01, ML-01
**Success Criteria** (what must be TRUE):
  1. Model scoreboards render as a Pareto frontier over reliability, cost, and latency, with no change to actual model-routing behavior (ADR-046).
  2. Candle, peft-rs, and qlora-rs resolve to a single unified version set across the workspace, verified passing on a GPU-backed CI lane (ADR-034).
**Plans**: TBD

## Progress

**Execution Order:**
Phases 1 → 2 → 3 form a dependency chain (crate surgery). Phases 4, 5, 6 are each independent of the others and of the Phase 1-3 chain — they can execute in any order, including in parallel with the crate-cleanup chain.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Dead Crate Cleanup — Remove & Confirm | 0/TBD | Not started | - |
| 2. Wire Up & Reclassify Dormant Crates | 0/TBD | Not started | - |
| 3. Extract Misplaced Crates to Plugin Architecture | 0/TBD | Not started | - |
| 4. GUI/Dashboard Architecture Consolidation | 0/TBD | Not started | - |
| 5. Multi-Agent Coordination & Trust Hardening | 0/TBD | Not started | - |
| 6. Model Routing Transparency & ML Dependency Health | 0/TBD | Not started | - |
