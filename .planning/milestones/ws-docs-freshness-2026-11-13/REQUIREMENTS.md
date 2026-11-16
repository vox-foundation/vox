# Requirements — v1.4 OpenWiki-Style Self-Organizing & Self-Healing Public Docs (workstream `docs-freshness`)

**Goal:** Turn `voxlang.org` into a self-organizing, continuously verified knowledge base that imports the best properties of OpenWiki/DeepWiki in-house (hierarchical subsystem maps from the code graph, deep symbol-level crate pages, auto-generated Mermaid dependency/call diagrams, calibrated trust signals, systematic debt burn-down, and a nightly DocPrism-style LLM claim verifier + draft-PR bot) with zero external subscription and zero new crate edges.

Brief: `CONTEXT.md`. Research: `research/SUMMARY.md`, `docs/src/architecture/docs-bot-evaluation-and-pivot-rule-2026.md` (EVAL-01), `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json` (EVAL-02).

## Decisions (user, 2026-10-08 through 2026-10-13)

- D1–D12 (v1.3): Separate GSD workstream `docs-freshness`; Starlight kept; draft PRs only (never auto-merge prose); case-by-case pruning; Internals sidebar isolation; tighten-only baseline ratchet; no LLM on PRs; executable tutorial tests (`TUT-02`); collapsible reader layout (`Phase 20.1`).
- D13 (2026-10-13, user): Milestone v1.4 in `docs-freshness` includes all five pillars (`SIGNAL`, `WIKI`, `DEBT`, `VERIFY`, `BOT`) to import OpenWiki-style self-organization and continuous self-healing without subscribing to external SaaS tools.
- D14 (2026-10-13, user): Avoid new crate dependency edges — implement LLM claim verification inside existing `vox-cli` / `vox-doc-pipeline` crates, routing through `vox_actor_runtime::llm` under authorized `TaskCategory::DocClaimJudge`, and reuse `SSOT_AUTOREGEN_TOKEN` for draft PR creation.
- D15 (2026-10-13, user): Continue global phase numbering at **Phases 32–36** (after `code-atlas` Phases 25–31) so phase numbers remain globally unique across all active workstreams.

---

## v1.4 Requirements

### 1. Trust Signal Calibration & Ledger Sync (`SIGNAL`)

Measured 2026-10-13: Only 28 of 972 pages (2.9%) in `contracts/reports/docs-freshness/ledger.v1.json` are `state: verified` (`689 broken`, `113 drifting`, `110 unanchored`, `32 historical`), and 0 of 7 tutorials show `verified` because of illustrative path false-positives, unseeded `verified_hash` values on command claims, and 127 `docs/src/api/*.generated.md` files created after the ledger snapshot.

- [ ] **SIGNAL-01**: `vox-doc-pipeline::claims` distinguishes illustrative user-project/example paths (`src/main.vox`, `app.vox`, `package.json`, `vite.config.ts`, `train.jsonl`, `vox.db`, etc.) from repository paths so valid tutorials, journeys, and how-to guides are not falsely marked `broken`.
- [ ] **SIGNAL-02**: Validated CLI command, env-var, and config-key claims in `ledger.v1.json` seed `verified_hash` from their resolved contract/registry hash rather than `"missing-file-hash"`, allowing verified tutorials and reference pages to reach `state: verified`.
- [ ] **SIGNAL-03**: `vox ci ssot-drift` runs freshness ledger generation after `crate-api-sync` and contract generators, ensuring all 127 `docs/src/api/*.generated.md` and generated reference files are registered and `state: verified` in `ledger.v1.json`.
- [ ] **SIGNAL-04**: Reader-facing conceptual pages (`explanation/`, `journeys/`, `learning/`, `index.mdx`) gain deterministic module/contract `code_anchors` so the `unanchored` count across public reader sections drops to zero.

### 2. OpenWiki-Style Auto-Organization, Deep Crate Symbols & Diagrams (`WIKI`)

