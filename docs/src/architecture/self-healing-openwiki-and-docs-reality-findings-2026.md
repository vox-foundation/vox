---
title: "Self-Healing OpenWiki & Documentation Reality Audit Findings (2026)"
description: "Forensic audit of documentation drift, unindexed KB shards, resolver suppression cheats, and mock verifiers across Vox, plus the Milestone v9.1 architecture for AST-grounded NLI verification and polyglot self-healing."
category: "Architecture SSOTs"
status: "research"
topic: "doc-governance"
domain: "documentation"
authorship: "llm-generated"
authorship_model: "claude-opus-4-6"
training_eligible: false
---

# Self-Healing OpenWiki & Documentation Reality Audit Findings (2026)

<!-- kb:summary -->
This forensic audit measures why Vox's documentation database, OpenWiki hubs, and freshness ledgers drifted behind the codebase at commit c2edb14aee6bba41c4bee98005be01cf1f968290. Live inspection reveals an unindexed knowledge base, frozen audit reports, 1,547 reality deficits across 1,098 pages, and eight systemic defects including mocked LLM verification, hardcoded resolver suppression cheats, split-brain CLI versus MCP tooling, and broken CI workflows. It defines the Milestone v9.1 architecture unifying AST-grounded NLI verification, partitioned multi-repo auto-indexing, uncapped polyglot OpenWiki synthesis, and four-gate prose self-healing.
<!-- /kb:summary -->

**Related SSOTs:** [How Vox documentation works](./docs-system-ssot.md) · [Documentation governance](../contributors/documentation-governance.md) · [Universal Multi-Repo KB & OpenWiki Research](./universal-kb-graphify-openwiki-multirepo-research-2026.md)

## 1. Measured Baseline (`HEAD = c2edb14aee6bba41c4bee98005be01cf1f968290`)

Despite strong architectural claims across tutorials and research docs, the live documentation database, OpenWiki generation tree, and drift audit ledgers are either unpopulated or frozen months behind the repository:

| Surface / Artifact | Observed State | Implication |
|---|---|---|
| Daemon KB (`vox kb status`) | `KB Library: not indexed` (`0` entries, `0` inbox findings); `~/Library/Application Support/vox/kb/library` untouched since `2026-10-05` | Agent/GUI KB queries have no fresh local library index |
| Repo-local DB ([`.vox/store.db`](../../../.vox/store.db)) | `schema_version = 99`, `0` rows in `kb_index_state`, `search_documents`, `search_document_chunks`, `kb_findings`, `kb_entries`, `knowledge_nodes`; `kb_heal_proposals` missing | Split-brain CLI proposal DB has never been populated or migrated |
| Global Library (`~/.vox/library/`) | `0` files | Cross-project promoted knowledge layer is empty |
| OpenWiki split-brain (`docs/wiki/` vs. `docs/src/`) | `docs/wiki/` does not exist (`vox ci wiki-sync --status` → `initialized: false`, `6/6` pages missing/stale), while `docs/src/` (`docs-astro` → `voxlang.org`) is the true canonical documentation database | `v7.9` invented a parallel `docs/wiki/` tree instead of wiring OpenWiki self-healing directly onto `docs/src/` (and `<canonical_docs_root>` on external repos) |
| Freshness ledger ([`ledger.v1.json`](../../../contracts/reports/docs-freshness/ledger.v1.json)) | Frozen at `2026-10-14` (`b64377d402`); claims `810` verified, `0` drifting, `0` broken | Weekly refresh workflow failed to keep the ledger current |
| Reality metrics ([`metrics.v1.json`](../../../contracts/reports/docs-reality-audit/metrics.v1.json)) | Frozen at `2026-05-12` (`inventory_claim_count: 10`, `findings_total: 0`) | Reality audit metrics have not updated since May 2026 |
| Live `vox audit docs --prune-report` | `1,098` pages scanned, `38,859` mentions (`28,156` resolved, `149` retired, `1,682` missing, `8,872` ambiguous), `1,547` actionable deficits; `1,208` pages evaluated (`446` keep, `317` archive, `306` movetointernals, `139` merge, `63` clusters) | Severe unaddressed drift even before removing resolver bypasses |

### Representative Untruths in Active `status: "current"` & Reference Docs

