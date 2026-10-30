# Roadmap: Vox (`docs-freshness`)

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 7–11 (parallel workstream `research-trial-flywheel`)
- 🚧 **v1.2 Unified Autonomy Control** — Phases 12–19 (parallel workstream `autonomy-ux`)
- ✅ **v1.3 Self-Maintaining Public Docs** — Phases 20–24 (this workstream, `docs-freshness`, shipped 2026-10-13)
- 🚧 **v1.4 Code Atlas** — Phases 25–31 (parallel workstream `code-atlas`)
- 🚧 **v1.4 OpenWiki-Style Self-Organizing & Self-Healing Public Docs** — Phases 32–36 (this workstream, `docs-freshness`, active)

## Overview

v1.4 in `docs-freshness` imports the best capabilities of OpenWiki/DeepWiki in-house — automatic topological organization from the codebase graph, deep symbol-level crate pages, auto-generated Mermaid dependency/call diagrams, calibrated reader trust signals, active technical-debt burn-down, and a nightly DocPrism-style LLM claim verifier + self-healing draft-PR bot — without subscribing to any commercial SaaS service and without adding new crate dependency edges (D13, D14).

It starts by calibrating `ledger.v1.json` and the claim extractor so valid tutorials, conceptual pages, and the 127 generated crate API pages show `Verified` on `voxlang.org` rather than false `Broken`/`Unanchored` banners (Phase 32). Next, it builds the OpenWiki-style auto-organization engine (`vox ci wiki-sync` and deep `crate-api-sync`), producing hierarchical Subsystem Architecture Hubs (L0–L5) and symbol-rich crate reference pages with deterministic Mermaid diagrams (Phase 33). In parallel/sequence, it consolidates the 29 duplicate clusters, reclassifies or repairs the 122 broken `status: current` architecture pages, and ratchets down the 920 baselined reader-facing deficits in `baseline.v1.json` (Phase 34). With a clean, organized corpus, Phase 35 implements the DocPrism/LCEF-style LLM claim verifier (`vox audit docs --verify-llm`) via `vox_actor_runtime::llm` (`TaskCategory::DocClaimJudge`), scored against `golden-drift-eval-set.v1.json`. Finally, Phase 36 wires `scripts/docs-fix-drafts.vox` and `.github/workflows/docs-self-heal.yml` (two-job privilege split, ≤3 open draft PRs, compiler/doctest-gated) plus continuous `voxlang.org` deployment verification.

## Phases

**Phase Numbering:**

- Continues globally at **Phase 32** (after `code-atlas` Phases 25–31, per D15).
- Completed v1.3 phases (20–24) remain recorded below for historical traceability.

### 🚧 v1.4 OpenWiki-Style Self-Organizing & Self-Healing Public Docs (Active)

