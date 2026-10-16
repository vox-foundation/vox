# Vox

## What This Is

Vox is a mature, actively-developed cross-platform Rust workspace (138+ crates) that implements its own programming language — the `.vox` frontend, compiler, and interpreter — plus the surfaces built on top of it: `vox-cli` (the `vox` binary), `vox-gui` (a Tauri 2 desktop application), a multi-agent orchestrator with MCP tool exposure, a local-first mesh (iroh QUIC) for distributed agent/model work, and a native-Rust ML training stack (Candle/QLoRA) for its own fine-tuned model ("Mens"). It is developed and stewarded as a FOSS project under an in-formation Vox Foundation, not a monetized product.

- **v1.0 (Architectural Housekeeping, shipped 2026-10-01):** Disposed of 20 audited dead/misplaced crates and closed the open architecture decisions a full-corpus ADR/SPEC ingest surfaced.
- **v1.3 (Self-Maintaining Public Docs, shipped 2026-10-13):** Shipped the deterministic drift gate, automated crate API reference generator, collapsible reader layout, and trust signal foundation.
- **v1.5 (Unified Holistic Stream, shipped 2026-10-16):** Converged all six parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and 42 feature worktrees across Waves 0–6 (74 phases, 241 plans) into `/Users/brbrainerd/dev/vox`.
- **v1.6 (Production Stability, Zero-Mock Surface Reality & Mens Continuous Training Flywheel, shipped 2026-10-16):** Completed Phases 77–88 (12 phases, 25 plans) delivering 5-spoke training hygiene + round-trip verification (`CORPUS`, `SPOKE`), supervised `vox-orchestrator-d` auto-bootstrap + gateway/IPC parity (`CORE`), real-backend E2E verification (`REAL`), `.vox` end-to-end execution parity (`VOX`), operational & data-intensive GUI surface hardening (`OPS`, `DATA`), `vox doctor --fix` + workspace CI gate parity (`GATE`), and the Mens continuous QLoRA/DPO flywheel (`TRAIN`, `EVAL`, `AUDIT`, `LOOP`).

## Core Value

