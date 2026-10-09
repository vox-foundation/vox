# Roadmap: Vox

## Milestones

- ✅ **v1.0 Architectural Housekeeping** — Phases 1–6 (shipped 2026-10-01)
- 🚧 **v1.1 Research Trial Flywheel** — Phases 7–11 (parallel workstream `research-trial-flywheel`)
- 🚧 **v1.2 Unified Autonomy Control** — Phases 12–19 (parallel workstream `autonomy-ux`)
- 🚧 **v1.3 Self-Maintaining Public Docs** — Phases 20–24 (this workstream, `docs-freshness`)
- 📋 **v1.4 LLM Claim Verification & Draft-PR Bot** — Planned next milestone (this workstream; requires user authorization, see below)

## Overview

v1.3 makes voxlang.org stay accurate to the code without constant manual upkeep, treating accuracy as a verification problem rather than a writing problem. It starts by getting the site deploying again — nothing else is visible to readers until it does — and making the public surface honest about which pages are research, roadmap, or internal (Phase 20). A deterministic, report-only mention extractor then measures the corpus and drives a case-by-case human prune pass, so drift detection is not drowned by ~270 dead research/roadmap notes (21). On that pruned corpus come a code-anchored freshness ledger, an in-place PR impact comment, and a tighten-only gate that blocks only new drift on reader-facing `current` pages (22), alongside more reference generated straight from contracts (23 ∥ 22). The milestone closes with reader- and agent-visible trust signals driven by the ledger, a single generated agent index, and a data-backed go/no-go for the v1.4 LLM layer (24). No LLM runs on pull requests (D10) and v1.3 adds no new crate edges (D6).

## Phases

**Phase Numbering:**

- Integer phases (20, 21, …): Planned milestone work
- Decimal phases (20.1, 20.2): Urgent insertions (marked with INSERTED)

### 🚧 v1.3 Self-Maintaining Public Docs

- [ ] **Phase 20: Deploy Unblock & Public-Surface Honesty** - voxlang.org deploying from `main` again with escalating liveness alerts, live smoke checks, correct git dates, status banners/noindex, an Internals section, and one authoritative design
- [ ] **Phase 21: Measure & Prune** - Deterministic report-only mention extractor + resolvers, `vox audit docs`, machine-generated Docs Reality Audit findings, prune report, and user-approved case-by-case dispositions applied with redirects
- [ ] **Phase 22: Deterministic Drift Gate & PR Impact** - Symbol-span freshness ledger, in-place PR impact comment, tighten-only gate on reader-facing `current` pages, expiring suppressions, human re-attestation, docs-definition-of-done warning
- [ ] **Phase 23: Generated Reference Expansion** - Env-var reference plus at least two more references generated from contracts, every generator registered in `ssot-drift`/`ssot-autoregen` and tested
- [ ] **Phase 24: Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go** - Ledger-driven page banners and sidebar badges, verified-only curated llms.txt, single generated agent index, vendor/academic evidence with a pivot rule, and a golden drift eval set

## Phase Details

### Phase 20: Deploy Unblock & Public-Surface Honesty

**Goal**: Readers and scrapers see the current docs on voxlang.org again, a broken deploy can never silently persist, and the public surface stops presenting research and roadmap notes as current reference.
**Depends on**: Nothing (first v1.3 phase; independent of v1.1 and v1.2)
**Requirements**: DEPLOY-01, DEPLOY-02, DEPLOY-03, DEPLOY-04, HONEST-01, HONEST-02, HONEST-03
**Gate (human)**: DEPLOY-01 is blocked on the user rotating the Cloudflare Pages API token (`Pages:Edit` on `vox-docs`). All other Phase 20 work can proceed in parallel, but none of it is reader-visible until that token is rotated.
**Research flag**: Standard patterns — skip phase research (ops work). One open question to settle by a local build: whether the `setup-content.mjs` content symlink defeats git-date lookup even with `fetch-depth: 0`.
**Success Criteria** (what must be TRUE):

  1. A push to `main` touching `docs/src/` produces a green `docs-deploy.yml` run and the change is visible on voxlang.org; issue #462 is closed with the root cause (expired/under-scoped Cloudflare token) recorded.
  2. A forced deploy failure opens or updates exactly one de-duplicated issue with the assignee pinged and no repeat comments, and a deploy with no success within N days raises a `ci-liveness` alert.
  3. After each deploy, smoke checks against the live site confirm the home page and `/voxup` respond, every URL in the published `llms.txt` resolves, and sampled pages contain no retired syntax — a deliberately broken llms.txt link fails the check.
  4. A page's "Last updated" date on the deployed site matches its last substantive commit (fmt, regen, and bulk-rename commits ignored), verified through the content symlink by a local build.
  5. A page with `status: research | roadmap | deprecated | legacy` shows a visible status banner and a `noindex` meta tag from frontmatter alone; architecture/research/roadmap notes appear only under an Internals sidebar section and in no published llms.txt variant; the Docs Reality Audit narrative and the `2026-08-22-docs-corpus-repair*` / `2026-09-14-deep-research-documentation-engine*` specs are merged into this workstream or marked superseded.

