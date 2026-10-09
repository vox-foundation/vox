# Roadmap: Vox

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01) — [archive](milestones/v1.0-ROADMAP.md)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 06.2–06.6 (inserted discoverability) and 7–11 (in progress)

## Overview

v1.1 turns Vox's existing research pipeline, telemetry, evaluation, model routing, and retrieval into a governed measure–compare–promote–replay loop. It builds bottom-up: one trial identity and atomic persistence first, then the race harness and scorecards that consume it, then the knowledge loop whose ablations need the harness, then promotion and replay that need all three, and finally adversarial hardening across the whole loop. Each phase is usable on its own: Phase 7 already yields an observational baseline.

## Phases

**Phase Numbering:**

- Integer phases (7, 8, 9): Planned milestone work
- Decimal phases (7.1, 7.2): Urgent insertions (marked with INSERTED)

<details>
<summary>✅ v1.0 Architectural Housekeeping (Phases 1–6) — shipped 2026-10-01</summary>

- [x] Phase 1: Dead Crate Cleanup — Remove & Confirm (2/2 plans)
- [x] Phase 2: Wire Up & Reclassify Dormant Crates (2/2 plans)
- [x] Phase 3: Extract Misplaced Crates to Plugin Architecture (6/6 plans)
- [x] Phase 4: GUI/Dashboard Architecture Consolidation (2/2 plans)
- [x] Phase 5: Multi-Agent Coordination & Trust Hardening (7/7 plans)
- [x] Phase 6: Model Routing Transparency & ML Dependency Health (4/4 plans)
- [ ] Phase 06.1: Hosted CI gate and fast local loop (INSERTED, in progress — see below)

</details>

### 🚧 v1.1 Research Trial Flywheel

- [ ] **Phase 06.2: Capability Index SSOT** (INSERTED) - One generated index joining CLI/MCP/GUI/docs/crates/skills/graph coverage; `vox explain`, `vox features`, generated llms.txt
- [ ] **Phase 06.3: Universal Palette** (INSERTED) - ⌘K finds and runs every indexed capability; action registry; `?` shortcut sheet
- [ ] **Phase 06.4: Capability Explorer & Coverage Gate** (INSERTED) - Domain-grouped explorer with status, generic_form tier, CI coverage gate, orphan disposition to 0
- [ ] **Phase 06.5: Architecture Atlas** (INSERTED) - Navigable L0–L5 layer/crate map with edges, blast radius, and capability cross-links
- [ ] **Phase 06.6: Time-to-Find Journeys** (INSERTED) - Playwright cold-start journeys with interaction budgets, gated in hosted CI
- [ ] **Phase 7: Trial Identity, Contracts & Observational Baseline** - One signed campaign/run identity, atomic Tier A persistence, structural telemetry, hermetic fixtures, and a no-policy-change baseline
- [ ] **Phase 8: Race Harness & Multidimensional Scorecards** - Bounded parallel condition matrix with spend reservations, arm isolation, group-sequential stopping, hard-gate + Pareto scoring, and reports
- [ ] **Phase 9: Verified Knowledge Loop** - Tier C evidence decision, authoritative findings with provenance, acknowledged projections, frozen per-arm manifests, and memory ablations
- [ ] **Phase 10: Promotion Governance & Replay** - Offline replay, shadow challengers, two-campaign confirmation, dual-control scoped designation, and suspension
- [ ] **Phase 11: Adversarial Trials & Hardening** - Adversarial/stress trials, mutation-tested gates, evidence bundles, and full gate pass

## Phase Details

### Phase 06.2: Capability Index SSOT (INSERTED)

**Goal:** One generated capability index is the single source that answers "what can Vox do, where is it, and how do I reach it" for CLI, GUI, MCP, docs, and agents.
**Depends on:** Nothing (reads existing `contracts/operations/catalog.v1.yaml`, `contracts/gui/surface-registry.v1.yaml`, `contracts/gui/omnisearch-index.v1.yaml`, `layers.toml`, skills catalog, `vox graph coverage`)
**Requirements**: DISC-01, DISC-02, DISC-03, DISC-04
**Success Criteria** (what must be TRUE):

  1. `contracts/capabilities/capability-index.v1.json` exists, is generated, and every operations-catalog entry has a row with surface, doc, crate, layer, and coverage kind (null only where genuinely absent).
  2. Adding a CLI command or GUI surface without regenerating fails `vox ci ssot-drift`.
  3. `vox explain mesh` and `vox features --domain secrets` print layer, CLI/GUI/MCP equivalents, and a docs link.
  4. `llms.txt` capability sections are generator output and link the MCP tool reference and GUI navigation.

