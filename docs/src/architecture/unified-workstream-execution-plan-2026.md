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
| **Workstream 3: `docs-freshness` v1.4** (`.planning/workstreams/docs-freshness/` + `.claude/worktrees/docs-freshness-p20-1`) | `b33b37011` (`#635`, `#638`, `#640`, `#642`) + in-tree v1.3 | **Phases 20, 20.1, 21–24, 32, 55–58 (complete):** `810 verified, 0 drifting, 0 broken, 0 unanchored` | **Phases 33–34** (Wave 2: OpenWiki Subsystem Hubs + 29 duplicate cluster merges + KB facet backlog), **Phases 35–36** (Wave 4) |
| **Workstream 4: `autonomy-ux`** (`.planning/workstreams/autonomy-ux/`) | In `/Users/brbrainerd/dev/vox` | **Phase 12 (`12-01`..`12-08`) & Phase 59 (`M1`–`M34` mutation proofs + code review) complete** | **Phases 13–17** (Wave 2), **Phases 18–19** (Wave 3) |
| **Workstream 5: `code-atlas`** (`.planning/workstreams/code-atlas/`) | In `/Users/brbrainerd/dev/vox` | Phase 25 context/architecture defined | **Phase 25** (Wave 1 data floor), **Phases 26–31** (Wave 3 GUI, absorbing `06.5`) |
| **Workstream 6: `research-trial-flywheel`** (`.planning/workstreams/research-trial-flywheel/`) | `feat/research-trial-flywheel` (`b64377d40`) | `07-02`, `07-03`, `07-04` + `12-01..08` complete; 9 remaining plans in Phase 07 + `12-09` written | **Phase 07 & 07.1** (Wave 1 at **DB Baseline 97**), **Phases 08–11 & 42** (Wave 2) |
| **Feature Worktree: `capability-profiles`** (`.worktrees/capability-profiles`) | `feat/capability-profiles` (`77f5dfa9a`) | `Exposure`, `SideEffectClass`, `Subsystem` in `vox-mcp-registry` & `catalog.v1.yaml` | Merged in **Phase 05 (`05-04`)** |
| **Feature Worktree: `true-workflow-durability`** (`.worktrees/true-workflow-durability`) | `feat/true-workflow-durability` (`ce9cbc67c`) | ADR-021/041 workflow leases, `VoxDbTracker`, `__vox_run_workflow` | Merged in **Phase 05 (`05-02`)** |
| **Feature Worktree: `warp-fork-axis-gui`** (`.claude/worktrees/warp-fork-axis-gui-b682cd`) | `warp-fork-axis-gui-b682cd` (`345542f4a`) | `term.open/write/resize/signal/close/list` daemon session host (`H1–H5`) | Merged in **Phase 05 (`05-02`)**; `H7–H8` completed in **Phase 46** (Wave 2) and `H6` + Warp block chrome completed in **Phase 47** (Wave 3 GUI Slot 13) |
| **Feature Worktree: `fmt-safety`** (`.claude/worktrees/api-key-management-16bb08`) | `wip/fmt-safety-2026-10-12` (`499c75bce`) | `vox fmt` comment-preservation and binop/arm fixes | Merged in **Phase 05 (`05-02`)**; `layout_only` safety guard completed in **Phase 70 (`70-01`)** |
| **Land Wave Worktrees (`land/w1..w5` + `fix/*`/`work/*`)** (`.claude/worktrees/agent-*`) | `land/w1-deps` (`a4e76b8a3`), `land/w2-hygiene` (`8300c74ff`), `land/w3-workflows` (`f206f7cfb`), `land/w4-quarantine` (`beba0a024`), `land/w5-research-tests` (`50977ba18`) + `fix/*`/`work/*` | 69 unmerged hygiene/deps/workflow/research-test commits across 9 worktrees + `2026-10-12-nightly-green-antigravity-handoff.md` + `evidence-ledger.v1.json` refresh | Sequenced in **Phase 70 (`70-01` — Wave 6)** |
| **Queued Milestone Context: `Clavis Next`** (`.planning/MILESTONE-CONTEXT.md`) | `a67925b57` (queued for `/gsd-new-milestone`) | Full specification for extensible provider manifests, `clavis://` URI resolver, `vox secrets import/acquire/status/rotate/sync`, and zero-trust agent credential broker | Sequenced in **Phase 71 (`71-01`)** & **Phase 72 (`72-01` — Wave 6)** |
| **Mens ML Training Worktrees (`mens/*` + 8 locked `.claude/worktrees/mens-*`)** | `mens/mac-lane-hardening` (`5cb8f9dec`), `mens/qwen38-27b-run` (`5a290248e`), `fix/cuda-mens-kv-cache-eos` (`3a015a024`), `fix/metal-hybrid-fallback-routing` (`8fb684d73`) | Apple Silicon (`M1..M4`) unified-memory presets, Qwen 3.8 27B QLoRA profile, KV-cache EOS & Metal fallback fixes, quantize/eval gates | Sequenced in **Phase 73 (`73-01` — Wave 6)** |
| **Populi Mesh Phase 3 Worktrees (`mesh-phase3-*`)** (`.claude/worktrees/mesh-phase3-*`) | `mesh-phase3-populi-httpop` (`1cd2b31a4`), `mesh-phase3-a2a-mailbox` (`00313ae50`), `mesh-phase3-queue-stats` (`9a903c94f`) | `PopuliHttpOp` ported onto `vox-mesh-transport` iroh QUIC, durable A2A mailbox, live queue stats, fail-closed `Dispatch`/`Wait` | Sequenced in **Phase 74 (`74-01` — Wave 6)** |
| **Sustainability, External Adoption & Standalone Crates** | In `/Users/brbrainerd/dev/vox` | **Phase 60 (`FUND-01..04`)** & **Phase 61 (`DIST-01..02`)** complete; `CRATE-01..02` of **Phase 62** complete | **Phase 62 (`62-01`)** (Wave 2) & **Phase 63 (`63-01` Empirical Preprint + `CR-L8` Flywheel)** (Wave 4) |
| **Cross-Portfolio Domain Completeness (`portfolio-domain-gap-audit-research-2026.md`)** | In `/Users/brbrainerd/dev/vox` | Audited across 35 `~/dev` repositories (`brVideo`, `ngi.video`, `fableforge`, `gigme`, `ramada`, `cineflow`, `open-melvor`, `greaterFool`, `talehall`, `ytfloat`) | **Phases 64–69 (`POLYTOOL`, `STREAM`, `RANK`, `USERDB`, `CLOUD`, `SIM` — Wave 5)** |

