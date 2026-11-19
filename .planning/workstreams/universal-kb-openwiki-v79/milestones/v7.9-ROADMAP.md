# Roadmap: Milestone v7.9 (`universal-kb-openwiki-v79`) — Universal Multi-Repo Knowledgebase, Graphify Associative Retrieval, Tiered Self-Healing OpenWiki & Zero-Mock Axis Studio + CV Flywheel

**Workstream:** `.planning/workstreams/universal-kb-openwiki-v79/`
**Research SSOT:** [`docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md`](../../../docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md)
**Scope:** Transform Vox's knowledgebase, Graphify structural/semantic index, OpenWiki system-documentation engine, and Axis GUI into a universal polyglot harness usable across Vox itself, any newly initialized repo (`vox init --harness`), and any onboarded brownfield repository (`vox repo add`): Two-Tier Hybrid repo-local (`docs/wiki/`, `.vox/cache/`, `--read-only` `~/.vox/repos/<id>/`) plus global `~/.vox/library/` storage (`REPO-01..04`, `KB-01..02`), polyglot Graphify with Code ↔ Doc ↔ Decision cross-edges and HippoRAG Personalized PageRank + Leiden community retrieval (`KB-03..04`, `GRAPH-01..04`), repo-agnostic 5-layer OKF OpenWiki synthesis (`WIKI-01..02`), Tier-1 deterministic blast-radius auto-heal + Tier-2 verified LLM prose healing (`WIKI-03..04`), zero-mock Axis Multi-Repo Switcher + OpenWiki Heal Studio + live Code Atlas with 4-theme contrast remediation (`GUI-01..04`), and end-to-end polyglot onboarding + multi-pass `BEFORE -> AFTER` CV Flywheel convergence across `Phases 465–470` (`24/24` requirements). Zero new third-party crates in `Cargo.lock`, zero new npm packages in `package.json`, zero `layers.toml` / `crate-edges` inversions, zero `VoxDb` handles in `vox-gui`, and 100% model-agnostic LLM routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [x] **Phase 465: Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage** (`REPO-01..04`, `KB-01..02`) — *Complete (`465-VERIFICATION.md`)*
- [x] **Phase 466: Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval** (`KB-03..04`, `GRAPH-01..04`) — *Complete (`466-VERIFICATION.md`)*
- [x] **Phase 467: Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions** (`WIKI-01..02`) — *Complete (`467-VERIFICATION.md`)*
- [x] **Phase 468: Tiered Self-Updating & Self-Fixing Documentation Engine (AST Blast-Radius + Verified Prose Heal Queue)** (`WIKI-03..04`) — *Complete (`468-VERIFICATION.md`)*
- [x] **Phase 469: Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation** (`GUI-01..04`) — *Complete (`469-VERIFICATION.md`)*
- [x] **Phase 470: End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence** (`CV-01..04`) — *Complete (`470-VERIFICATION.md`)*

---

### Phase 465: Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/465-universal-multirepo-onboarding-and-hybrid-kb-storage/`
**Goal**: Make `vox init --harness` and `vox repo {init, add, remove, switch, list, sync}` onboard any brownfield or greenfield repository (including zero-footprint `--read-only` mode), persist live `active_repository_id` in `vox-orchestrator-d`, replace hardcoded `docs/src` corpus roots with dynamic per-repo `KbCorpusManifest` discovery, and partition `VoxDb` + Tantivy shards by `(repository_id, corpus_tier)`.
**Depends on**: None (continues sequentially after `v7.8` `Phase 464`)
**Requirements**: REPO-01, REPO-02, REPO-03, REPO-04, KB-01, KB-02
**Success Criteria** (what must be TRUE):

  1. Running `vox repo init` or `vox init --harness` on an arbitrary Rust, TypeScript/JS, Python, Go, Vox, or polyglot project generates valid `schema_version: 1` `.vox/repositories.yaml`, stack-tailored `VOX.md` + `AGENTS.md`, `.voxignore` (and synced `.cursorignore`/`.aiignore`/`.aiexclude`), `.well-known/llms.txt`, `docs/wiki/`, and gitignored `.vox/cache/{kb,graphify,openwiki}/` with `vox_repo_*` MCP parity in `catalog.v1.yaml`.
  2. `vox repo add <path> [--read-only]` registers either Two-Tier Hybrid workspace repositories or zero-footprint external reference repos under `~/.vox/repos/<repository_id>/`, while `vox repo switch <id>` updates `active_repository_id` and federated sibling scope live in `vox-orchestrator-d` without daemon restarts.
  3. `crates/vox-search` resolves dynamic `KbCorpusManifest` roots per repository (`docs/src/` on Vox, `docs/wiki/`, `docs/`, `adr/`, `VOX.md`, `AGENTS.md`, `README.md`, package READMEs, or `Vox.toml`/`.vox/kb.yaml` overrides) without failing when `docs/src` is absent, while keeping Vox's `contracts/kb/eval/` BEIR golden set bit-identical.
  4. `VoxDb` (`search_documents`, `search_document_chunks`, `kb_index_state`, `kb_findings`) and Tantivy shards partition cleanly by `(repository_id, corpus_tier)` (`repo_local | federated | global_library`) with single-flight daemon reindexing, per-repo git-clean `ContextTrustTier` checks, and shared user learnings stored under `~/.vox/library/`.