**Plans:** 5/8 plans executed (06.2-08 optional, owner-gated)

Plans:
**Wave 1**

- [x] 06.2-01-PLAN.md — Tracer: index schema + `vox ci capability-index` generator + ssot-drift gate + embedded loader (wave 1)

**Wave 2** *(blocked on Wave 1 completion)*

- [x] 06.2-02-PLAN.md — Owner crate/layer (D-04), domain vocabulary (D-09), tier, unresolved-owner ratchet (D-05) (wave 2)
- [x] 06.2-03-PLAN.md — vox-capability-registry resolver (exact→fuzzy) + explain card / features renderers (wave 2)

**Wave 3** *(blocked on Wave 2 completion)*

- [x] 06.2-04-PLAN.md — `vox explain` / `vox features` CLI + MCP twins via operations catalog (wave 3)

**Wave 4** *(blocked on Wave 3 completion)*

- [x] 06.2-05-PLAN.md — Surface/crate/skill rows, GUI join, graph-coverage snapshot + `--refresh-coverage` (wave 4)

**Wave 5** *(blocked on Wave 4 completion)*

- [ ] 06.2-06-PLAN.md — `capabilities:` frontmatter claims → doc links/rows; doc-lint/Starlight/governance contract (wave 5)

**Wave 6** *(blocked on Wave 5 completion)*

- [ ] 06.2-07-PLAN.md — Generated llms.txt/llms-full.txt regions, vox-docs.json key, GUI TS artifact (wave 6)

**Wave 7** *(blocked on Wave 6 completion)*

- [ ] 06.2-08-PLAN.md — OPTIONAL nightly coverage-refresh job (owner-gated checkpoint) (wave 7)

### Phase 06.3: Universal Palette (INSERTED)

**Goal:** ⌘K is the one place to find and run anything in Vox.
**Depends on:** Phase 06.2
**Requirements**: DISC-05, DISC-06, DISC-07
**Success Criteria** (what must be TRUE):

  1. Omnibar results draw from the capability index; searching any CLI verb, MCP tool, skill, crate, doc, setting, or keybind returns it.
  2. Selecting a CLI verb runs it or opens a prefilled Catalog form; selecting a surface navigates to it.
  3. `?` opens a generated shortcut sheet; every keybind appears as a palette action.
  4. A Playwright spec with mock IPC covers palette search + run, with review-bundle screenshots.

**Plans**: TBD

### Phase 06.4: Capability Explorer & Coverage Gate (INSERTED)

**Goal:** Users can browse everything Vox can do by domain, and CI keeps every capability reachable or explicitly exempted.
**Depends on:** Phase 06.2
**Requirements**: DISC-08, DISC-09, DISC-10, DISC-11
**Success Criteria** (what must be TRUE):

  1. A top-level Capability Explorer surface (promoted from the hidden Settings → Coverage table) groups capabilities by domain with status badges and links to surface/CLI/docs.
  2. Catalog generic forms count as `generic_form`; the registry's `none` tier (80 of 112 today) shrinks accordingly.
  3. A CI check fails on a capability with no representation and no owner+reason exemption.
  4. The undispositioned OrphanBackend/DeadEnd count is 0 and ratcheted.

**Plans**: TBD

### Phase 06.5: Architecture Atlas (INSERTED)

**Goal:** Contributors can see Vox's layers, crates, and their relationships in the GUI and jump from structure to capabilities, code, and docs.
**Depends on:** Phase 06.2
**Requirements**: DISC-12, DISC-13, DISC-14
**Success Criteria** (what must be TRUE):

  1. The Atlas surface renders L0–L5 bands with every `layers.toml` crate and its dependency edges.
  2. Selecting a crate shows its crate-map community/blast radius, the capabilities it backs, and source/doc links.
  3. Explorer ↔ Atlas cross-links work in both directions.
  4. A Playwright visual stepper covers atlas navigation in light/dark themes.

**Plans**: TBD

### Phase 06.6: Time-to-Find Journeys (INSERTED)

