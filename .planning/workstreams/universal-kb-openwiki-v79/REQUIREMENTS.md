# Requirements: Vox — Milestone v7.9 (`universal-kb-openwiki-v79`)

**Defined:** 2026-11-17
**Workstream:** `.planning/workstreams/universal-kb-openwiki-v79/`
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v7.9 Requirements

Requirements for Milestone `v7.9` (*Universal Multi-Repo Knowledgebase, Graphify Associative Retrieval, Tiered Self-Healing OpenWiki & Zero-Mock Axis Studio + CV Flywheel*). Every requirement maps to exactly one phase in `ROADMAP.md` (`Phases 465–470`).

### Universal Multi-Repo Onboarding & Federation (`REPO`)

- [x] **REPO-01**: Unified `vox repo {init, add, remove, switch, list, sync}` and `vox init --harness` commands (`vox_repo_*` MCP parity synced in `contracts/operations/catalog.v1.yaml`) onboard any brownfield or greenfield repository (Rust, TypeScript/JS, Python, Go, Vox, or polyglot) with valid `schema_version: 1` `.vox/repositories.yaml`, stack-tailored `VOX.md` + `AGENTS.md`, `.voxignore` (plus derived ignore files), `.well-known/llms.txt`, `docs/wiki/`, and `.vox/cache/{kb,graphify,openwiki}/`.
- [x] **REPO-02**: Active workspace repository context (`active_repository_id` plus optional federated sibling repository set) is persisted in `vox-orchestrator-d` session/workspace state and switches live across CLI, MCP, and Axis GUI without restarting the daemon.
- [x] **REPO-03**: Two-Tier Hybrid storage enforces strict `repository_id` isolation for repo-local committed documentation (`docs/wiki/`, `docs/`, `VOX.md`) and gitignored `.vox/cache/` shards while keeping cross-project curated knowledge and promoted learnings in `~/.vox/library/` gated by `KbScopeMask`.
- [x] **REPO-04**: Zero-footprint external mode (`--read-only` / `--external-cache-only`, `access_mode: read_only`) allows `vox repo add <path> --read-only` to index third-party reference repositories under `~/.vox/repos/<repository_id>/` without writing `.vox/` or `docs/` files into the target checkout.

### Repo-Agnostic KB Corpus, Scoped Hybrid Index & Trust Governance (`KB`)

- [x] **KB-01**: Replace hardcoded `KB_CORPUS_ROOTS = &["docs/src"]` (`crates/vox-search/src/kb_ids.rs`, `crates/vox-search/src/library/corpus.rs`) with dynamic per-repo `KbCorpusManifest` discovery (`docs/src/` on Vox, `docs/wiki/`, `docs/`, `adr/`, `VOX.md`, `AGENTS.md`, `README.md`, and package READMEs, overridable in `Vox.toml` / `.vox/kb.yaml`), preserving 100% backward compatibility with Vox's `contracts/kb/eval/` BEIR golden set.
- [x] **KB-02**: Partition `VoxDb` (`search_documents`, `search_document_chunks`, `kb_index_state`, `kb_findings`) and Tantivy index shards by `(repository_id, corpus_tier)` (`repo_local | federated | global_library`) with single-flight daemon reindexing (`vox kb reindex [--repo <id>]`) and per-repo git-clean `ContextTrustTier` verification.
- [x] **KB-03**: Enable bi-directional promotion from `kb_findings` and `MemPalace` session learnings into either repo-local `docs/wiki/` or global `~/.vox/library/`, gated by `kb_text::sanitize`, `promotion_eligibility`, and mechanical citation verification.
- [x] **KB-04**: Provide `vox kb eval --multi-repo` measuring `Recall@5`, `MRR`, multi-hop grounding ratchets, and strict `0` cross-repo leakage when querying with `scope = repo_local`.

### Polyglot Graphify, Cross-Entity Edges & Associative Retrieval (`GRAPH`)

- [x] **GRAPH-01**: Eliminate `graphify-out/` namespace collisions by relocating CI build/cycle/audit maps into `.vox/cache/ci-maps/` via `vox_config::paths`, isolating Graphify shards under `.vox/cache/graphify/<repo_id>/<corpus_id>/`, and dynamically generating per-repo Graphify corpus manifests (`repo-code-graph`, `doc-wiki-graph`, `gui-surface-graph`) on onboarding.
- [x] **GRAPH-02**: Extend `vox-graph-reader` deterministic AST/structural extraction (`rs`, `ts/tsx/js/jsx`, `py`, `vox`, plus Go packages, SQL/DDL table schemas, CLI/HTTP/MCP/OpenAPI endpoints, and Markdown `#Lstart-Lend` / symbol citations) with incremental `blake3` file-hash + git-diff rebuilds and Leiden community clustering.
- [x] **GRAPH-03**: Materialize **Code ↔ Doc ↔ Decision** cross-edges (`documented_in`, `cites_symbol`, `governed_by_adr`, `touched_by_decision`) tagged with `EXTRACTED` / `INFERRED` / `AMBIGUOUS` honesty levels in `vox-graph-reader` and `VoxDb` `knowledge_edges`.
- [x] **GRAPH-04**: Integrate **HippoRAG-style Personalized PageRank (PPR)** over seed entities plus **Leiden Community Summaries** as first-class retrieval legs in `vox-search::execute_search_plan` RRF fusion (`RetrievalEvidenceEnvelope`), improving multi-hop architectural recall while minimizing token overhead.

