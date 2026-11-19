# Project Research Summary

**Project:** Vox (`v7.9` — `universal-kb-openwiki-v79`)
**Domain:** Universal Multi-Repo Knowledgebase, Graphify Associative Retrieval, Tiered Self-Healing OpenWiki & Zero-Mock Axis Studio + CV Flywheel
**Researched:** 2026-11-17
**Confidence:** HIGH

## Executive Summary

Vox already ships strong building blocks for retrieval, AST graph indexing, documentation governance, and desktop UI (`vox-search`, `vox-db`, `vox-graph-reader`, `vox-doc-pipeline`, `vox-project-scaffold`, and `vox-gui`), but an end-to-end codebase audit shows they remain structurally coupled to the Vox monorepo itself. Specifically, `KB_CORPUS_ROOTS` hardcodes `&["docs/src"]` (`crates/vox-search/src/kb_ids.rs:18`) and `walk_corpus` errors when `docs/src` is absent (`crates/vox-search/src/library/corpus.rs:131-140`); `vox init` and `vox repo init` are disconnected and emit an incomplete `.vox/repositories.yaml` (`crates/vox-cli-research/src/repo_init.rs:109-120`) without `add/remove/switch` lifecycle verbs (`crates/vox-cli-research/src/repo.rs:9-41`); Graphify corpora are static to Vox (`contracts/retrieval/vox-graph-corpora.v1.yaml`), collide with CI artifacts under `graphify-out/`, and lack **Code ↔ Doc ↔ Decision** cross-edges or Personalized PageRank (PPR) / community legs in `vox-search`; OpenWiki (`crates/vox-doc-pipeline/src/pipeline/openwiki.rs`) only understands Vox's `layers.toml` + Rust `Cargo.toml` and `kb_staleness.rs` only flags drift instead of repairing it; and Axis GUI surfaces (`DocReader.tsx`, `CodeAtlasCanvas.tsx`, `MemoryView.tsx`, `RepositoryView.tsx`) ship hardcoded paper mocks, swallowed `.catch(() => {})` calls, cream-on-marble/dark-only theme contrast bugs, and no workspace multi-repo switcher or heal diff inspector.

The `v7.9` architecture resolves these gaps within Vox's existing `layers.toml` crate hierarchy (`0` new third-party crates, `0` new npm packages, `0` `VoxDb` handles in `vox-gui`, and `100%` `vox_actor_runtime::llm` routing). First, unify project/repository onboarding under a **Two-Tier Hybrid** storage model: committed repo-local `docs/wiki/`, `VOX.md`, `AGENTS.md`, `.voxignore`, and `.well-known/llms.txt`; gitignored `.vox/cache/` shards (plus `--read-only` zero-footprint shards under `~/.vox/repos/<repository_id>/`); federated recall via `.vox/repositories.yaml`; and cross-repo user knowledge under `~/.vox/library/`. Second, eliminate `graphify-out/` collisions, expand `vox-graph-reader` across polyglot code + SQL/API/UI schemas + Markdown citations, link **Code ↔ Doc ↔ Decision** cross-edges, and fuse **HippoRAG-style PPR + Leiden community summaries** into `vox-search`. Third, generalize OpenWiki across all five system layers using non-destructive OKF `<!-- openwiki:auto:start -->` fences (**Tier 1** zero-LLM blast-radius auto-heal) paired with gated LLM prose proposals (**Tier 2** `kb_heal_proposals`). Fourth, deliver an `AppShell` Multi-Repo Switcher + Onboarding Wizard, zero-mock OpenWiki/KB Heal Studio, live Graphify Code Atlas, 4-theme contrast remediation, and multi-pass `BEFORE -> AFTER` CV Flywheel convergence across `5` viewports × `4` themes.

## Key Findings

### Recommended Stack

Stay 100% inside the pinned Vox Rust + TypeScript workspace (`0` new `Cargo.lock` entries, `0` new `package.json` dependencies, `0` `layers.toml` inversions):

- **L0 / L2 Core & Graph (`vox-kb-facets`, `vox-db-types`, `vox-repository`, `vox-project-scaffold`, `vox-graph-reader`, `vox-diagram`, `vox-config`):** Polyglot project detection, `--harness` onboarding, dynamic `KbCorpusManifest` and per-repo Graphify manifest resolution, Go/SQL/endpoint/Markdown AST/symbol extraction, Code ↔ Doc ↔ Decision edge linking, and deterministic Personalized PageRank (PPR) traversal.
- **L3 Retrieval, Wiki & Daemon (`vox-db`, `vox-search`, `vox-doc-pipeline`, `vox-orchestrator-mcp`, `vox-actor-runtime`):** `(repository_id, corpus_tier)` partitioned `VoxDb` + Tantivy shards, `kb_heal_proposals` table, PPR + Leiden community summary legs in `execute_search_plan`, polyglot 5-layer OpenWiki generator, Tier-1 in-place auto-heal + Tier-2 `kb_rewrite_guard` / doc-lint / doctest-gated LLM diff synthesis, and MCP tools registered in `contracts/operations/catalog.v1.yaml`.
- **L5 Surfaces (`vox-cli`, `vox-orchestrator-d`, `vox-gui`):** `vox repo {init, add, remove, switch, list, sync}`, `vox wiki {init, build, status, diff, heal}`, daemon session `active_repository_id` + single-writer reindexer, `voxTransport` Tauri bridge, and Radix `data-slot` Style Dictionary 4-theme UI converged via `@ngi/platform-ssot` + Playwright + VLM CV Flywheel.

