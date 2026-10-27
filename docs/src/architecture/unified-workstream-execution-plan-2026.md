---
title: "Unified Workstream & Worktree Execution Master Plan (2026 Q4)"
description: "Single-branch, dependency-ordered master plan unifying all six parallel GSD workstreams and four feature worktrees for execution under Antigravity IDE with Gemini 3.8 Flash and Gemini 4 Argon."
category: "Architecture SSOTs"
status: "current"
sort_order: 15
topic: "agent-orchestrator"
domain: "orchestration"
authorship: "llm-assisted"
authorship_model: "claude-3-5-sonnet"
training_eligible: false
---

# Unified Workstream & Worktree Execution Master Plan (2026 Q4)

<!-- kb:summary -->
Establishes the single-branch execution and convergence plan across six parallel GSD workstreams and feature worktrees for Vox in 2026 Q4. Sequences four dependency-ordered waves covering baseline consolidation, durable execution and capability profiles, deep research, and final release criteria under model-routed agent execution.
<!-- /kb:summary -->

> [!IMPORTANT]
> **Single Stream & Zero Worktree Deletion Policy:** All parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and active feature worktrees (`capability-profiles`, `true-workflow-durability`, `warp-fork-axis-gui-b682cd`, `api-key-management-16bb08`) are now unified into `.planning/workstreams/unified/` (active in `.planning/active-workstream` and mirrored at `.planning/ROADMAP.md`) inside `/Users/brbrainerd/dev/vox`. **Do not delete, prune, or clean any existing worktrees under `.claude/worktrees/` or `.worktrees/`.**

---

## 1. Inventory of Unified Workstreams & Feature Worktrees