- [ ] **Phase 32: Trust Signal Calibration & Ledger Synchronization** - Eliminate illustrative-path false positives (`src/main.vox`, etc.), seed `verified_hash` for valid command/config claims, register all 127 `docs/src/api/*.generated.md` files in `ledger.v1.json` via `ssot-drift`, and anchor conceptual reader pages so valid docs show `Verified` on `voxlang.org`
- [ ] **Phase 33: OpenWiki-Style Subsystem Hubs, Deep Crate Symbols & Mermaid Diagrams** - Synthesize hierarchical Subsystem Architecture Overview pages (`docs/src/architecture/subsystems/`) from `layers.toml` + `vox-graph-reader`, enrich `docs/src/api/*.generated.md` with public `struct`/`enum`/`trait`/`fn` tables and source-line links, and emit validated Mermaid dependency/call diagrams
- [ ] **Phase 34: Corpus Consolidation & Baseline Deficit Ratchet** - Merge/redirect the 29 duplicate clusters and 51 stale non-architecture pages from `prune-report.v1.json`, reclassify or repair the 122 broken `status: current` architecture docs, and systematically burn down the 920 baselined deficits in `baseline.v1.json`
- [ ] **Phase 35: Constrained LLM Claim Verifier (`vox audit docs --verify-llm`)** - Implement DocPrism/LCEF-style structured claim verification in `vox-doc-pipeline` / `vox-cli` via `vox_actor_runtime::llm` (`TaskCategory::DocClaimJudge`) with mandatory `[path:Lstart-Lend]` citations, content-hash caching, budget caps, and golden eval set scoring ($\le 10\%$ FP, $\ge 80\%$ precision)
- [ ] **Phase 36: Self-Healing Draft-PR Bot & Continuous Website Sync** - Ship `scripts/docs-fix-drafts.vox` and `.github/workflows/docs-self-heal.yml` (read-only LLM patch job → no-LLM draft-PR job using `SSOT_AUTOREGEN_TOKEN`), compiler/doctest/link-gated, capped at $\le 3$ open draft PRs with precision tracking and live `voxlang.org` post-deploy verification
- [ ] **Phase 37: Vox Syntax Highlighting on voxlang.org and Editors** - One shared Vox TextMate grammar drives Shiki/expressive-code on `voxlang.org` and the VS Code (and TextMate-compatible) extension, the LSP adds semantic tokens, and a CV screenshot gate proves every Vox code block on the site is fully colored
- [ ] **Phase 38: First-Read Docs Accuracy Pass** - Audit and correct every page a new reader meets first (README and the docs it links, the `voxlang.org` front page, and the top of the sidebar) against the current code, removing drift, contradictions and unsupported claims

### ✅ v1.3 Self-Maintaining Public Docs (Shipped 2026-10-13)

- [x] **Phase 20: Deploy Unblock & Public-Surface Honesty** (12/12 plans complete)
- [x] **Phase 20.1: Collapsible Reader Layout (INSERTED)** (4/4 plans complete)
- [x] **Phase 21: Measure & Prune** (4/4 plans complete)
- [x] **Phase 22: Deterministic Drift Gate & PR Impact** (4/4 plans complete)
- [x] **Phase 23: Generated Reference Expansion & Automated Crate API Docs** (4/4 plans complete)
- [x] **Phase 24: Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go** (4/4 plans complete)

---

## Phase Details (v1.4)

### Phase 32: Trust Signal Calibration & Ledger Synchronization

**Goal**: Reader trust banners and sidebar badges on `voxlang.org` accurately reflect reality — valid tutorials, generated crate API pages, and anchored conceptual pages display `Verified against <sha>` instead of false `Broken` or `Unverified` badges.
**Depends on**: Phase 24 (v1.3 ledger and trust-banner infrastructure)
**Requirements**: SIGNAL-01, SIGNAL-02, SIGNAL-03, SIGNAL-04
**Research flag**: Skip phase research — all root causes were measured directly in `ledger.v1.json` and `vox-doc-pipeline::claims`.
**Success Criteria** (what must be TRUE):

  1. Running `vox audit docs` and regenerating `ledger.v1.json` no longer reports illustrative user-project paths (`src/main.vox`, `app.vox`, `package.json`, `vite.config.ts`, `train.jsonl`, `vox.db`) as broken repository mentions, and all 7 tutorials reach `state: verified` in `ledger.v1.json`.
  2. Extracted CLI command, env-var, and config-key claims that resolve against `command-registry.yaml`, `env-vars.v1.yaml`, and `registry.v1.yaml` record their resolved contract hash as `verified_hash` rather than `"missing-file-hash"`.
  3. `vox ci ssot-drift` verifies that every `docs/src/**/*.generated.md` file (including all 127 `docs/src/api/*.generated.md` pages) exists in `ledger.v1.json` with `state: verified`, and fails if `crate-api-sync` or `command-sync` runs without updating the ledger.
  4. Every `status: current` page under `tutorials/`, `explanation/`, `journeys/`, `learning/`, and `index.mdx` has at least one valid `code_anchor` or resolved claim, reducing `unanchored` count in those reader sections to 0.

**Plans**: TBD (to be planned via `/gsd-plan-phase 32 --ws docs-freshness`)

