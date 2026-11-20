# Feature Landscape: Self-Healing OpenWiki & Continuous Docs Sync (`v9.1`)

**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`  
**Researched:** 2026-11-19

## 1. Table Stakes (Required for Baseline Honesty & Functionality)

| Capability | Current Defect | Target Behavior |
|---|---|---|
| **Honest Ground-Truth Resolution** | `resolver.rs` hardcodes 58 deleted crate aliases and passes any `crates/<known>/...`, `docs/...`, `vox <root> <fake>`, or `crate::foo` claim | Every path checked on disk, every crate checked against Cargo workspace/catalog, every CLI invocation checked against `command-registry.yaml` + clap, retired symbols tracked in `retired-symbols.v1.yaml` |
| **Complete Drift Gating** | `gate.rs` exempts `architecture/`, `adr/`, `contributors/`, `ci/`, `api/`, `index.mdx`, and root guides (`1,547` deficits pass gate) | All `status: "current"` pages and root guides (`AGENTS.md`, `README.md`) gated behind shrink-only `baseline.v1.json` |
| **Automatic Multi-Repo KB Indexing** | `vox kb status` reports `not indexed`; `vox_kb_reindex` fails on repos lacking `docs/src/` | Daemon auto-indexes/refreshes on startup, `HEAD` advance, doc writes, and repo switch; `index_library_scoped` discovers `docs/src` (Astro/Starlight like `voxlang.org`), else `docs/`, else root Markdown (`AGENTS.md`, `VOX.md`, `README.md`) per `(repository_id, corpus_tier)` |
| **Unified CLI / MCP / GUI Living Docs** | CLI skips polyglot synthesis when `layers.toml` exists; MCP writes 1-node stub diagrams in `docs/wiki/` and no-ops afterward | Single `vox-doc-pipeline` engine targets `docs/src/` on Vox (`voxlang.org`) and `<canonical_docs_root>/` on external repos across `vox ci wiki-sync`, `vox repo --wiki`, MCP `vox_repo_catalog_refresh`, and Axis `OpenWikiHealStudio` |
| **Working CI Drift & Self-Heal Workflows** | `docs-drift-refresh.yml` calls nonexistent `vox audit docs-reality` & wrong JSON paths; `docs-self-heal.yml` only runs `cargo test` | Green weekly/nightly workflows refreshing `docs-freshness/{ledger,findings,metrics}.v1.json` (plus read-only NLI/golden checks in `docs-self-heal.yml` and `--draft` PR staging in `docs-drift-refresh.yml`) |

## 2. Differentiators (SOTA Patterns Adapted to Vox)

1. **Swimm-Style AST Symbol Coupling (`#Lstart-Lend` + `blake3` Body Hashes):**
   - Bind documentation anchors to AST symbol identities (`crate::mod::fn`, TS exports, Python classes/defs, SQL tables, `.vox` actors/workflows/tables) in `vox-graph-reader`.
   - When lines shift, signatures change, or symbols rename within a file, Tier-1 zero-LLM auto-heal repairs anchors and `openwiki:auto` blocks cleanly.
2. **CodeWiki / DeepWiki Hierarchical Rollup (No Silent Truncation):**
   - Replace `openwiki_polyglot`'s `60`-entry and `40`-crate caps with full workspace enumeration organized by `layers.toml` (when present) or Leiden community clusters + directory/package hierarchy (for arbitrary external repos).
   - Emit compact summary hubs linking per-subsystem/per-community anchor tables so large monorepos (133+ crates) and tiny libraries both get 100% coverage without bloated single pages.
3. **Span-Grounded NLI Claim Verification (`vox_actor_runtime::llm`):**
   - Extract factual claims + cited symbols/CLI commands/config keys from Markdown paragraphs.
   - Retrieve exact AST slices + `git diff` hunks + ground-truth registry definitions; classify `Verified | Drifted | Unanchored` with mandatory on-disk `[path:Lstart-Lend]` citations (never self-citing the doc itself).
   - Cache results by `(claim_hash, symbol_span_hash)` under `DocVerifyBudget` (`max_claims_per_run`, `max_tokens_per_run`).
4. **Tier-2 Surgical Section/Paragraph Diffs + Four Deterministic Gates:**
   - Synthesize minimal unified diffs targeting the exact stale section or paragraph (not only `<!-- kb:summary -->` or footer notes).
   - Gate every candidate through: (1) `kb_rewrite_guard` + on-disk citation existence/line bounds + NLI entailment, (2) `vox-doc-pipeline --lint-only` (frontmatter + SKOS facets + summary), (3) `vox-diagram` validation (`0` legacy Mermaid), and (4) `.vox` doctest compilation.
   - Stage passing diffs in daemon `kb_heal_proposals` for 1-click apply in Axis `OpenWikiHealStudio`, CLI `--auto-apply`, or non-auto-merged CI Draft PRs.
5. **Lifecycle Staleness & Honest Demotion (`review_by` + `legacy`):**
   - Prioritize verification queues by `usage * days_overdue` (`kb_staleness.rs`).
   - Automatically propose or execute `status: "legacy"` + `superseded_by` demotion for historical/superseded ADRs and archive plans rather than pretending deleted subsystems still exist.

## 3. Anti-Features (Explicitly Rejected)

- **Silent Raw LLM Overwrites of Human Prose:** Unverified or uncited rewrites outside `openwiki:auto` fences corrupt architectural intent ("why") and introduce hallucinations.
- **Vendor-Locked LLM Calls:** Calling OpenAI/Anthropic/Gemini HTTP APIs directly outside `vox_actor_runtime::llm` violates `AGENTS.md` and fails `vox audit code`.
- **Ignoring External Repo Layouts:** Assuming `docs/src/` or `layers.toml` exists breaks universal multi-repo usability.
- **Synthetic Test Pass Masks:** Any hardcoded alias lists, prefix-match shortcuts, or string-inequality benchmark mocks that hide real drift.
