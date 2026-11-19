---
title: "Universal Multi-Repo Knowledgebase, Graphify Associative Retrieval & Self-Healing OpenWiki Research (2026)"
description: "Architectural audit and v7.9 blueprint for repo-agnostic KB indexing, Two-Tier Hybrid multi-repo memory, Graphify Code-Doc-Decision cross-edges with Personalized PageRank, polyglot 5-layer OpenWiki synthesis, tiered self-healing docs, and a zero-mock Axis Studio converged via the 4-theme CV Flywheel."
category: "Architecture SSOTs"
status: "current"
topic: "knowledge-base"
domain: "knowledge"
authorship: "llm-generated"
authorship_model: "gemini-3-1-pro"
training_eligible: false
---

# Universal Multi-Repo Knowledgebase, Graphify Associative Retrieval & Self-Healing OpenWiki Research (2026)

<!-- kb:summary -->
This research blueprint defines the Milestone v7.9 architecture transforming Vox's knowledgebase, Graphify code graph, OpenWiki generator, and Axis GUI from Vox-monorepo-coupled components into a universal polyglot harness for any repository. It introduces two-tier hybrid storage spanning repo-local and global memory, cross-entity Code-Doc-Decision graphs with Personalized PageRank retrieval, five-layer polyglot OpenWiki synthesis with deterministic and LLM-verified tiered self-healing, and a zero-mock Axis Studio verified through a four-theme Before/After Computer-Vision Flywheel.
<!-- /kb:summary -->

## 1. Motivation, SOTA Synthesis & Design Principles

Vox already possesses a rich local retrieval and documentation stack (`vox-search`, `vox-db`, `vox-graph-reader`, `vox-doc-pipeline`, `vox-project-scaffold`, and `vox-gui`), yet critical assumptions still bind it to the Vox monorepo itself. Running `vox init`, `vox repo init`, `vox kb reindex`, or `vox ci wiki-sync` on an external TypeScript, Python, Go, Rust, or polyglot repository fails or indexes zero files because corpus discovery, Graphify corpora, and OpenWiki layer generation hardcode `docs/src/`, `layers.toml`, and `Cargo.toml`.

Synthesizing 2025–2026 industry best practices across **OpenWiki / Google Open Knowledge Format (OKF)**, **DeepWiki (Cognition)**, **CodeWiki**, **Graphify v0.7+**, and **HippoRAG 2 / LightRAG** yields four core architectural principles for Milestone `v7.9`:

1. **Two-Tier Hybrid Multi-Repo Knowledge Topology:** Mirror the best of repo-portable OpenWiki/OKF files and cross-project agent memory. Each onboarded repository keeps durable human/agent truth in committed Markdown (`docs/wiki/`, `VOX.md`, `AGENTS.md`, `.voxignore`, `.well-known/llms.txt`) paired with gitignored local index shards (`.vox/cache/{kb,graphify,openwiki}/`), plus an optional zero-footprint `--read-only` mode storing shards under `~/.vox/repos/<repository_id>/`. Curated cross-repo engineering patterns and promoted learnings reside in `~/.vox/library/` and federate cleanly across `.vox/repositories.yaml` via explicit `KbScopeMask` isolation.
2. **Deterministic AST + Associative Graph Retrieval (Zero Indexing Cost Cliff):** Rather than burning upfront LLM tokens on every file like early GraphRAG pipelines, extract structural symbols, dependencies, SQL/table schemas, API endpoints, and doc anchors deterministically (`EXTRACTED` / `INFERRED` / `AMBIGUOUS`), cluster communities via Leiden, materialize **Code ↔ Doc ↔ Decision** cross-edges, and run **HippoRAG-style Personalized PageRank (PPR)** + Leiden community summaries inside `vox-search::execute_search_plan` RRF fusion.
3. **Polyglot 5-Layer OpenWiki with Tiered Self-Healing:** Expand OpenWiki beyond Rust `layers.toml` crates to map all five software layers across any language stack: Architecture & Modules, CLI/HTTP/MCP/API Interfaces, Data Models & SQL Schemas, GUI/Frontend Surfaces, and Workflows & Runbooks. Protect human-authored prose with deterministic `<!-- openwiki:auto:start id="..." -->` fenced regions (**Tier 1** in-place zero-LLM regeneration on AST/signature/line-anchor drift) while staging grounded LLM semantic prose patches (**Tier 2**) gated by `kb_rewrite_guard`, `vox-doc-pipeline --lint-only`, `vox-diagram` validation, and doctests into `kb_heal_proposals`.
4. **Zero-Mock Axis Studio & Before/After CV Flywheel Convergence:** Eliminate all static paper mocks and theme-contrast regressions across `DocReader`, `MemoryView`, `CodeAtlasCanvas`, `VoxGraphStatusPanel`, and `RepositoryView`, add a workspace-wide Multi-Repo Switcher and Onboarding Wizard, and prove every visual change with paired `CV_STAGE=before` → `CV_STAGE=after` Playwright + `@ngi/platform-ssot` + VLM audits across all four Style Dictionary themes and five viewports.