**Plans**: TBD

### Phase 466: Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/466-polyglot-graphify-cross-edges-and-ppr-retrieval/`
**Goal**: Eliminate `graphify-out/` collisions, generate dynamic Graphify corpora per onboarded repository, extend deterministic AST/schema/route/Markdown extraction to materialize Code ↔ Doc ↔ Decision cross-edges, fuse HippoRAG Personalized PageRank (PPR) and Leiden community summaries into `vox-search`, enable dual-target `kb_findings`/`MemPalace` promotion, and add multi-repo BEIR evaluation.
**Depends on**: Phase 465
**Requirements**: KB-03, KB-04, GRAPH-01, GRAPH-02, GRAPH-03, GRAPH-04
**Success Criteria** (what must be TRUE):

  1. CI map outputs (`dep-cycles`, `build-bench`, `crate_audit`) relocate out of `graphify-out/` into `.vox/cache/ci-maps/` via `vox_config::paths`, and every onboarded repository auto-provisions dynamic Graphify corpus manifests (`repo-code-graph`, `doc-wiki-graph`, `gui-surface-graph`) under `.vox/cache/graphify/<repo_id>/<corpus_id>/` (or `~/.vox/repos/<repo_id>/` in read-only mode).
  2. `vox-graph-reader` deterministically extracts `.rs`, `.ts/.tsx/.js/.jsx`, `.py`, `.vox`, Go packages, SQL/DDL schemas, CLI/HTTP/MCP/OpenAPI endpoints, and Markdown `#Lstart-Lend`/symbol citations using `blake3` incremental rebuilds and Leiden community clustering (`0` upfront LLM calls).
  3. **Code ↔ Doc ↔ Decision** cross-edges (`documented_in`, `cites_symbol`, `governed_by_adr`, `touched_by_decision`) with `EXTRACTED` / `INFERRED` / `AMBIGUOUS` confidence labels populate `vox-graph-reader` and `VoxDb` `knowledge_edges`, and `vox-search::execute_search_plan` fuses HippoRAG Personalized PageRank (PPR) + Leiden community summary legs into `RetrievalEvidenceEnvelope`.
  4. `kb_findings` and `MemPalace` learnings promote cleanly to either repo-local `docs/wiki/` or global `~/.vox/library/` via `kb_text::sanitize` + `promotion_eligibility`, and `vox kb eval --multi-repo` verifies `Recall@5`, `MRR`, multi-hop grounding, and `0` cross-repo leakage when `scope = repo_local`.

**Plans**: TBD

### Phase 467: Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/467-polyglot-five-layer-openwiki-and-okf-fences/`
**Goal**: Upgrade `vox-doc-pipeline::openwiki` from a Rust-`layers.toml`-only tool into a universal polyglot OpenWiki generator spanning all five system layers with OKF-aligned frontmatter, validated `vox-diagram` topologies, and human-preserving auto-fences exposed over CLI and MCP.
**Depends on**: Phases 465 and 466
**Requirements**: WIKI-01, WIKI-02
**Success Criteria** (what must be TRUE):

  1. `vox wiki {init, build, status, diff}` and `vox_wiki_*` MCP tools (registered in `catalog.v1.yaml`) generate OpenWiki hubs cleanly on Rust/Cargo (with or without `layers.toml`), TypeScript/Node workspaces, Python projects, Go modules, `.vox` apps, and mixed polyglot repositories.
  2. Generated wikis cover all 5 system layers — (1) Architecture & Module/Community Topology, (2) CLI/HTTP/MCP/API Interfaces, (3) Data Models & SQL/VoxDb Schemas, (4) GUI/Frontend Surfaces & Flows, and (5) Workflows, Actors & Runbooks — emitting valid `vox-diagram` figures (`0` legacy Mermaid blocks).
  3. Machine-generated sections sit strictly inside `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->` fences so repeated `vox wiki build` runs update structural facts deterministically while leaving surrounding human-authored prose and custom notes 100% intact.
  4. All generated and authored OpenWiki pages carry valid OKF/KB frontmatter (`topic`, `domain`, `authorship`, `kb:summary`, `#Lstart-Lend` source anchors) and pass `vox-doc-pipeline --lint-only` on both Vox and external repositories.