### Expected Features

**Must have (table stakes):**
- Seamless `vox init --harness` and `vox repo {init, add, remove, switch, list, sync}` for any language stack (Rust, TS/Node, Python, Go, Vox, polyglot) plus `--read-only` zero-footprint mode.
- Repo-agnostic `vox kb` indexing and `(repository_id, corpus_tier)` isolation across repo-local, federated sibling, and global `~/.vox/library/` scopes.
- Polyglot Graphify code + doc + decision graphs cleanly isolated under `.vox/cache/graphify/` with PPR + community RRF retrieval.
- 5-layer OKF-aligned OpenWiki generation (`Architecture`, `Interfaces`, `Data Models`, `GUI Surfaces`, `Workflows & Runbooks`) using `vox-diagram` (`0` legacy Mermaid) and non-destructive `<!-- openwiki:auto:start -->` fences.
- Tiered Self-Heal: zero-LLM Tier-1 in-place regeneration of AST signatures, `#Lstart-Lend` anchors, and contract tables + gated Tier-2 semantic prose diff proposals (`kb_heal_proposals`).
- Zero-mock Axis Multi-Repo Switcher, Onboarding Wizard, OpenWiki/KB Heal Diff Inspector, live Graphify Code Atlas, and 4-theme WCAG contrast compliance.

**Should have (competitive):**
- Blast-radius `git diff` targeting so edits only touch impacted OpenWiki hubs and narrative docs rather than full-repo sweeps.
- Bi-directional `kb_findings` + `MemPalace` promotion into repo `docs/wiki/` or `~/.vox/library/`.
- Multi-repo BEIR evaluation (`vox kb eval --multi-repo`) gating Recall@5, MRR, and zero cross-repo leakage.

**Defer (`v8.0+`):**
- Real-time CRDT multi-user collaborative wiki editing and multi-tenant hosted OpenWiki SaaS sync.
- Automatic unsupervised direct git commits to `main` for Tier-2 prose edits without opt-in flags or CI PR review.

### Architecture Approach

1. **Unify Repo Identity & Storage (`Phase 465`):** Remove hardcoded `docs/src` assumptions in `vox-search`, make `vox_repository` and `vox_project_scaffold` the shared onboarding core across CLI/MCP/GUI, partition `VoxDb` + Tantivy by `(repository_id, corpus_tier)`, and expose `~/.vox/library/` + `~/.vox/repos/<id>/`.
2. **Close the Graphify ↔ Retrieval Loop (`Phase 466`):** Purge CI map collisions from `graphify-out/`, generate per-repo Graphify manifests automatically, extract polyglot + SQL/route/Markdown cross-edges with honesty tiers, and wire HippoRAG PPR + Leiden community summaries into `vox-search`.
3. **Synthesize Polyglot 5-Layer OpenWiki (`Phase 467`):** Generalize `vox-doc-pipeline::openwiki` from Rust-only `layers.toml` into a polyglot 5-layer OKF generator emitting `vox-diagram` topologies and preserving human prose outside `<!-- openwiki:auto:start -->` markers.
4. **Automate Tiered Self-Healing (`Phase 468`):** Wire `git diff` -> Graphify blast radius -> Tier-1 deterministic in-place repair + Tier-2 `vox_actor_runtime::llm` proposal synthesis gated by `kb_rewrite_guard`, doc-lint, `vox-diagram`, and doctests (`kb_heal_proposals`).
5. **Rebuild Axis Studio & Converge CV Flywheel (`Phases 469–470`):** Replace every mock and contrast defect across `DocReader`, `MemoryView`, `CodeAtlasCanvas`, `VoxGraphStatusPanel`, and `RepositoryView`; add the `AppShell` Repo Switcher + Diff Inspector; and prove end-to-end polyglot onboarding + `BEFORE -> AFTER` 4-theme × 5-viewport CV Flywheel convergence.

### Critical Pitfalls

1. **Cross-Repo Context Bleed:** Unfiltered queries could leak private repo chunks into another project; enforce `(repository_id, corpus_tier)` predicates and `KbScopeMask` at the `vox-search` / `vox-db` boundary and gate with `vox kb eval --multi-repo`.
2. **Clobbering Human Narrative Documentation:** Never overwrite whole Markdown files during OpenWiki rebuilds; fence machine-owned sections with `<!-- openwiki:auto:start id="..." -->` and route prose updates through unified diff proposals.
3. **LLM Doc-Heal Hallucination:** Gate every Tier-2 semantic prose proposal through `kb_rewrite_guard`, `vox-doc-pipeline --lint-only`, `vox-diagram` validation, and doctests before inserting into `kb_heal_proposals`.
4. **GraphRAG Cost Cliff:** Keep AST parsing, cross-edge extraction, Leiden clustering, and HippoRAG PPR 100% deterministic (`0` LLM tokens) on default `build`/`reindex` runs.
5. **Theme & Portal Contrast Failures:** Replace `text-[#F3E5AB]`, `bg-black/20`, and dark-only Tailwind hues (`text-amber-300`) with Style Dictionary semantic tokens and scan open Radix portals in Playwright Axe + VLM passes across all 4 themes.