### Phase 33: OpenWiki-Style Subsystem Hubs, Deep Crate Symbols & Mermaid Diagrams

**Goal**: `voxlang.org` automatically organizes its architecture and crate documentation from the live codebase graph (`layers.toml` + `vox-graph-reader` + rustdoc/source AST), giving readers and AI agents OpenWiki-grade subsystem overviews, deep public symbol tables with line links, and auto-generated Mermaid diagrams with zero manual maintenance.
**Depends on**: Phase 32
**Requirements**: WIKI-01, WIKI-02, WIKI-03, WIKI-04
**UI hint**: yes
**Research flag**: Light research on fast deterministic Rust item + span extraction in `crate_api_docs.rs` (`syn` / `vox-graph-reader` symbol spans vs `rustdoc` JSON speed across 127 crates).
**Success Criteria** (what must be TRUE):

  1. Running `vox ci wiki-sync --write` deterministically generates Subsystem Architecture Hub pages under `docs/src/architecture/subsystems/` covering all L0–L5 layers and domain subsystems (Compiler, Runtime & Durable Workflows, Orchestrator & AgentOS, VoxDB & Turso, Populi Mesh, Mens ML, GUI & Surfaces), mapping every workspace crate and active SSoT ADR/spec.
  2. Every `docs/src/api/<crate>.generated.md` page lists the crate's public `struct`, `enum`, `trait`, `fn`, and `type` items with signatures, first-paragraph docstrings, and clickable GitHub source line links (`[src/...#Lstart-Lend]`), replacing empty `| Module | — |` rows.
  3. Every Subsystem Hub and Crate API page contains a valid, deterministically generated Mermaid dependency/architecture diagram (`flowchart TD` / `flowchart LR`) derived from `vox-graph-reader` / `Cargo.toml` edges, and `vox-doc-pipeline` lints all generated Mermaid blocks with 0 errors.
  4. The Starlight sidebar (`docs-astro/src/utils/sidebar.mjs`) presents the Subsystem Architecture Hubs as a top-level navigable Architecture Wiki tree while keeping raw research/roadmap/historical notes inside `Internals`.

**Plans**: TBD (to be planned via `/gsd-plan-phase 33 --ws docs-freshness`)

### Phase 34: Corpus Consolidation & Baseline Deficit Ratchet

**Goal**: Obsolete, duplicate, and broken documentation across the corpus is consolidated or repaired case by case, shrinking `baseline.v1.json` and eliminating stale references to retired crates and CLI commands on `voxlang.org`.
**Depends on**: Phase 32 (calibrated extractor); can run in parallel with Phase 33
**Requirements**: DEBT-01, DEBT-02, DEBT-03
**Research flag**: Skip phase research — target pages and deficits are already enumerated in `prune-report.v1.json` and `baseline.v1.json`.
**Success Criteria** (what must be TRUE):

  1. All 29 duplicate-cluster merge candidates in `prune-report.v1.json` (such as `docs/src/ref/env-vars*`, hand-written `plugin-catalog.md` / `distribution-bundles.md`, and `api/vox-codegen-ts.md`) are consolidated into their canonical SSoT or `.generated.md` pages, with 301 redirects in `docs-astro/public/_redirects` and 0 broken internal links on `dist/`.
  2. All 158 `docs/src/architecture/` pages marked `status: current` are audited: point-in-time investigation/design snapshots are reclassified to `status: research | roadmap | historical` (moving to `Internals`), and every remaining `status: current` architecture SSoT passes `vox audit docs` with `state: verified`.
  3. Stale references to retired crates (`vox-mcp`, `vox-fullstack`, `vox-oratio`, `vox-dashboard`, `vox-vscode`, `vox-pm`, `vox-socrates-policy`, `vox-compilerd`, `vox-server`, `vox-ml`, `vox-edge`, `vox-schola`) and removed CLI commands in reader-facing `reference/`, `how-to/`, `explanation/`, and `operations/` pages are repaired, reducing `baseline.v1.json` deficits by at least 75% and ratcheting the baseline file down.

