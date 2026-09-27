# Roadmap: Vox

## Overview

Vox is a mature, working system; this roadmap is not a build-from-zero journey but a closure pass. It starts with the lowest-risk work (deleting confirmed-dead crates and confirming catalog hygiene), moves through progressively more invasive crate surgery (activating dormant code, then extracting misplaced crates into the plugin architecture), and finishes by formally closing out four clusters of "current but not yet locked" architecture decisions that a full-corpus ADR/SPEC ingest surfaced as open: GUI/dashboard architecture, multi-agent trust, and model/ML routing health.

## Phases

**Phase Numbering:**

- Integer phases (1, 2, 3): Planned milestone work
- Decimal phases (2.1, 2.2): Urgent insertions (marked with INSERTED)

- [x] **Phase 1: Dead Crate Cleanup — Remove & Confirm** - Delete zero-consumer crates, confirm frozen crates need no action, verify catalog hygiene (completed 2026-09-22)
- [x] **Phase 2: Wire Up & Reclassify Dormant Crates** - Activate functionally-complete but never-adopted crates in their intended call path (completed 2026-09-25)
- [ ] **Phase 3: Extract Misplaced Crates to Plugin Architecture** - Move CORE-inappropriate crates into the plugin system
- [x] **Phase 4: GUI/Dashboard Architecture Consolidation** - Ratify ADR-045, verify CommandCatalog SSOT alignment, enforce the Vox-native/React interop UI boundary, confirm Tauri desktop convergence (completed 2026-09-25)
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

**Plans**: 2/2 plans executed

> **Scope note (from 01-RESEARCH.md, 2026-09-22):** all ten crates named in Success Criterion 1 were already deleted in commits `e828828a9`, `72bde3718`, and `0a6aae51e` (2026-05), and the type/logic migrations behind Criteria 2 and 3 already landed. This phase is therefore a *confirmation-and-residue-cleanup* phase, not a deletion phase. Its only file diffs are one CI matrix line and one `catalog.toml` comment block.

Plans:
**Wave 1**