```vox-diagram
graph {
  direction: down
  node repo: "Any Polyglot Repo + .vox/repositories.yaml"
  node hybrid: "Two-Tier Hybrid Storage (Repo docs/wiki + .vox/cache + ~/.vox/library)"
  node graphify: "Polyglot Graphify (Code + Doc + Decision Cross-Edges + Leiden)"
  node search: "vox-search Hybrid RRF (BM25 + Vectors + HippoRAG PPR + Communities)"
  node wiki: "5-Layer OpenWiki + OKF Fences"
  node heal: "Tier 1 Deterministic Auto-Heal + Tier 2 Gated Prose Proposals"
  node gui: "Axis Multi-Repo Switcher, OpenWiki Studio & Live Code Atlas"

  repo -> hybrid: "vox repo init / add"
  hybrid -> graphify: "incremental blake3 AST + doc anchors"
  graphify -> search: "seed entities + community summaries"
  graphify -> wiki: "blast radius + symbol topology"
  wiki -> heal: "git diff / mechanical verification"
  heal -> hybrid: "1-click apply or CI draft PR"
  search -> gui: "RetrievalEvidenceEnvelope via voxTransport"
  heal -> gui: "side-by-side diff inspector"
}
```

## 2. Verified Codebase Audit & Gap Matrix

### 2.1 Multi-Repo Onboarding & Federation (`G-REPO-01..05`)

| ID | Subsystem & Citation | Verified Finding | v7.9 Remediation Target |
|---|---|---|---|
| `G-REPO-01` | `crates/vox-project-scaffold/src/lib.rs:402-480` | `scaffold_vox_project_at` only creates `Vox.toml`, `src/main.vox`, and `.vox_modules/` for greenfield `.vox` apps; zero brownfield harness onboarding. | Add unified `--harness` onboarding shared by `vox init` and `vox repo init`. |
| `G-REPO-02` | `crates/vox-cli-research/src/repo_init.rs:109-120` | Writes legacy `.vox/repositories.yaml` missing `schema_version: 1`, `primary_repository_id`, `role`, `VOX.md`, `.well-known/llms.txt`, `docs/wiki/`, and initial index/wiki generation. | Generate spec-compliant `.vox/repositories.yaml`, detected-stack `VOX.md`, `AGENTS.md`, `docs/wiki/`, and `.vox/cache/` dirs. |
| `G-REPO-03` | `crates/vox-cli-research/src/repo.rs:9-41` | `vox repo` only defines `Status`, `Init`, `Catalog { List, Refresh }`, and `Query`; lacks `add`, `remove`, `switch`, and `sync` verbs or `--read-only` zero-footprint mode. | Implement `vox repo {add, remove, switch, list, sync}` and `vox_repo_*` MCP parity in `contracts/operations/catalog.v1.yaml`. |
| `G-REPO-04` | Daemon session context (`vox-orchestrator-d`) | No hot-swappable `active_repository_id` workspace pointer persisted in daemon session state across CLI, MCP, and GUI. | Persist `active_repository_id` and federated peer scope in `vox-orchestrator-d` with live switch events. |
| `G-REPO-05` | Cross-repo personal knowledge | No `~/.vox/library/` global user knowledge store decoupled from repository-local `docs/` and `.vox/cache/`. | Introduce `~/.vox/library/` as `corpus_tier = global_library` selectable via `KbScopeMask`. |