**Plans**: TBD (to be planned via `/gsd-plan-phase 34 --ws docs-freshness`)

### Phase 35: Constrained LLM Claim Verifier (`vox audit docs --verify-llm`)

**Goal**: Semantic drift that deterministic regex/symbol resolution cannot decide is caught by a DocPrism/LCEF-style LLM claim verifier routed through `vox_actor_runtime::llm`, backed by mandatory source-line citations, content-hash caching, and golden eval set benchmarks.
**Depends on**: Phase 32, Phase 34 (clean corpus so LLM budget is spent only on genuine drift candidates)
**Requirements**: VERIFY-01, VERIFY-02, VERIFY-03, VERIFY-04
**Research flag**: Light phase research — prompt schema and LCEF external-filter rules for `DocClaimJudge`.
**Success Criteria** (what must be TRUE):

  1. `contracts/orchestration/model-routing.v1.yaml` defines `TaskCategory::DocClaimJudge` and `vox.toml` defines `[audit.docs]` budget caps; all LLM calls in `vox audit docs --verify-llm` route through `vox_actor_runtime::llm` with 0 new crate dependency edges (`vox ci crate-edges` and `vox-code-audit` pass).
  2. Running `vox audit docs --verify-llm` inspects only `drifting` or `broken` pages, passes each extracted prose claim + anchored symbol span (`[path:Lstart-Lend]`) to `DocClaimJudge`, and deterministically rejects any LLM verdict whose cited file path or line range does not exist.
  3. Verified `(claim_hash, symbol_span_hash)` tuples are persisted in `contracts/reports/docs-freshness/llm-verify-cache.v1.json`; re-running `vox audit docs --verify-llm` on unchanged code makes 0 LLM calls, and a budget cap halts execution cleanly if token/cost limits are reached.
  4. Scoring `vox audit docs --verify-llm` against the 10-case `contracts/reports/docs-freshness/golden-drift-eval-set.v1.json` achieves $\ge 80\%$ precision and $\le 10\%$ false-positive rate.

**Plans**: TBD (to be planned via `/gsd-plan-phase 35 --ws docs-freshness`)

### Phase 36: Self-Healing Draft-PR Bot & Continuous Website Sync

**Goal**: Confirmed documentation drift produces compiler-, doctest-, and link-verified draft PRs for human review without developer fatigue, while `voxlang.org` stays continuously updated on every merge.
**Depends on**: Phase 33, Phase 35
**Requirements**: BOT-01, BOT-02, BOT-03, BOT-04
**Research flag**: Skip phase research — follows the existing `ssot-autoregen` two-step security pattern in `.github/workflows/ci.yml`.
**Success Criteria** (what must be TRUE):

  1. `scripts/docs-fix-drafts.vox` and `.github/workflows/docs-self-heal.yml` execute a two-job split: an unprivileged read-only job runs `vox audit docs --verify-llm` and uploads a candidate patch artifact; a separate no-LLM job using `SSOT_AUTOREGEN_TOKEN` validates and opens/updates a draft PR (never auto-merged).
  2. The bot rejects any patch that touches files outside `docs/src/**` (or touches `*.generated.md` / `docs/src/archive/`), or that fails `vox-doc-pipeline --lint-only`, `vox ci doctest-md --strict`, `vox ci retired-symbol-check`, or `vox ci check-links`.
  3. At most 3 doc-bot draft PRs can be open concurrently (existing drafts are updated in place), precision metrics (merged vs. closed-unmerged) are recorded in `contracts/reports/docs-freshness/bot-metrics.v1.json`, and draft creation auto-pauses if acceptance falls below threshold.
  4. Merging a PR that changes crate APIs, subsystem topology, or docs automatically updates `docs/src/api/*.generated.md`, `docs/src/architecture/subsystems/*.generated.md`, and `ledger.v1.json` via `ssot-autoregen`, deploys to `voxlang.org`, and passes live post-deploy smoke checks covering trust banners and Subsystem Wiki routes.

