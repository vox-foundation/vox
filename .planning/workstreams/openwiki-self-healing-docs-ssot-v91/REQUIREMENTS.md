# Requirements: Vox — Milestone v9.1 (`openwiki-self-healing-docs-ssot-v91`)

**Defined:** 2026-11-19
**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`
**Research SSOT:** [`docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md`](../../../docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md)
**Core Value:** The compiler, orchestrator, runtime, knowledge base, and public documentation site (`voxlang.org`) that everything else depends on must keep building, staying verified, and passing CI from a single unified worktree (`/Users/brbrainerd/dev/vox`) or federated sibling repositories with zero LLM split-brain, while remaining **100% functional offline/standalone** with zero dependency on `vox-ops` and **100% transparent, disableable, strongly-typed telemetry**.

## v9.1 Requirements

Requirements for Milestone `v9.1` (*Honest Docs Reality Audit, Auto-Indexed Multi-Repo KB, Uncapped Polyglot OpenWiki & 4-Gate LLM Self-Healing*). Every requirement maps to exactly one phase in `ROADMAP.md` (`Phases 534–540`).

### 1. Honest Ground-Truth Resolver & Drift Gate (`TRUTH`)

- [x] **TRUTH-01**: Remove the 58 hardcoded deleted-crate aliases from [`crates/vox-doc-pipeline/src/claims/resolver.rs`](../../../crates/vox-doc-pipeline/src/claims/resolver.rs#L101-L161) and migrate retired crates and commands into [`contracts/documentation/retired-symbols.v1.yaml`](../../../contracts/documentation/retired-symbols.v1.yaml) with canonical replacements.
- [x] **TRUTH-02**: Remove the `!target.contains("nonexistent")` path, command, and symbol bypasses in [`crates/vox-doc-pipeline/src/claims/resolver.rs`](../../../crates/vox-doc-pipeline/src/claims/resolver.rs#L411-L590), verifying file paths on disk, full `vox` subcommand paths against `contracts/cli/command-registry.yaml` + compiled clap definitions, and Rust/Vox/TypeScript symbols against `vox-graph-reader`.
- [x] **TRUTH-03**: Expand `evaluate_drift_gate` in [`crates/vox-doc-pipeline/src/claims/gate.rs`](../../../crates/vox-doc-pipeline/src/claims/gate.rs#L77-L121) to cover **all** `status: "current"` pages across `docs/src/**` (including `architecture/`, `adr/`, `contributors/`, `ci/`, `api/`, and `index.mdx`) and root agent guides (`AGENTS.md`, `README.md`) behind a shrink-only `contracts/reports/docs-freshness/baseline.v1.json` ratchet.
- [x] **TRUTH-04**: Update and promote the normative architecture SSOT [`docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md`](../../../docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md) (`status: "current"`) and regenerate honest `contracts/reports/docs-freshness/ledger.v1.json` and `contracts/reports/docs-freshness/{findings,metrics}.v1.json` / `docs-reality-audit/{findings,metrics}.v1.json` snapshots at `HEAD`.

### 2. Incremental Multi-Repo KB Auto-Indexing & Unified `VoxDb` (`KB-AUTO`)

- [x] **KB-AUTO-01**: Switch `vox_kb_reindex`, `vox_kb_search`, `vox_kb_status`, and `vox kb` ([`crates/vox-orchestrator-mcp/src/kb_tools/library.rs`](../../../crates/vox-orchestrator-mcp/src/kb_tools/library.rs#L173-L191)) from `KbCorpusManifest::vox_default` to `KbCorpusManifest::discover_for_repo_with_id` + `index_library_scoped`, preferring canonical `docs/src/` (Astro/Starlight/mdBook like `voxlang.org`) else `docs/` else root Markdown (`AGENTS.md`, `VOX.md`, `README.md`) without double-walking `docs/` or requiring a separate `docs/wiki/` folder, while partitioning Tantivy shards and `kb_index_state` by `(repository_id, corpus_tier)` and preserving `contracts/kb/eval/` BEIR golden compatibility.
- [x] **KB-AUTO-02**: Add non-blocking background KB auto-bootstrap and incremental `blake3` + `HEAD` refresh in `vox-orchestrator-d` on daemon start, active repo switch, `write_doc_markdown`, and heal-proposal application.
- [x] **KB-AUTO-03**: Unify `vox ci wiki-sync` proposal DB access ([`crates/vox-cli/src/commands/ci/run_body.rs`](../../../crates/vox-cli/src/commands/ci/run_body.rs#L151-L199)) onto the canonical daemon `VoxDb` / `daemon_db_proxy` (eliminating the `.vox/store.db` split-brain) and fix `--heal --auto-apply` execution ordering.
- [x] **KB-AUTO-04**: Index discovered canonical docs roots (`docs/src/` on Vox/`voxlang.org`, `docs/` or root guides `AGENTS.md`, `VOX.md`, `README.md` on external repos) and `~/.vox/library/` (`CorpusTier::GlobalLibrary`) with strict per-repo scope isolation (`0` cross-repo leaks when querying `repo_local`).

### 3. Uncapped In-Place Living Code-to-Docs Sync & CLI/MCP/GUI Unification (`WIKI-UNIFY`)

- [ ] **WIKI-UNIFY-01**: Replace the stubbed 1-node writer in [`crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs`](../../../crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs#L572-L687) with the shared `vox-doc-pipeline` AST-to-docs sync + blast-radius engine targeting the repository's canonical docs root (`docs/src/` on Vox, discovered `<docs_root>/` on external repos) across all MCP/GUI `wiki_*` actions without creating a separate `docs/wiki/` silo on Vox.
- [ ] **WIKI-UNIFY-02**: Remove silent `MAX_SCAN_FILES = 6_000`, `MAX_ENTRIES_PER_LAYER = 60`, and `layers.toml` `.take(40)` truncation caps in [`crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs`](../../../crates/vox-doc-pipeline/src/pipeline/openwiki_polyglot.rs#L15-L16) via `ignore::WalkBuilder` and hierarchical Leiden/subsystem grouping for complete monorepo and polyglot coverage.
- [ ] **WIKI-UNIFY-03**: Upgrade polyglot extraction (`Rust`, `TypeScript/JS`, `Python`, `Go`, `Vox`, `SQL`) to use `vox-graph-reader` Tree-sitter/`syn` AST spans, docstrings, HTTP/MCP/CLI routes, and `Code <-> Doc <-> Decision` cross-edges, driving in-place `subsystems/*.generated.md`, `api/*.generated.md`, and `<!-- vox:auto:start -->` / `<!-- openwiki:auto:start -->` sections inside canonical docs (`docs/src/` on Vox, `<docs_root>/` on external repos).
- [ ] **WIKI-UNIFY-04**: Upgrade Tier-1 deterministic auto-heal ([`crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs`](../../../crates/vox-doc-pipeline/src/pipeline/openwiki_heal.rs#L263-L421)) to repair shifted `#Lstart-Lend` anchors, symbol signature updates, and deterministic auto-fences in-place across `docs/src/` (and `<docs_root>/` on external repos) with `0` LLM calls.

### 4. Model-Agnostic LLM Claim NLI Verifier & Section-Scoped Tier-2 Self-Heal (`LLM-HEAL`)

- [ ] **LLM-HEAL-01**: Replace the mock `run_advisory_claim_verification` ([`crates/vox-doc-pipeline/src/claims/self_heal_bot.rs`](../../../crates/vox-doc-pipeline/src/claims/self_heal_bot.rs#L257-L341)) with real AST-span-grounded NLI entailment verification via `vox_actor_runtime::llm`, cached by `(claim_hash, blake3_symbol_span_hash)` under `DocVerifyBudget` and strictly forbidding doc-to-self citations.
- [ ] **LLM-HEAL-02**: Replace the string-inequality `evaluate_golden_drift_set` ([`crates/vox-doc-pipeline/src/claims/llm_verifier.rs`](../../../crates/vox-doc-pipeline/src/claims/llm_verifier.rs#L302-L382)) with a real `>= 50`-pair on-disk code-anchor benchmark (`precision >= 0.80`, `false_positive_rate <= 0.10`) plus deterministic AST/contract fallback when offline.
- [ ] **LLM-HEAL-03**: Upgrade Tier-2 prose synthesis ([`crates/vox-search/src/promotion.rs`](../../../crates/vox-search/src/promotion.rs#L451-L530)) from summary-only/footer-append edits to surgical section- and paragraph-scoped unified diffs triggered automatically from `Drifted` NLI verdicts and blast-radius reality deficits.
- [ ] **LLM-HEAL-04**: Enforce all four deterministic gates (`verify_tier2_candidate_four_gates`: `kb_rewrite_guard` + on-disk line-range code/contract citations, `vox-doc-pipeline --lint-only`, `vox-diagram` check, `.vox` doctests) plus NLI entailment verification before staging into `kb_heal_proposals` or applying under `--auto-apply`.

### 5. Autonomous CI Drift Refresh, PR Blast-Radius & Self-Heal Verification Loop (`CI-LOOP`)

- [ ] **CI-LOOP-01**: Fix [`.github/workflows/docs-drift-refresh.yml`](../../../.github/workflows/docs-drift-refresh.yml#L41-L95) to invoke `vox audit docs --update-ledger --write-findings` and stage canonical `contracts/reports/docs-freshness/{ledger,findings,metrics}.v1.json` + Tier-1 healed `docs/src/**` inside guarded `--draft` PRs (never calling `pr merge` or `--auto`), updating [`docs_system_parity.rs`](../../../crates/vox-cli/src/commands/ci/docs_system_parity.rs#L149) and [`ci_workflow_contract.rs`](../../../crates/vox-cli/tests/ci_workflow_contract.rs#L933) in lockstep.
- [ ] **CI-LOOP-02**: Upgrade [`.github/workflows/docs-self-heal.yml`](../../../.github/workflows/docs-self-heal.yml#L20-L37) (honoring `ci_workflow_contract.rs` strictly read-only `contents: read` constraint) to execute live OpenWiki AST/NLI claim verification, golden drift eval, and `BotMetricsLedger` checks (`auto_pause < 0.80` precision), while routing automated `--draft` PR staging (`<= 3` branches) through `docs-drift-refresh.yml` / `scripts/docs-fix-drafts.vox`.
- [ ] **CI-LOOP-03**: Wire zero-LLM `git diff` AST blast-radius & anchor repair checks into `vox ci ssot-drift` / pre-push and PR impact reporting so source edits auto-heal deterministic fences locally and flag impacted narrative prose before merge.
- [ ] **CI-LOOP-04**: Wire hash-jittered `review_by` + usage-weighted verification queues (`crates/vox-search/src/kb_staleness.rs`) and 1-click/batch `status: "legacy"` + `superseded_by` demotion across CLI, MCP, and Axis `OpenWikiHealStudio`.

### 6. Vox `docs/src/` (`voxlang.org`) Corpus Truth Sweep & Superseded Doc Remediation (`CORPUS`)

- [ ] **CORPUS-01**: Remediate factual untruths across flagship reader-facing and SSOT docs (`tut-knowledgebase-and-openwiki.mdx`, `expl-architecture.md`, `expl-compiler-lowering.md`, `mens-training.md`, `mens-serving-ssot.md`, `cli.md`, `oratio-speech.md`, `codex-http-api.md`, `populi.md`, `AGENTS.md`, `docs-system-ssot.md`), aligning `tut-knowledgebase-and-openwiki.mdx` and `docs-system-ssot.md` on in-place `docs/src/` (`voxlang.org`) living docs rather than a separate `docs/wiki/` folder.
- [ ] **CORPUS-02**: Materialize and enrich in-place living OpenWiki-style auto-maintained architecture/interface/schema/surface/workflow sections (`<!-- vox:auto:start -->` / `subsystems/*.generated.md` / `api/*.generated.md`) directly inside `docs/src/` (`voxlang.org` Astro database) from live ASTs and `docs/src/architecture/layers.toml` with zero separate `docs/wiki/` directory.
- [ ] **CORPUS-03**: Triage and remediate Vox's `1,547` reality deficits and `317` archive / `139` merge candidates — marking superseded ADRs (`001`, `004`, `006`, `007`, `015`, `037`, `048`, and siblings) as `status: "legacy"` / `"deprecated"` with `superseded_by`, archiving obsolete point-in-time plans into `docs/src/archive/`, and fixing active `status: "current"` references.
- [ ] **CORPUS-04**: Re-index the Vox KB (`vox kb reindex`), tighten `baseline.v1.json`, `corpus-ratchet.v1.json`, `kb-facet-backlog.v1.yaml`, `ledger.v1.json`, and `docs-freshness/{findings,metrics}.v1.json`, achieving `0` unsuppressed deficits across all `status: "current"` pages.

### 7. Cross-Repo E2E Proof, Axis Studio Verification & Closure (`VERIFY`)

- [ ] **VERIFY-01**: Run hermetic and live polyglot external-repo & federated sibling-repo proof (`Rust + TypeScript + Python + Go` without `docs/src/` or `layers.toml`, in both read-write against `<docs_root>/` and `--read-only` modes) verifying onboarding → KB auto-index → in-place 5-layer living docs → code mutation → Tier-1 + Tier-2 4-gate LLM heal → retrieval update with `0` cross-repo leakage.
- [ ] **VERIFY-02**: Verify Vox Axis `OpenWikiHealStudio` (with 1-click Tier-2 `Scan & Propose Heals` action), `DocReader`, `MemoryView`, and `RepositoryCatalogPanel` end-to-end via Vitest and Playwright (`0` mocks, `0` direct `VoxDb` leaks).
- [ ] **VERIFY-03**: Pass all documentation site and tutorial gates (`tutorial-verify.vox`, `vox-doc-pipeline --lint-only`, `vox ci doctest-md --strict`, `vox kb eval`, `pnpm --dir docs-astro build`, `node:test`, Playwright `tests/build-output`).
- [ ] **VERIFY-04**: Pass all workspace guards and `/gsd-code-review --fix` (`vox audit arch`, `vox audit code`, `vox audit docs --check-drift`, `vox ci ssot-drift`) and publish `contracts/reports/v91-self-healing-openwiki-audit-proof.v1.json`.

## Future Requirements (`v9.2+`)

- **WIKI-CRDT-01**: Real-time multi-author CRDT wiki editing and team annotations synced across federated Populi nodes.
- **GRAPH-REMOTE-01**: Pre-built `.graph.zst` artifact streaming across Populi mesh nodes for multi-million-LoC monorepos.

## Out of Scope

| Feature | Reason |
|---|---|
| Direct vendor HTTP SDK calls bypassing `vox_actor_runtime::llm` | Violates `AGENTS.md §Model-Agnostic LLM Boundary` (`llm_provider_call` hard error) |
| Direct `VoxDb` handles or SQLite opens inside `vox-gui` | Violates Axis IPC boundary (`voxTransport` -> `vox-orchestrator-d`) |
| Unverified LLM overwrites outside `openwiki:auto` fences without 4 deterministic gates | Risks silent doc hallucination and loss of human engineering rationale |
| Adding new third-party Rust crates to `Cargo.lock` or npm packages to `package.json` | All needed parsers, graph algorithms, Tantivy, and `vox-diagram` already ship in-tree |

## Traceability

| Requirement | Phase | Status |
|---|---|---|
| TRUTH-01 | Phase 534 | Complete |
| TRUTH-02 | Phase 534 | Complete |
| TRUTH-03 | Phase 534 | Complete |
| TRUTH-04 | Phase 534 | Complete |
| KB-AUTO-01 | Phase 535 | Complete |
| KB-AUTO-02 | Phase 535 | Complete |
| KB-AUTO-03 | Phase 535 | Complete |
| KB-AUTO-04 | Phase 535 | Complete |
| WIKI-UNIFY-01 | Phase 536 | Complete |
| WIKI-UNIFY-02 | Phase 536 | Complete (module rollups, not Leiden) |
| WIKI-UNIFY-03 | Phase 536 | Partial (SQL FK via graph-reader; other stacks keep in-crate extractors; carried to Phase 539/540) |
| WIKI-UNIFY-04 | Phase 536 | Complete |
| LLM-HEAL-01 | Phase 537 | In progress (grounded offline verifier + doc-self rejection done; LLM judge pending) |
| LLM-HEAL-02 | Phase 537 | Pending |
| LLM-HEAL-03 | Phase 537 | Pending |
| LLM-HEAL-04 | Phase 537 | Pending |
| CI-LOOP-01 | Phase 538 | Pending |
| CI-LOOP-02 | Phase 538 | Pending |
| CI-LOOP-03 | Phase 538 | Pending |
| CI-LOOP-04 | Phase 538 | Pending |
| CORPUS-01 | Phase 539 | Pending |
| CORPUS-02 | Phase 539 | Pending |
| CORPUS-03 | Phase 539 | Pending |
| CORPUS-04 | Phase 539 | Pending |
| VERIFY-01 | Phase 540 | Pending |
| VERIFY-02 | Phase 540 | Pending |
| VERIFY-03 | Phase 540 | Pending |
| VERIFY-04 | Phase 540 | Pending |

**Coverage:**
- v9.1 requirements: 28 total
- Mapped to phases: 28 (`Phases 534–540`)
- Unmapped: 0 ✓

---
*Requirements defined: 2026-11-19*
*Last updated: 2026-11-19 after initial v9.1 definition*