- [`tut-knowledgebase-and-openwiki.mdx`](../tutorials/tut-knowledgebase-and-openwiki.mdx#L249-L255) (`status: "current"`): Claims `vox audit docs --verify-llm` and `vox audit docs --self-heal-plan` synthesize unified diffs via `vox_actor_runtime::llm` and stage 4-gate patches into `kb_heal_proposals`. Neither command invokes an LLM, checks source code, or writes `kb_heal_proposals`.
- [`expl-architecture.md`](../explanation/expl-architecture.md) (lines 19–120) (`status: "current"`): Claims the compiler AST and codegen live consolidated inside `crates/vox-compiler/src/` (extracted to `vox-ast`, `vox-codegen`, and `vox-rn-codegen`), references nonexistent `vox-codegen-rust` and `vox-compiler::codegen_rust`, documents retired Burn `model_final.bin` training output, and jumps directly from Stage 3 to Stage 6.
- [`mens-training.md`](../reference/mens-training.md) (lines 29–56): Presents deleted Burn 0.19 / `wgpu` / `burn_hf_load.rs` / `vox-schola serve` / `scripts/check_cuda_feature_builds.sh` paths as active in-tree lanes, and states `--lora-exclude-lm-head` is rejected at runtime even though `v6.9` implemented it.
- [`cli.md`](../reference/cli.md) (lines 18–66): Lists retired subcommands and modules (`vox pm`, `vox codex`, `vox oratio`, `commands::schola`).
- `110+` active docs under `docs/src/` cite deleted crates (`vox-codex`, `vox-arca`, `vox-pm`, `vox-schola`, `vox-oratio`, `vox-ludus`, `vox-toestub`, `vox-compilerd`, `vox-ssg`), including [`001-burn-backend-selection.md`](../adr/001-burn-backend-selection.md) (recorded as `verified` in `ledger.v1.json` despite Burn removal) and root [`AGENTS.md`](../../../AGENTS.md) (referencing `32` `SURFACE_REGISTRY` entries while [`surfaceRegistry.generated.ts`](../../../crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts#L13) defines `33`).

## 2. Eight Root-Cause Failures Preventing Self-Maintenance

1. **Mocked-Out LLM Claim Verifier & Eval Harness:**
   `run_advisory_claim_verification` in [`self_heal_bot.rs`](../../../crates/vox-doc-pipeline/src/claims/self_heal_bot.rs#L257-L341) constructs an empty `symbol_hashes = BTreeMap::new()`, performs zero LLM calls and zero code lookups, stamps every extracted claim as `Verified`, and cites the Markdown file back to itself (`[{rel}:L1-L{line}]`). Meanwhile `evaluate_golden_drift_set` in [`llm_verifier.rs`](../../../crates/vox-doc-pipeline/src/claims/llm_verifier.rs#L302-L382) checks `snippet_stale != snippet_fixed` in a 5× loop to report `100%` synthetic precision for `run_self_heal_bot_check`.
2. **Hardcoded Ground-Truth Suppression Cheats:**
   [`resolver.rs`](../../../crates/vox-doc-pipeline/src/claims/resolver.rs#L101-L161) injects `58` deleted crates (`vox-arca`, `vox-codex`, `vox-schola`, `vox-oratio`, `vox-codegen-rust`, etc.) directly into `workspace_crates`. Furthermore, [`resolver.rs`](../../../crates/vox-doc-pipeline/src/claims/resolver.rs#L411-L590) marks any nonexistent path under `crates/<known>/`, `docs/`, `scripts/`, `examples/`, `contracts/`, or `tests/` as `Resolved` (unless the string contains `"nonexistent"`), accepts any `vox <valid_root> <fake_subcmd>` as `Resolved`, and treats any `crate::deleted_symbol` as `Resolved`.
3. **Blind Drift Gate Scope:**
   `is_reader_facing_page` in [`gate.rs`](../../../crates/vox-doc-pipeline/src/claims/gate.rs#L77-L121) exempts `architecture/`, `adr/`, `contributors/`, `ci/`, `api/`, `index.mdx`, and root agent files. As a result, `vox audit docs --check-drift` prints `PASSED` with `0` baselined deficits ([`baseline.v1.json`](../../../contracts/reports/docs-freshness/baseline.v1.json)) while `1,547` detected deficits remain unresolved.
4. **Broken CI Workflows Locked In by Parity Tests:**
   [`.github/workflows/docs-drift-refresh.yml`](../../../.github/workflows/docs-drift-refresh.yml#L41-L95) invokes `./target/debug/vox audit docs-reality` (a nonexistent subcommand; the real subcommand is `vox audit docs` in [`audit.rs`](../../../crates/vox-cli-research/src/audit.rs#L54)) and stages `contracts/reports/docs-freshness/{findings,metrics}.v1.json` instead of `contracts/reports/docs-reality-audit/`. Those broken strings are enforced by [`docs_system_parity.rs`](../../../crates/vox-cli/src/commands/ci/docs_system_parity.rs#L149) and [`ci_workflow_contract.rs`](../../../crates/vox-cli/tests/ci_workflow_contract.rs#L933). Meanwhile [`.github/workflows/docs-self-heal.yml`](../../../.github/workflows/docs-self-heal.yml#L20-L37) executes only a read-only `cargo test`.
5. **Split-Brain & Stubbed OpenWiki Across CLI vs. MCP/GUI:**
   CLI `vox ci wiki-sync` in [`run_body.rs`](../../../crates/vox-cli/src/commands/ci/run_body.rs#L151-L237) skips polyglot living docs whenever `docs/src/architecture/layers.toml` exists, returns early on `--auto-apply` before `--heal` runs, and writes/reads `.vox/store.db` instead of the daemon `vox.db`. Conversely, MCP/GUI `execute_openwiki_mcp_action` in [`repo_catalog_tools.rs`](../../../crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs#L572-L687) bypasses `vox-doc-pipeline` entirely—generating single-node stub diagrams inside a separate `docs/wiki/` directory rather than keeping Vox's canonical `docs/src/` (`voxlang.org`) Astro documentation database (and `<canonical_docs_root>` on external repos) synchronized in-place, and returning early with no updates once files exist.
6. **Disconnected Tier-2 Prose Synthesis:**
   Nothing routes blast-radius findings or NLI claim failures into `vox_actor_runtime::llm` automatically. [`stage_verified_prose_heal_proposal`](../../../crates/vox-search/src/promotion.rs#L451-L530) is invoked only manually from MCP with `llm_config: None`, only rewrites `<!-- kb:summary -->` or appends a footer bullet, and omits [`verify_tier2_candidate_four_gates`](../../../crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs#L473-L585).
7. **Manual-Only KB Indexing & Vox-Only Manifest Coupling:**
   `vox-orchestrator-d` never triggers background or incremental KB indexing on startup, `HEAD` change, or doc edits. In [`library.rs`](../../../crates/vox-orchestrator-mcp/src/kb_tools/library.rs#L173-L191) and [`library/mod.rs`](../../../crates/vox-search/src/library/mod.rs#L74-L90), `vox_kb_reindex` hardcodes `KbCorpusManifest::vox_default` (requiring `docs/src/`), failing on external repos that use `docs/` or root `README.md`, and reuses a single unpartitioned Tantivy directory.
8. **Silent Polyglot Truncation Caps & Shallow Extraction:**
   [`openwiki_polyglot.rs`](../../../crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs#L15-L16) enforces `MAX_SCAN_FILES = 6_000`, `MAX_ENTRIES_PER_LAYER = 60`, and `.take(40)` on `layers.toml`, silently dropping more than 70% of Vox's workspace crates and scanning source files with line regexes instead of `vox-graph-reader` Tree-sitter/`syn` AST spans.

## 3. Target Self-Healing Architecture (Milestone `v9.1`)

Combining OpenWiki-style living documentation primitives directly over Vox's canonical `docs/src/` (`docs-astro` → `voxlang.org`) Astro database and each external repo's canonical `<docs_root>/`—with AST-coupled anchors (Swimm pattern), hierarchical Leiden/subsystem decomposition (CodeWiki/DeepWiki pattern), and span-grounded NLI entailment over `vox_actor_runtime::llm`—eliminates the separate `docs/wiki/` split-brain using crates already present in the workspace:

```vox-diagram
graph {
    direction: down
    node Diff: "Git Diff + Multi-Repo Discovery"
    node Graph: "vox-graph-reader AST (tree-sitter + syn + Cross-Edges)"
    node Tier1: "Tier-1 Zero-LLM In-Place Auto-Fences & #L Anchors"
    node NLI: "Span-Grounded NLI Verifier (vox_actor_runtime::llm)"
    node Tier2: "Tier-2 Section-Scoped Prose Diff + 4 Gates"
    node Store: "Unified Daemon VoxDb + Partitioned Tantivy KB"
    node Apply: "voxlang.org (docs/src) / Draft PRs / Axis Studio"
    Diff -> Graph: "incremental blake3"
    Graph -> Tier1: "blast radius"
    Graph -> NLI: "resolved AST spans"
    NLI -> Tier2: "drifted claims"
    Tier1 -> Store: "auto-reindex"
    Tier2 -> Store: "kb_heal_proposals"
    Store -> Apply: "verified diffs"
}
```

## 4. Milestone `v9.1` Closure Plan (`Phases 534–540`)

| Phase | Scope | Requirements |
|---|---|---|
| **534** | Remove resolver suppression cheats, expand drift gate across `status: "current"` + agent guides, publish SSOT & honest snapshots | `TRUTH-01..04` |
| **535** | Daemon auto-indexing, `(repository_id, corpus_tier)` Tantivy/DB partitioning, eliminate `.vox/store.db` split-brain | `KB-AUTO-01..04` |
| **536** | Replace MCP OpenWiki stubs, remove 60-entry caps via Leiden/AST hierarchy, unify Tier-1 anchor auto-healing across `docs/src/` & `<docs_root>/` | `WIKI-UNIFY-01..04` |
| **537** | Real AST-grounded `vox_actor_runtime::llm` NLI verifier, 50+ pair eval, 4-gate section/paragraph Tier-2 healing | `LLM-HEAL-01..04` |
| **538** | Repair `docs-drift-refresh.yml` & `docs-self-heal.yml`, pre-push/PR blast-radius checks, `review_by` + `legacy` demotion | `CI-LOOP-01..04` |
| **539** | Remediate Vox flagship/reference untruths, enrich living sections in `docs/src/` (`voxlang.org`), triage superseded ADRs/plans, drive `current` deficits to `0` | `CORPUS-01..04` |
| **540** | Polyglot external-repo E2E proof (RW & `--read-only`), Axis Studio verification, Starlight/CI gates, `/gsd-code-review --fix` | `VERIFY-01..04` |
