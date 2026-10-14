# Unified Roadmap: Vox Holistic Integrated Stream (v1.5 Unified)

## Overview

This roadmap unifies all six parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and the four active feature worktrees (`capability-profiles`, `true-workflow-durability`, `warp-fork-axis-gui-b682cd`, `api-key-management-16bb08`) into a **single stream, branch, and worktree (`/Users/brbrainerd/dev/vox`)** without deleting any existing worktrees on disk.

**Why Order Matters:**
1. **VoxDb Schema Baselines (`95 -> 96 -> 97 -> 98`):** `kb` Phase 3 (`03-03`) owns **baseline 96** (`kb_findings` + `trust_tier`). `research-trial-flywheel` Phase 07 (`07-06`) is sequenced immediately after at **baseline 97** (`trials` domain). `autonomy-ux` Phase 17 (`AttentionInbox`) takes **baseline 98** if a persistent inbox table is added.
2. **Operation Catalog (`contracts/operations/catalog.v1.yaml`) & `vox-mcp-registry`:** Phase 05 merges the schema extensions from `discoverability` (`06.2`), `capability-profiles`, `kb` (`02`–`03`), and `docs-freshness` (`23`) first and runs `vox run scripts/ssot-regen.vox` once before Wave 1 begins.
3. **Graph Data Floor (`Phase 25`) Before Both OpenWiki (`Phase 33`) & GUI Atlas (`Phases 26–31`):** `vox-graph-reader` v5 (`EXTRACTOR_VERSION = "5"`, typed edges, `crate -> file -> symbol` hierarchy, `.vox` AST kinds, `vox_search_atlas`) lands in Wave 1 (`Phase 25`) so both Wave 2's OpenWiki Subsystem Hub/Mermaid generator (`Phase 33`) and Wave 3's GUI Code & Architecture Atlas (`Phases 26–31`) share one backend.
4. **Shared React Surfaces (`Loquela.tsx`, `ChatSurface.tsx`, `Sidebar.tsx`, `DocReader.tsx`) Strictly Sequenced in Wave 3:** All backend/daemon RPCs land in Waves 0–2; Wave 3 executes GUI phases sequentially on shared components so parallel subagents never clobber `Loquela.tsx` or `ChatSurface.tsx`.

---

## Completed / In-Tree Baseline Phases (Preserved & Merged in Wave 0)

<details>
<summary>✅ Workstream <code>kb</code> Phases 01–04 (Completed on <code>claude/vox-knowledge-base-research-2964d2</code> — 48/48 plans)</summary>

- [x] **Phase 01: Retrieval Eval Baseline** (10/10 plans complete)
- [x] **Phase 02: Library Index & KB Tool Surface (`vox_kb_*`)** (13/13 plans complete)
- [x] **Phase 03: Provenance, Trust Tiers & Leak Closure (VoxDb Baseline 96)** (12/12 plans complete)
- [x] **Phase 04: Taxonomy Contract (`vox-kb-facets` L0) & Facet Lint** (13/13 plans complete)

</details>

<details>
<summary>✅ Workstream <code>discoverability</code> Phases 06.1–06.3 (Completed on <code>claude/vox-discoverability-a72c8c</code> — 21/21 plans)</summary>

- [x] **Phase 06.1: Hosted CI Gate & Fast Local Loop** (1/1 plan complete)
- [x] **Phase 06.2: Capability Index SSOT (`vox explain` / `vox features`)** (8/8 plans complete)
- [x] **Phase 06.3: Universal Palette (`⌘K` Omnibar)** (12/12 plans complete)

</details>

<details>
<summary>✅ Workstream <code>docs-freshness</code> v1.3 Phases 20–24 (Completed in <code>/Users/brbrainerd/dev/vox</code> — 32/32 plans)</summary>