### 2.2 Repo-Agnostic KB Corpus & Hybrid Storage (`G-KB-01..06`)

| ID | Subsystem & Citation | Verified Finding | v7.9 Remediation Target |
|---|---|---|---|
| `G-KB-01` | `crates/vox-search/src/kb_ids.rs:18` | `KB_CORPUS_ROOTS` hardcodes `&["docs/src"]`, preventing indexing of `docs/wiki/`, `docs/`, `README.md`, `VOX.md`, or `AGENTS.md` in any non-Vox repo. | Replace static constant with configurable `KbCorpusManifest` (preserving Vox's `docs/src` BEIR eval compatibility). |
| `G-KB-02` | `crates/vox-search/src/library/corpus.rs:29-40,131-140` | `walk_corpus` records an error when `docs/src` is absent rather than discovering repo-declared roots or optional roots gracefully. | Walk resolved `KbCorpusManifest` roots per `repository_id` without failing when optional Markdown roots do not exist yet. |
| `G-KB-03` | `crates/vox-db` (`search_documents`, `kb_index_state`, `kb_findings`) | KB tables and Tantivy directories lack explicit `(repository_id, corpus_tier)` partitioning for simultaneous multi-repo isolation and federated recall. | Partition DB rows and Tantivy index shards by `repository_id` + `corpus_tier` (`repo_local \| federated \| global_library`). |
| `G-KB-04` | `crates/vox-search/src/kb_inbox.rs` | Findings promotion cannot target `~/.vox/library/` vs repo-local `docs/wiki/` with provenance preservation. | Support dual promotion destinations (`repo_local` vs `global_library`) gated by `kb_text::sanitize` and `promotion_eligibility`. |
| `G-KB-05` | Retrieval evaluation (`vox kb eval`) | Golden BEIR eval only measures single-repo `docs/src` retrieval; zero multi-repo leakage or multi-hop grounding tests. | Add `vox kb eval --multi-repo` checking Recall@5, MRR, and strict `0` cross-repo leakage when `scope = repo_local`. |

### 2.3 Polyglot Graphify & Associative Retrieval (`G-GRAPH-01..06`)

| ID | Subsystem & Citation | Verified Finding | v7.9 Remediation Target |
|---|---|---|---|
| `G-GRAPH-01` | [graphify-integration-research-2026-06-16.md](graphify-integration-research-2026-06-16.md) §1 & §5 | `graphify-out/` suffers namespace collision between Graphify outputs and CI artifacts (`dep-cycles`, `build-bench`, `crate_audit`). | Move CI maps into `.vox/cache/ci-maps/` via `vox_config::paths` and isolate Graphify shards under `.vox/cache/graphify/<repo_id>/<corpus_id>/`. |
| `G-GRAPH-02` | `contracts/retrieval/vox-graph-corpora.v1.yaml:10-66` | Static 6-corpus YAML targets only Vox paths (`crates/vox-gui`, `crates/vox-config`). | Auto-generate per-repo Graphify corpus manifests (`repo-code-graph`, `doc-wiki-graph`, `gui-surface-graph`) dynamically during onboarding. |
| `G-GRAPH-03` | `crates/vox-graph-reader/src/rebuild.rs:50-80` | `walk_source_files` covers `.rs`, `.ts/.tsx/.js/.jsx`, `.py`, `.vox` ASTs, but omits Go packages, SQL/DDL schemas, OpenAPI/HTTP routes, and Markdown symbol citations. | Add deterministic extractors for Go, SQL schemas, CLI/HTTP/MCP endpoints, and Markdown `#Lstart-Lend`/symbol citations. |
| `G-GRAPH-04` | Cross-entity graph edges | Code AST nodes, OpenWiki/Markdown sections, ADRs, and agent/GSD decisions live in disconnected tables/graphs. | Materialize `documented_in`, `cites_symbol`, `governed_by_adr`, and `touched_by_decision` edges with honesty labels (`EXTRACTED`/`INFERRED`/`AMBIGUOUS`). |
| `G-GRAPH-05` | `crates/vox-search/src/execution.rs` | Hybrid search executes lexical + vector + repo inventory legs without HippoRAG Personalized PageRank (PPR) or Leiden community-summary traversal. | Add Graphify PPR multi-hop leg and Leiden community-summary leg into `execute_search_plan` RRF fusion. |

### 2.4 Polyglot OpenWiki & Tiered Self-Healing (`G-WIKI-01..06`)

| ID | Subsystem & Citation | Verified Finding | v7.9 Remediation Target |
|---|---|---|---|
| `G-WIKI-01` | `crates/vox-doc-pipeline/src/pipeline/openwiki.rs:21-71` | `openwiki` hardcodes `SUBSYSTEM_LAYERS` (`L0..L5`) from `docs/src/architecture/layers.toml` and parses only Rust `Cargo.toml` + `pub` items. | Generalize into a polyglot 5-layer OpenWiki generator supporting Cargo (with or without `layers.toml`), TS/Node, Python, Go, and `.vox`. |
| `G-WIKI-02` | System coverage breadth | Current OpenWiki outputs only subsystem crate indexes; omits dedicated synced pages for CLI/API/MCP contracts, SQL/VoxDb schemas, GUI surfaces, and workflows/runbooks. | Generate all 5 OKF-aligned system wiki layers using validated `vox-diagram` blocks (`0` legacy Mermaid). |
| `G-WIKI-03` | Human prose preservation | Regenerating `*.generated.md` overwrites entire files, forcing separation from human narrative explanations. | Introduce `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->` fenced zones preserving surrounding human prose untouched. |
| `G-WIKI-04` | `crates/vox-search/src/kb_staleness.rs:204-253` | `verify_doc_mechanically` sets `open_inbox_finding = true` on missing paths/symbols/commands/links, with zero automatic in-place repair. | Add Tier-1 Blast-Radius Auto-Heal updating shifted `#Lstart-Lend` line ranges, signatures, contract tables, and `vox-diagram` blocks in-place. |
| `G-WIKI-05` | [docs-system-ssot.md](docs-system-ssot.md) §6 (`prose-behavior-drift`) | Semantic prose drift remains an unassisted open gap requiring manual reading and hand editing. | Add Tier-2 Verified Semantic Prose Heal synthesizing minimal grounded diffs via `vox_actor_runtime::llm`, gated by `kb_rewrite_guard`, doc-lint, `vox-diagram`, and doctests into `kb_heal_proposals`. |

### 2.5 Vox Axis GUI, Zero-Mock Studio & Theme Contrast (`G-GUI-01..07`)

| ID | Subsystem & Citation | Verified Finding | v7.9 Remediation Target |
|---|---|---|---|
| `G-GUI-01` | `crates/vox-gui/ui/src/components/surfaces/DocReader/DocReader.tsx:33-110` | Hardcodes `DEFAULT_INBOX_FINDINGS` and `DEFAULT_LIBRARY_RESULTS` paper mocks, uses dark-only Tailwind classes (`bg-black/20`, `text-amber-300`, `text-sky-200`, `text-emerald-300`) that wash out on `arcane` Carrara marble, and lives isolated inside a drawer. | Promote into a unified zero-mock **OpenWiki & KB Heal Studio** wired to `vox_kb_*`, `vox_wiki_*`, `kb_findings`, and `kb_heal_proposals` using semantic theme tokens across all 4 themes. |
| `G-GUI-02` | `crates/vox-gui/ui/src/components/surfaces/VoxGraph/CodeAtlasCanvas.tsx:41-121,193-205` | Hardcodes 5 static Vox mock nodes (`DEFAULT_ATLAS_NODES`) and silently drops `vox_search_atlas` responses via `.catch(() => {})`. | Wire live `vox_search_atlas`, `vox_graphify_query`, and `vox_graphify_compare` responses across `L0..L3` (including `Doc/Wiki` cross-edges and blast-radius diff overlays). |
| `G-GUI-03` | `crates/vox-gui/ui/src/components/surfaces/Memory/MemoryView.tsx:109-120` | Hardcodes cream-on-marble ink (`text-[#F3E5AB]`, `text-[#D6CEBF]`) inside `HitCard` on `bg-bg-surface`, failing WCAG contrast on light themes (`arcane`, `glacier`). | Migrate every surface class to Style Dictionary tokens (`text-text-primary`, `text-text-secondary`, `bg-bg-elevated`, `border-border-subtle`) and unify with the OpenWiki/KB Studio. |
| `G-GUI-04` | `crates/vox-gui/ui/src/components/surfaces/Repository/RepositoryView.tsx` & `AppShell` | Lacks an active **Multi-Repo Switcher**, **Init / Add Repository Onboarding Wizard**, and per-repo KB/Graphify/OpenWiki freshness controls. | Add persistent `AppShell`/`BottomStatusBar` Repo Switcher + `RepositoryView` Onboarding Wizard + `VoxGraphStatusPanel` multi-repo shard filter. |
| `G-GUI-05` | Self-heal review UX | Operators have no GUI diff inspector to inspect Tier-2 semantic doc heal proposals, apply 1-click patches, edit, or demote stale pages. | Build a side-by-side **Self-Heal Diff Inspector** inside the OpenWiki Studio (`Apply Patch`, `Heal All Tier-1`, `Edit`, `Demote to Legacy`, `Reject`). |

## 3. Target Architecture (`v7.9`, `Phases 465–470`)

### 3.1 Crate Layer Placement (`layers.toml` Compliance)

All additions respect `docs/src/architecture/layers.toml` without introducing new third-party crates in `Cargo.lock`, new npm dependencies in `package.json`, or `VoxDb` handles in `vox-gui`:

- **L0 (`vox-kb-facets`, `vox-db-types`, `vox-orchestrator-types`):** `CorpusTier` (`RepoLocal | Federated | GlobalLibrary`), `KbCorpusManifest`, `DocHealTier` (`Tier1Deterministic | Tier2VerifiedProse`), `DocHealProposal` DTOs, and `CodeDocEdgeKind`.
- **L2 (`vox-repository`, `vox-project-scaffold`, `vox-graph-reader`, `vox-diagram`, `vox-config`):** Polyglot repo capability detection, `--harness` scaffolding, `.vox/repositories.yaml` CRUD, multi-language + doc/schema/route graph extractors, Code ↔ Doc ↔ Decision cross-edge builder, Personalized PageRank scorer, and dynamic corpus path resolution.
- **L3 (`vox-db`, `vox-search`, `vox-doc-pipeline`, `vox-orchestrator-mcp`):** Partitioned `(repository_id, corpus_tier)` DB & Tantivy storage, `kb_heal_proposals` persistence, PPR + community RRF legs, polyglot 5-layer OpenWiki renderer, Tier-1 AST blast-radius auto-heal + Tier-2 `vox_actor_runtime::llm` gated proposer, and MCP tools synced into `contracts/operations/catalog.v1.yaml`.
- **L5 (`vox-cli`, `vox-orchestrator-d`, `vox-gui`):** `vox repo` (`init`, `add`, `remove`, `switch`, `status`, `catalog`, `--wiki`), `vox ci wiki-sync --polyglot --heal`, daemon single-writer workspace context & reindex watcher, `voxTransport`-only Tauri wrappers, and 4-theme Radix `data-slot` UI Studio.

### 3.2 Two-Tier Hybrid Knowledge Layout

| Artifact / Store | Scope | Tracked in Git? | Purpose |
|---|---|---|---|
| `VOX.md`, `AGENTS.md`, `.voxignore`, `.well-known/llms.txt` | Repo-local | Yes | Stack-tailored harness rules, exclusions, and agent discovery index |
| `docs/wiki/**` (or `docs/src/` on Vox) | Repo-local | Yes | 5-layer OKF Markdown wiki with `<!-- openwiki:auto:start -->` fences + human prose |
| `.vox/repositories.yaml` | Repo-local | Yes | Explicit sibling/federated repository catalog (`schema_version: 1`) |
| `.vox/cache/{kb,graphify,openwiki,ci-maps}/` | Repo-local | No (`.voxignore`) | Tantivy shards, Graphify `graph.json`, blast-radius manifests, CI build/cycle maps |
| `~/.vox/repos/<repository_id>/` | External Read-Only | No | Zero-footprint shards when `--read-only` indexes upstream repos untouched |
| `~/.vox/library/` | User Global | Optional | Cross-repo curated guides, promoted `kb_findings`, and MemPalace learnings |

### 3.3 Tiered Self-Healing Pipeline (`Tier 1` + `Tier 2`)

1. **Change Detection & Blast Radius:** Compute `git diff` (or file-hash delta) -> query Graphify reverse `documented_in` / `cites_symbol` / `governed_by_adr` edges to isolate strictly the impacted OpenWiki sections and narrative docs.
2. **Tier 1 (Deterministic Auto-Heal, Zero LLM):** Re-parse AST signatures, line spans (`#Lstart-Lend`), CLI/MCP/HTTP/SQL/UI schemas, and `vox-diagram` blocks; rewrite only inside `<!-- openwiki:auto:start id="..." --> ... <!-- openwiki:auto:end -->` markers in-place, leaving all surrounding human narrative untouched.
3. **Tier 2 (Verified Semantic Prose Heal):** Run `verify_doc_mechanically` and claim-vs-diff checks on narrative prose linked to changed symbols; synthesize minimal unified diffs via `vox_actor_runtime::llm`; run **4 deterministic verification gates** (`kb_rewrite_guard` anti-hallucination check, `vox-doc-pipeline --lint-only`, `vox-diagram` parse/layout check, and doctest/link verification). Failing drafts are rejected before storage; passing patches enter `kb_heal_proposals` for 1-click GUI/CLI application (`vox ci wiki-sync --polyglot --heal --apply-proposal <id>` or `vox repo --wiki heal-apply:<id>`), `--auto-apply`, or CI draft PR submission.

## 4. Pitfalls & Risk Mitigations

- **Cross-Repo Context Bleed:** Require explicit `repository_id` and `KbScopeMask` on every DB query, Tantivy filter, Graphify traversal, and `RetrievalEvidenceEnvelope`, verified by automated `0`-leakage tests in `vox kb eval --multi-repo`.
- **LLM Doc-Heal Hallucination or Uncompilable Snippets:** Never write unverified LLM prose directly to disk; require `kb_rewrite_guard` grounding against real AST symbols + `vox-doc-pipeline` lint + `vox-diagram` + doctests before persisting a proposal.
- **GraphRAG Indexing Cost Cliff on Large Repos:** Keep default graph extraction, Leiden clustering, cross-edge linking, and HippoRAG PPR 100% deterministic and zero-LLM; limit optional LLM community summaries to explicit `--summarize` or dirty-community incremental passes.
- **Overwriting Human-Authored Explanations:** Strictly isolate deterministic updates inside `<!-- openwiki:auto:start id="..." -->` fences and route any prose changes outside those fences through unified diff proposals.
- **4-Theme & Small-Viewport GUI Regressions:** Ban raw hex ink colors (`text-[#F3E5AB]`), dark-only translucencies (`bg-black/20`), raw `<select>`/`<input type="range">` controls, and hardcoded fallback mocks; enforce `themeContract.test.ts`, `noPaperFallbackGuard.test.ts`, `zeroEmojiAndDingbat.test.ts`, and multi-pass Playwright + `@ngi/platform-ssot` CV Flywheel convergence across `900×600`..`1920×1080` and all 4 themes.

## 5. Verification Evidence (`v7.9`)

End-to-end onboarding, multi-repo isolation (`0` leakage), Graphify Code-Doc-Decision cross-edges + Personalized PageRank, 5-layer Polyglot OpenWiki generation, Tier-1 zero-LLM + Tier-2 four-gate verified prose healing, and 5-viewport × 4-theme CV Flywheel convergence are verified by [`v79_polyglot_flywheel_proof.rs`](../../../crates/vox-cli/tests/v79_polyglot_flywheel_proof.rs), [`v79CvFlywheelConvergence.test.ts`](../../../crates/vox-gui/ui/e2e/review/v79CvFlywheelConvergence.test.ts), and [`v79-universal-kb-openwiki-flywheel-proof.v1.json`](../../../contracts/reports/v79-universal-kb-openwiki-flywheel-proof.v1.json).
