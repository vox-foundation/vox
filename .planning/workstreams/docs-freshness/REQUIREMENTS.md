# Requirements — v1.3 Self-Maintaining Public Docs (workstream `docs-freshness`)

**Goal:** voxlang.org stays accurate to the code without constant manual upkeep. Anything derivable from code is generated; everything else is anchored to code and checked deterministically in CI; drift is surfaced to humans and LLM agents; readers and agents can see what is verified. LLM claim verification and the draft-PR bot follow in v1.4.

Brief: `CONTEXT.md`. Research: `research/SUMMARY.md` (+ STACK, FEATURES, ARCHITECTURE, PITFALLS).

## Decisions (user, 2026-10-08)

- D1: Separate GSD workstream `docs-freshness`, phases 20+.
- D2: All five outcomes in scope across v1.3–v1.4: drift detection, generated reference, agent-drafted fixes, case-by-case pruning, reader freshness signals.
- D3: LLM fixes arrive only as **draft PRs for human review** — never auto-merged (v1.4).
- D4: Build, mostly in-house (hybrid). Keep Starlight. Vendor bots that call LLM hosts directly are design references only (they violate the LLM-facade and secrets policy).
- D5: Pruning is **case by case** from a generated audit report — no blanket or time-based archiving.
- D6: v1.3 = phases 20–24 (no new crate edges). LLM verification + draft-PR bot = v1.4.
- D7: Internal architecture/research/roadmap notes move to a separate **Internals** section: out of the main sidebar, `noindex`, excluded from llms.txt.
- D8: Drift gate **blocks new drift only** (tighten-only baseline ratchet) on reader-facing `status: current` pages; existing drift is baselined.
- D9: No hosted-bot trial. Instead, gather published accuracy and acceptance data for hosted docs bots and set a pivot rule: if they are far better than what we can build, switch to buying.
- D10: No LLM runs on pull requests. PR-time checks are deterministic only.
- D11 (2026-10-08, user): Links on the site must work, repo Markdown must render as doc pages, and tutorials especially must all be current. This promotes docs-as-tests for tutorials from out of scope into v1.3.
- D12 (2026-10-08, user): Both sidebars collapse. The left one folds to a rail that expands on hover; the right one can be hidden, with a small control to bring it back. The article widens into the freed space. Everything works on mobile, including the landing page. Inserted as Phase 20.1.

## v1.3 Requirements

### Deploy & liveness (DEPLOY)

- [ ] **DEPLOY-01**: voxlang.org deploys successfully from `main` again (Cloudflare Pages token rotated by the user with `Pages:Edit` on `vox-docs`), and issue #462 is closed with the root cause recorded.
- [ ] **DEPLOY-02**: A docs-deploy failure escalates once (single de-duplicated issue, assignee pinged, no repeat comments). A deploy that hasn't succeeded within N days raises a liveness alert through the existing `ci-liveness` mechanism.
- [ ] **DEPLOY-03**: Post-deploy smoke checks run against the live site: home page and `/voxup` respond, every URL in the published `llms.txt` resolves, and no retired syntax appears on sampled pages.
- [x] **DEPLOY-04**: Per-page git dates are correct on the deployed site (full-history checkout; content symlink verified by a local build), and mechanical commits (fmt, regen, bulk renames) are excluded from the date.

### Public-surface honesty (HONEST)

- [x] **HONEST-01**: Pages with `status: research | roadmap | deprecated | legacy` render a visible status banner and are `noindex`, driven by frontmatter alone.
- [x] **HONEST-02**: Architecture/research/roadmap notes live under a separate **Internals** sidebar section, excluded from the main navigation, search-engine indexing, and every published llms.txt variant (D7).
- [x] **HONEST-03**: The overlapping designs (dormant Docs Reality Audit Program narrative, `docs/superpowers/specs/2026-08-22-docs-corpus-repair*`, `docs/superpowers/specs/2026-09-14-deep-research-documentation-engine*`) are merged into this workstream or marked superseded, so one design is authoritative.