- [x] **Phase 20: Deploy Unblock & Public Surface Honesty** (5/5 plans complete)
- [x] **Phase 20.1: Collapsible Reader Layout & Progressive Disclosure** (6/6 plans complete)
- [x] **Phase 21: Corpus Measure & Prune Audit** (6/6 plans complete)
- [x] **Phase 22: Deterministic Drift Gate & PR Impact** (5/5 plans complete)
- [x] **Phase 23: Generated Reference Expansion & Crate API Docs** (5/5 plans complete)
- [x] **Phase 24: Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go** (5/5 plans complete)

</details>

---

## Active Unified Execution Waves

### Wave 0: Single-Branch & Worktree Convergence (`[SEQUENTIAL]`, `[MODEL: ARGON-4]`)

- [x] **Phase 05: Unified Branch & Worktree Convergence** — Checkpoint the 403 uncommitted files in `/Users/brbrainerd/dev/vox` onto `feat/unified-2026-q4` without touching any external worktrees, then merge `origin/main`, backend feature worktrees (`true-workflow-durability`, `fmt-safety`, `warp-fork-axis-gui` `term.*`), `kb` Phases 01–04 (DB baseline 96), `discoverability` Phases 06.1–06.3, and `capability-profiles`, finishing with a single `vox run scripts/ssot-regen.vox` pass. (Completed 2026-10-13)

### Wave 1: Foundational Data Floors, Safety Contracts & Trust Calibration

