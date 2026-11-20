# Roadmap: Milestone v9.1 (`openwiki-self-healing-docs-ssot-v91`) — Honest Docs Reality Audit, Auto-Indexed Multi-Repo KB, Uncapped Polyglot OpenWiki & 4-Gate LLM Self-Healing

**Workstream:** `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/`  
**Research SSOT:** [`docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md`](../../../docs/src/architecture/self-healing-openwiki-and-docs-reality-findings-2026.md)  
**Requirements:** [`REQUIREMENTS.md`](REQUIREMENTS.md) (`28` requirements across `Phases 534–540`)

**Scope:** Audit and eliminate every false-green drift mask, mock LLM verifier, split-brain database path, and truncation cap that allowed Vox's canonical `docs/src/` (`voxlang.org` Astro database) and searchable `VoxDb`/Tantivy KB to lag behind the codebase. Deliver OpenWiki-style living documentation features directly in-place over `docs/src/` (and each external repo's canonical `<docs_root>/`) with **no separate `docs/wiki/` folder on Vox**: remove resolver suppression cheats and expand `evaluate_drift_gate` across all `status: "current"` docs behind a shrink-only ratchet (`TRUTH-01..04`); make KB indexing incremental and automatic in `vox-orchestrator-d` across partitioned `(repository_id, corpus_tier)` Tantivy/DB shards for any repo layout while eliminating `.vox/store.db` split-brain (`KB-AUTO-01..04`); replace MCP stubs and 60-entry caps with `vox-graph-reader` Tree-sitter/`syn` + Leiden hierarchical in-place synthesis and unified Tier-1 AST anchor repair across `docs/src/` (`WIKI-UNIFY-01..04`); replace the mocked claim verifier and string-diff golden benchmark with AST-span-grounded `vox_actor_runtime::llm` NLI verification and 4-gate section/paragraph Tier-2 self-healing (`LLM-HEAL-01..04`, Tier-1 deterministic auto-write + Tier-2 diffs staged to `kb_heal_proposals` / Draft PRs with optional `--auto-apply`); repair CI refresh & read-only NLI verification workflows plus `review_by` staleness/demotion (`CI-LOOP-01..04`); remediate Vox's own untruths, enrich living auto-maintained sections directly in `docs/src/`, triage superseded ADRs/archive candidates (`CORPUS-01..04`); and prove zero-touch polyglot multi-repo self-healing, Axis `OpenWikiHealStudio` wiring, Starlight/Playwright gates, and `/gsd-code-review --fix` (`VERIFY-01..04`). Zero new crates in `Cargo.lock`, zero new npm packages in `package.json`, full `layers.toml` compliance, and 100% model-agnostic routing via `vox_actor_runtime::llm`.

## Phase Checklist

- [x] **Phase 534: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate** (`TRUTH-01..04`)
- [x] **Phase 535: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon `VoxDb`** (`KB-AUTO-01..04`)
- [x] **Phase 536: Uncapped In-Place Living Code-to-Docs Sync & Unified CLI/MCP/GUI Engine** (`WIKI-UNIFY-01..04`; `-03` partial, carried forward)
- [ ] **Phase 537: Model-Agnostic LLM Claim NLI Verifier & Section-Scoped Tier-2 Self-Heal** (`LLM-HEAL-01..04`)
- [ ] **Phase 538: Autonomous CI Drift Refresh, PR Blast-Radius & Self-Heal Verification Loop** (`CI-LOOP-01..04`)
- [ ] **Phase 539: Vox `docs/src/` (`voxlang.org`) Truth Sweep, Living Docs Enrichment & Superseded Doc Remediation** (`CORPUS-01..04`)
- [ ] **Phase 540: Polyglot External-Repo E2E Proof, Axis Studio Verification & Full-Stack Closure** (`VERIFY-01..04`)

---