### Working links (LINKS)

Measured 2026-10-08: 2,803 relative `.md` links across 438 live pages are written relative to the repo, but Starlight serves each page as a directory (`/tutorials/tut-getting-started/`), so the browser resolves them one level too deep. For example `../reference/installation.md` becomes `/tutorials/reference/installation.md`, which returns 404. Another 1,909 links point outside `docs/src/` (top targets: `AGENTS.md` ×123, `LANGUAGE_DESIGN_PRIORITIES.md` ×23, Rust sources, contracts) and all 404. `vox ci check-links` validates repo paths, so it passes. The lychee step on the built site only prints guidance and never fails.

- [x] **LINKS-01**: Relative links between docs pages (`foo.md`, `../x/bar.md#anchor`, `index.md` / `README.md`) render as working site routes. They are rewritten at build time (remark plugin in `docs-astro/src/plugins/`) to Starlight slugs, anchors preserved; source Markdown stays repo-relative so GitHub rendering keeps working.
- [x] **LINKS-02**: Links that leave `docs/src/` resolve. Repo Markdown files (e.g. `AGENTS.md`, `LANGUAGE_DESIGN_PRIORITIES.md`, `README.md`, `CONTRIBUTING.md`, crate READMEs) are rendered as doc pages on the site from a single source (no hand copies). Non-Markdown repo files (source, contracts, `Cargo.toml`) link to their GitHub blob on `main`.
- [x] **LINKS-03**: The built site fails CI on any broken internal link (blocking check over `dist/`), including a deliberately broken fixture. External links are checked nightly, cached, and non-blocking.

### Reader layout (SITE)

Added 2026-10-08 (D12). The site runs stock Starlight 0.38 with no component overrides; neither sidebar can be collapsed.

- [ ] **SITE-01**: The left navigation sidebar collapses to a narrow rail. While collapsed, hovering over or focusing the rail temporarily expands it as an overlay without moving the article. A toggle pins it open or collapsed, and the choice persists across pages (localStorage).
- [ ] **SITE-02**: The right "On this page" sidebar can be hidden with a toggle. When hidden, a small, unobtrusive control stays visible to bring it back, and the choice persists across pages.
- [ ] **SITE-03**: With either or both sidebars collapsed or hidden, the article widens into the freed space up to a readable maximum width. Wide content (tables, code, diagrams) uses the extra room.
- [ ] **SITE-04**: Full responsive support on docs pages and the voxlang.org landing page, at phone, tablet and desktop widths. Mobile keeps Starlight's menu drawer, touch devices get no hover-only behaviour, all controls are keyboard-operable with ARIA state, the layout is stable on first paint (no flash), and reduced-motion preferences are honoured.
- [ ] **SITE-05**: Playwright visual-stepper specs in `docs-astro/tests/` cover each sidebar state × {phone, tablet, desktop} × {light, dark} plus the landing page, and save screenshots to a review bundle. `docs-quality.yml` runs them.

### Tutorials current (TUT)

- [x] **TUT-01**: Every tutorial (7 today) is audited and corrected against current code. Every `vox` command and flag exists in `command-registry.yaml`, every `vox` snippet compiles as a doctest (`vox:skip` only with a stated reason), install and prerequisite steps match `reference/installation.md`, and every link resolves. Each tutorial records the commit it was verified against.
- [ ] **TUT-02**: Tutorials are executable docs-as-tests. A `.vox` harness runs each tutorial's steps end to end in CI (on PRs touching `docs/src/tutorials/**`, the CLI, or the compiler, plus nightly), and a failing tutorial blocks the merge.

### Measurement & pruning (MEASURE)