The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`).

## Current Milestone: v1.6 Complete (`Phases 77–88` Shipped)

**Status:** All `v1.5` (`Phases 01–76`) and `v1.6` (`Phases 77–88`) milestones are 100% complete and verified. Ready for `/gsd-new-milestone` (`v1.7`) when the next roadmap cycle begins.

**Active Stream Roadmap:** [`.planning/workstreams/unified/ROADMAP.md`](workstreams/unified/ROADMAP.md) (mirrored at [`.planning/ROADMAP.md`](ROADMAP.md))
**Active Stream Requirements:** [`.planning/workstreams/unified/REQUIREMENTS.md`](workstreams/unified/REQUIREMENTS.md) (mirrored at [`.planning/REQUIREMENTS.md`](REQUIREMENTS.md))

## Requirements

### Validated

- ✓ `.vox` compiler pipeline (lex → parse → HIR → typeck → codegen) and `--interp` execution path — `crates/vox-compiler/`
- ✓ `vox` CLI with full subcommand surface (`crates/vox-cli/`)
- ✓ Multi-agent orchestrator with D1-D10 policy modules and MCP tool/HTTP exposure — `crates/vox-orchestrator/`, `crates/vox-orchestrator-mcp/`
- ✓ Tauri 2 desktop GUI (`vox-gui`) as the sole orchestration surface (ADR-045)
- ✓ iroh QUIC mesh transport for Populi (ADR-047, locked)
- ✓ Durable functions runtime (`workflow`/`activity`/`actor`/`@scheduled`) with journal-backed replay (ADR-019/021/041, locked)
- ✓ Native Rust ML training (Candle + qlora-rs) (ADR-003/006/034, locked)
- ✓ Codex/Arca/Turso as the sole database stack (ADR-004, locked)
- ✓ Secrets resolution exclusively through `vox_secrets::resolve_secret(...)` (Clavis/`vox-secrets`)
- ✓ **Wave 0 (`Phase 05` — v1.5):** Unified branch & worktree convergence onto `feat/unified-2026-q4` (`CONV-01..04`)
- ✓ **Wave 1 (`Phases 12, 07, 07.1, 25, 32` — v1.5):** Autonomy safety floor (`FLOOR-01..05`, 0/34 surviving mutants), Trial identity & Tier A persistence at DB baseline 97 (`TRIAL-01`, `STORE-01..02`, `BENCH-09`), Unified Code & Architecture Graph Data Floor (`GRAPH-01..02`), and Docs trust signal calibration (`SIGNAL-01..04`, `810/810 verified`)
- ✓ **Wave 2 (`Phases 33–34, 13–17, 37–42, 46, 49, 52–53, 55–56, 58, 60–62` — v1.5):** OpenWiki Subsystem Hubs & deep crate symbols (`WIKI-01..04`), Docs corpus consolidation & debt burn-down (`DEBT-01..03`), Autonomy oversight daemon & consumers (`OVS-01..03`, `ATTN-01` at DB baseline 98), KB local embeddings/context/inbox/rewrites/staleness (`EMB`, `INJ`, `GROW`, `REWR`, `STALE`), Research Trial Race Harness & promotion governance (`RACE`, `KNOW`, `PROMO`, `HARD`, `BENCH-13`), Terminal Session Host CLI Attach & Agent `term_run` (`TERM-01..04`), LLM-First Structured Shell (`Nushell` + `Vox-Shell` `SHELL-01..04`), Deep Research Realistic Power (`RES-01..06`), SCIENTIA Automatic Publication Pipeline Closure (`PUB-01..04`), Autonomous Mermaid Observation, Validation & LLM Self-Repair (`MERMAID-01..04`), Continuous `README.md` & `voxlang.org` Frontpage Codebase Sync (`FRONTPAGE-01..03`), Comprehensive Unverified & Drifting Docs Resolution (`DOCVER-01..03`), Sustainability, Open Collective Activation & Grant Pack (`FUND-01..04`), External Binary Adoption & Git-Rev Codegen Fallback (`DIST-01..02`), and Standalone Crate Unbundling & `crates.io` Publication (`CRATE-01..03`)
- ✓ **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19, 47–48, 50–51, 54, 57` — v1.5):** Capability Explorer & Coverage Gate (`DISC-08..11`, `ExplorerView.tsx`, `gui-coverage-gate` with 294/294 CLI families and 349/349 MCP tools covered or audited), Code & Architecture Atlas GUI (`ATLAS-01..04`), KB `Library`/Context Inspector/Inbox/Health GUI (`GUI-KB-01..08`), `Oversight: 1..5` composer control + Claims Ledger (`GUI-OVS-01..03`, `CLAIMS-01`), Axis Console Warp-Parity Terminal & WebGL Renderer (`TERM-05..08`), Axis Harness River & Live Developer Dogfooding (`RIVER-01..04`), Bidirectional Chat ↔ Terminal Co-Execution & Security Levels (`CTERM-01..04`), Unified 7-Source IntelliSense (`COMP-01..04`), Unified Research & Scientia Publication Studio GUI (`SCIGUI-01..05`), and `voxlang.org` Visual Theme, Interactive Chrome, Sidebar Hiders & Beginner Frontpage (`CHROME-01..04`, `ONBOARD-01..02`)
- ✓ **Wave 4 (`Phases 35–36, 06.6, 59, 63` — v1.5):** Constrained nightly LLM doc claim verifier (`VERIFY-01..04`), self-healing draft-PR bot & freshness ratchet (`BOT-01..04`), cold-start time-to-find journeys (`DISC-15..17`), Phase 12 Mutation Gate Completion & Full Milestone `/gsd-code-review --fix` (`REVIEW-01..02`), and Empirical LLM Code-Gen Benchmark Preprint & `CR-L8` Flywheel (`EMP-01..03`)
- ✓ **Wave 5 (`Phases 64–69` — v1.5):** Polyglot `@tool` & `@form` auto-surface (`POLYTOOL-01..04`), Real-Time `Stream[T]`/`Channel[Send,Recv]`/reactive `db.Table.subscribe` & managed `ProcessRun` (`STREAM-01..04`), Effect-enforced spend/write guards (`uses { Spend, Write }`), `<WaitingNotice>` view contracts (`when waiting`) & `@ranker`/`@eval_shadow` ordering gates (`RANK-01..04`), User-facing hybrid search (`@embed`, `@searchable` FTS5+HNSW+Phonetic), partial `db.Table.patch` & `@table(scoped_by: tenant)` (`USERDB-01..04`), Presigned `Upload[T]`/`BlobStore` (S3/R2/Local + Range + OPFS), `@webhook` verification, `@rate_limit` & `@metered` credit ledger (`CLOUD-01..04`), and Decorator Golden Parity Sweep + Deterministic Tick/Skill Engine Reference Template (`SIM-01..04`)
- ✓ **Wave 6 (`Phases 70–76` — v1.5):** Worktree Land Wave (`land/w1..w5`) & Nightly-Green Closure (`NIGHTLY-01..05`), Clavis Next Parts 1 & 2 (`CLAVIS-01..08`), Mens Native ML Training Apple Silicon & Quantize Hardening (`MENS-01..04`), Populi Mesh Phase 3 `PopuliHttpOp` over iroh QUIC & A2A Mailbox (`MESH-01..03`), MENS 2026 `.vox` Script Writer Overhaul & Verifiable SOTA 5-Tier `vox chat` Model Routing (`MSCRIPT-01..06`, `SOTA-01..05`), and Humane, Pro-Social & Anti-Addictive Chat, Cost Honesty & Mastery UX (`HUMANE-01..04`)
- ✓ **v1.6 (`Phases 77–88`):** Multi-spoke `TrainingRecordV2` schema + `VoxDb` snapshots (`CORPUS-01..06`), 5-spoke round-trip verification (`SPOKE-01..06`), supervised `vox-orchestrator-d` auto-bootstrap & gateway/IPC parity (`CORE-01..05`), hermetic real-backend Playwright E2E harness + 32-surface `honestyScan.ts` (`REAL-01..04`), `.vox` `--interp` ↔ `RustAxum`/`RustTauri`/`TypeScript` execution parity (`VOX-01..05`), operational & data-intensive GUI surface hardening + CLI/dedicated floor ratchets (`OPS-01..05`, `DATA-01..05`), `vox doctor --fix` + full workspace CI gate parity (`GATE-01..04`), and Mens continuous QLoRA/DPO training flywheel & 2-cycle autonomous burn-in (`TRAIN-01..05`, `EVAL-01..05`, `AUDIT-01..05`, `LOOP-01..04`)