- [x] 01-01-PLAN.md — D-03 verification pass proving Success Criteria 1-3, plus the stale `all-features check` CI matrix fix (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 01-02-PLAN.md — Success Criterion 4: verify catalog ghost entries, annotate the three KEEP-FROZEN crates in `catalog.toml` per D-02 (wave 2)

### Phase 2: Wire Up & Reclassify Dormant Crates

**Goal**: Functionally-complete crates that were never adopted are active in their intended call path, and crate classification matches actual usage.
**Depends on**: Phase 1
**Requirements**: REQ-dead-crate-wire-up
**Success Criteria** (what must be TRUE):

  1. The exec-policy gate (vox-container or vox-cli-core) calls `vox_exec_grammar::risk::classify`, matching the ADR-026 contract in `contracts/terminal/exec-policy.v1.yaml`.
  2. vox-orchestrator depends on vox-mcp-registry and validates/enumerates MCP tool names via `TOOL_REGISTRY`; vox-mcp-meta no longer exists in the workspace.
  3. The crate catalog lists vox-search and vox-doc-inventory as CORE, matching their existing unconditional use by vox-cli and vox-orchestrator.

**Plans**: 2 plans

> **Scope note (from 02-RESEARCH.md, 2026-09-23):** SC#1 is already satisfied (`risk::classify` is called from `check_terminal.rs::run_check_rust_fallback` and `vox-container::log_exec_risk`; `vox-exec-grammar` never existed as a crate), vox-mcp-meta is already deleted, and vox-search is already CORE. The real work is SC#2's fail-closed `TOOL_REGISTRY` guard in `vox-orchestrator` (one user-authorized crate-edge exception, 2026-09-25) and one doc row for vox-doc-inventory. Live MCP-dispatch wiring of tool receipts is Phase 5 (TRUST-01).

Plans:
**Wave 1**

- [x] 02-01-PLAN.md — SC#2: vox-orchestrator depends on vox-mcp-registry; `ToolReceiptLedger::issue_intent` fails closed on names absent from `TOOL_REGISTRY` (test-first tracer, mutation-proven); `crate-graph.v1.json` regenerated in the same commit (wave 1)

**Wave 2** *(blocked on Wave 1 completion — shared git index, not shared files)*

- [x] 02-02-PLAN.md — Verify SC#1 and vox-mcp-meta absence; reclassify vox-doc-inventory DEAD -> CORE in `crate-classification-2026-05-08.md` (SC#3) (wave 2)

### Phase 3: Extract Misplaced Crates to Plugin Architecture

**Goal**: Crates that don't belong in the CORE compile graph move to the plugin architecture without losing functionality, and CORE loses its last direct Candle dependency bleed.
**Depends on**: Phase 2
**Requirements**: REQ-dead-crate-extract-to-plugin, REQ-dead-crate-misplaced
**Success Criteria** (what must be TRUE):

  1. vox-grammar-export stays a CORE library: crate-audit decision D-4/D-18 (docs/src/architecture/crate-audit-and-plan-2026.md; commit 0a8d1518c, 2026-05-24) deleted the 61-LoC vox-plugin-grammar-export pass-through and its ABI extension, superseding the 2026-05-08 PRD's EXTRACT disposition (Phase 3 D-11). Its zero-consumer automaton module and vox-populi's unused dependency on it are removed.
  2. vox-webhook ships as vox-plugin-webhook, and its events reach the orchestrator through the plugin host with no crate dependency on the plugin: an opt-in [orchestrator.webhook] section makes vox-orchestrator-mcp load the plugin, start its token-authenticated listener, poll its WebhookInbox extension, route each event with the OrchestratorInboxItem kind table (re-derived under the defactor rule) and submit it to the HopperIntake as IntakeSource::Webhook (Phase 3 D-03/D-04/D-10/D-13/D-14).
  3. vox-ssg is not a CORE crate: it was folded into vox-cli/src/utils/ssg/ (vox-cli is layer 5) by 9d385a60b (2026-05-12) and has no `[crates]` entry in docs/src/architecture/layers.toml (Phase 3 D-05).
  4. The vox-oratio extraction is complete — Candle Whisper STT is reachable only through vox-plugin-speech and vox-speech has no Candle feature (D-08/D-12) — and no CORE crate (layers 0-3 per docs/src/architecture/layers.toml, D-09) declares an unconditional Candle dependency; optional, feature-gated Candle dependencies remain allowed (D-06).

**Plans**: 6 plans

> **Scope note (from 03-CONTEXT.md D-09..D-14, 2026-09-25):** D-11 supersedes D-01/D-02 — there is no grammar-export plugin; SC#1 closes by citing crate-audit D-4/D-18 plus dead-code cleanup. SC#3 is evidence-only (D-05). SC#2 uses a new `WebhookInbox` poll extension (ABI 12 -> 13) and an opt-in poller in vox-orchestrator-mcp (D-10/D-13/D-14). SC#4's CORE half was vox-quantize alone (D-09). D-09 follow-up (2026-09-27, `276852014`, `2a2a7e477`): `docs/src/architecture/layers.toml` is now the only layer map. `vox ci crate-edges` reads it and `contracts/ci/crate-layers.v1.json` is deleted. `vox-plugin-mens-candle-core` moved to L4, so SC#4 still holds: no CORE crate has an unconditional Candle dependency. Plans run one per wave: they share the git index and the lockfile / crate-graph / build-map / ROADMAP files.

Plans:
**Wave 1**

- [x] 03-01-PLAN.md — SC#1 per D-11 (drop grammar-export's dead automaton module and vox-populi's unused edge, with derived contracts) and SC#3 per D-05 (evidence, where-things-live row, SC/REQUIREMENTS wording) (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 03-02-PLAN.md — SC#4 CORE half: vox-quantize's Candle dependency behind an `engine` feature, consumers opt in, CORE Candle scan proven load-bearing (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 03-03-PLAN.md — SC#2 plugin side: `WebhookInbox` poll extension (ABI 12 -> 13, eleven manifests in lockstep) and a fail-closed, token-only webhook listener (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 03-04-PLAN.md — SC#2 host side: opt-in `[orchestrator.webhook]` section and vox-orchestrator-mcp poller routing events into the hopper as `IntakeSource::Webhook` (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [x] 03-05-PLAN.md — SC#4 oratio half, repoint: host-registered Whisper transcriber seam in vox-speech; vox-gui and vox-ml-cli route Candle STT through vox-plugin-speech (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [x] 03-06-PLAN.md — SC#4 oratio half, delete: vox-speech's in-process Candle Whisper, feature and deps removed; decoding re-homed to `audio-decode`; final SC#4 evidence (wave 6)

### Phase 4: GUI/Dashboard Architecture Consolidation

**Goal**: The Tauri GUI is the ratified, sole orchestration surface, with a clear, enforced boundary between Vox-native and React/TanStack interop UI code.
**Depends on**: Nothing (independent — can run in parallel with the crate-cleanup chain)
**Requirements**: GUI-01, GUI-02, GUI-03, GUI-04
**Success Criteria** (what must be TRUE):

  1. `docs/src/adr/045-tauri-gui-replaces-axum-dashboard.md` carries an explicit "Status: Accepted" line and its classification changes from `locked: false`/medium-confidence to locked.
  2. Every command `vox-gui` exposes traces back to `vox-cli`'s `CommandCatalog` SSOT — an audit finds no orphaned or duplicated command surface between the two.
  3. A documented rule (per `external-frontend-interop-plan-2026.md`, which superseded ADR-027 on 2026-05-03) states which UI primitives are Vox-native vs React/TanStack interop, and existing components are checked against it with no undocumented crossovers.
  4. ADR-037's desktop-convergence clause is confirmed complete and its own status reflects "Accepted" for that clause, independent of the already-superseded mobile clause.

**Plans**: 2/2 plans executed

Plans:
**Wave 1**

- [x] 04-01-PLAN.md — Ratify ADR-045 (body Status line + intel sync) and close out ADR-037's desktop clause with cited code evidence (GUI-01, GUI-04)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 04-02-PLAN.md — Audit the vox-gui command surface against the CommandCatalog SSOT and write the Vox-native/React boundary rule in both halves (GUI-02, GUI-03)

### Phase 5: Multi-Agent Coordination & Trust Hardening

**Goal**: Multiple concurrent agents can safely contend for shared resources, and every tool call they make is independently auditable.
**Depends on**: Phase 3 (advisory — file overlap: both land work in crates/vox-orchestrator; not a semantic dependency)
**Requirements**: MESH-01, TRUST-01
**Success Criteria** (what must be TRUE):

  1. `ResourceLockManager` exists and is used for multi-agent resource coordination, correctly handling lease expiration and contention per ADR-025.
  2. Agent tool calls produce HMAC receipts that the two-tier formal-intent verification system (ADR-029) can independently check for authenticity.

**Plans**: TBD

### Phase 6: Model Routing Transparency & ML Dependency Health

**Goal**: Model selection is observable as a cost/latency/reliability tradeoff, and the local ML training stack runs on a unified, GPU-CI-verified dependency set.
**Depends on**: Phase 3 (advisory — file overlap: both land work in crates/vox-orchestrator; not a semantic dependency)
**Requirements**: MODEL-01, ML-01
**Success Criteria** (what must be TRUE):

  1. Model scoreboards render as a Pareto frontier over reliability, cost, and latency, with no change to actual model-routing behavior (ADR-046).
  2. Candle, peft-rs, and qlora-rs resolve to a single unified version set across the workspace, verified passing on a GPU-backed CI lane (ADR-034).

**Plans**: TBD

## Progress

**Execution Order:**
Phases 1 → 2 → 3 form a dependency chain (crate surgery). Phase 4 is independent of everything else and can run any time, including in parallel with the crate-cleanup chain. Phases 5 and 6 each depend on Phase 3 (advisory — file overlap on `crates/vox-orchestrator`, not a semantic dependency) but are independent of each other, so both can start once Phase 3 completes.

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 1. Dead Crate Cleanup — Remove & Confirm | 2/2 | Complete    | 2026-09-22 |
| 2. Wire Up & Reclassify Dormant Crates | 2/2 | Complete    | 2026-09-25 |
| 3. Extract Misplaced Crates to Plugin Architecture | 6/6 | Executed — live UAT pending (03-UAT.md) | - |
| 4. GUI/Dashboard Architecture Consolidation | 2/2 | Complete    | 2026-09-25 |
| 5. Multi-Agent Coordination & Trust Hardening | 0/TBD | Not started | - |
| 6. Model Routing Transparency & ML Dependency Health | 0/TBD | Not started | - |
