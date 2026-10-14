# Unified Requirements: Vox Holistic Integrated Stream (v1.5 Unified)

**Defined:** 2026-10-13
**Single Stream & Worktree:** `/Users/brbrainerd/dev/vox` (`workstreams/unified` + root `.planning/`)
**Execution Target:** Antigravity IDE (`[MODEL: FLASH-3.8]` default worker + `[MODEL: ARGON-4]` high-reasoning/convergence escalation)

This file consolidates all requirements across the six previously parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and the four active feature worktrees (`capability-profiles`, `true-workflow-durability`, `warp-fork-axis-gui-b682cd`, `api-key-management-16bb08`) into a single dependency-ordered traceability ledger.

---

## Wave 0: Single-Branch & Worktree Convergence (`CONV`)

- [ ] **CONV-01**: All 42 existing git worktrees under `.claude/worktrees/` and `.worktrees/` and all 6 historical `.planning/workstreams/<name>/` directories are preserved on disk untouched (zero worktree deletions or `git clean` losses).
- [ ] **CONV-02**: All 403 uncommitted files in `/Users/brbrainerd/dev/vox` (`docs-freshness` v1.3 Phases 20–24, `research-trial-flywheel` Phase 7/12 WIP, and GUI composer WIP) are cleanly checkpointed on the single unified branch (`feat/unified-2026-q4`) and reconciled with `origin/main` (`#596`–`#634` + `#635`, `#638`, `#640`, `#642`).
- [ ] **CONV-03**: Backend feature worktrees (`feat/true-workflow-durability` `ce9cbc67c`, `wip/fmt-safety-2026-10-12` `499c75bce`, `warp-fork-axis-gui-b682cd` `345542f4a` `term.*`) and `kb` Phases 1–4 (`claude/vox-knowledge-base-research-2964d2` `e60182f6f` at **VoxDb schema baseline 96**) merge cleanly into the single branch with green crate tests.
- [ ] **CONV-04**: `discoverability` Phases `06.1`–`06.3` (`claude/vox-discoverability-a72c8c` `d3a93f756`) and `capability-profiles` (`feat/capability-profiles` `77f5dfa9a`) merge into `contracts/operations/catalog.v1.yaml` and `crates/vox-mcp-registry/`, followed by a single `vox run scripts/ssot-regen.vox` pass with zero `ssot-drift` or `crate-edges` violations.

---

## Wave 1: Foundational Data Floors, Safety Contracts & Trust Calibration

### 1A. Autonomy Safety Floor (`FLOOR` — Phase 12, 8 plans ready)
- [ ] **FLOOR-01**: Action-only safety floor (`contracts/security/permission-modes.v2.yaml` + `classify_safety_floor` over structured action fields, never raw strings) blocks destructive actions regardless of oversight level (`12-01`..`12-03`).
- [ ] **FLOOR-02**: All four runtime lowering paths are closed (`VOX_PERMISSION_MODE` env var rejected outside test binary; MCP `vox_config_set` rejects `security.*`/`autonomy.*`; workspace `Vox.toml` can only raise oversight above user default; `--full-auto`/`--dangerously-*` requires TTY or `VOX_HEADLESS_UNSAFE_ACK=1` with persistent audit banner) (`12-04`..`12-08`).