**Plans**: TBD
**UI hint**: yes

### Phase 21: Measure & Prune

**Goal**: The corpus is measured deterministically and shrunk to what deserves to be public, with every disposition decided by the user from evidence — before any blocking gate exists.
**Depends on**: Phase 20
**Requirements**: MEASURE-01, MEASURE-02, MEASURE-03, MEASURE-04, MEASURE-05, MEASURE-06
**Gate (human)**: MEASURE-06 requires the user to approve page dispositions case by case (D5) and to confirm the policy that `research`/`roadmap` pages leave the public sidebar by default.
**Research flag**: Light research — mention-classification rules (illustrative code vs. code that prescribes behaviour; `vox:skip` / `text` fence handling).
**Success Criteria** (what must be TRUE):

  1. Running `vox audit docs` locally or in CI emits findings JSONL with `schema_version`, classifying every extracted path, crate, Rust symbol, `vox` command/flag, env var, and config key mention as resolved / missing / retired / ambiguous, without gating any build; mentions inside `vox:skip` / `text` fences are not reported as prescriptions.
  2. A hand-labelled sample has a recorded false-positive rate for the extractor, committed alongside the report, before any gate is enabled.
  3. `contracts/reports/docs-reality-audit/findings.v1.json` is populated by the extractor (schema bumped with `source`, `fingerprint`, `detector`), contains no duplicate fingerprints, and the metrics distinguish "empty because unpopulated" from "healthy".
  4. A prune report lists every live page as keep / merge / move-to-Internals / archive candidate with dead-mention count, inbound links, last substantive edit, status, section, and duplicate-topic cluster.
  5. The user-approved moves, merges, and archives are applied, and every removed public URL redirects to its replacement (no 404 on the deployed site for a previously public URL).

**Plans**: TBD

### Phase 22: Deterministic Drift Gate & PR Impact

**Goal**: Code changes that break reader-facing docs are caught at PR time by deterministic checks only, contributors see which docs their PR affects, and existing drift can only shrink.
**Depends on**: Phase 21 (extractor, measured false-positive rate, pruned corpus)
**Requirements**: DRIFT-01, DRIFT-02, DRIFT-03, DRIFT-04, DRIFT-05, DRIFT-06, DRIFT-07
**Gate (human)**: The user authorizes switching the gate to blocking on reader-facing `current` pages once the baseline is recorded.
**Research flag**: Needs phase research — symbol-span anchor hashing design (graphify item spans vs. file-sha fallback that churns on rustfmt), the human attestation mechanism, and false-positive measurement before blocking is enabled.
**Success Criteria** (what must be TRUE):

  1. A PR that changes a file or symbol referenced by reader-facing docs gets exactly one PR comment listing those docs, and later pushes edit that comment in place rather than adding new ones.
  2. `contracts/reports/docs-freshness/ledger.v1.json` (marked `linguist-generated`) records per page its anchors, symbol-span content hashes, `last_verified_commit`, and a `state` of verified / drifting / broken / unanchored / historical; reformatting an anchored file without changing the symbol does not flip its page to drifting.
  3. Authored frontmatter accepts only `owner`, `review_cadence`, and optional `code_anchors` as new keys, defined once in `vox-doc-pipeline`; a parity test fails if the governance doc drifts, and a hand-written `last_verified` is rejected by lint.
  4. A PR introducing a new broken mention on a `status: current` page under reference / tutorials / how-to / explanation fails the gate, while pre-existing baselined drift does not; the baseline file can only shrink, and a suppression that is missing owner/reason/expiry or is past its expiry fails the gate.
  5. A human can re-stamp a reviewed page as `verified` via CLI or PR label, and a PR that changes a public CLI command, env var, or contract without touching its doc or generator receives a deterministic warning.

**Plans**: TBD

### Phase 23: Generated Reference Expansion