---

## 2. Critical Ordering Invariants ("Why Order Matters")

```vox-diagram
graph {
    node W0: "Wave 0: Phase 05 Branch & Worktree Convergence (DONE)"
    node DB96: "VoxDb Baseline 96 (kb Phase 03) + Merged catalog.v1.yaml + SessionHost H1-H5"
    node P12: "Wave 1: Phase 12 Safety Floor (12-01..08, DONE)"
    node P07: "Wave 1: Phase 07 & 07.1 Trials Domain (VoxDb Baseline 97)"
    node P25: "Wave 1: Phase 25 vox-graph-reader v5 + vox_search_atlas"
    node P32: "Wave 1: Phase 32 Docs Trust Signal Calibration (DONE via Phase 58)"
    node P33: "Wave 2: Phase 33 OpenWiki Subsystem Hubs & Phases 55-58 Docs/Mermaid (55-58 DONE)"
    node P34: "Wave 2: Phase 34 Duplicate Cluster Consolidation & KB Facet Burn-Down"
    node P13_17: "Wave 2: Phases 13-17 Autonomy Daemon & Attention Inbox (Baseline 98)"
    node P46: "Wave 2: Phases 46 (DONE), 49 Structured Shell, 52-53 Scientia, 60-61 (DONE), 62 Crates"
    node P08_11: "Wave 2: Phases 08-11 & 42 Trial Race Harness & Benchmarks"
    node P37_41: "Wave 2: Phases 37-41 KB Embeddings, Context Injection & Staleness"
    node P26_31: "Wave 3: Sequenced GUI (06.4 -> 26..31 Atlas -> 43..45 KB -> 18..19 Oversight -> 47-48 DONE -> 50-51, 54, 57 DONE)"
    node P35_36: "Wave 4: Phases 35-36 Doc Verifier/Bot, 59 (DONE) & 63 Empirical Preprint + CR-L8 Flywheel"
    node P06_6: "Wave 4: Phase 06.6 Cold-Start Time-to-Find Journeys"
    node W5: "Wave 5: Phases 64-69 Cross-Portfolio Domain Runtime (POLYTOOL, STREAM, RANK, USERDB, CLOUD, SIM)"
    node W6: "Wave 6: Phases 70-74 Worktree Land Wave, Clavis Next, Mens Apple Silicon & Populi Mesh Phase 3"
    W0 -> DB96
    DB96 -> P12
    DB96 -> P07
    DB96 -> P25
    DB96 -> P32
    P25 -> P33
    P32 -> P33
    P32 -> P34
    P12 -> P13_17
    P12 -> P46
    P07 -> P08_11
    DB96 -> P37_41
    P25 -> P26_31
    P13_17 -> P26_31
    P37_41 -> P26_31
    P46 -> P26_31
    P34 -> P35_36
    P37_41 -> P35_36
    P26_31 -> P06_6
    DB96 -> W5
    DB96 -> W6
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
   - `Sidebar.tsx`, `ChatSurface.tsx`, `Loquela.tsx`, `DriveConsole.tsx`, `DocReader.tsx`, and `Console/*.tsx` (`transport.ts`) are touched across streams.
   - All backend RPCs land in Waves 0–2; Wave 3 runs strictly sequentially:
     - **Slot 1 (`Phase 06.4`):** `Sidebar.tsx` + `CapabilityExplorer.tsx`
     - **Slot 2–7 (`Phases 26–31`):** `AtlasCanvas.tsx` -> `ChatSurface.tsx` (dock) -> `Loquela.tsx` (click-to-`@`)
     - **Slot 8–10 (`Phases 43–45`):** `DocReader.tsx` (`Library`) -> `ChatSurface.tsx` (Context Inspector) -> `Loquela.tsx` (`@kb` chips)
     - **Slot 11–12 (`Phases 18–19`):** `Loquela.tsx` + `DriveConsole.tsx` + `RiskPopover.tsx` (`Oversight: 1..5` + `InboxDigestPanel.tsx`)
     - **Slot 13 (`Phase 47` — DONE):** `commands/pty.rs` + `transport.ts` + `TerminalTab.tsx` (`@xterm/addon-webgl`, retire `osc633.ts`) + `Console.tsx` (`BlockStreamView.tsx` Warp block cards + session picker)
     - **Slot 14 (`Phase 48` — DONE):** `HarnessRiverPanel.tsx` (`WaveformTierStrip`, `AgentTrajectoryBraid`, `VerificationFunnelFooter`) + `GamifyView.tsx`
     - **Slot 15–17 (`Phases 50, 51, 54`):** Bidirectional Chat↔Terminal (`Phase 50`) -> Unified IntelliSense (`Phase 51`) -> Scientia Publication Studio GUI (`Phase 54`)
     - **Slot 18 (`Phase 57` — DONE):** `voxlang.org` Visual Theme, Sidebar/TOC Hiders & Beginner Frontpage (`Phase 57`)

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
| **5. Parallel subagent file clobbering** | Only tasks marked `[PARALLEL-SAFE]` with **100% disjoint `files_modified` sets** may run concurrently. Any task touching `catalog.v1.yaml`, `manifest.rs`, `Loquela.tsx`, `ChatSurface.tsx`, `Sidebar.tsx`, or `transport.ts` is `[SEQUENTIAL]`. |

### B. Model Assignment Matrix (`[MODEL: FLASH-3.8]` vs `[MODEL: ARGON-4]`)

| Phase / Plan Range | Assigned Model | Rationale |
|---|---|---|
| **Wave 0: `Phase 05` (`05-01`..`05-04` — DONE)** | **`[MODEL: ARGON-4]`** | Multi-branch git merge conflict resolution across `catalog.v1.yaml`, `vox-db` baseline 96, `layers.toml`, and 403 dirty working-tree files. |
| **Wave 1: `Phase 12` (DONE), `Phase 07` (`07-01`..`12`) & `07.1`, `Phase 25`, `Phase 32` (DONE)** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | Security-critical `permission-modes.v2.yaml` floor classifier, Ed25519 signing, VoxDb baseline 97 migration, and `vox-graph-reader` v5 (`ARGON-4`); CLI/telemetry/trust-signal calibration (`FLASH-3.8`). |
| **Wave 2: `Phases 33–34`, `13–17`, `37–41`, `08–11 & 42`, `46` (DONE), `49`, `52–53`, `55–56` (DONE), `58` (DONE), `60–61` (DONE), `62`** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | OpenWiki Subsystem Hubs, Autonomy `EffectiveAutonomy` resolver, Metal/CPU embeddings, Trial racing, Nushell structured shell, Scientia DB synthesis + persistent `web_gather`, autonomous Mermaid self-repair, Tier 2.5 git-rev codegen, and standalone crate unbundling. |
| **Wave 3: `Phase 06.4`, `26–31`, `43–45`, `18–19`, `47–48` (DONE), `50–51`, `54`, `57` (DONE)** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | Strictly sequenced React/Tauri/Starlight surfaces across `CapabilityExplorer`, Code Atlas, KB `Library`, `Oversight: 1..5`, Axis Console & River, Bidirectional Chat↔Terminal, Unified IntelliSense, Scientia Studio, and `voxlang.org` beginner frontpage. |
| **Wave 4: `Phases 35–36`, `06.6`, `59` (DONE), `63`** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | LCEF nightly doc claim verifier (`Phase 35`), self-healing draft-PR bot (`Phase 36`), cold-start journeys (`06.6`), Phase 12 mutation gate closure + full `/gsd-code-review --fix` (`Phase 59`), and `N=100` `.vox` vs. TS empirical benchmark preprint + `CR-L8` flywheel (`Phase 63`). |
| **Wave 5: `Phases 64–69` (`POLYTOOL`, `STREAM`, `RANK`, `USERDB`, `CLOUD`, `SIM`)** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | Cross-portfolio language & runtime completeness across `.vox` `@tool`/`@form` auto-surface, `Stream[T]`/`Channel`/`db.Table.subscribe`/`ProcessRun`, `uses { Spend, Write }`/`<WaitingNotice>`/`@ranker`, `@searchable`/`@embed`/`db.Table.patch`/`@table(scoped_by)`, `BlobStore`/`@webhook`/`@metered`, and deterministic tick/skill engine. |
| **Wave 6: `Phases 70–75` (`NIGHTLY`, `CLAVIS`, `MENS`, `MESH`, `MSCRIPT/SOTA`)** | **`[MODEL: ARGON-4]`** + **`[MODEL: FLASH-3.8]`** | Worktree land wave (`land/w1..w5`) + `2026-10-12-nightly-green` Tasks 1–12 + `evidence-ledger.v1.json` refresh (`Phase 70`), Clavis Next Parts 1 & 2 (`Phases 71–72`), Mens Apple Silicon/Qwen 3.8 27B/quantize hardening (`Phase 73`), Populi Mesh Phase 3 `PopuliHttpOp` over iroh QUIC (`Phase 74`), and MENS 2026 `.vox` Script Writer Overhaul + SOTA 5-Tier `vox chat` Routing (`Phase 75` — DONE). |

---

## 4. Immediate Execution Queue (Ready Right Now)

Because `.planning/active-workstream` is set to `unified` and **Wave 0 (`Phase 05`)**, **`Phase 12`**, **`Phase 32`**, **`Phase 46`**, **`Phase 47`**, **`Phase 48`**, **`Phase 55`**, **`Phase 56`**, **`Phase 57`**, **`Phase 58`**, **`Phase 59`**, **`Phase 60`**, **`Phase 61`**, and **`Phase 75`** are complete, you can immediately run in Headless Task-Driver Mode (`/drive-task <plan-path> <task-num>`) or Interactive Mode:

1. **Wave 1 Remaining Foundations (`Phase 07`, `Phase 07.1`, `Phase 25`):**
   - `.planning/workstreams/unified/phases/07-trial-identity-contracts-observational-baseline/07-01-PLAN.md` (`07-05`..`07-12`)
   - `.planning/workstreams/unified/phases/07.1-code-gen-benchmark-plumbing-provenance/12-09-PLAN.md`
   - `.planning/workstreams/unified/phases/25-unified-graph-data-floor/25-01-PLAN.md`
2. **Worktree Land Wave, Nightly-Green Closure & Evidence-Ledger Refresh (`Phase 70`):**
   - `.planning/workstreams/unified/phases/70-worktree-land-wave-nightly-green-closure/70-01-PLAN.md`
3. **Clavis Next Part 1 & Part 2 (`Phases 71–72`):**
   - `.planning/workstreams/unified/phases/71-clavis-next-manifests-uri-acquisition/71-01-PLAN.md`
   - `.planning/workstreams/unified/phases/72-clavis-next-zero-trust-agent-broker-rotation/72-01-PLAN.md`
4. **Mens Apple Silicon & Populi Mesh Phase 3 (`Phases 73–74`):**
   - `.planning/workstreams/unified/phases/73-mens-apple-silicon-quantize-hardening/73-01-PLAN.md`
   - `.planning/workstreams/unified/phases/74-populi-mesh-phase3-httpop-mailbox/74-01-PLAN.md`
5. **Standalone Crate Unbundling (`Phase 62`), Deep Research & Scientia (`Phases 52–54`), & Empirical Preprint + `CR-L8` Flywheel (`Phase 63`):**
   - `.planning/workstreams/unified/phases/62-standalone-crate-unbundling-crates-io/62-01-PLAN.md`
   - `.planning/workstreams/unified/phases/52-deep-research-power-ssot-observability/52-01-PLAN.md`
   - `.planning/workstreams/unified/phases/63-empirical-codegen-benchmark-preprint/63-01-PLAN.md`
6. **Wave 2–5 Queued Tracks (`Phases 06.4, 08–11, 13–19, 26–31, 33–45, 49–51, 64–69`):** `.planning/workstreams/unified/ROADMAP.md`