- [ ] **MEASURE-01**: A deterministic mention extractor in `vox-doc-pipeline` (`claims` module) finds file paths, crate names, Rust symbols, `vox` CLI commands/flags, env vars, and config keys in Markdown prose and code fences. It honours `vox:skip` / `text` fences and tells "mentions" apart from "prescribes".
- [ ] **MEASURE-02**: Each extracted mention is resolved against the repo (filesystem, `command-registry.yaml`, `env-vars.v1.yaml`, `retired-symbols.v1.yaml`, git grep; `vox graph` when available) and classified as resolved / missing / retired / ambiguous.
- [ ] **MEASURE-03**: Report-only CLI (`vox audit docs`, findings JSONL with `schema_version`) runs locally and in CI without gating. The false-positive rate is measured on a hand-labelled sample before any gate is enabled.
- [ ] **MEASURE-04**: The Docs Reality Audit's findings are **machine-generated** from the extractor. The schema is bumped with `source`, `fingerprint`, `detector`; findings are de-duplicated by fingerprint; metrics tell "empty because unpopulated" apart from "healthy".
- [ ] **MEASURE-05**: A prune report lists every live page as keep / merge / move-to-Internals / archive candidate, with evidence: dead-mention count, inbound links, last substantive edit, status, section, and duplicate-topic clusters.
- [ ] **MEASURE-06**: The user approves page dispositions case by case, and the approved moves, merges and archives are applied with redirects for removed public URLs.

### Drift gate & PR impact (DRIFT)

- [ ] **DRIFT-01**: Every PR gets one PR comment, edited in place, listing reader-facing docs that reference files or symbols the PR changed (reverse index from code anchor to docs).
- [ ] **DRIFT-02**: A generated freshness ledger (`contracts/reports/docs-freshness/ledger.v1.json`, `linguist-generated`) records per page: anchors, anchor content hashes at the symbol-span level, `last_verified_commit`, and `state ∈ {verified, drifting, broken, unanchored, historical}`.
- [ ] **DRIFT-03**: Authored frontmatter gains only `owner`, `review_cadence`, and optional `code_anchors`. These keys are defined once in `vox-doc-pipeline` and parity-tested against the governance doc. There is no hand-written `last_verified`.
- [ ] **DRIFT-04**: A gate blocks merges that introduce **new** broken mentions on `status: current` pages under reference / tutorials / how-to / explanation. Existing drift is baselined; the baseline can only tighten.
- [ ] **DRIFT-05**: Every suppression carries owner, reason, and expiry date. Expired suppressions fail the gate.
- [ ] **DRIFT-06**: A human can re-attest a page (CLI or PR label) to re-stamp it `verified` in the ledger after review.
- [ ] **DRIFT-07**: A docs-definition-of-done rule: a PR that changes a public CLI command, env var, or contract without touching its doc or generator gets a deterministic warning.

### Generated reference (GEN)

- [ ] **GEN-01**: Env-var reference is generated from `contracts/config/env-vars.v1.yaml`, replacing the hand-maintained page.
- [ ] **GEN-02**: At least two more hand-maintained references become `*.generated.md` from their contracts (candidates: config keys, error codes, MCP tool catalog, operations catalog). Each one names its reader.
- [ ] **GEN-03**: Every `*.generated.md` has a registered generator wired into `ssot-drift` and `ssot-autoregen`. Generators have tests, including for dead paths.

### Reader trust & agent surfaces (TRUST)

- [ ] **TRUST-01**: Each public page shows its ledger state: "Verified against `<sha>`" / "May be out of date — code changed since verification" / "Unverified". Sidebar badges mark non-verified pages.
- [ ] **TRUST-02**: A curated `current` llms.txt set contains only `verified` reader-facing pages. CI fails if any curated llms.txt URL points to a non-verified or Internals page.
- [ ] **TRUST-03**: The hand-committed `.well-known/llms-full.txt` is retired; agents get a single generated source (no split-brain).
- [ ] **TRUST-04**: Every GUI/site change for TRUST-01 has Playwright visual-stepper coverage, with screenshots in the review bundle.

### v1.4 go/no-go evaluation (EVAL)