- [ ] **WIKI-01**: A deterministic generator (`vox ci wiki-sync` in `vox-cli-ci`) synthesizes hierarchical **Subsystem Architecture Overview** pages under `docs/src/architecture/subsystems/` from `docs/src/architecture/layers.toml` (L0–L5), `where-things-live.md`, and `vox-graph-reader`, organizing all 127 workspace crates and active SSoT ADRs/specs by domain topology (Compiler & Language, Runtime & Durable Workflows, Orchestrator & AgentOS, VoxDB & Turso Data Layer, Populi Mesh, Mens ML Training, GUI & Surfaces).
- [ ] **WIKI-02**: `vox ci crate-api-sync` (`crates/vox-cli-ci/src/crate_api_docs.rs`) extracts public `struct`, `enum`, `trait`, `fn`, and `type` items with signatures, docstrings, and GitHub line links (`[src/...#Lstart-Lend]`) for every workspace crate, replacing placeholder `| Module | — |` tables.
- [ ] **WIKI-03**: Each generated Subsystem Hub and Crate API page includes deterministic **Mermaid architecture and dependency diagrams** (`flowchart LR` / `flowchart TD`) derived from `vox-graph-reader` crate-dependency and call-graph edges, validated for Mermaid syntax by `vox-doc-pipeline`.
- [ ] **WIKI-04**: Starlight sidebar navigation (`docs-astro/src/utils/sidebar.mjs`) surfaces the generated Subsystem Architecture Wiki Hubs as a structured, reader-navigable architecture tree while keeping raw research/roadmap/historical notes collapsed under `Internals`.

### 3. Corpus Consolidation & Baseline Deficit Ratchet (`DEBT`)

- [ ] **DEBT-01**: Merge or archive the 29 duplicate-cluster pages and 51 stale non-architecture pages identified in `contracts/reports/docs-freshness/prune-report.v1.json` (including `docs/src/ref/env-vars*`, hand-written duplicates alongside `.generated.md` files, and deprecated stubs like `docs/src/api/vox-codegen-ts.md`), adding 301 redirects in `docs-astro/public/_redirects` for every retired public URL.
- [ ] **DEBT-02**: Audit and reclassify or repair the 158 `docs/src/architecture/` pages still marked `status: current` (122 currently `broken`), demoting point-in-time design notes to `status: research | roadmap | historical` and anchoring genuine SSoT pages to current code symbols.
- [ ] **DEBT-03**: Systematically burn down the 920 baselined reader-facing deficits in `contracts/reports/docs-freshness/baseline.v1.json` by repairing stale references (`vox-mcp`, `vox-fullstack`, `vox-oratio`, `vox-dashboard`, `vox-vscode`, removed CLI subcommands) across high-debt reader pages (`reference/cli.md`, `mens-training.md`, `telemetry-metric-contract.md`, `how-to-train-mens-*`, `oratio-speech.md`, `scientia-ssot-handbook.md`) and ratcheting `baseline.v1.json` downward.

### 4. Nightly LLM Claim Verification (`VERIFY`)

- [ ] **VERIFY-01**: Register `TaskCategory::DocClaimJudge` in `contracts/orchestration/model-routing.v1.yaml` and `[audit.docs]` budget configuration in `vox.toml`, routing all LLM verification calls through `vox_actor_runtime::llm` from `vox-cli` / `vox-doc-pipeline` with zero new crate dependency edges.
- [ ] **VERIFY-02**: Implement DocPrism/LCEF-style structured claim verification (`vox audit docs --verify-llm`) that evaluates only `drifting` or `broken` pages against exact symbol spans hashed in `ledger.v1.json`, requiring every verdict to cite verifiable repo file and line ranges (`[path:Lstart-Lend]`).
- [ ] **VERIFY-03**: Add content-hash caching (`contracts/reports/docs-freshness/llm-verify-cache.v1.json`) and per-run token/spend caps so unchanged `(claim_hash, symbol_span_hash)` pairs are never re-evaluated, keeping steady-state spend within the \$13–\$45/mo budget.
- [ ] **VERIFY-04**: Score `vox audit docs --verify-llm` against the 10-case `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json`, enforcing $\le 10\%$ false-positive rate and $\ge 80\%$ precision on historical drift commits before enabling automated patch drafting.

### 5. Self-Healing Draft-PR Automation & Continuous Website Sync (`BOT`)

- [ ] **BOT-01**: Implement `scripts/docs-fix-drafts.vox` and `.github/workflows/docs-self-heal.yml` with a strict two-job privilege split: an unprivileged read-only job runs `vox audit docs --verify-llm` and produces a verified diff artifact; a separate no-LLM job using `SSOT_AUTOREGEN_TOKEN` opens or updates a draft PR.
- [ ] **BOT-02**: Restrict bot patches strictly to `docs/src/**` (excluding `*.generated.md` and `docs/src/archive/`), and require every candidate patch to pass `vox-doc-pipeline --lint-only`, `vox ci doctest-md --strict`, `vox ci retired-symbol-check`, and `vox ci check-links` before opening or updating a draft PR.
- [ ] **BOT-03**: Enforce alert-fatigue and precision guardrails: cap open doc-bot draft PRs at $\le 3$ concurrently, update existing draft PRs in place rather than opening duplicates, record acceptance/closure precision in `contracts/reports/docs-freshness/bot-metrics.v1.json`, and auto-pause draft PR creation if precision drops below threshold.
- [ ] **BOT-04**: Ensure continuous front-facing website freshness on `voxlang.org`: merging any code or documentation PR keeps `crate-api-sync`, `wiki-sync`, and `ledger.v1.json` synchronized via `ssot-autoregen`, triggers `docs-deploy.yml`, and verifies live trust banners and Subsystem Wiki routes in post-deploy smoke checks.