**Goal:** Discoverability is measured, not asserted: scripted journeys prove common things are found fast and stay fast.
**Depends on:** Phases 06.3, 06.4, 06.5
**Requirements**: DISC-15, DISC-16
**Success Criteria** (what must be TRUE):

  1. At least 8 cold-start journeys spanning contributor, `.vox` author, and GUI-operator tasks each pass within their recorded interaction budget.
  2. Journeys capture screenshots into `crates/vox-gui/ui/review-bundle/latest/`.
  3. The suite runs in the hosted `ui` lane and fails on budget regression.

**Plans**: TBD

### Phase 7: Trial Identity, Contracts & Observational Baseline

**Goal**: Every research run is attributable to a signed, immutable campaign and persisted atomically, with structural-only telemetry and hermetic tests, so a baseline can be captured without changing any policy.
**Depends on**: Nothing (first v1.1 phase)
**Requirements**: TRIAL-01, TRIAL-02, TRIAL-03, TRIAL-04, TRIAL-05, STORE-01, STORE-02, TELEM-01, TELEM-02, METER-01, SCORE-04, TEST-01, BASE-01
**Gate**: Human approval of the canonical campaign/run Tier A schema before any migration (STORE-01).
**Success Criteria** (what must be TRUE):

  1. Signing a campaign freezes it; any tampered field fails verification; supersession yields a new linked `campaign_id`.
  2. `campaign_id`/`run_id`/`replicate_id`/`attempt_index`/`retry_of_run_id` appear consistently in pipeline, eval, telemetry, and DB rows for the same attempt.
  3. A forced mid-write failure leaves no partial campaign/run/sample batch in the database.
  4. The telemetry allowlist rejects unknown and content-bearing keys, and default tests pass with network access disabled.
  5. The production runner refuses an unsigned campaign or one signed by a non-allowlisted key (mutation-tested).
  6. Every LLM call in one run shares one trace context, and two attempts of the same query get distinct session IDs.
  7. Run results carry tokens, cost, latency, TTFT, and tool-call counts; a hallucinated no-evidence answer is not scored as an abstention.
  8. A baseline campaign records every BASE-01 metric with no policy change.

**Plans**: TBD
**Research**: `.planning/research/TRIAL-DESIGN.md`, `OBSERVABILITY.md`, `QUALITY-EVAL.md`

### Phase 8: Race Harness & Multidimensional Scorecards

**Goal**: One `vox research eval` invocation races preregistered conditions in parallel within a hard budget and produces comparable, statistically honest scorecards.
**Depends on**: Phase 7
**Requirements**: RACE-01, RACE-02, RACE-03, RACE-04, SCORE-01, SCORE-02, SCORE-03, REPORT-01
**Success Criteria** (what must be TRUE):

  1. Concurrent arms never exceed the campaign's reserved token/call/cost budget, including under reservation races, because admission goes through a durable ledger rather than the in-memory `BudgetManager`.
  2. Two arms with different configs never share result-cache, verdict-cache, or learned-search-policy state.
  3. Stopping boundaries are evaluated only at completed batches; spend, safety, and wall-time stops halt immediately and fail closed.
  4. Scorecards show hard gates and a Pareto frontier with versioned metrics; a hallucinated no-evidence answer fails a hard gate; the legacy `quality_score` cannot drive eligibility.
  5. The campaign report compares each arm against baseline and champion in JSON and a concise CLI summary.

**Plans**: TBD

### Phase 9: Verified Knowledge Loop

**Goal**: Verified findings accumulate authoritatively and searchably, and trials measure whether that knowledge helps later runs without contaminating evaluations.
**Depends on**: Phase 8
**Requirements**: STORE-03, KNOW-01, KNOW-02, KNOW-03, KNOW-04, KNOW-05
**Gate**: Human decision on Tier C evidence-artifact ownership and retention (STORE-03) before replay- or promotion-eligible live campaigns.
**Success Criteria** (what must be TRUE):

  1. A verified finding is retrievable through research queries with full provenance; negative findings are retrievable too.
  2. A failed memory or search projection is visible with status and is reconciled on retry.
  3. A finding written after signing never appears in that campaign's arms.
  4. All four memory ablations run and report knowledge-reuse impact.

**Plans**: TBD

### Phase 10: Promotion Governance & Replay