## Implications for Roadmap

Suggested 6-phase structure (`Phases 465–470`, continuing cleanly after `v7.8` `Phase 464`):

- **Phase 465 — Universal Multi-Repo Onboarding, Dynamic KB Corpus Discovery & Two-Tier Hybrid Storage (`REPO-01..04`, `KB-01..02`):** Delivers `vox repo {init, add, remove, switch, list, sync}`, `vox init --harness`, `--read-only` zero-footprint mode, daemon `active_repository_id`, dynamic `KbCorpusManifest`, and `(repository_id, corpus_tier)` DB/Tantivy isolation.
- **Phase 466 — Polyglot Graphify Corpora, Code ↔ Doc ↔ Decision Cross-Edges & Associative PPR/Community Retrieval (`KB-03..04`, `GRAPH-01..04`):** Resolves `graphify-out/` collisions, generates dynamic per-repo Graphify corpora, extracts polyglot/schema/route/Markdown cross-edges, wires HippoRAG PPR + Leiden community summaries into RRF, enables dual `kb_findings`/`MemPalace` promotion, and ships `vox kb eval --multi-repo`.
- **Phase 467 — Repo-Agnostic 5-Layer OpenWiki Synthesis & Non-Destructive OKF Deterministic Regions (`WIKI-01..02`):** Upgrades `vox-doc-pipeline::openwiki` into a polyglot 5-layer OKF engine (`vox wiki {init, build, status, diff}`, `vox_wiki_*` MCP tools) with `vox-diagram` figures and human-preserving auto-fences.
- **Phase 468 — Tiered Self-Updating & Self-Fixing Documentation Engine (`WIKI-03..04`):** Implements Graphify blast-radius diff tracing, Tier-1 zero-LLM in-place signature/anchor/table/diagram auto-healing, and Tier-2 grounded LLM prose healing gated into `kb_heal_proposals` (`--apply`, `--auto-apply`, CI draft PR).
- **Phase 469 — Vox Axis Multi-Repo Switcher, Zero-Mock OpenWiki/KB Heal Studio, Live Code Atlas & 4-Theme Remediation (`GUI-01..04`):** Adds `AppShell` Repo Switcher + Onboarding Wizard, turns `DocReader` + `MemoryView` into a zero-mock OpenWiki & Self-Heal Diff Studio, connects `CodeAtlasCanvas` + `VoxGraphStatusPanel` to live multi-repo Graphify APIs, and fixes all light/dark theme contrast defects.
- **Phase 470 — End-to-End Polyglot Repo Onboarding Proof, Concept-Art Plates & Multi-Pass Before/After CV Flywheel Convergence (`CV-01..04`):** Proves full polyglot onboarding + Tier-1/Tier-2 self-healing, captures paired `BEFORE -> AFTER` CV Flywheel convergence across `4` themes × `5` viewports against `v79-concept-*` plates, and passes all workspace CI/doc/arch gates.

## Sources

- **In-Repo Architecture & Codebase Anchors:** `docs/src/architecture/universal-kb-graphify-openwiki-multirepo-research-2026.md`, `docs/src/architecture/search-retrieval-ssot-2026.md`, `docs/src/architecture/graphify-integration-research-2026-06-16.md`, `docs/src/architecture/docs-system-ssot.md`, `crates/vox-search/src/kb_ids.rs`, `crates/vox-search/src/library/corpus.rs`, `crates/vox-search/src/kb_staleness.rs`, `crates/vox-doc-pipeline/src/pipeline/openwiki.rs`, `crates/vox-graph-reader/src/rebuild.rs`, `crates/vox-cli-research/src/{repo.rs,repo_init.rs}`, `crates/vox-project-scaffold/src/lib.rs`, `crates/vox-gui/ui/src/components/surfaces/{DocReader/DocReader.tsx,Memory/MemoryView.tsx,VoxGraph/CodeAtlasCanvas.tsx,VoxGraph/VoxGraphStatusPanel.tsx,Repository/RepositoryView.tsx}`.
- **External SOTA References:** OpenWiki & Google Open Knowledge Format (OKF) incremental codebase wikis; Cognition DeepWiki & CodeWiki multi-agent AST/diagram pipelines; Graphify (`safishamsi/graphify`) deterministic AST extraction, Leiden community clustering, and `EXTRACTED`/`INFERRED`/`AMBIGUOUS` edge honesty; HippoRAG 2 (Personalized PageRank associative multi-hop retrieval) and LightRAG (dual local/global entity + community retrieval).