- [ ] **EVAL-01**: A research note in `docs/src/architecture/` collects published and vendor-reported accuracy, false-positive, and draft-PR acceptance data for hosted docs bots (Promptless, Dosu, Mintlify agent, DeepDocs, etc.). It also covers academic baselines (DocPrism, CASCADE), states what the user should expect from each, and defines a numeric pivot rule (D9).
- [ ] **EVAL-02**: A golden eval set of past drift cases (commits `e2bb133a1`, `49a08f34e`, `f48dbc810`, …) is committed, so v1.4's LLM verifier and any vendor can be scored on the same cases.

## v1.4 Requirements (deferred — next milestone)

- **VERIFY-01..**: `vox-doc-verify` crate (needs user-authorized crate edges) → nightly LLM claim verification through `vox_actor_runtime::llm`, new `DocClaimJudge` routing category, cited evidence lines that must resolve, content-hash cache, per-run budget (about $13–45/mo).
- **BOT-01..**: `scripts/docs-fix-drafts.vox` + scheduled workflow. Two jobs: an LLM job with read-only access produces a patch; a PR job with no LLM opens it. Edits allowed only under `docs/src/**` (generated files excluded). Every patch is re-linted, re-doctested and re-checked for retired symbols. At most 3 open draft PRs; precision is tracked; security review required. Needs a PAT secret.

## Out of Scope

- Migrating off Starlight to a hosted docs platform — the rewrite cost outweighs the benefit (D4).
- LLM checks on pull requests (D10).
- Auto-merging any prose change (D3).
- Blanket or time-based archiving (D5).
- Hosted docs MCP endpoint with a feedback tool — deferred until Cloudflare logs show agent and reader demand. (Tutorial docs-as-tests moved into scope as TUT-02, per D11.)
- Vale prose linting — `retired-symbol-check` stays the source of truth for banned terms.

## Traceability

| Requirement | Phase | Status |
|---|---|---|
| DEPLOY-01 | Phase 20 | Pending |
| DEPLOY-02 | Phase 20 | Pending |
| DEPLOY-03 | Phase 20 | Pending |
| DEPLOY-04 | Phase 20 | Complete |
| HONEST-01 | Phase 20 | Complete |
| HONEST-02 | Phase 20 | Complete |
| HONEST-03 | Phase 20 | Complete |
| LINKS-01 | Phase 20 | Complete |
| LINKS-02 | Phase 20 | Complete |
| LINKS-03 | Phase 20 | Complete |
| TUT-01 | Phase 20 | Complete |
| SITE-01 | Phase 20.1 | Pending |
| SITE-02 | Phase 20.1 | Pending |
| SITE-03 | Phase 20.1 | Pending |
| SITE-04 | Phase 20.1 | Pending |
| SITE-05 | Phase 20.1 | Pending |
| MEASURE-01 | Phase 21 | Pending |
| MEASURE-02 | Phase 21 | Pending |
| MEASURE-03 | Phase 21 | Pending |
| MEASURE-04 | Phase 21 | Pending |
| MEASURE-05 | Phase 21 | Pending |
| MEASURE-06 | Phase 21 | Pending |
| DRIFT-01 | Phase 22 | Pending |
| DRIFT-02 | Phase 22 | Pending |
| DRIFT-03 | Phase 22 | Pending |
| DRIFT-04 | Phase 22 | Pending |
| DRIFT-05 | Phase 22 | Pending |
| DRIFT-06 | Phase 22 | Pending |
| DRIFT-07 | Phase 22 | Pending |
| TUT-02 | Phase 22 | Pending |
| GEN-01 | Phase 23 | Pending |
| GEN-02 | Phase 23 | Pending |
| GEN-03 | Phase 23 | Pending |
| TRUST-01 | Phase 24 | Pending |
| TRUST-02 | Phase 24 | Pending |
| TRUST-03 | Phase 24 | Pending |
| TRUST-04 | Phase 24 | Pending |
| EVAL-01 | Phase 24 | Pending |
| EVAL-02 | Phase 24 | Pending |

Coverage: 39/39 v1.3 requirements mapped, no duplicates.