### Polyglot Multi-Layer OpenWiki & Tiered Self-Healing (`WIKI`)

- [x] **WIKI-01**: Generalize `vox-doc-pipeline::openwiki` into a repo-agnostic polyglot OpenWiki engine (`vox wiki {init, build, status, diff}` + `vox_wiki_*` MCP tools synced in `catalog.v1.yaml`) supporting Cargo/Rust (with or without `layers.toml`), TypeScript/Node workspaces, Python projects, Go modules, and `.vox` projects.
- [x] **WIKI-02**: Synthesize structured OKF-aligned Markdown pages spanning **all 5 system layers** — (1) Architecture & Module/Community Topology, (2) CLI/HTTP/MCP/API Interfaces, (3) Data Models & SQL/VoxDb Schemas, (4) GUI/Frontend Surfaces & Flows, and (5) Workflows, Actors & Runbooks — using validated `vox-diagram` blocks (`0` legacy Mermaid) and non-destructive `<!-- openwiki:auto:start id="..." -->` deterministic regions that preserve surrounding human prose.
- [x] **WIKI-03**: Implement **Tier 1 Blast-Radius Auto-Heal**: on `git diff` or `vox wiki sync`, trace modified AST symbols/files across Graphify `documented_in` reverse edges and automatically regenerate deterministic regions in-place (updated signatures, shifted `#Lstart-Lend` anchors, CLI/MCP/table inventories, and `vox-diagram` edges) with zero LLM calls.
- [x] **WIKI-04**: Implement **Tier 2 Verified Semantic Prose Heal**: detect narrative/claim drift via `verify_doc_mechanically` + NLI claim checking, synthesize minimal grounded patches via `vox_actor_runtime::llm`, gate every candidate through `kb_rewrite_guard` + `vox-doc-pipeline --lint-only` + `vox-diagram` check + doctests, and persist passing unified diffs in `kb_heal_proposals` (`VoxDb`) for 1-click apply (`vox wiki heal --apply <id>`), `--auto-apply`, or CI draft PRs.

### Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki Studio & Live Code Atlas (`GUI`)

- [x] **GUI-01**: Add an always-accessible **Workspace Multi-Repo Switcher & "Init / Add Repository" Onboarding Wizard** in `AppShell`, `BottomStatusBar`, and `RepositoryView` displaying per-repo stack badges, KB/Graphify/OpenWiki freshness, 1-click `Onboard Repo`, and federated scope toggles routed 100% via `voxTransport` -> `vox-orchestrator-d` (`0` `VoxDb` handles in `vox-gui`, `0` direct `invoke`/`listen` leaks).
- [x] **GUI-02**: Unify and promote `DocReader` + `MemoryView` into a **Zero-Mock Knowledgebase & OpenWiki Studio** (`knowledge` domain): delete hardcoded `DEFAULT_INBOX_FINDINGS` and `DEFAULT_LIBRARY_RESULTS` in `DocReader.tsx`, wire live `vox_kb_*`, `vox_wiki_*`, `kb_findings` triage, and a **Side-by-Side Self-Heal Diff Inspector** (`Apply Patch`, `Heal All Tier-1`, `Edit`, `Demote to Legacy`, `Reject`).
- [x] **GUI-03**: Upgrade `VoxGraphStatusPanel` and `CodeAtlasCanvas` to **100% live multi-repo Graphify data**: remove hardcoded `DEFAULT_ATLAS_NODES` / `DEFAULT_ATLAS_EDGES` and swallowed `.catch(() => {})` calls in `CodeAtlasCanvas.tsx`; bind `vox_search_atlas`, `vox_graphify_query`, and `vox_graphify_compare` across `L0 Communities/Packages`, `L1 Files`, `L2 Symbols`, and `L3 Doc/Wiki Cross-Edges` with blast-radius overlays and click-to-`@mention` into `Loquela`.
- [x] **GUI-04**: Remediate all visual, contrast, and component-contract defects across `MemoryView.tsx` (purge cream-on-marble `text-[#F3E5AB]` / `text-[#D6CEBF]`), `DocReader.tsx` (purge dark-only `bg-black/20`, `text-amber-300`, `text-sky-200`, `text-emerald-300`), `VoxGraphStatusPanel.tsx`, and `RepositoryView.tsx` using Style Dictionary semantic tokens across all 4 themes (`arcane`, `glacier`, `slate`, `high-contrast`), `data-slot` Radix primitives, `<RomanSurfaceHeader>` empirical brass gauges, `rawEmojiCount === 0`, and `<400`-LoC child files.

### End-to-End Multi-Repo & Self-Heal Verification + Before/After CV Flywheel (`CV`)

