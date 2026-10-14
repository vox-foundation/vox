# Vox

## What This Is

Vox is a mature, actively-developed cross-platform Rust workspace (138+ crates) that implements its own programming language — the `.vox` frontend, compiler, and interpreter — plus the surfaces built on top of it: `vox-cli` (the `vox` binary), `vox-gui` (a Tauri 2 desktop application), a multi-agent orchestrator with MCP tool exposure, a local-first mesh (iroh QUIC) for distributed agent/model work, and a native-Rust ML training stack (Candle/QLoRA) for its own fine-tuned model ("Mens"). It is developed and stewarded as a FOSS project under an in-formation Vox Foundation, not a monetized product.

v1.0 (Architectural Housekeeping, shipped 2026-10-01) disposed of 20 audited dead/misplaced crates and closed the open architecture decisions a full-corpus ADR/SPEC ingest surfaced. v1.3 (Self-Maintaining Public Docs, completed 2026-10-13) shipped the deterministic drift gate, automated crate API reference generator, collapsible reader layout, and trust signal foundation.

## Core Value

The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`).

## Current Milestone: v1.5 Unified Holistic Stream (`unified`)

**Goal:** Converge all six parallel GSD workstreams (`kb`, `discoverability`, `docs-freshness` v1.3/v1.4, `autonomy-ux`, `code-atlas`, `research-trial-flywheel`) and active feature worktrees (`capability-profiles`, `true-workflow-durability`, `warp-fork-axis-gui`, `fmt-safety`) into a **single branch, worktree (`/Users/brbrainerd/dev/vox`), and dependency-ordered execution plan** engineered for Antigravity IDE (`[MODEL: FLASH-3.8]` + `[MODEL: ARGON-4]`), without deleting any existing worktrees on disk.

**Canonical Master Plan:** [`docs/src/architecture/unified-workstream-execution-plan-2026.md`](../docs/src/architecture/unified-workstream-execution-plan-2026.md)
**Active Stream Roadmap:** [`.planning/workstreams/unified/ROADMAP.md`](workstreams/unified/ROADMAP.md) (mirrored at [`.planning/ROADMAP.md`](ROADMAP.md))

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
- ✓ `kb` Phases 01–04: Retrieval eval harness, Library index & `vox_kb_*` MCP tools, Provenance/trust tiers at VoxDb baseline 96, and `vox-kb-facets` L0 taxonomy crate — completed on `claude/vox-knowledge-base-research-2964d2`
- ✓ `discoverability` Phases 06.1–06.3: Hosted CI fast local loop, generated capability index (`contracts/capabilities/capability-index.v1.json`, `vox explain`, `vox features`), and Universal `⌘K` Palette — completed on `claude/vox-discoverability-a72c8c`
- ✓ `docs-freshness` v1.3 Phases 20–24: Deploy unblock, collapsible reader layout, corpus measure/prune audit, deterministic drift gate, automated crate API docs (`docs/src/api/*.generated.md`), and reader trust signals — completed in `/Users/brbrainerd/dev/vox`

### Active (Sequenced Across Waves 0–4 in `REQUIREMENTS.md`)

- [ ] **Wave 0 (`Phase 05`):** Unified branch & worktree convergence onto `feat/unified-2026-q4` (`CONV-01..04`)
- [ ] **Wave 1 (`Phases 12, 07, 07.1, 25, 32`):** Autonomy safety floor (`FLOOR-01..02`), Trial identity & Tier A persistence at DB baseline 97 (`TRIAL-01`, `STORE-01..02`, `BENCH-09`), Unified Code & Architecture Graph Data Floor (`GRAPH-01..02`), and Docs trust signal calibration (`SIGNAL-01..04`)
- [ ] **Wave 2 (`Phases 33, 34, 13–17, 37–41, 08–11 & 42`):** OpenWiki Subsystem Hubs & deep crate symbols (`WIKI-01..04`), Docs corpus consolidation & debt burn-down (`DEBT-01..03`), Autonomy oversight daemon & consumers (`OVS-01..03`, `ATTN-01`), KB local embeddings/context/inbox/rewrites/staleness (`EMB`, `INJ`, `GROW`, `REWR`, `STALE`), and Research Trial Race Harness & promotion governance (`RACE`, `KNOW`, `PROMO`, `HARD`, `BENCH-13`)
- [ ] **Wave 3 (`Phases 06.4, 26–31, 43–45, 18–19`):** Sequenced GUI unification across `CapabilityExplorer.tsx`, Code & Architecture Atlas canvas/dock/click-to-`@`, KB `Library`/Context Inspector, and `Oversight: 1..5` composer control + Claims Ledger
- [ ] **Wave 4 (`Phases 35, 36, 06.6`):** Constrained nightly LLM doc claim verifier (`VERIFY-01..04`), self-healing draft-PR bot & freshness ratchet (`BOT-01..04`), and cold-start time-to-find journeys (`DISC-15..17`)

## Constraints

- **Zero Worktree Deletion:** Never delete or clean up existing worktrees under `.claude/worktrees/` or `.worktrees/`.
- **Build toolchain:** Pinned Rust `1.98.1` (`rust-toolchain.toml`); no cmake, nasm, Go, perl, or libclang via any dependency.
- **Crate dependency edges:** CI-gated by `vox ci crate-edges` against `contracts/ci/crate-edges.allow.v1.json`.
- **LLM provider boundary:** All LLM calls go through `vox_actor_runtime::llm`.
- **Cryptography & Secrets:** Application crypto exclusively through `vox-crypto`; secrets exclusively through `vox_secrets::resolve_secret(...)`.
- **VoxScript-first glue:** Automation is `.vox` run via `vox run`, never new `.sh`/`.ps1`/`.py`.

## Workstreams (Unified into `unified`)

- **`unified` (Active Default — `.planning/workstreams/unified/`):** Single converged stream integrating all six historical workstreams below in strict dependency order.
- `kb` (Historical source preserved at `.planning/workstreams/kb/`) — v1.1 Vox Knowledge Base
- `discoverability` (Historical source preserved at `.planning/workstreams/discoverability/`) — Phases 06.1–06.6
- `docs-freshness` (Historical source preserved at `.planning/workstreams/docs-freshness/`) — v1.3 (complete) & v1.4 OpenWiki Self-Organizing Docs (Phases 32–36)
- `autonomy-ux` (Historical source preserved at `.planning/workstreams/autonomy-ux/`) — v1.2 Unified Autonomy Control (Phases 12–19)
- `code-atlas` (Historical source preserved at `.planning/workstreams/code-atlas/`) — v1.4 Code Atlas (Phases 25–31)
- `research-trial-flywheel` (Historical source preserved at `.planning/workstreams/research-trial-flywheel/`) — v1.1 Research Trial Flywheel (Phases 07–11, 12–13)

---
*Last updated: 2026-10-13 — All parallel workstreams unified into `.planning/workstreams/unified/`*