### Phase 534: Honest Ground-Truth Resolver, Unmasked Reality Audit & Expanded Drift Gate

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/534-honest-ground-truth-resolver-and-drift-gate/`
**Goal**: Eliminate every hardcoded suppression cheat in `GroundTruthRegistry`, verify file paths, CLI commands, and AST symbols honestly, expand `evaluate_drift_gate` across all `status: "current"` docs and root agent guides behind a shrink-only baseline ratchet, and publish honest `HEAD` ledgers alongside the normative SSOT.
**Depends on**: None (continues sequentially after active workstream `v9.0` `Phase 533`)
**Requirements**: TRUTH-01, TRUTH-02, TRUTH-03, TRUTH-04
**Success Criteria** (what must be TRUE):

  1. `crates/vox-doc-pipeline/src/claims/resolver.rs` contains zero hardcoded deleted-crate aliases and zero `!target.contains("nonexistent")` path/command/symbol bypasses; all retired crates and commands resolve against `contracts/documentation/retired-symbols.v1.yaml` with canonical replacements.
  2. `GroundTruthRegistry` verifies file paths on disk, validates full `vox` subcommands against `contracts/cli/command-registry.yaml` + compiled clap paths, and checks Rust/Vox/TypeScript symbol citations against `vox-graph-reader`.
  3. `evaluate_drift_gate` (`vox audit docs --check-drift`) evaluates all `status: "current"` pages across `docs/src/**` (`architecture/`, `adr/`, `contributors/`, `ci/`, `api/`, `index.mdx`, etc.) and root `AGENTS.md`/`README.md` behind a shrink-only `baseline.v1.json` ratchet so no new deficit can pass silently.
  4. `docs/src/architecture/openwiki-self-healing-docs-ssot-2026.md` (`status: "current"`) passes `vox-doc-pipeline --lint-only`, and `ledger.v1.json` + `docs-freshness/{findings,metrics}.v1.json` reflect unmasked `HEAD` reality.

**Plans**: `534-PLAN.md` (completed & verified: `534-VERIFICATION.md`, `534-SUMMARY.md`)

### Phase 535: Incremental Multi-Repo KB Auto-Indexing & Unified Daemon `VoxDb`

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/535-incremental-multirepo-kb-auto-indexing-and-db-unification/`
**Goal**: Make KB indexing automatic, incremental, and repo-agnostic inside `vox-orchestrator-d` across partitioned `(repository_id, corpus_tier)` Tantivy/DB shards while eliminating the `.vox/store.db` split-brain across CLI, MCP, and GUI.
**Depends on**: Phase 534
**Requirements**: KB-AUTO-01, KB-AUTO-02, KB-AUTO-03, KB-AUTO-04
**Success Criteria** (what must be TRUE):

  1. `vox_kb_reindex`, `vox_kb_search`, `vox_kb_status`, and `vox kb` use `KbCorpusManifest::discover_for_repo_with_id` + `index_library_scoped` partitioned by `(repository_id, corpus_tier)` (preferring `docs/src/` else `docs/` else root Markdown, without double-walking `docs/` or requiring `docs/wiki/`) so external repos without `docs/src/` index cleanly while Vox's `contracts/kb/eval/` BEIR suite remains green.
  2. `vox-orchestrator-d` runs non-blocking background KB bootstrap and incremental `blake3` + `HEAD` reindexing on startup, active repository switch, `write_doc_markdown`, and heal-proposal apply so `vox kb status` never sits unindexed on an active workspace.
  3. `vox ci wiki-sync` proposal reads/writes (`--apply-proposal`, `--reject-proposal`, `--auto-apply`) route through canonical `VoxDb` / `daemon_db_proxy` (zero `.vox/store.db` split-brain), and `--heal --auto-apply` executes Tier-1 heal before applying staged Tier-2 proposals.
  4. Canonical docs (`docs/src/` on Vox/`voxlang.org`, `<docs_root>/` + `AGENTS.md`/`VOX.md`/`README.md` on external repos) and `~/.vox/library/` index into distinct corpus tiers with strict `0` cross-repo result leakage in `repo_local` queries.

**Plans**: `535-PLAN.md` (completed & verified: `535-VERIFICATION.md`, `535-SUMMARY.md`)

### Phase 536: Uncapped In-Place Living Code-to-Docs Sync & Unified CLI/MCP/GUI Engine

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/536-uncapped-polyglot-openwiki-and-tier1-ast-heal/`
**Goal**: Eliminate the stubbed MCP OpenWiki path and silent `60`-entry / `40`-crate caps, upgrade polyglot extraction to full `vox-graph-reader` Tree-sitter/`syn` + Leiden hierarchical synthesis targeting canonical `docs/src/` (and `<docs_root>/` on external repos) in-place, and unify Tier-1 zero-LLM `#Lstart-Lend` repair across CLI, MCP, and GUI.
**Depends on**: Phase 535
**Requirements**: WIKI-UNIFY-01, WIKI-UNIFY-02, WIKI-UNIFY-03, WIKI-UNIFY-04
**Success Criteria** (what must be TRUE):

  1. `crates/vox-orchestrator-mcp/src/repo_catalog_tools.rs` delegates all `wiki_init`, `wiki_build`, `wiki_status`, `wiki_diff`, `wiki_sync`, and `wiki_heal_tier1` actions to the shared `vox-doc-pipeline` engine targeting `docs/src/` on Vox (or discovered `<docs_root>/` on external repos) with zero 1-node placeholder stub generator and zero separate `docs/wiki/` folder created on Vox.
  2. `openwiki_polyglot.rs` removes `6,000`-file, `60`-entry, and `40`-crate truncation caps in favor of `ignore::WalkBuilder` + hierarchical Leiden/subsystem rollups covering 100% of modules across large monorepos and small external projects.
  3. Polyglot extraction consumes `vox-graph-reader` AST spans, docstrings, HTTP/MCP/CLI routes, SQL DDL, and `Code <-> Doc <-> Decision` cross-edges across Rust, TS/JS, Python, Go, Vox, and SQL, driving in-place `subsystems/*.generated.md`, `api/*.generated.md`, and deterministic `<!-- vox:auto:start -->` / `<!-- openwiki:auto:start -->` regions inside `docs/src/` (and `<docs_root>/` on external repos).
  4. Tier-1 deterministic healing (`vox ci wiki-sync --heal` / `wiki_heal_tier1`) automatically updates shifted `#Lstart-Lend` anchors, symbol signatures, and auto-fences in-place across `docs/src/` (and `<docs_root>/` on external repos) with `0` LLM calls and zero disturbance to surrounding human prose.

**Plans**: `536-VERIFICATION.md` / `536-SUMMARY.md` written — PARTIAL (`WIKI-UNIFY-03` open; see verification report)

### Phase 537: Model-Agnostic LLM Claim NLI Verifier & Section-Scoped Tier-2 Self-Heal

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/537-llm-nli-claim-verifier-and-tier2-prose-heal/`
**Goal**: Replace mocked verification and string-diff golden benchmarks with real AST-span-grounded `vox_actor_runtime::llm` NLI claim auditing and 4-gate section/paragraph Tier-2 prose diff synthesis staged in `kb_heal_proposals`.
**Depends on**: Phase 536
**Requirements**: LLM-HEAL-01, LLM-HEAL-02, LLM-HEAL-03, LLM-HEAL-04
**Success Criteria** (what must be TRUE):

  1. `run_advisory_claim_verification` resolves real `vox-graph-reader` AST symbol spans + hashes, calls `vox_actor_runtime::llm` (with deterministic offline fallback) under `DocVerifyBudget`, caches verdicts by `(claim_hash, symbol_span_hash)`, and rejects any doc-to-self citation.
  2. `evaluate_golden_drift_set` scores `>= 50` real code-anchor vs. claim pairs on disk (`precision >= 0.80`, `false_positive_rate <= 0.10`) without synthetic loop multiplication or string-inequality cheats.
  3. `stage_verified_prose_heal_proposal` and `vox audit docs --self-heal-plan` automatically synthesize surgical section- and paragraph-scoped unified diffs for `Drifted` NLI verdicts and reality deficits (not just `kb:summary` rewrites or footer bullets).
  4. Every Tier-2 candidate passes all four deterministic gates (`verify_tier2_candidate_four_gates`: `kb_rewrite_guard` with citation-justified numeric replacement + on-disk `[path:Lstart-Lend]` checks, `vox-doc-pipeline --lint-only`, `vox-diagram` validation, `.vox` doctest check) plus NLI entailment before persisting in `kb_heal_proposals` or applying via `--auto-apply`.

**Plans**: TBD

### Phase 538: Autonomous CI Drift Refresh, PR Blast-Radius & Self-Heal Verification Loop

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/538-autonomous-ci-workflows-and-staleness-lifecycle/`
**Goal**: Repair broken CI documentation workflows (respecting `ci_workflow_contract.rs` read/write split), wire zero-LLM AST blast-radius checks into pre-push and PR validation, and automate `review_by` staleness prioritization and `legacy` demotion lifecycles.
**Depends on**: Phase 537
**Requirements**: CI-LOOP-01, CI-LOOP-02, CI-LOOP-03, CI-LOOP-04
**Success Criteria** (what must be TRUE):

  1. `.github/workflows/docs-drift-refresh.yml` runs valid `vox audit docs --update-ledger --write-findings` and stages canonical `contracts/reports/docs-freshness/{ledger,findings,metrics}.v1.json` + Tier-1 auto-healed `docs/src/**` inside guarded `--draft` PRs (no `pr merge` / `--auto`), with `docs_system_parity.rs` and `ci_workflow_contract.rs` passing cleanly.
  2. `.github/workflows/docs-self-heal.yml` (honoring `ci_workflow_contract.rs` `contents: read` constraint) runs live OpenWiki AST/NLI claim verification, golden drift eval, and `BotMetricsLedger` checks (`auto_pause < 0.80` precision), while `docs-drift-refresh.yml` / `scripts/docs-fix-drafts.vox` owns write-side `--draft` PR staging (`<= 3` branches).
  3. `vox ci ssot-drift` / pre-push and `vox audit docs --pr-impact` trace `git diff` AST blast radiuses to auto-heal deterministic fences locally and emit actionable impacted-prose alerts before merge.
  4. `kb_staleness` (`review_by` + `usage * days_overdue` prioritization) and `status: "legacy"` (`superseded_by`) demotion run cleanly from CLI, MCP, and `OpenWikiHealStudio`.

**Plans**: TBD

### Phase 539: Vox `docs/src/` (`voxlang.org`) Truth Sweep, Living Docs Enrichment & Superseded Doc Remediation

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/539-vox-corpus-truth-sweep-and-wiki-bootstrap/`
**Goal**: Eliminate untruths across Vox's flagship tutorials, references, explanations, and SSOTs, materialize living AST-coupled OpenWiki sections directly inside `docs/src/` (`voxlang.org`), triage superseded ADRs and archive candidates, and burn down `status: "current"` deficits to `0`.
**Depends on**: Phase 538
**Requirements**: CORPUS-01, CORPUS-02, CORPUS-03, CORPUS-04
**Success Criteria** (what must be TRUE):

  1. Flagship and high-traffic active docs (`tut-knowledgebase-and-openwiki.mdx`, `expl-architecture.md`, `expl-compiler-lowering.md`, `mens-training.md`, `mens-serving-ssot.md`, `cli.md`, `oratio-speech.md`, `codex-http-api.md`, `populi.md`, `AGENTS.md`, `docs-system-ssot.md`) match shipped code with zero phantom crates, commands, mock pipeline claims, or split-brain `docs/wiki/` dogma.
  2. Living AST-coupled architecture, interface, schema, GUI surface, and workflow auto-maintained sections (`subsystems/*.generated.md`, `api/*.generated.md`, and `<!-- vox:auto:start -->` / `<!-- openwiki:auto:start -->` blocks) live directly inside `docs/src/` (`voxlang.org` Astro database) with zero separate `docs/wiki/` directory created on Vox.
  3. Superseded ADRs (`001`, `004`, `006`, `007`, `015`, `037`, `048`, and siblings) carry accurate `status: "legacy"` / `"deprecated"` + `superseded_by` metadata, obsolete point-in-time planning docs are archived cleanly into `docs/src/archive/`, and `self-healing-openwiki-and-docs-reality-findings-2026.md` + `docs-system-ssot.md` are synced.
  4. Live `vox kb reindex` succeeds across Vox corpora, and tightened `baseline.v1.json`, `corpus-ratchet.v1.json`, `kb-facet-backlog.v1.yaml`, `ledger.v1.json`, and `docs-freshness/{findings,metrics}.v1.json` yield `0` unsuppressed deficits on all `status: "current"` pages.

**Plans**: TBD

### Phase 540: Polyglot External-Repo E2E Proof, Axis Studio Verification & Full-Stack Closure

**Directory**: `.planning/workstreams/openwiki-self-healing-docs-ssot-v91/phases/540-polyglot-e2e-proof-and-full-stack-closure/`
**Goal**: Prove seamless zero-touch operation on external polyglot repositories (read-write against `<docs_root>/` and `--read-only`), verify Axis `OpenWikiHealStudio` + `DocReader` + `MemoryView` + `RepositoryCatalogPanel`, pass all Starlight/CI gates, and close `/gsd-code-review --fix`.
**Depends on**: Phase 539
**Requirements**: VERIFY-01, VERIFY-02, VERIFY-03, VERIFY-04
**Success Criteria** (what must be TRUE):

  1. Hermetic and live E2E integration tests onboard an external polyglot repo (`Rust + TS + Python + Go`, zero `docs/src/` or `layers.toml`) in both read-write (`<docs_root>/`) and `--read-only` modes, build its KB + in-place 5-layer living docs, mutate source signatures/lines/behavior, execute Tier-1 auto-heal + Tier-2 4-gate LLM prose heal, verify immediate KB retrieval updates, and assert `0` cross-repo leakage.
  2. Axis GUI (`OpenWikiHealStudio` including `Scan & Propose Heals`, `DocReader`, `MemoryView`, `RepositoryCatalogPanel`) passes Vitest and Playwright verification with zero mocks and zero `VoxDb` IPC boundary leaks.
  3. `tutorial-verify.vox`, `vox-doc-pipeline --lint-only`, `vox ci doctest-md --strict`, `vox kb eval`, `pnpm --dir docs-astro build`, `node:test`, and Playwright `tests/build-output` all pass green with `0` broken links or `vox-diagram` errors.
  4. `vox audit arch`, `vox audit code`, `vox audit docs --check-drift`, `vox ci ssot-drift`, and `/gsd-code-review --fix` pass cleanly, producing `contracts/reports/v91-self-healing-openwiki-audit-proof.v1.json`.

**Plans**: TBD