### Active

- None — `v1.6` complete (`100%`).

## Constraints

- **Zero Worktree Deletion:** Never delete or clean up existing worktrees under `.claude/worktrees/` or `.worktrees/`.
- **Build toolchain:** Pinned Rust `1.98.1` (`rust-toolchain.toml`); no cmake, nasm, Go, perl, or libclang via any dependency.
- **Crate dependency edges:** CI-gated by `vox ci crate-edges` against `contracts/ci/crate-edges.allow.v1.json`.
- **LLM provider boundary:** All LLM calls go through `vox_actor_runtime::llm`.
- **Cryptography & Secrets:** Application crypto exclusively through `vox-crypto`; secrets exclusively through `vox_secrets::resolve_secret(...)`.
- **VoxScript-first glue:** Automation is `.vox` run via `vox run`, never new `.sh`/`.ps1`/`.py`.

## Key Decisions

| Decision | Rationale | Outcome |
|----------|-----------|---------|
| **D15: Zero Worktree Deletion** | Preserve all 42 git worktrees under `.claude/worktrees/` and `.worktrees/` and 6 historical `.planning/workstreams/<name>/` directories during convergence. | ✓ Good — zero work lost across 86 phases |
| **D16: VoxDb Schema Sequence (`95 -> 96 -> 97 -> 98`)** | Strictly order schema migrations across `kb` (`96`), `research-trial-flywheel` (`97`), and `autonomy-ux` (`98`) with `BaselineStampedWithoutTables` guards. | ✓ Good — zero schema digest conflicts |
| **D17: Single `catalog.v1.yaml` Convergence (`05-04`)** | Reconcile all CLI/MCP/GUI operations in one catalog pass before regenerating SSOT artifacts. | ✓ Good — 996 CLI + 349 MCP tools + 32 GUI surfaces unified |
| **D18: Shared `vox-graph-reader` v5 Floor (`Phase 25`)** | Build one budgeted `AtlasQuery` (`400` nodes / `800` edges) serving both OpenWiki Subsystem Hubs (`Phase 33`) and the GUI Code Atlas (`Phases 26–31`). | ✓ Good — eliminated duplicate AST/graph extractors |
| **D19: Sequential Wave 3 GUI Unification** | Order React surface edits (`06.4 -> 26..31 -> 43..45 -> 18..19 -> 47..57`) on shared files (`Loquela.tsx`, `ChatSurface.tsx`, `Sidebar.tsx`, `Console/*.tsx`). | ✓ Good — zero merge conflicts across 10 GUI tracks |
| **D20: Clean-Room `SessionHost` (`vox-terminal-core`)** | Unify `vox term` and Axis GUI Console on `SessionHost` (`DaemonHost` + `LocalHost`) without AGPL Warp code. | ✓ Good — retired duplicate OSC-633 reducers and session managers |
| **D21: Action-Only Safety Floor (`classify_floor`)** | Classify destructive actions over structured fields (`permission-modes.v2.yaml`) with 34 mutation proofs (`0` surviving mutants). | ✓ Good — unbypassable across CLI, GUI, MCP, and HTTP |
| **D22: Value-Ordered Zero-Mock Surface Reality (`v1.6`)** | Prioritize Core Daily Driver stability (`Chat`, `Terminal`, `Daemon` auto-bootstrap) and a real-backend E2E test harness before secondary operational/research surfaces. | ✓ Good — all 12 `v1.6` phases (`77–88`) verified and shipped |