**Plans**: TBD (to be planned via `/gsd-plan-phase 36 --ws docs-freshness`)

### Phase 37: Vox Syntax Highlighting on voxlang.org and Editors

**Goal**: Every Vox code block on `voxlang.org` and every `.vox` file in VS Code (and TextMate-compatible editors) is fully and consistently colored from one grammar, and a screenshot-based CV gate keeps it that way.
**Depends on**: Nothing (independent of Phases 32–36)
**Requirements**: TBD
**Research flag**: Light — confirm which grammar `docs-astro/src/plugins/vox-grammar.mjs` loads versus `apps/editor/vox-vscode/syntaxes/vox.tmLanguage.json`, and whether `vox-lsp` already emits semantic tokens.
**Baseline (CV check 2026-10-30, `voxlang.org/tutorials/tut-first-app/`)**: keywords (`table`, `query`, `mutation`, `component`, `routes`, `return`), builtin types (`str`, `int`, `bool`, `Result`), call names, strings and literals are colored; declared type/component names (`Task`, `TaskList`), constructors (`Ok`), record field names, and slot labels (`view:`) render as plain text.
**Success Criteria** (what must be TRUE):

  1. One Vox TextMate grammar is the single source for both the site's Shiki `langs` and the VS Code extension's `syntaxes/` (no forked copy), covering the full current keyword set from AGENTS.md §Grammar Unification (bare-keyword blocks, data-layer soft keywords, decorators) plus declaration names, constructors, field names and slot labels; retired spellings get no special scope.
  2. `vox-lsp` provides semantic tokens for `.vox` files, so editors that support them color declarations and references beyond what the TextMate grammar can see.
  3. Every fenced block whose source is Vox renders with `data-language="vox"` on `voxlang.org` (no Vox code falls back to `text`), except `vox-diagram`, which stays on its own renderer.
  4. A Playwright CV gate over a set of Vox-heavy `voxlang.org` pages (tutorials, reference, golden examples) captures screenshots and asserts per-token computed colors: no identifier token in a Vox block keeps the default foreground color, in both light and dark themes; it runs in CI and fails on regression.
  5. The VS Code extension, installed from the repo, colors the golden examples under `examples/golden/` identically to the site (screenshot evidence in the review bundle).

**Plans**: TBD (to be planned via `/gsd-plan-phase 37 --ws docs-freshness`)

### Phase 38: First-Read Docs Accuracy Pass