**Goal**: Reference pages that are derivable from contracts are generated from them, so they cannot drift.
**Depends on**: Phase 21 (prune decisions settle which reference pages survive); runs in parallel with Phase 22
**Requirements**: GEN-01, GEN-02, GEN-03
**Research flag**: Standard patterns — skip phase research (existing generator + `ssot-autoregen` pattern).
**Success Criteria** (what must be TRUE):

  1. The env-var reference on the site is a `*.generated.md` produced from `contracts/config/env-vars.v1.yaml`; adding a var to the contract and running the generator makes it appear, and the hand-maintained page is gone.
  2. At least two more hand-maintained references (from config keys, error codes, MCP tool catalog, or operations catalog) are now `*.generated.md`, and each names its intended reader on the page.
  3. Every `*.generated.md` in `docs/src/` maps to a registered generator: editing a contract without regenerating fails `ssot-drift`, `ssot-autoregen` regenerates it on PRs, and generator tests cover dead/missing contract paths.

**Plans**: TBD

### Phase 24: Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go

**Goal**: Readers and LLM agents can see what is verified, agents get one trustworthy generated index, and the user has evidence to decide whether v1.4 builds the LLM layer or buys it.
**Depends on**: Phase 22 (ledger states); Phase 23 output is included if complete
**Requirements**: TRUST-01, TRUST-02, TRUST-03, TRUST-04, EVAL-01, EVAL-02
**Research flag**: Standard patterns for trust signals (Starlight `routeData`, existing `git-dates.mjs`); EVAL-01 is itself a research deliverable.
**Success Criteria** (what must be TRUE):

  1. Each public page shows "Verified against `<sha>`", "May be out of date — code changed since verification", or "Unverified" matching its ledger state, and the sidebar badges every non-verified page.
  2. The curated `current` llms.txt set contains only `verified` reader-facing pages, and CI fails when any curated llms.txt URL points to a non-verified or Internals page.
  3. The hand-committed `.well-known/llms-full.txt` is gone and agents are served a single generated source.
  4. A Playwright visual-stepper spec covers each banner state and the sidebar badges, writing screenshots to the review bundle.
  5. A research note in `docs/src/architecture/` reports published accuracy, false-positive, and draft-PR acceptance data for hosted docs bots and academic baselines with a numeric pivot rule (D9), and a committed golden set of past drift cases (`e2bb133a1`, `49a08f34e`, `f48dbc810`, …) can score any verifier on the same cases.

**Plans**: TBD
**UI hint**: yes

## Next Milestone: v1.4 LLM Claim Verification & Draft-PR Bot (planned)

Not started; scope is `VERIFY-*` and `BOT-*` in `REQUIREMENTS.md`. Proceeds only if Phase 24's go/no-go (EVAL-01 pivot rule, scored on the EVAL-02 golden set) favours building over buying.

- **Nightly LLM claim verification** — `vox-doc-verify` crate via `vox_actor_runtime::llm`, only on what the deterministic layer cannot decide; cited evidence lines must resolve; content-hash cache; per-run budget (~$13–45/mo).
- **Draft-PR bot** — `scripts/docs-fix-drafts.vox` + scheduled workflow; unprivileged LLM job produces a patch, separate no-LLM job opens a draft PR; `docs/src/**` only (generated files excluded); re-lint, re-doctest, retired-symbol check; ≤3 open drafts; precision tracked; security review before merge. Never auto-merged (D3).

**Authorization prerequisites (user-only):**

1. Crate-edge exceptions: new L3 crate `vox-doc-verify` (plus `layers.toml` / `where-things-live.md` rows); edges `vox-doc-verify → vox-actor-runtime, vox-doc-pipeline, vox-config` and `vox-cli → vox-doc-verify`.
2. A PAT secret for bot PRs (reuse `SSOT_AUTOREGEN_TOKEN` or a new scoped token) — PRs opened with `GITHUB_TOKEN` do not trigger `docs-quality.yml`.
3. Model-routing contract change: new `TaskCategory::DocClaimJudge` in `model-routing.v1.yaml`, plus a `vox.toml [audit.docs]` budget.

## Parallelism

- 20 → 21 → 22 → 24 is the critical path.
- 23 runs in parallel with 22 once 21's prune decisions land.
- Within 20, everything except DEPLOY-01 can proceed before the token rotation; reader-visible verification waits on it.
- Runs in parallel with v1.1 (`research-trial-flywheel`, phases 7–11) and v1.2 (`autonomy-ux`, phases 12–19); do not touch their files.

## Progress

| Phase | Plans Complete | Status | Completed |
|-------|----------------|--------|-----------|
| 20. Deploy Unblock & Public-Surface Honesty | 0/TBD | Not started | - |
| 21. Measure & Prune | 0/TBD | Not started | - |
| 22. Deterministic Drift Gate & PR Impact | 0/TBD | Not started | - |
| 23. Generated Reference Expansion | 0/TBD | Not started | - |
| 24. Reader Trust Signals, Agent Surfaces & v1.4 Go/No-Go | 0/TBD | Not started | - |