| Source Stream / Worktree | Branch & HEAD SHA | Completed Baseline | Remaining Unified Phases |
|---|---|---|---|
| **Main Worktree** (`/Users/brbrainerd/dev/vox`) | `feat/research-trial-flywheel` (`b64377d40` + 403 uncommitted files) | `docs-freshness` v1.3 (`Phases 20, 20.1, 21–24`, 32/32 plans); `research-trial-flywheel` `07-02..04` & `12-01..08` | Checkpointed in **Phase 05 (`05-01`)** |
| **Workstream 1: `kb`** (`.claude/worktrees/research-detection-chat-gui-05d826`) | `claude/vox-knowledge-base-research-2964d2` (`e60182f6f`, PR #645) | **Phases 01–04 (48/48 plans, CI-green):** Eval harness, `vox_kb_*` MCP tools, **VoxDb Baseline 96**, `vox-kb-facets` L0 crate | Merged in `05-03`; remaining phases sequenced as **Phases 37–41** (Wave 2) and **Phases 43–45** (Wave 3) |
| **Workstream 2: `discoverability`** (`.claude/worktrees/vox-discoverability-a72c8c`) | `claude/vox-discoverability-a72c8c` (`d3a93f756`) | **Phases 06.1–06.3 (21/21 plans):** Fast CI loop, Capability Index SSOT (`1544` rows, `vox explain`/`features`), Universal Palette `⌘K` | Merged in `05-04`; **Phase 06.4** (13 ready plans) starts Wave 3; `06.5` merged into **Phases 25–31**; **Phase 06.6** ends Wave 4 |
| **Workstream 3: `docs-freshness` v1.4** (`.planning/workstreams/docs-freshness/` + `.claude/worktrees/docs-freshness-p20-1`) | `b33b37011` (`#635`, `#638`, `#640`, `#642`) + in-tree v1.3 | **Phases 20, 20.1, 21–24 (32/32 plans)** | **Phase 32** (Wave 1), **Phases 33–34** (Wave 2), **Phases 35–36** (Wave 4) |
| **Workstream 4: `autonomy-ux`** (`.planning/workstreams/autonomy-ux/`) | In `/Users/brbrainerd/dev/vox` | **Phase 12 has 8 checker-verified plans (`12-01`..`12-08-PLAN.md`)** ready to execute | **Phase 12** (Wave 1), **Phases 13–17** (Wave 2), **Phases 18–19** (Wave 3) |
| **Workstream 5: `code-atlas`** (`.planning/workstreams/code-atlas/`) | In `/Users/brbrainerd/dev/vox` | Phase 25 context/architecture defined | **Phase 25** (Wave 1 data floor), **Phases 26–31** (Wave 3 GUI, absorbing `06.5`) |
| **Workstream 6: `research-trial-flywheel`** (`.planning/workstreams/research-trial-flywheel/`) | `feat/research-trial-flywheel` (`b64377d40`) | `07-02`, `07-03`, `07-04` + `12-01..08` complete; 9 remaining plans in Phase 07 + `12-09` written | **Phase 07 & 07.1** (Wave 1 at **DB Baseline 97**), **Phases 08–11 & 42** (Wave 2) |
| **Feature Worktree: `capability-profiles`** (`.worktrees/capability-profiles`) | `feat/capability-profiles` (`77f5dfa9a`) | `Exposure`, `SideEffectClass`, `Subsystem` in `vox-mcp-registry` & `catalog.v1.yaml` | Merged in **Phase 05 (`05-04`)** |
| **Feature Worktree: `true-workflow-durability`** (`.worktrees/true-workflow-durability`) | `feat/true-workflow-durability` (`ce9cbc67c`) | ADR-021/041 workflow leases, `VoxDbTracker`, `__vox_run_workflow` | Merged in **Phase 05 (`05-02`)** |
| **Feature Worktree: `warp-fork-axis-gui`** (`.claude/worktrees/warp-fork-axis-gui-b682cd`) | `warp-fork-axis-gui-b682cd` (`345542f4a`) | `term.open/write/resize/signal/close/list` daemon session host | Merged in **Phase 05 (`05-02`)** |
| **Feature Worktree: `fmt-safety`** (`.claude/worktrees/api-key-management-16bb08`) | `wip/fmt-safety-2026-10-12` (`499c75bce`) | `vox fmt` comment-preservation and binop/arm fixes | Merged in **Phase 05 (`05-02`)** |

---

## 2. Critical Ordering Invariants ("Why Order Matters")

```vox-diagram
graph {
    node W0: "Wave 0: Phase 05 Branch & Worktree Convergence (ARGON-4)"
    node DB96: "VoxDb Baseline 96 (kb Phase 03) + Merged catalog.v1.yaml"
    node P12: "Wave 1: Phase 12 Safety Floor (12-01..08)"
    node P07: "Wave 1: Phase 07 & 07.1 Trials Domain (VoxDb Baseline 97)"
    node P25: "Wave 1: Phase 25 vox-graph-reader v5 + vox_search_atlas"
    node P32: "Wave 1: Phase 32 Docs Trust Signal Calibration"
    node P33: "Wave 2: Phase 33 OpenWiki Subsystem Hubs & Mermaid"
    node P34: "Wave 2: Phase 34 Docs Debt Burn-Down (920 -> <100)"
    node P13_17: "Wave 2: Phases 13-17 Autonomy Daemon & Attention Inbox (Baseline 98)"
    node P08_11: "Wave 2: Phases 08-11 & 42 Trial Race Harness & Benchmarks"
    node P37_41: "Wave 2: Phases 37-41 KB Embeddings, Context Injection & Staleness"
    node P26_31: "Wave 3: Sequenced GUI (06.4 -> 26..31 Atlas -> 43..45 KB -> 18..19 Oversight)"
    node P35_36: "Wave 4: Phases 35-36 Nightly LLM Doc Verifier & Self-Healing Bot"
    node P06_6: "Wave 4: Phase 06.6 Cold-Start Time-to-Find Journeys"
    W0 -> DB96
    DB96 -> P12
    DB96 -> P07
    DB96 -> P25
    DB96 -> P32
    P25 -> P33
    P32 -> P33
    P32 -> P34
    P12 -> P13_17
    P07 -> P08_11
    DB96 -> P37_41
    P25 -> P26_31
    P13_17 -> P26_31
    P37_41 -> P26_31
    P34 -> P35_36
    P37_41 -> P35_36
    P26_31 -> P06_6
}
```

1. **Database Schema Baseline Monotonicity (`crates/vox-db/src/schema/manifest.rs`):**
   - `main` is at `95` (`model_benchmark_results`).
   - `kb` Phase 03 (`03-03`) already took **`BASELINE_VERSION = 96`** (`kb_findings` + `search_documents.trust_tier`). Merging it in `05-03` locks `96`.
   - `research-trial-flywheel` Plan `07-06-PLAN.md` and `docs/src/architecture/research-trial-tier-a-schema-2026.md` have been updated to **`BASELINE_VERSION = 97`** (`trials` domain).
   - `autonomy-ux` Phase 17 (`AttentionInbox`) is assigned **`BASELINE_VERSION = 98`** if a persistent table is required.
2. **Single-Pass Contract & Registry Convergence (`contracts/operations/catalog.v1.yaml`):**
   - Four streams extend `catalog.v1.yaml`: `kb` (`vox_kb_*`), `docs-freshness` (20 CLI rows), `discoverability` (`vox explain`/`features`, `side_effect_class`, `mcp_namespace`), and `capability-profiles` (`Exposure`, `SideEffectClass`, `Subsystem`).
   - Plan `05-04-PLAN.md` merges all four schema extensions first and runs `vox run scripts/ssot-regen.vox` once. Subsequent phases add individual rows via targeted line edits and never rewrite or re-sort `catalog.v1.yaml`.
3. **Graph Data Floor (`Phase 25`) Precedes Both OpenWiki (`Phase 33`) and GUI Atlas (`Phases 26–31`):**
   - `Phase 25` upgrades `vox-graph-reader` (`EXTRACTOR_VERSION = "5"`) with typed edges, `crate -> file -> symbol` hierarchy, `.vox` declaration kinds (`ast_vox.rs`), and `vox_search_atlas`.
   - Both `Phase 33` (OpenWiki Subsystem Hubs & Mermaid generator) and `Phases 26–31` (Unified Code & Architecture Atlas GUI, absorbing `discoverability` Phase `06.5`) consume `Phase 25`'s output instead of building duplicate AST/graph extractors.
4. **Strict Sequential File Lock on Shared GUI Components (`Wave 3`):**
   - `Sidebar.tsx`, `ChatSurface.tsx`, `Loquela.tsx`, `DriveConsole.tsx`, and `DocReader.tsx` are touched by four streams (`discoverability`, `code-atlas`, `kb`, `autonomy-ux`).
   - All backend RPCs land in Waves 0–2; Wave 3 runs strictly sequentially:
     - **Slot 1 (`Phase 06.4`):** `Sidebar.tsx` + `CapabilityExplorer.tsx`
     - **Slot 2–7 (`Phases 26–31`):** `AtlasCanvas.tsx` -> `ChatSurface.tsx` (dock) -> `Loquela.tsx` (click-to-`@`)
     - **Slot 8–10 (`Phases 43–45`):** `DocReader.tsx` (`Library`) -> `ChatSurface.tsx` (Context Inspector) -> `Loquela.tsx` (`@kb` chips)
     - **Slot 11–12 (`Phases 18–19`):** `Loquela.tsx` + `DriveConsole.tsx` + `RiskPopover.tsx` (`Oversight: 1..5` + `InboxDigestPanel.tsx`)

---

## 3. Antigravity IDE Execution Protocol: Gemini 3.8 Flash + Gemini 4 Argon

Per [`docs/src/architecture/gemini-3-5-flash-antigravity-limitations-2026-06-18.md`](gemini-3-5-flash-antigravity-limitations-2026-06-18.md) and [`docs/src/contributors/antigravity-handoff-and-skill-gaps-2026-06-18.md`](../contributors/antigravity-handoff-and-skill-gaps-2026-06-18.md), execution under Antigravity IDE uses a two-tier model routing discipline:

### A. Mitigating Gemini 3.8 Flash Weaknesses (`[MODEL: FLASH-3.8]`)

| Gemini 3.8 Flash Weakness | Mandatory Plan & Execution Safeguard |
|---|---|
| **1. Abrupt mid-task quota/turn cutoff with no checkpoint** | Every task touches **at most 1–3 source files + 1 test file**, ends in a green `timeout 900s cargo test -p <crate> --lib` (or `pnpm --dir crates/vox-gui/ui test`), and writes its `SUMMARY.md` immediately. |
| **2. Symbol/signature hallucination & 128K context drop-off** | Every task begins with a **Pre-Flight Verification (`read_first`)** using `rg -n` or `view_file` on the exact line range before any edit. Tasks never require loading >5 files into context. |
| **3. Destructive whole-file rewrites on large contracts** | Rule 11 (`00-core.md`): Never rewrite a whole existing file. Use `replace_file_content` on exact line ranges (`<= 80` lines). Check `git diff --numstat` after every contract edit. |
| **4. Infinite self-correction loops on subtle compiler/borrow errors** | **Two-Strike Circuit Breaker:** If the same compile or test failure persists after **2 attempts** on Gemini 3.8 Flash, immediately halt with `DRIVE: STOPPED <reason>` and switch the task to **Gemini 4 Argon (`[MODEL: ARGON-4]`)**. |
| **5. Parallel subagent file clobbering** | Only tasks marked `[PARALLEL-SAFE]` with **100% disjoint `files_modified` sets** may run concurrently. Any task touching `catalog.v1.yaml`, `manifest.rs`, `Loquela.tsx`, `ChatSurface.tsx`, or `Sidebar.tsx` is `[SEQUENTIAL]`. |

### B. Model Assignment Matrix (`[MODEL: FLASH-3.8]` vs `[MODEL: ARGON-4]`)

| Phase / Plan Range | Assigned Model | Rationale |
|---|---|---|
| **Wave 0: `Phase 05` (`05-01`..`05-04`)** | **`[MODEL: ARGON-4]`** | Multi-branch git merge conflict resolution across `catalog.v1.yaml`, `vox-db` baseline 96, `layers.toml`, and 403 dirty working-tree files. |
| **Wave 1: `Phase 12` (`12-01`..`12-03`, `12-08`)** | **`[MODEL: ARGON-4]`** | Security-critical `permission-modes.v2.yaml` action-only floor classifier and adversarial mutation proof. |
| **Wave 1: `Phase 12` (`12-04`..`12-07`)** | **`[MODEL: FLASH-3.8]`** | Scoped single-crate lowering-path closures (`vox-config`, `vox-orchestrator-mcp`, `vox-cli`) with pre-written TDD specs. |
| **Wave 1: `Phase 07` (`07-01`, `07-05`, `07-06`, `07-07`)** | **`[MODEL: ARGON-4]`** | Ed25519 canonical-bytes signing, VoxDb baseline 97 migration + loud guard, and atomic `PersistFailpoint` transaction state machine. |
| **Wave 1: `Phase 07` (`07-08`..`07-12`) & `07.1` (`12-09`)** | **`[MODEL: FLASH-3.8]`** | Telemetry label propagation, CLI wiring, observational campaign runner, and `12-09` benchmark bundle persistence. |
| **Wave 1: `Phase 25` (`25-01`)** | **`[MODEL: ARGON-4]`** | `vox-graph-reader` `EXTRACTOR_VERSION = "5"` schema, typed edge kinds, and `crate -> file -> symbol` hierarchy design. |
| **Wave 1: `Phase 25` (`25-02`..`25-04`) & `Phase 32` (`32-01`..`32-04`)** | **`[MODEL: FLASH-3.8]`** | `.vox` declaration kind mapping, budgeted `AtlasQuery`, and deterministic `vox-doc-pipeline` trust signal calibration. |
| **Wave 2: `Phase 33` (`33-01`) & `Phase 34` (`34-01`)** | **`[MODEL: ARGON-4]`** | OpenWiki Subsystem Hub architecture synthesis and semantic merging of 29 duplicate doc clusters. |
| **Wave 2: `Phase 33` (`33-02`..`33-04`) & `Phase 34` (`34-02`..`34-04`)** | **`[MODEL: FLASH-3.8]`** | `crate_api_docs.rs` symbol table extraction, Mermaid syntax linter, frontmatter status reclassification, and path-deficit repair batches. |
| **Wave 2: `Phases 13, 16, 17, 37, 38, 08–11`** | **`[MODEL: ARGON-4]`** | Pure `EffectiveAutonomy` resolver, sync chat doubt/gate wiring, `AttentionAdmission` gate, Metal/CPU embeddings, bounded `@kb` injection, and statistical trial racing. |
| **Wave 2: `Phases 14, 15, 39, 40, 41, 42`** | **`[MODEL: FLASH-3.8]`** | Daemon RPC boilerplate, background task wiring, `kb_findings` MCP tool, page-one doc rewrites, and staleness CLI. |
| **Wave 3: `Phase 06.4` (`06.4-01`..`13`), `27`, `29`, `31`, `43`, `45`, `19`** | **`[MODEL: FLASH-3.8]`** | Self-contained React view components, badge overlays, and Playwright verification specs. |
| **Wave 3: `Phases 26, 28, 30, 44, 18`** | **`[MODEL: ARGON-4]`** | Complex React Flow + ELKjs viewport culling, dockview `ChatSurface.tsx` integration, and `Loquela.tsx` composer unification (`@` mentions, `@kb` chips, `Oversight: 1..5`). |
| **Wave 4: `Phase 35` (`35-01`..`35-03`)** | **`[MODEL: ARGON-4]`** | DocPrism-style LCEF prompt engineering, AST symbol slicer, and `TaskCategory::DocClaimJudge` false-positive calibration. |
| **Wave 4: `Phase 36` & `Phase 06.6`** | **`[MODEL: FLASH-3.8]`** | `docs-self-heal.yml` workflow, `thresholds.v1.yaml` ratchet, and the 5 cold-start Playwright time-to-find journeys. |

---

## 4. Immediate Execution Queue (Ready Right Now)

Because `.planning/active-workstream` is now set to `unified`, you can execute in either of two modes:

1. **Interactive / Full Branch Convergence Mode (`[MODEL: ARGON-4]`):**
   - Execute **Phase 05 (`05-01-PLAN.md` -> `05-04-PLAN.md`)** in `.planning/workstreams/unified/phases/05-unified-branch-worktree-convergence/` to checkpoint the 403 uncommitted files onto `feat/unified-2026-q4` and merge all external worktree branches into `/Users/brbrainerd/dev/vox`.
2. **Headless Task-Driver Mode (`/drive-task <plan-path> <task-num>` under `.agents/rules/00-core.md` where git commands are gated):**
   - Immediately run the ready Wave 1 plans whose target files are clean in `/Users/brbrainerd/dev/vox`:
     - **Autonomy Safety Floor (`Phase 12`):** `.planning/workstreams/unified/phases/12-safety-floor-lowering-paths/12-01-PLAN.md` .. `12-08-PLAN.md`
     - **Trial Identity & DB Baseline 97 (`Phase 07`):** `.planning/workstreams/unified/phases/07-trial-identity-contracts-observational-baseline/07-01-PLAN.md` and `07-05-PLAN.md` .. `07-12-PLAN.md`
     - **Capability Explorer (`Phase 06.4` — after `05-04` merge):** `.planning/workstreams/unified/phases/06.4-capability-explorer-coverage-gate-inserted/06.4-01-PLAN.md` .. `06.4-13-PLAN.md`