**Goal**: The pages a new user reads first are accurate against the current code, consistent with each other, and ordered around what a newcomer needs (what Vox is, how to install it, a first program, what works today).
**Depends on**: Nothing (independent of Phases 32–37; Phase 32's trust-signal calibration makes its banners more useful but is not required)
**Requirements**: TBD
**Research flag**: Skip — this is an audit against the repo, not new technology.
**Scope (first-read set)**: `README.md`; every doc it links (Getting Started, Syntax Reference, Stability Matrix, Installing Vox, First Full-Stack App, CLI reference, Contributor Hub, `where-things-live.md`, v1.0 release criteria, the benchmark page); `docs/src/index.mdx` (the `voxlang.org` front page) and everything it links; and the first two sidebar sections as ordered by `docs-astro/src/utils/sidebar.mjs` / its section-order contract.
**Baseline findings (2026-10-30, from reading README.md)**:
  - "The central claim" row is graded **Working** with $N=100$ / McNemar results from `vox-vs-ts-empirical-benchmark-2026.md` (LLM-authored, `authorship_model: antigravity-flash`), while the README's own research-honesty note still calls the claim an unestablished hypothesis. One of them is wrong; the numbers must trace to committed run artifacts or the row goes back to Research.
  - "What's in the Box" names `ars` skills and `dei` visual review; `vox-ars` and `vox-dei` are retired surfaces (AGENTS.md §Retired Surfaces), so feature-flag names need checking against `crates/vox-cli/Cargo.toml` and the prose updated.
  - Many Status rows are still dated 2026-09-21 (LSP, REPL, VS Code extension, `vox emit`, registry); "Last reviewed" is 2026-10-14. The LSP row needs re-checking against Phase 37's findings.
  - The `voxlang.org` tutorial page at `/tutorials/tut-first-app/` showed its title twice (fixed 2026-10-30 by `docs-astro/src/plugins/remark-strip-title-h1.mjs`); confirm no other first-read page has a duplicate or missing H1.
**Success Criteria** (what must be TRUE):

  1. Every command, flag, path, crate, feature flag and URL on the first-read set exists in the current tree or CLI registry (`vox ci check-links`, `vox audit docs`, and a manual pass over anything those tools cannot resolve), and every code sample on those pages compiles via the doctest pipeline or runs with `vox run`.
  2. The README Status tables are re-walked: each row is re-verified and re-dated or explicitly left with its old date; no row claims more than its evidence shows, and the README and `index.mdx` agree on every shared claim (the `why_vox` sync lint passes).
  3. Any quantitative claim on the first-read set (benchmark results, tool counts, test counts) links to a committed artifact that reproduces it; claims without one are removed or downgraded.
  4. The first sidebar sections lead with Getting Started → Install → first program → language tour → Stability, in that order, with no Internals, research or archived pages ahead of them; the front page links resolve to those same pages.
  5. A Playwright smoke check walks the front page and the first-read set on the deployed site (one H1 per page, no 404 links, no "Broken" trust banner) and runs in CI.

**Plans**: TBD (to be planned via `/gsd-plan-phase 38 --ws docs-freshness`)

---

## Parallelism

- **Phase 32** is the immediate foundation (calibrates the extractor and ledger).
- **Phase 33** (OpenWiki Subsystem Hubs, Deep Crate API docs, Mermaid diagrams) and **Phase 34** (Corpus Consolidation & Baseline Deficit Ratchet) can run in parallel after Phase 32.
- **Phase 35** (LLM Claim Verifier) depends on Phase 32 + 34 so LLM tokens aren't wasted on pre-existing dead mentions.
- **Phase 36** (Draft-PR Bot & Continuous Website Sync) depends on Phase 33 + 35.
- **Phase 37** (Vox Syntax Highlighting) is independent and can run at any time.
- **Phase 38** (First-Read Docs Accuracy Pass) is independent and can run at any time; it re-checks the README LSP row after Phase 37 if both run.
- Runs in parallel with `research-trial-flywheel` (Phases 7–11), `autonomy-ux` (Phases 12–19), and `code-atlas` (Phases 25–31); do not touch their files.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 20. Deploy Unblock & Public-Surface Honesty (v1.3) | 12/12 | Complete | 2026-10-12 |
| 20.1. Collapsible Reader Layout (v1.3) | 5/5 | Complete | 2026-10-13 |
| 21. Measure & Prune (v1.3) | 4/4 | Complete | 2026-10-13 |
| 22. Deterministic Drift Gate & PR Impact (v1.3) | 4/4 | Complete | 2026-10-13 |
| 23. Generated Reference Expansion (v1.3) | 4/4 | Complete | 2026-10-13 |
| 24. Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go (v1.3) | 4/4 | Complete | 2026-10-13 |
| 32. Trust Signal Calibration & Ledger Synchronization (v1.4) | 0/TBD | Not started | — |
| 33. OpenWiki-Style Subsystem Hubs, Deep Crate Symbols & Mermaid Diagrams (v1.4) | 0/TBD | Not started | — |
| 34. Corpus Consolidation & Baseline Deficit Ratchet (v1.4) | 0/TBD | Not started | — |
| 35. Constrained LLM Claim Verifier (v1.4) | 0/TBD | Not started | — |
| 36. Self-Healing Draft-PR Bot & Continuous Website Sync (v1.4) | 0/TBD | Not started | — |
| 37. Vox Syntax Highlighting on voxlang.org and Editors (v1.4) | 0/TBD | Not started | — |
| 38. First-Read Docs Accuracy Pass (v1.4) | 0/TBD | Not started | — |