**Plans**: TBD

### Phase 468: Tiered Self-Updating & Self-Fixing Documentation Engine (AST Blast-Radius + Verified Prose Heal Queue)

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/468-tiered-self-healing-documentation-engine/`
**Goal**: Turn documentation drift detection into a two-tier self-updating and self-fixing closed loop: Tier-1 zero-LLM in-place deterministic healing across AST blast radiuses, plus Tier-2 grounded LLM prose healing verified by four deterministic gates and staged in `kb_heal_proposals`.
**Depends on**: Phases 466 and 467
**Requirements**: WIKI-03, WIKI-04
**Success Criteria** (what must be TRUE):

  1. On `git diff` or `vox wiki sync`, the blast-radius resolver queries Graphify reverse `documented_in` / `cites_symbol` / `governed_by_adr` edges to pinpoint impacted docs without scanning unrelated files.
  2. **Tier 1 Self-Heal** deterministically updates changed AST symbol signatures, shifted `#Lstart-Lend` source line ranges, CLI/MCP/HTTP/SQL/UI inventory tables, and `vox-diagram` topologies inside `<!-- openwiki:auto:start -->` fences in-place with `0` LLM calls and `0` changes to human narrative outside the fences.
  3. **Tier 2 Semantic Prose Heal** detects narrative claim drift via `verify_doc_mechanically` + NLI claim auditing, generates minimal grounded unified diffs through `vox_actor_runtime::llm`, and gates every draft through `kb_rewrite_guard` + `vox-doc-pipeline --lint-only` + `vox-diagram` check + doctests before persisting passing diffs in `kb_heal_proposals` (`VoxDb`).
  4. Operators can review, apply, or automate staged repairs via `vox wiki heal {list, diff, apply, reject, --auto-apply}`, `vox_wiki_heal_*` MCP tools, or non-blocking CI draft PRs, closing the narrative `prose-behavior-drift` gap cleanly.

**Plans**: TBD

### Phase 469: Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/469-axis-multirepo-switcher-and-openwiki-studio/`
**Goal**: Eliminate every hardcoded mock and contrast glitch across Axis Knowledge and Workspace surfaces, add an `AppShell`/`BottomStatusBar` Multi-Repo Switcher and Onboarding Wizard, promote `DocReader` + `MemoryView` into a unified OpenWiki & Self-Heal Diff Studio, and bind `CodeAtlasCanvas` + `VoxGraphStatusPanel` to live multi-repo Graphify APIs.
**Depends on**: Phases 465, 466, 467, and 468
**Requirements**: GUI-01, GUI-02, GUI-03, GUI-04
**Success Criteria** (what must be TRUE):

  1. `AppShell` / `BottomStatusBar` / `RepositoryView` provide an always-accessible **Multi-Repo Switcher & Init/Add Onboarding Wizard** displaying active repository stack badges, KB/Graphify/OpenWiki freshness, `--read-only` / Two-Tier Hybrid scope toggles, and 1-click onboarding routed exclusively through `voxTransport` -> `vox-orchestrator-d` (`0` `VoxDb` handles in `vox-gui`, `0` direct `invoke`/`listen` leaks).
  2. `DocReader` + `MemoryView` operate as a unified **Zero-Mock Knowledgebase & OpenWiki Studio** (`DEFAULT_INBOX_FINDINGS` and `DEFAULT_LIBRARY_RESULTS` deleted), wired to live `vox_kb_*`, `vox_wiki_*`, `kb_findings` promotion (`repo_local` vs `~/.vox/library/`), and a **Side-by-Side Self-Heal Diff Inspector** (`Apply Patch`, `Heal All Tier-1`, `Edit`, `Demote to Legacy`, `Reject`).
  3. `CodeAtlasCanvas` and `VoxGraphStatusPanel` delete `DEFAULT_ATLAS_NODES` / `DEFAULT_ATLAS_EDGES` and swallowed `.catch(() => {})` calls, rendering live multi-repo `vox_search_atlas` / `vox_graphify_query` / `vox_graphify_compare` nodes/edges across `L0..L3` (`Doc/Wiki` cross-edges + blast-radius badges) and click-to-`@mention` into `Loquela`.
  4. All cream-on-marble (`text-[#F3E5AB]`/`text-[#D6CEBF]`) and dark-only (`bg-black/20`, `text-amber-300`, `text-sky-200`, `text-emerald-300`) defects across `MemoryView.tsx`, `DocReader.tsx`, `VoxGraphStatusPanel.tsx`, and `RepositoryView.tsx` are converted to Style Dictionary semantic tokens (`arcane`, `glacier`, `slate`, `high-contrast`), `data-slot` Radix primitives, empirical `<RomanSurfaceHeader>` brass gauges, `rawEmojiCount === 0`, and `<400`-LoC child files.