---

## Validated Requirements (Shipped in v1.3, Phases 20–24)

- [x] **DEPLOY-01..04**: `voxlang.org` Cloudflare Pages deploy restored, single-issue failure escalation + `ci-liveness`, live post-deploy smoke checks, and accurate git dates across symlinked content (Phase 20).
- [x] **HONEST-01..03**: Frontmatter-driven status banners + `noindex`, `Internals` sidebar section, and supersession of overlapping doc-audit specs (Phase 20).
- [x] **LINKS-01..03**: Build-time relative `.md` link rewriting, `/repo/` mounted Markdown pages, and blocking built-site link checks (Phase 20).
- [x] **TUT-01..02**: All 7 tutorials audited against current CLI/compiler and gated by an executable `.vox` tutorial test harness in CI (Phases 20, 22).
- [x] **SITE-01..05**: Collapsible left hover-rail and right TOC sidebars, wide-article layout, responsive mobile/tablet/desktop support, and Playwright visual-stepper suite (Phase 20.1).
- [x] **MEASURE-01..06**: Deterministic claim extractor + resolvers in `vox-doc-pipeline::claims`, `vox audit docs`, machine-generated Docs Reality Audit findings, and initial prune dispositions (Phase 21).
- [x] **DRIFT-01..07**: Symbol-span freshness ledger (`ledger.v1.json`), tighten-only baseline ratchet (`baseline.v1.json`), expiring suppressions, PR impact comment, and docs-definition-of-done warning (Phase 22).
- [x] **GEN-01..05**: Generated `env-vars.generated.md`, `config-keys.generated.md`, `mcp-tools.generated.md`, 127 `docs/src/api/*.generated.md` files, CLI `build_catalog()` compliance, and retired-decorator prose lint (Phase 23).
- [x] **TRUST-01..04 & EVAL-01..02**: Reader trust banners & sidebar badges, curated `llms.txt` verification gate, single `/llms-full.txt`, 10-case golden drift eval set, and Decision D9 in-house build verdict (Phase 24).

## Out of Scope

- Subscribing to commercial SaaS wiki/doc bots (OpenWiki hosted, Promptless, Dosu, Mintlify) — rejected by the Decision D9 evaluation (`docs-bot-evaluation-and-pivot-rule-2026.md`).
- Adding new crate dependency edges (`vox-doc-verify` standalone crate) — all v1.4 verification logic lives inside `vox-doc-pipeline` and `vox-cli` (D14).
- Running LLM inference synchronously on pull requests — PR checks remain 100% deterministic; LLM verification runs on a schedule or explicit CLI invocation (D10).
- Auto-merging any LLM-drafted prose change without human review (D3).

## Traceability (v1.4)

| Requirement | Phase | Status |
|---|---|---|
| SIGNAL-01 | Phase 32 | Planned |
| SIGNAL-02 | Phase 32 | Planned |
| SIGNAL-03 | Phase 32 | Planned |
| SIGNAL-04 | Phase 32 | Planned |
| WIKI-01 | Phase 33 | Planned |
| WIKI-02 | Phase 33 | Planned |
| WIKI-03 | Phase 33 | Planned |
| WIKI-04 | Phase 33 | Planned |
| DEBT-01 | Phase 34 | Planned |
| DEBT-02 | Phase 34 | Planned |
| DEBT-03 | Phase 34 | Planned |
| VERIFY-01 | Phase 35 | Planned |
| VERIFY-02 | Phase 35 | Planned |
| VERIFY-03 | Phase 35 | Planned |
| VERIFY-04 | Phase 35 | Planned |
| BOT-01 | Phase 36 | Planned |
| BOT-02 | Phase 36 | Planned |
| BOT-03 | Phase 36 | Planned |
| BOT-04 | Phase 36 | Planned |

Coverage: 19/19 v1.4 requirements mapped across Phases 32–36, no duplicates.