### 1B. Trial Identity, Contracts & Tier A Persistence at DB Baseline 97 (`TRIAL` / `STORE` — Phase 07 & 07.1)
- [ ] **TRIAL-01**: Canonical `TrialCampaignId` (Trusty URI of canonical prereg bytes) and `TrialRunId` thread through `scientia_research_sessions` (`trial:{run_id}`), LLM telemetry labels, and `research_trial.*` metrics (`07-01`, `07-02`, `07-03`, `07-05`, `07-07`..`07-10`).
- [ ] **STORE-01**: `trials` domain (`trial_campaigns`, `trial_campaigns_log`, `trial_runs`, `trial_samples`) lands in `vox-db` at **schema baseline 97** (immediately after `kb` Phase 3's baseline 96) with `StoreError::BaselineStampedWithoutTables` loud guard (`07-04`, `07-06`).
- [ ] **STORE-02**: Campaign creation and terminal run + sample persistence execute in one atomic transaction with `PersistFailpoint` verification (`07-07`, `07-11`, `07-12`).
- [ ] **BENCH-09**: Phase `07.1` (`12-09`) persists signed code-gen benchmark provenance bundles into `trial_runs` and `trial_samples` at baseline 97 (`07.1-09`).

### 1C. Unified Code & Architecture Graph Data Floor (`GRAPH` / `DISC-12` — Phase 25)
- [ ] **GRAPH-01**: `vox-graph-reader` (`EXTRACTOR_VERSION = "5"`) emits typed edges (`CrateDep`, `ModDecl`, `UsePath`, `CallCandidate`), `crate -> file -> symbol` hierarchy, `.vox` declaration kinds (`ast_vox.rs`), and `layers.toml` layer assignments (`25-01`..`25-02`).
- [ ] **GRAPH-02**: Budgeted `AtlasQuery` (`lod.rs`, hard cap 400 nodes / 800 edges) and `vox_search_atlas` MCP/daemon RPC serve both the Phase 33 OpenWiki generator and the Phase 26–31 GUI Atlas (`25-03`..`25-04`).

### 1D. Docs Trust Signal Calibration & Ledger Sync (`SIGNAL` — Phase 32)
- [ ] **SIGNAL-01**: Illustrative/placeholder paths (`crates/foo/src/lib.rs`, `<crate>`, `path/to/...`) are exempted or fenced so false-positive `broken_path_ref` deficits are eliminated.
- [ ] **SIGNAL-02**: Every non-archived page with `0` active deficits and valid `source_ anchors` has `verified_hash` seeded in `contracts/reports/docs-freshness/ledger.v1.json`, raising public `verified` badge share from `2.9%` (`28/972`) to `>= 60%`.
- [ ] **SIGNAL-03**: `run_ssot_drift` runs `vox ci docs-freshness-ledger --write` after `crate-api-sync` so all 127 `docs/src/api/*.generated.md` pages are tracked in `ledger.v1.json`.
- [ ] **SIGNAL-04**: All 110 `unanchored` non-archived conceptual/tutorial pages receive either `source_code:`/`source_contracts:` anchors or `status: reference-only`.

---

## Wave 2: Core Backend Engines (OpenWiki Generator, Docs Burn-Down, Autonomy Daemon, KB Embeddings/Context, Trial Race Harness)

### 2A. OpenWiki Subsystem Hubs, Deep Crate Symbols & Diagrams (`WIKI` — Phase 33)
- [ ] **WIKI-01**: `vox ci subsystem-docs-sync` generates OpenWiki-style Subsystem Hub pages (`docs/src/architecture/subsystems/<layer-slug>.generated.md`) from `layers.toml` and Phase 25's `vox-graph-reader`.
- [ ] **WIKI-02**: `crate_api_docs.rs` extracts top-level `pub struct`, `pub enum`, `pub trait`, and `pub fn` signatures + doc comments + source file `#L<line>` links + reverse dependents (`Used By`) into `docs/src/api/<crate>.generated.md`.
- [ ] **WIKI-03**: Generated Subsystem Hubs and Crate API pages include deterministic Mermaid dependency and pipeline flowcharts validated by a Mermaid syntax linter.
- [ ] **WIKI-04**: `docs/src/.well-known/llms.txt` and Starlight sidebar navigation auto-index all Subsystem Hubs and Crate API pages.

### 2B. Docs Corpus Consolidation & Technical Debt Burn-Down (`DEBT` — Phase 34)
- [ ] **DEBT-01**: The 29 duplicate clusters in `contracts/reports/docs-freshness/prune-report.v1.json` are consolidated into single canonical SSOT pages with Starlight redirects.
- [ ] **DEBT-02**: All 122 broken `status: current` architecture pages are audited:aspirational specs move to `status: roadmap`/`superseded`/`deprecated`, and shipped pages have their `source_*` anchors repaired (`0` broken `status: current` pages).
- [ ] **DEBT-03**: Reader-facing `baseline.v1.json` deficits (`920` across `112` pages) are burned down to `< 100`, and `contracts/documentation/kb-facet-backlog.v1.yaml` (`556` pages) is reduced by `>= 50%`.

### 2C. Autonomy Oversight Contract, Daemon RPC, Consumers & Attention Inbox (`OVS` / `ATTN` — Phases 13–17)
- [ ] **OVS-01**: `OversightLevel` (`1..5`: Autonomous, Trusted, Balanced, Cautious, Supervised) and pure `resolve_effective_autonomy(level, mode, floor)` land in `vox-config` with one-way migration from legacy permissions (`Phase 13`).
- [ ] **OVS-02**: Daemon holds session-scoped `OversightLevel` (`orch.oversight_get`, `orch.oversight_set`, `orch.oversight_changed` event) shared by GUI, CLI, and MCP (`Phase 14`).
- [ ] **OVS-03**: Both background orchestrator tasks (`Phase 15`) and sync chat turns (`Phase 16`: tool approval, `hitl_doubt`, `mens-gate`, `budget.rs`) consume `EffectiveAutonomy` from the single resolver.
- [ ] **ATTN-01**: Cross-agent `AttentionAdmission` gate + persistent `AttentionInbox` (at DB baseline 98 if schema required) batches lower-priority interruptions into a single digest (`Phase 17`).

### 2D. KB Local Embeddings, Context Injection, Web Fallback, Rewrites & Staleness (`EMB` / `INJ` / `GROW` / `REWR` / `STALE` — Phases 37–41)
- [ ] **EMB-01..06**: Local Metal/CPU embedding pipeline in `vox-search` populates vector chunks incrementally and unlocks semantic + RRF hybrid parity on `vox_kb_query` (`Phase 37`, formerly `kb` Phase 5).
- [ ] **INJ-01..06**: Bounded, session-pinned, deduped `@kb` context injection into chat/agent turns with structured per-turn telemetry (`Phase 38`, formerly `kb` Phase 7).
- [ ] **GROW-01..06**: Opt-in web fallback (`quarantined_web`) + `vox_kb_record_finding` into the baseline-96 `kb_findings` inbox with mechanical citation verification (`Phase 39`, formerly `kb` Phase 9).
- [ ] **REWR-01..05**: Top-25 eval-prioritized page-one rewrites with compilable `.vox` doctests lift `MRR@5 >= 0.70` and `Recall@5 >= 0.85` (`Phase 40`, formerly `kb` Phase 10).
- [ ] **STALE-01..05**: Symbol-hash (`vox-graph-reader`) + git-diff staleness detector queues stale `verified_ finding` and doc entries with `vox kb verify` (`Phase 41`, formerly `kb` Phase 11).

### 2E. Research Trial Race Harness, Verified Knowledge, Promotion & Triggered Benchmarks (`RACE` / `KNOW` / `PROMO` / `HARD` / `BENCH` — Phases 08–11 & 42)
- [ ] **RACE-01..04**: Signed preregistration, atomic spend reservations, group-sequential stopping, and paired interleaved arm racing (`Phase 08`).
- [ ] **KNOW-01..04**: Frozen per-arm KB snapshots (`vox_kb_*`) and memory ablation runs (`Phase 09`).
- [ ] **PROMO-01..04**: Two-campaign promotion governance (`exploratory` + `holdout_confirmation`) emitting human-reviewed config diff proposals (`Phase 10`).
- [ ] **HARD-01..04**: Offline replay verification, adversarial contamination checks, and graduation gate (`Phase 11`).
- [ ] **BENCH-13**: Triggered `.vox` language code-gen benchmark campaign under Phase 07/08 governance (`Phase 42`, formerly `research-trial-flywheel` Phase 13).

---

## Wave 3: Sequenced GUI Unification (`crates/vox-gui/ui/` — Strictly Ordered on Shared Files)

### 3A. Capability Explorer & Coverage Gate (`DISC-09..11` — Phase 06.4, 13 plans ready)
- [ ] **DISC-09..11**: `Capabilities` sidebar view (`CapabilityExplorer.tsx`) browses all CLI/MCP/GUI capabilities by category with live status badges, action buttons, and the `vox ci capability-surface-coverage` gate (`06.4-01`..`06.4-13`).

### 3B. Unified Code & Architecture Atlas GUI (`ATLAS` / `DISC-12..14` — Phases 26–31)
- [ ] **ATLAS-01..04**: Viewport-culled React Flow + ELKjs canvas over Phase 25's `vox_search_atlas` (`Phase 26`), branch/worktree diff overlay (`Phase 27`), dockable widget beside chat in `ChatSurface.tsx` (`Phase 28`), live file-watcher sync + active agent badges (`Phase 29`), click-to-`@` insertion into `Loquela.tsx` (`Phase 30`), and Playwright visual proof (`Phase 31`).

### 3C. KB Library, Per-Turn Context Inspector & Inbox/Health GUI (`GUI-KB` — Phases 43–45)
- [ ] **GUI-KB-01..03**: Unified `Library` view absorbs `DocReader.tsx` with facet filter bar, hybrid search, trust badges, and `Open in editor` (`Phase 43`, formerly `kb` Phase 6).
- [ ] **GUI-KB-04..06**: Per-turn Context Inspector drawer in `ChatSurface.tsx` + `@kb` composer chip/scope controls in `Loquela.tsx` (`Phase 44`, formerly `kb` Phase 8).
- [ ] **GUI-KB-07..08**: Inbox review & KB Health tabs inside `Library` (`Phase 45`, formerly `kb` Phase 12).

### 3D. Autonomy Oversight Composer Control, Digest Panel & Claims Ledger (`GUI-OVS` — Phases 18–19)
- [ ] **GUI-OVS-01..03**: Single `Oversight: 1..5` composer control in `Loquela.tsx` replaces `RiskPopover.tsx` and `DriveConsole.tsx`'s separate dial, plus `InboxDigestPanel.tsx` for batched attention items (`Phase 18`).
- [ ] **CLAIMS-01**: `contracts/orchestration/autonomy-claims.v1.yaml` maps every UI string in the Oversight control to an automated backend behavioral test (`Phase 19`).

---

## Wave 4: Self-Healing Automation & End-to-End Cold-Start Sign-Off

### 4A. Constrained Nightly LLM Doc Claim Verifier (`VERIFY` — Phase 35)
- [ ] **VERIFY-01..04**: `vox ci docs-claim-verify` extracts structured `(doc_path, section, symbol_or_command, prose_claim)` tuples for git-diff-impacted pages and verifies them via `vox_actor_runtime::llm` (`TaskCategory::DocClaimJudge`) with zero hallucinated warnings.

### 4B. Self-Healing Docs Draft-PR Bot & Freshness Ratchet (`BOT` — Phase 36)
- [ ] **BOT-01..04**: Nightly `.github/workflows/docs-self-heal.yml` runs `vox ci docs-heal --apply` (validating patches with `vox-doc-pipeline --lint-only` + doctest check), opens a draft PR with `SSOT_AUTOREGEN_TOKEN`, and ratchets `thresholds.v1.yaml` to `verified >= 85%` and `0` broken `status: current` pages.

### 4C. Cold-Start Time-to-Find Journeys (`DISC-15..17` — Phase 06.6)
- [ ] **DISC-15..17**: Five scripted cold-start Playwright journeys complete in `<= 60s` and `<= 3` interactions each across Palette (`⌘K`), Capability Explorer, Code & Architecture Atlas, Library, and Oversight Composer (`Phase 06.6`).

---

## Traceability Summary

| Wave | Unified Phases | Requirement Families | Primary Model Routing |
|------|----------------|----------------------|-----------------------|
| **Wave 0** | `Phase 05` (absorbs completed `01..04`, `06.1..06.3`, `20..24`) | `CONV-01..04` | `[MODEL: ARGON-4]` (`[SEQUENTIAL]`) |
| **Wave 1** | `Phase 12`, `Phase 07` + `07.1`, `Phase 25`, `Phase 32` | `FLOOR`, `TRIAL`/`STORE`/`BENCH-09`, `GRAPH`, `SIGNAL` | `ARGON-4` (schema/floor/typestate) + `FLASH-3.8` (unit/CLI/calibration) |
| **Wave 2** | `Phase 33`, `Phase 34`, `Phases 13..17`, `Phases 37..41`, `Phases 08..11 & 42` | `WIKI`, `DEBT`, `OVS`/`ATTN`, `EMB`/`INJ`/`GROW`/`REWR`/`STALE`, `RACE`/`KNOW`/`PROMO`/`HARD`/`BENCH-13` | `FLASH-3.8` (generators/doc-debt/consumers) + `ARGON-4` (resolver/stats/embeddings) |
| **Wave 3** | `Phase 06.4`, `Phases 26..31`, `Phases 43..45`, `Phases 18..19` | `DISC-09..11`, `ATLAS`/`DISC-12..14`, `GUI-KB`, `GUI-OVS`/`CLAIMS` | `[SEQUENTIAL]` on shared React files; `FLASH-3.8` components + `ARGON-4` composer/dock integration |
| **Wave 4** | `Phase 35`, `Phase 36`, `Phase 06.6` | `VERIFY-01..04`, `BOT-01..04`, `DISC-15..17` | `ARGON-4` (`Phase 35` LCEF verifier) + `FLASH-3.8` (`Phase 36` bot & `06.6` Playwright journeys) |