## Workstreams (Unified into `unified`)

- **`unified` (Active Default — `.planning/workstreams/unified/`):** Single converged stream hosting `v1.5` (shipped `Phases 01–76`) and `v1.6` (shipped `Phases 77–88`).
- `kb` (Historical source preserved at `.planning/workstreams/kb/`) — v1.1 Vox Knowledge Base
- `discoverability` (Historical source preserved at `.planning/workstreams/discoverability/`) — Phases 06.1–06.6
- `docs-freshness` (Historical source preserved at `.planning/workstreams/docs-freshness/`) — v1.3 & v1.4 OpenWiki Self-Organizing Docs (Phases 20–24, 32–36)
- `autonomy-ux` (Historical source preserved at `.planning/workstreams/autonomy-ux/`) — v1.2 Unified Autonomy Control (Phases 12–19)
- `code-atlas` (Historical source preserved at `.planning/workstreams/code-atlas/`) — v1.4 Code Atlas (Phases 25–31)
- `research-trial-flywheel` (Historical source preserved at `.planning/workstreams/research-trial-flywheel/`) — v1.1 Research Trial Flywheel (Phases 07–11, 12–13)

## Evolution

This document evolves at phase transitions and milestone boundaries.

**After each phase transition** (via `/gsd-transition`):
1. Requirements invalidated? → Move to Out of Scope with reason
2. Requirements validated? → Move to Validated with phase reference
3. New requirements emerged? → Add to Active
4. Decisions to log? → Add to Key Decisions
5. "What This Is" still accurate? → Update if drifted

**After each milestone** (via `/gsd-complete-milestone`):
1. Full review of all sections
2. Core Value check — still the right priority?
3. Audit Out of Scope — reasons still valid?
4. Update Context with current state

---
*Last updated: 2026-10-16 — Completed Milestone v1.6 (`Phases 77–88`)*
