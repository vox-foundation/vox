# Technology Stack: Self-Healing OpenWiki & Multi-Repo Documentation Sync (`v9.1`)

**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`  
**Researched:** 2026-11-19  
**Constraint:** **Zero new third-party crates** in `Cargo.lock` and **zero new npm packages** in `package.json`.

## 1. Existing Workspace Stack (Ready to Wire)

Every capability required for AST-coupled anchors, hierarchical repo-wiki generation, incremental Tantivy/SQLite indexing, NLI claim verification, and Axis GUI review is already compiled inside the workspace:

| Layer | Crate / Module | Existing Capabilities | Role in `v9.1` |
|---|---|---|---|
| **L0** | `vox-kb-facets`, `vox-orchestrator-types` | SKOS taxonomy (`kb-taxonomy.v1.yaml`), `ContextTrustTier`, `KbScopeMask` | Facet validation, trust partitioning |
| **L1** | `vox-graph-reader` (`crates/vox-graph-reader/`) | `tree-sitter` (`rust`, `typescript`, `python`), `syn` (`full`, `visit`), `ignore`, `petgraph`, `leiden-rs`, `blake3`, `polyglot.rs`, `polyglot_edges.rs`, `rebuild.rs` | Authoritative polyglot AST symbol + span + docstring extraction, Leiden community clustering, incremental `blake3` rebuilds, `Code <-> Doc <-> Decision` cross-edges |
| **L2** | `vox-diagram` (`crates/vox-diagram/`) | Deterministic `vox-diagram` parser, validator (`check`), formatter, markdown fence scanner | Validated architectural/topology figures (`0` legacy Mermaid) |
| **L3** | `vox-doc-pipeline` (`crates/vox-doc-pipeline/`) | `claims/*` (`extractor`, `resolver`, `gate`, `ledger`, `prune`, `llm_verifier`, `self_heal_bot`), `pipeline/*` (`openwiki`, `openwiki_polyglot`, `openwiki_heal`, `lint`, `doctest`, `facets`) | Ground-truth registry, reality audit, drift gate, 5-layer OpenWiki synthesis, Tier-1 anchor heal, 4-gate validator |
| **L3** | `vox-search` (`crates/vox-search/`) | `library/*` (`chunk`, `corpus`, `lexical` Tantivy mmap), `kb_ids::KbCorpusManifest`, `kb_staleness`, `kb_rewrite_guard`, `promotion` | Scoped `(repository_id, corpus_tier)` KB indexing, staleness priority queue, Tier-2 proposal staging (`kb_heal_proposals`) |
| **L3** | `vox-actor-runtime::llm` | `infer_with_retry`, `llm_chat`, `llm_stream` | Model-agnostic LLM boundary (`AGENTS.md`) for NLI claim verification & Tier-2 prose diff synthesis |
| **L3** | `vox-db` (`crates/vox-db/`) | `VoxDb`, `kb_index_state`, `search_documents`, `search_document_chunks`, `kb_findings`, `kb_heal_proposals`, `knowledge_edges` | Canonical persistent store (`~/Library/Application Support/vox/vox.db` / platform data dir) + `daemon_db_proxy` |
| **L3** | `vox-orchestrator-mcp` (`crates/vox-orchestrator-mcp/`) | `repo_catalog_tools.rs`, `kb_tools/library.rs`, `kb_tools/findings.rs` | MCP endpoints (`vox_repo_*`, `vox_kb_*`) delegating directly to `vox-doc-pipeline` & `vox-search` |
| **L5** | `vox-cli`, `vox-cli-research`, `vox-cli-ci`, `vox-orchestrator-d` | `vox audit docs`, `vox kb`, `vox repo`, `vox ci wiki-sync`, `vox ci ssot-drift`, daemon background tasks | CLI workflows, daemon background KB auto-indexing, pre-push & CI gates |
| **L5** | `crates/vox-gui/ui` | `OpenWikiHealStudio.tsx`, `OpenWikiDiffPane.tsx`, `DocReader.tsx`, `MemoryView.tsx`, `RepositoryCatalogPanel.tsx` | Zero-mock operator studio over `voxTransport` -> MCP -> `vox-orchestrator-d` |

## 2. Stack Gaps & Fixes

1. **`vox-doc-pipeline` -> `vox-graph-reader` / `vox_actor_runtime` Wiring:**
   - Currently `vox-doc-pipeline/Cargo.toml` (`L3`) does NOT depend on `vox-graph-reader` (`L1`) or `vox-actor-runtime` (`L3`).
   - Adding `vox-graph-reader` (`L1`) to `vox-doc-pipeline` (`L3`) respects `layers.toml` strict downward dependencies (`L3 -> L1`), giving `openwiki_polyglot`, `openwiki_heal`, and `claims/resolver` full `tree-sitter` + `syn` AST + `ignore::WalkBuilder` capabilities without duplicating parsers.
   - Orchestrator/search crates (`vox-search`, `vox-cli-research`, `vox-orchestrator-mcp`) already link `vox-actor-runtime::llm` and `vox-db`, so LLM NLI calls and `kb_heal_proposals` staging live cleanly in `vox-search` / `vox-cli-research` (or via a trait callback injected into `vox-doc-pipeline`), avoiding any L3 cycle or `vox-doc-pipeline` bloat.
2. **External Language Parsers Already Covered vs. Needed Fallbacks:**
   - `tree-sitter-rust`, `tree-sitter-typescript`, and `tree-sitter-python` + `syn` handle Rust, TS/TSX/JS/JSX, and Python structurally.
   - `vox-compiler` (already a dependency of `vox-doc-pipeline`) parses `.vox` declarations directly.
   - `vox-graph-reader::polyglot` already provides Go, SQL/DDL, HTTP route, and Markdown citation extraction — these should be unified into `openwiki_polyglot` so OpenWiki never uses ad-hoc line regexes that miss signatures, multi-line declarations, or docstrings.

## 3. What NOT to Add

- **Do NOT** add `reqwest`, `async-openai`, `anthropic`, or vendor HTTP clients to `vox-doc-pipeline` (`AGENTS.md §Model-Agnostic LLM Boundary` forbids direct vendor SDKs).
- **Do NOT** add external Mermaid CLI tools or new JS dependencies (`vox-diagram` is already the deterministic L2 SSOT).
- **Do NOT** introduce a second SQLite database path (`.vox/store.db` must stop receiving CLI proposal writes in favor of the daemon `VoxDb` / IPC proxy).