- [ ] **Phase 12: Autonomy Safety Floor & Lowering-Path Closures** (`[SEQUENTIAL]`, `[MODEL: ARGON-4]` + `[MODEL: FLASH-3.8]`) — Execute the 8 verified plans (`12-01`..`12-08-PLAN.md`) locking `contracts/security/permission-modes.v2.yaml`, `classify_safety_floor`, and all 4 runtime lowering-path closures with mutation proofs.
- [ ] **Phase 07: Trial Identity, Contracts & Tier A Persistence at DB Baseline 97** (`[SEQUENTIAL]` on `vox-db`, `[MODEL: ARGON-4]` for `07-01`/`07-05`/`07-06`/`07-07` + `[MODEL: FLASH-3.8]` for `07-08`..`07-12`) — Complete Phase 07 (`07-01`, `07-05`..`07-12`) migrating `trials` into `vox-db` at **schema baseline 97** (after `kb`'s baseline 96).
- [ ] **Phase 07.1: Code-Gen Benchmark Signed Provenance Bundle (`12-09`)** (`[SEQUENTIAL]` after Phase 07, `[MODEL: FLASH-3.8]`) — Complete `12-09-PLAN.md` (`07.1-09`) wiring signed benchmark provenance into Phase 07's `trial_runs` and `trial_samples` at baseline 97 (`12-01`..`12-08` already complete).
- [ ] **Phase 25: Unified Code & Architecture Graph Data Floor** (`[PARALLEL-SAFE]` with Phase 12/07, `[MODEL: ARGON-4]` for `25-01` schema + `[MODEL: FLASH-3.8]` for `25-02`..`25-04`) — Upgrade `vox-graph-reader` (`EXTRACTOR_VERSION = "5"`) with typed edges, `crate -> file -> symbol` hierarchy, `.vox` declaration kinds (`ast_vox.rs`), budgeted `AtlasQuery` (`lod.rs`), and `vox_search_atlas`.
- [ ] **Phase 32: Docs Trust Signal Calibration & Ledger Sync** (`[PARALLEL-SAFE]` with Phase 25, `[MODEL: FLASH-3.8]`) — Eliminate illustrative-path false positives, seed `verified_hash` in `ledger.v1.json`, wire `docs-freshness-ledger --write` after `crate-api-sync` in `run_ssot_drift`, and anchor the 110 unanchored conceptual pages.

### Wave 2: Core Backend Engines (OpenWiki Generator, Docs Debt Burn-Down, Autonomy Daemon, KB Embeddings/Context, Trial Race Harness)

- [ ] **Phase 33: OpenWiki Subsystem Hubs, Deep Crate Symbols & Validated Mermaid Diagrams** (`[SEQUENTIAL]` after Phase 25 & 32, `[MODEL: FLASH-3.8]` + `[MODEL: ARGON-4]` for `33-01`) — Generate OpenWiki-style Subsystem Hub pages (`docs/src/architecture/subsystems/*.generated.md`), enrich `docs/src/api/*.generated.md` with symbol tables and reverse dependents, and validate Mermaid diagrams.
- [ ] **Phase 34: Docs Corpus Consolidation & Technical Debt Burn-Down** (`[PARALLEL-SAFE]` with Phase 33 on authored docs, `[MODEL: FLASH-3.8]` + `[MODEL: ARGON-4]` for `34-01` duplicate merges) — Consolidate 29 duplicate clusters, reclassify 122 broken `status: current` architecture docs, burn down `baseline.v1.json` deficits (`920 -> <100`), and shrink `kb-facet-backlog.v1.yaml` by `>= 50%`.
- [ ] **Phase 13: Autonomy Oversight Contract, Pure Resolver & CLI/MCP Surface** (`[SEQUENTIAL]` after Phase 12, `[MODEL: ARGON-4]`) — Land `OversightLevel` (`1..5`), pure `resolve_effective_autonomy`, and one-way legacy config migration.
- [ ] **Phase 14: Daemon Session State, Persistence & `orch.oversight_*` RPC** (`[SEQUENTIAL]` after Phase 13, `[MODEL: FLASH-3.8]`) — Wire session-scoped `OversightLevel` in `vox-orchestrator-daemon` with `orch.oversight_changed` broadcast.
- [ ] **Phase 15: Background Task Consumer Wiring** (`[PARALLEL-SAFE]` after Phase 14, `[MODEL: FLASH-3.8]`) — Route orchestrator background tasks through `EffectiveAutonomy`.
- [ ] **Phase 16: Sync Chat Consumer Wiring (`hitl_doubt`, `mens-gate`, `budget.rs`)** (`[PARALLEL-SAFE]` after Phase 14, `[MODEL: ARGON-4]`) — Route sync chat approval, doubt loop, Mens gate, and spend cap through `EffectiveAutonomy`.
- [ ] **Phase 17: Cross-Agent Attention Admission & Inbox Digest Backend** (`[SEQUENTIAL]` after Phases 15–16, `[MODEL: ARGON-4]`) — Land `AttentionAdmission` gate and `AttentionInbox` (at DB baseline 98 if schema required).
- [ ] **Phase 37: KB Local Embedding Pipeline & Semantic Parity** (formerly `kb` Phase 5; `[PARALLEL-SAFE]`, `[MODEL: ARGON-4]`) — Wire local Metal/CPU embeddings in `vox-search` and hybrid RRF on `vox_kb_query`.
- [ ] **Phase 38: KB Bounded Session-Pinned Context Injection** (formerly `kb` Phase 7; `[SEQUENTIAL]` after Phase 37, `[MODEL: ARGON-4]`) — Inject deduped, token-budgeted `@kb` citations into chat/agent turns.
- [ ] **Phase 39: KB Web Fallback & Agent Finding Inbox** (formerly `kb` Phase 9; `[SEQUENTIAL]` after Phase 38, `[MODEL: FLASH-3.8]`) — Wire `quarantined_web` fallback and `vox_kb_record_finding` into the baseline-96 `kb_findings` inbox.
- [ ] **Phase 40: KB Eval-Prioritized Page-One Rewrites** (formerly `kb` Phase 10; `[PARALLEL-SAFE]` after Phase 34, `[MODEL: FLASH-3.8]`) — Rewrite top-25 failure-mode pages with compilable `.vox` doctests (`MRR@5 >= 0.70`).
- [ ] **Phase 41: KB Staleness Detection & Refresh Queue** (formerly `kb` Phase 11; `[SEQUENTIAL]` after Phase 25 & 39, `[MODEL: FLASH-3.8]`) — Detect stale findings and docs via symbol-hash (`vox-graph-reader`) + git-diff and expose `vox kb verify`.
- [ ] **Phase 08: Campaign Preregistration, Spend Reservations & Race Harness** (`[SEQUENTIAL]` after Phase 07, `[MODEL: ARGON-4]`) — Signed campaign preregistration, atomic spend reservations, group-sequential stopping, and paired arm racing.
- [ ] **Phase 09: Verified Knowledge Loop & Memory Ablations** (`[SEQUENTIAL]` after Phase 08 & 39, `[MODEL: ARGON-4]`) — Tier C evidence storage, frozen per-arm KB manifests, and memory ablation campaigns.
- [ ] **Phase 10: Two-Campaign Promotion Governance & Proposals** (`[SEQUENTIAL]` after Phase 09, `[MODEL: ARGON-4]`) — Dual-campaign (`exploratory` + `holdout_confirmation`) promotion gate emitting human-reviewed config diff proposals.
- [ ] **Phase 11: Offline Replay, Adversarial Hardening & Graduation** (`[SEQUENTIAL]` after Phase 10, `[MODEL: ARGON-4]`) — Offline bundle verifier, contamination detector, and v1.1 graduation gate.
- [ ] **Phase 42: Triggered `.vox` Language Code-Gen Benchmark Campaign** (formerly `research-trial-flywheel` Phase 13; `[SEQUENTIAL]` after Phase 07.1 & 08, `[MODEL: FLASH-3.8]`) — Execute and persist comparative `.vox` code-gen benchmark campaigns.

### Wave 3: Sequenced GUI Unification (`crates/vox-gui/ui/` — Strictly Sequenced on Shared React Files)

- [ ] **Phase 06.4: Capability Explorer & Coverage Gate** (`[SEQUENTIAL]` GUI slot 1, `[MODEL: FLASH-3.8]` — 13 plans `06.4-01`..`13` ready) — Ship `CapabilityExplorer.tsx` in `Sidebar.tsx` and the `vox ci capability-surface-coverage` gate.
- [ ] **Phase 26: Viewport-Culled Code & Architecture Atlas Canvas** (`[SEQUENTIAL]` GUI slot 2 after `06.4`, `[MODEL: ARGON-4]` layout + `[MODEL: FLASH-3.8]` nodes) — Ship React Flow + ELKjs canvas consuming Phase 25's `vox_search_atlas` (absorbs `discoverability` Phase `06.5` `DISC-12..14`).
- [ ] **Phase 27: Atlas Branch & Worktree Diff Overlay** (`[SEQUENTIAL]` GUI slot 3, `[MODEL: FLASH-3.8]`) — Add git/jj branch and worktree comparison overlay to the Atlas canvas.
- [ ] **Phase 28: Dockable Atlas Widget Beside Chat** (`[SEQUENTIAL]` GUI slot 4 — locks `ChatSurface.tsx`, `[MODEL: ARGON-4]`) — Mount the Atlas as a dockable panel alongside chat in `ChatSurface.tsx`.
- [ ] **Phase 29: Atlas Live File-Watcher Sync & Active Agent Badges** (`[SEQUENTIAL]` GUI slot 5, `[MODEL: FLASH-3.8]`) — Wire live file-change invalidation and orchestrator active-agent badges onto Atlas nodes.
- [ ] **Phase 30: Atlas Click-to-`@` Composer Chip & Vox Chrome** (`[SEQUENTIAL]` GUI slot 6 — locks `Loquela.tsx`, `[MODEL: ARGON-4]`) — Wire node click-to-`@` mention insertion into `Loquela.tsx`.
- [ ] **Phase 31: Atlas Playwright Visual Proof** (`[SEQUENTIAL]` GUI slot 7, `[MODEL: FLASH-3.8]`) — End-to-end Playwright visual and interaction verification for the unified Code & Architecture Atlas.
- [ ] **Phase 43: KB Library GUI Surface (Absorbs `DocReader.tsx`)** (formerly `kb` Phase 6; `[SEQUENTIAL]` GUI slot 8 — locks `DocReader.tsx`/`Sidebar.tsx`, `[MODEL: FLASH-3.8]`) — Upgrade `DocReader.tsx` into the unified `Library` surface with facet filters, hybrid search, and trust badges.
- [ ] **Phase 44: KB Per-Turn Context Inspector & Composer `@kb` Controls** (formerly `kb` Phase 8; `[SEQUENTIAL]` GUI slot 9 — locks `ChatSurface.tsx`/`Loquela.tsx`, `[MODEL: ARGON-4]`) — Add the per-turn Context Inspector drawer and `@kb` composer scope controls.
- [ ] **Phase 45: KB Inbox Review & Health GUI Tabs** (formerly `kb` Phase 12; `[SEQUENTIAL]` GUI slot 10, `[MODEL: FLASH-3.8]`) — Add Inbox finding promotion/rejection and KB Health tabs to `Library`.
- [ ] **Phase 18: Autonomy Oversight Composer Control & Inbox Digest Panel** (`[SEQUENTIAL]` GUI slot 11 — locks `Loquela.tsx`/`DriveConsole.tsx`/`RiskPopover.tsx`, `[MODEL: ARGON-4]`) — Replace `RiskPopover.tsx` and `DriveConsole.tsx`'s separate dial with the unified `Oversight: 1..5` control and `InboxDigestPanel.tsx`.
- [ ] **Phase 19: Autonomy Claims Ledger & End-to-End Verification** (`[SEQUENTIAL]` GUI slot 12, `[MODEL: FLASH-3.8]`) — Ship `contracts/orchestration/autonomy-claims.v1.yaml` and Playwright + Rust E2E claim tests.

### Wave 4: Self-Healing Automation & End-to-End Cold-Start Sign-Off

- [ ] **Phase 35: Constrained Nightly LLM Doc Claim Verification Engine** (`[SEQUENTIAL]` after Phase 34 & 41, `[MODEL: ARGON-4]`) — Build `vox ci docs-claim-verify` using `vox_actor_runtime::llm` (`TaskCategory::DocClaimJudge`) and Phase 41's staleness queue to detect semantic prose drift against AST/source slices.
- [ ] **Phase 36: Self-Healing Docs Draft-PR Bot & Freshness Ratchet** (`[SEQUENTIAL]` after Phase 35, `[MODEL: FLASH-3.8]`) — Wire `vox ci docs-heal --apply`, `.github/workflows/docs-self-heal.yml` (`SSOT_AUTOREGEN_TOKEN`), and ratchet `thresholds.v1.yaml` (`verified >= 85%`, `broken == 0` for `status: current`).
- [ ] **Phase 06.6: Cold-Start Time-to-Find Journeys & Master Milestone Sign-Off** (`[SEQUENTIAL]` after Wave 3 & Phase 36, `[MODEL: FLASH-3.8]`) — Execute the 5 cold-start Playwright journeys (`<= 60s`, `<= 3` interactions) across the unified application and seal the milestone.

---

## Phase Details (Immediate Wave 0 & Wave 1 Execution Queue)

### Phase 05: Unified Branch & Worktree Convergence
**Goal**: Converge all uncommitted work in `/Users/brbrainerd/dev/vox` and all active feature worktrees (`origin/main`, `true-workflow-durability`, `fmt-safety`, `warp-fork-axis-gui`, `kb` Phases 1–4, `discoverability` Phases 06.1–06.3, `capability-profiles`) onto a single branch in `/Users/brbrainerd/dev/vox` without deleting any worktrees on disk.
**Depends on**: Completed phases `01..04`, `06.1..06.3`, `20..24`
**Requirements**: `CONV-01`, `CONV-02`, `CONV-03`, `CONV-04`
**Plans**: 4/4 plans complete (`05-01-PLAN.md` .. `05-04-PLAN.md`)
- [x] `05-01-PLAN.md` — `[MODEL: ARGON-4]` `[SEQUENTIAL]` Verify all 42 worktrees intact, create unified branch `feat/unified-2026-q4` from current `/Users/brbrainerd/dev/vox` HEAD (`b64377d40`), checkpoint the 403 uncommitted files in logical atomic commits (`docs-freshness` v1.3, `research-trial-flywheel` WIP, `vox-gui` WIP), and merge `origin/main` + `docs-freshness-p20-1`.
- [x] `05-02-PLAN.md` — `[MODEL: ARGON-4]` `[SEQUENTIAL]` Merge backend feature branches: `feat/true-workflow-durability` (`ce9cbc67c`), `wip/fmt-safety-2026-10-12` (`499c75bce`), and `warp-fork-axis-gui-b682cd` (`345542f4a` `term.*` session host).
- [x] `05-03-PLAN.md` — `[MODEL: ARGON-4]` `[SEQUENTIAL]` Merge `claude/vox-knowledge-base-research-2964d2` (`e60182f6f` — `kb` Phases 01–04, `vox-kb-facets` L0 crate, `vox_kb_*` MCP tools, and **VoxDb schema baseline 96**) and verify `cargo test -p vox-db -p vox-kb-facets -p vox-search`.
- [x] `05-04-PLAN.md` — `[MODEL: ARGON-4]` `[SEQUENTIAL]` Merge `claude/vox-discoverability-a72c8c` (`d3a93f756` — `06.1`–`06.3`) and `feat/capability-profiles` (`77f5dfa9a`), reconcile `contracts/operations/catalog.v1.yaml` + `crates/vox-mcp-registry/`, and run `vox run scripts/ssot-regen.vox` + `vox ci crate-edges`.

### Phase 12: Autonomy Safety Floor & Lowering-Path Closures
**Goal**: Lock the non-bypassable action-only safety floor (`permission-modes.v2.yaml`) and close all four runtime lowering paths before any autonomy resolver or UI work runs.
**Depends on**: Phase 05
**Requirements**: `FLOOR-01`, `FLOOR-02`
**Plans**: 8 plans (`12-01-PLAN.md` .. `12-08-PLAN.md` — already written and checker-verified in `phases/12-safety-floor-lowering-paths/`)

### Phase 07: Trial Identity, Contracts & Tier A Persistence at DB Baseline 97
**Goal**: Complete canonical trial identity, structural telemetry, and Tier A persistence (`trials` domain) in `vox-db` at **schema baseline 97** (immediately after `kb` Phase 3's baseline 96), plus `07.1` (`12-09`) signed code-gen benchmark bundle.
**Depends on**: Phase 05 (specifically `05-03` baseline 96)
**Requirements**: `TRIAL-01`, `STORE-01`, `STORE-02`, `BENCH-09`
**Plans**: 12 plans in `07-trial-identity-contracts-observational-baseline/` (`07-02`, `07-03`, `07-04` complete; `07-01`, `07-05`..`07-12` ready, with `07-06` updated to baseline 97) + `12-09-PLAN.md` in `07.1-code-gen-benchmark-plumbing-provenance/`

### Phase 25: Unified Code & Architecture Graph Data Floor
**Goal**: Upgrade `vox-graph-reader` (`EXTRACTOR_VERSION = "5"`) with typed edges, `crate -> file -> symbol` hierarchy, `.vox` AST kinds, budgeted `AtlasQuery`, and `vox_search_atlas` MCP/daemon RPC to serve both OpenWiki (`Phase 33`) and the GUI Atlas (`Phases 26–31`).
**Depends on**: Phase 05
**Requirements**: `GRAPH-01`, `GRAPH-02`, `DISC-12`
**Plans**: 4 plans (`25-01` .. `25-04`)

### Phase 32: Docs Trust Signal Calibration & Ledger Sync
**Goal**: Eliminate illustrative-path false positives, seed `verified_hash` in `ledger.v1.json`, wire `docs-freshness-ledger --write` after `crate-api-sync` in `run_ssot_drift`, and anchor the 110 unanchored conceptual pages.
**Depends on**: Phase 05
**Requirements**: `SIGNAL-01`, `SIGNAL-02`, `SIGNAL-03`, `SIGNAL-04`
**Plans**: 4 plans (`32-01` .. `32-04`)