**Goal**: The system proposes and validates its next experiment but cannot promote a worse or unaudited policy, and any campaign can be replayed offline.
**Depends on**: Phase 9
**Requirements**: REPLAY-01, PROMO-01, PROMO-02, PROMO-03, PROMO-04, PROMO-05
**Success Criteria** (what must be TRUE):

  1. Replay with provider access disabled reproduces a campaign's scores from captured evidence.
  2. A challenger lacking holdout confirmation, replay success, or provenance completeness is never eligible.
  3. Designation fails without two independent authorized approvers, rejects role conflicts, and never mutates runtime policy.
  4. The safety principal can suspend a champion alone; revocation or reinstatement requires the quorum; evidence expiry suspends eligibility.

**Plans**: TBD

### Phase 11: Adversarial Trials & Hardening

**Goal**: The whole loop survives adversarial conditions, every guard is proven by mutation, and each campaign ships an auditable evidence bundle.
**Depends on**: Phase 10
**Requirements**: AUDIT-01, AUDIT-02, AUDIT-03, AUDIT-04
**Success Criteria** (what must be TRUE):

  1. Adversarial trials (hallucination, conflicting memory, provider failure, budget exhaustion, cancellation) all end in the expected fail-closed state.
  2. Removing any security/reliability guard makes its test fail.
  3. Every campaign emits an evidence bundle with a working replay command.
  4. `vox ci data-storage-guard`, drift checks, docs lint, and `vox ci pre-push --complete` pass.

**Plans**: TBD

### Phase 06.1: Hosted CI gate and fast local loop (INSERTED)

**Goal:** Hosted CI is the only gate and it is fast and green; the laptop runs only `cargo check` and the edited crate's tests; no git hook builds anything; work reaches CI as a short-lived draft PR per batch.
**Requirements**: TBD
**Depends on:** Nothing (independent of Phase 6; Phase 0 of the plan unblocks nightly for every other phase)
**Evidence:** `docs/src/architecture/ci-and-build-loop-findings-2026.md`
**Execution plan:** `docs/superpowers/plans/2026-10-03-hosted-ci-fast-local-loop.md` (phases P0–P4, agy-drivable tasks marked, handoff prompt at the end)
**Success Criteria** (what must be TRUE):

  1. Three consecutive scheduled nightlies are green.
  2. `git commit` hooks finish in under 5 s and `git push` hooks in under 30 s with no cargo invocation (guard test enforces it).
  3. PR CI for a small affected set finishes in under 8 min; a full-workspace PR run finishes under 18 min with no shard at the 30-min cap.
  4. Actions cache usage is under 9 GB.
  5. No required check depends on a self-hosted runner or on this laptop.

**Execution plan status:** tracked inline in the execution plan above (task-level `Status:` lines).

## Progress

**Execution Order:** Phases execute in numeric order: 06.2 → 06.3 → 06.4 → 06.5 → 06.6 → 7 → 8 → 9 → 10 → 11

| Phase | Milestone | Plans Complete | Status | Completed |
|-------|-----------|----------------|--------|-----------|
| 1–6 | v1.0 | 23/23 | Complete | 2026-10-01 |
| 06.1 Hosted CI gate and fast local loop (INSERTED) | v1.0 | plan-doc | In progress | - |
| 06.2 Capability Index SSOT (INSERTED) | v1.1 | 5/8 | In Progress|  |
| 06.3 Universal Palette (INSERTED) | v1.1 | 0/TBD | Not started | - |
| 06.4 Capability Explorer & Coverage Gate (INSERTED) | v1.1 | 0/TBD | Not started | - |
| 06.5 Architecture Atlas (INSERTED) | v1.1 | 0/TBD | Not started | - |
| 06.6 Time-to-Find Journeys (INSERTED) | v1.1 | 0/TBD | Not started | - |
| 7. Trial Identity, Contracts & Observational Baseline | v1.1 | 0/TBD | Not started | - |
| 8. Race Harness & Multidimensional Scorecards | v1.1 | 0/TBD | Not started | - |
| 9. Verified Knowledge Loop | v1.1 | 0/TBD | Not started | - |
| 10. Promotion Governance & Replay | v1.1 | 0/TBD | Not started | - |
| 11. Adversarial Trials & Hardening | v1.1 | 0/TBD | Not started | - |