- [x] **CV-01**: Hermetic and live integration proof onboarding an external polyglot repository (`Rust + TS + Python + Vox`), building its KB + Graphify + 5-layer OpenWiki, mutating source symbols/lines, executing Tier-1 in-place auto-heal + Tier-2 verified prose heal, and asserting cross-repo isolation + multi-hop retrieval uplift in `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json`.
- [x] **CV-02**: Capture `CV_STAGE=before` baseline screenshots and DOM/contrast metrics across all affected Knowledge & Workspace surfaces (`memory`, `vox-search` / `CodeAtlas`, OpenWiki/DocReader Studio, `repository`, `AppShell` Repo Switcher), and generate dual-theme Concept-Art reference plates (`v79-concept-openwiki-studio-{carrara,slate}.jpg` and `v79-concept-multirepo-atlas-{carrara,slate}.jpg`) wired into `vlmVisionFlywheel.ts`.
- [x] **CV-03**: Converge multi-pass Playwright + `@ngi/platform-ssot` + VLM Computer-Vision Flywheel (`CV_STAGE=after`) across **5 viewports** (`900×600`, `1100×720`, `1280×800`, `1440×900`, `1920×1080`) × **4 themes** (`arcane`, `glacier`, `slate`, `high-contrast`) including open Radix Repo-Switcher and Diff-Inspector portals, achieving `visualAfterScore >= 0.92`, positive `before -> after` contrast/sharpness delta, `axeViolations === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `digitalColorLeakCount === 0`, and `rawEmojiCount === 0`, with a paired Before/After visual scorecard.
- [x] **CV-04**: Pass `vox-arch-check`, `vox audit code`, `vox ci operations-verify`, `vox-doc-pipeline --lint-only` (promoting `docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md` to `status: "current"` and syncing `where-things-live.md` + `docs-system-ssot.md`), `noPaperFallbackGuard.test.ts`, `themeContract.test.ts`, `ipcBoundaries.test.ts`, and all affected Rust/Vitest regression suites with `0` errors.

## Future Requirements (`v8.0+`)

- **WIKI-CRDT-01**: Multi-author real-time collaborative wiki editing and conflict-free team annotations synchronized across federated developer nodes.
- **GRAPH-REMOTE-01**: Distributed Populi mesh Graphify shard replication and pre-built `.graph.zst` artifact streaming for multi-million-LoC monorepos.

## Out of Scope

| Feature | Reason |
|---------|--------|
| Direct vendor HTTP SDK calls bypassing `vox_actor_runtime::llm` | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (`llm_provider_call` hard error) |
| Direct `VoxDb` handles or child CLI-DB spawns inside `vox-gui` | Violates Axis IPC boundary; all GUI state/mutations traverse `voxTransport` -> `vox-orchestrator-d` |
| Unverified LLM overwrites directly into human prose outside OKF auto-fences | Risks doc hallucination and silent loss of human design rationale; Tier-2 prose changes must pass 4 deterministic gates and route via proposals |
| Adding new third-party Rust crates or npm packages for graph/wiki rendering | Keep zero new `Cargo.lock`/`package.json` deps; leverage existing `vox-graph-reader`, `vox-diagram`, `vox-search`, `radix-ui`, and `lucide-react` |

## Traceability

| Requirement | Phase | Status |
|-------------|-------|--------|
| REPO-01 | Phase 465 | Complete (Phase 465) |
| REPO-02 | Phase 465 | Complete (Phase 465) |
| REPO-03 | Phase 465 | Complete (Phase 465) |
| REPO-04 | Phase 465 | Complete (Phase 465) |
| KB-01 | Phase 465 | Complete (Phase 465) |
| KB-02 | Phase 465 | Complete (Phase 465) |
| KB-03 | Phase 466 | Complete (Phase 466) |
| KB-04 | Phase 466 | Complete (Phase 466) |
| GRAPH-01 | Phase 466 | Complete (Phase 466) |
| GRAPH-02 | Phase 466 | Complete (Phase 466) |
| GRAPH-03 | Phase 466 | Complete (Phase 466) |
| GRAPH-04 | Phase 466 | Complete (Phase 466) |
| WIKI-01 | Phase 467 | Complete (Phase 467) |
| WIKI-02 | Phase 467 | Complete (Phase 467) |
| WIKI-03 | Phase 468 | Complete (Phase 468) |
| WIKI-04 | Phase 468 | Complete (Phase 468) |
| GUI-01 | Phase 469 | Complete (Phase 469) |
| GUI-02 | Phase 469 | Complete (Phase 469) |
| GUI-03 | Phase 469 | Complete (Phase 469) |
| GUI-04 | Phase 469 | Complete (Phase 469) |
| CV-01 | Phase 470 | Complete (Phase 470) |
| CV-02 | Phase 470 | Complete (Phase 470) |
| CV-03 | Phase 470 | Complete (Phase 470) |
| CV-04 | Phase 470 | Complete (Phase 470) |

**Coverage:**
- v7.9 requirements: 24 total
- Mapped to phases: 24 (`Phases 465–470`)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-17*
*Last updated: 2026-11-17 after initial v7.9 definition*