**Plans**: TBD

### Phase 470: End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence

**Directory**: `.planning/workstreams/universal-kb-openwiki-v79/phases/470-polyglot-e2e-proof-and-cv-flywheel/`
**Goal**: Prove the entire universal KB + Graphify + OpenWiki + Self-Heal loop end-to-end on a real external polyglot repository, capture paired `BEFORE -> AFTER` CV Flywheel convergence across all modified Axis GUI surfaces against dual-theme Concept-Art plates, and verify all workspace CI/doc/arch gates green.
**Depends on**: Phases 465, 466, 467, 468, and 469
**Requirements**: CV-01, CV-02, CV-03, CV-04
**Success Criteria** (what must be TRUE):

  1. End-to-end integration test suite onboards a clean external polyglot fixture (`Rust + TypeScript + Python + Vox`), builds KB + Graphify + 5-layer OpenWiki, mutates AST signatures/lines/prose, runs Tier-1 auto-heal + Tier-2 verified prose heal, verifies zero cross-repo leakage + PPR recall gain, and writes `contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json`.
  2. Baseline `CV_STAGE=before` captures and dual-theme Concept-Art reference plates (`v79-concept-openwiki-studio-{carrara,slate}.jpg` and `v79-concept-multirepo-atlas-{carrara,slate}.jpg`) are registered in `vlmVisionFlywheel.ts`, passing `noPaperFallbackGuard.test.ts`, `themeContract.test.ts`, `zeroEmojiAndDingbat.test.ts`, and `ipcBoundaries.test.ts` with `0` violations.
  3. Multi-pass Playwright + `@ngi/platform-ssot` + VLM CV Flywheel (`CV_STAGE=after`) across **5 viewports** (`900×600`, `1100×720`, `1280×800`, `1440×900`, `1920×1080`) × **4 themes** (`arcane`, `glacier`, `slate`, `high-contrast`) — including open Repo-Switcher and Diff-Inspector Radix portals — achieves `visualAfterScore >= 0.92`, positive `before -> after` contrast/sharpness delta, `axeViolations === 0`, `clippedInteractiveCount === 0`, `overlappingControlPairs === 0`, `digitalColorLeakCount === 0`, and `rawEmojiCount === 0`, emitting a paired Before/After visual scorecard.
  4. `cargo run -p vox-arch-check`, `vox audit code`, `vox ci operations-verify`, `cargo run -p vox-doc-pipeline -- --lint-only` (promoting `docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md` to `status: "current"` and updating `where-things-live.md` + `docs-system-ssot.md`), and all affected Rust + Vitest suites pass with `0` errors.

**Plans**: Complete (`470-PLAN.md`, `470-VERIFICATION.md`, `470-REVIEW.md`, `470-SUMMARY.md`)

---

## Progress

| Phase | Requirements | Plans Complete | Status | Completed |
|-------|--------------|----------------|--------|-----------|
| 465. Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage | `REPO-01..04`, `KB-01..02` (6) | 1/1 | Complete | 2026-11-17 |
| 466. Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval | `KB-03..04`, `GRAPH-01..04` (6) | 1/1 | Complete | 2026-11-17 |
| 467. Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions | `WIKI-01..02` (2) | 1/1 | Complete | 2026-11-17 |
| 468. Tiered Self-Updating & Self-Fixing Documentation Engine (AST Blast-Radius + Verified Prose Heal Queue) | `WIKI-03..04` (2) | 1/1 | Complete | 2026-11-17 |
| 469. Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation | `GUI-01..04` (4) | 1/1 | Complete | 2026-11-17 |
| 470. End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence | `CV-01..04` (4) | 1/1 | Complete | 2026-11-17 |
