---
phase: 20-deploy-unblock-public-surface-honesty
verified: 2026-10-09T20:10:00Z
status: passed
score: 7/7 must-haves verified
re_verification:
  previous_status: human_needed
  previous_score: 6/7
  gaps_closed:
    - "DEPLOY-02 escalation half: live drill passed (20-UAT.md test 1)"
  gaps_remaining: []
  regressions: []
covered_files:
  - ".git-blame-ignore-revs"
  - ".github/workflows/ci-liveness.yml"
  - ".github/workflows/docs-deploy.yml"
  - ".github/workflows/docs-quality.yml"
  - ".github/workflows/link_checker.yml"
  - ".planning/workstreams/docs-freshness/REQUIREMENTS.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-01-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-01-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-02-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-02-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-03-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-03-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-04-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-04-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-05-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-05-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-06-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-06-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-07-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-07-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-08-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-08-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-09-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-09-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-10-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-10-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-11-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-11-SUMMARY.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-12-PLAN.md"
  - ".planning/workstreams/docs-freshness/phases/20-deploy-unblock-public-surface-honesty/20-12-SUMMARY.md"
  - "README.md"
  - "apps/build-tools/render-durable-animation/package.json"
  - "apps/editor/vox-vscode/README.md"
  - "contracts/documentation/site-mounted-repo-docs.v1.json"
  - "contracts/documentation/tutorial-verification.v1.json"
  - "docs-astro/astro.config.mjs"
  - "docs-astro/package.json"
  - "docs-astro/patches/starlight-llms-txt@0.10.0.patch"
  - "docs-astro/playwright.config.ts"
  - "docs-astro/pnpm-lock.yaml"
  - "docs-astro/pnpm-workspace.yaml"
  - "docs-astro/public/_redirects"
  - "docs-astro/scripts/setup-content.mjs"
  - "docs-astro/src/content.config.ts"
  - "docs-astro/src/pages/feed.xml.ts"
  - "docs-astro/src/pages/retired.astro"
  - "docs-astro/src/plugins/remark-doc-links.mjs"
  - "docs-astro/src/routeData.ts"
  - "docs-astro/src/utils/doc-slug.mjs"
  - "docs-astro/src/utils/feed-entries.mjs"
  - "docs-astro/src/utils/git-dates.mjs"
  - "docs-astro/src/utils/page-index.mjs"
  - "docs-astro/src/utils/page-status.mjs"
  - "docs-astro/src/utils/repo-mounts.mjs"
  - "docs-astro/src/utils/sidebar.mjs"
  - "docs-astro/tests/build-output/archive.spec.ts"
  - "docs-astro/tests/build-output/dates.spec.ts"
  - "docs-astro/tests/build-output/honesty.spec.ts"
  - "docs-astro/tests/build-output/links.spec.ts"
  - "docs-astro/tests/build-output/llms.spec.ts"
  - "docs-astro/tests/build-output/repo-docs.spec.ts"
  - "docs-astro/tests/fixtures/broken-link/index.html"
  - "docs-astro/tests/lib/dist.ts"
  - "docs-astro/tests/lib/llms-links.mjs"
  - "docs-astro/tests/smoke.spec.ts"
  - "docs-astro/tests/unit/doc-slug.test.mjs"
  - "docs-astro/tests/unit/feed-entries.test.mjs"
  - "docs-astro/tests/unit/git-dates.test.mjs"
  - "docs-astro/tests/unit/llms-links.test.mjs"
  - "docs-astro/tests/unit/page-status.test.mjs"
  - "docs-astro/tests/unit/remark-doc-links.test.mjs"
  - "docs-astro/tests/unit/repo-mounts.test.mjs"
  - "docs-astro/tests/unit/setup-content.test.mjs"
  - "docs-astro/tests/unit/source-link-targets.test.mjs"
  - "docs-astro/tests/unit/tutorial-record.test.mjs"
  - "docs/agents/doc-inventory.json"
  - "docs/ci/build-timings/README.md"
  - "docs/src/tutorials/tut-actor-basics.md"
  - "docs/src/tutorials/tut-first-app.md"
  - "docs/src/tutorials/tut-first-vox-app-checkpoints.md"
  - "docs/src/tutorials/tut-getting-started.md"
  - "docs/src/tutorials/tut-ui-integration.md"
  - "docs/src/tutorials/tut-workflow-durability.md"
  - "docs/src/tutorials/use-a-react-component-from-vox.md"
  - "docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md"
  - "docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md"
  - "docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md"
  - "scripts/docs/tutorial-verify.vox"
  - "scripts/render-durable-animation.vox"
  - "tree-sitter-vox/README.md"
covered_digest: "v1:sha256:f2f261e8642635d0a5b397b085c36d4186da8b9577589f203f38e202fb61539f"
behavior_unverified: 0
overrides_applied: 0
deferred:
  - truth: "Research/findings pages that ship as `status: current` (review WR-06: 64 of 102 `*research*`/`*findings*` architecture pages) get the honesty treatment"
    addressed_in: "Phase 21"
    evidence: "Phase 21 goal: 'The corpus is measured deterministically and shrunk to what deserves to be public, with every disposition decided by the user from evidence'"
  - truth: "The tutorial record goes stale when the command registry or installation.md changes, not only when the tutorial blob changes (review IN-05)"
    addressed_in: "Phase 22"
    evidence: "Phase 22 goal: 'Code changes that break reader-facing docs are caught at PR time by deterministic checks only'"
---

# Phase 20: Deploy Unblock & Public-Surface Honesty Verification Report

**Phase Goal:** Readers and scrapers see the current docs on voxlang.org again, a broken deploy can never silently persist, and the public surface stops presenting research and roadmap notes as current reference.
**Verified:** 2026-10-09T20:10:00Z
**Status:** passed
**Re-verification:** Yes — after the DEPLOY-02 live drill (20-UAT.md). The only covered file changed since the initial report is `.github/workflows/docs-deploy.yml` (WR-05 path glob, PR #640); `docs-astro/tests/unit/workflow-paths.test.mjs` passes 4/4 against it and the drill's recovery run 37982158404 executed that exact file.

Verified against branch `docs-freshness/phase-20-close` (origin/main `43a74412b` + planning commit `757ff9327`, which touches only `.planning/`), the live site https://voxlang.org, and read-only GitHub state. SUMMARY claims were not used as evidence.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A push to `main` touching `docs/src/` produces a green docs-deploy run and the change is live; #462 closed with root cause recorded | ✓ VERIFIED | Run 37960645178: `push` on `main` for `43a74412b` (PR #635 squash, touches `docs/src/`), all four jobs green, failure notifier skipped. Phase-20 artifacts are live (`/retired/` 200 with noindex, Internals banners, `/repo/agents-md/`). #462 CLOSED 2026-10-09T16:56:12Z by the recovery job; brbrainerd's comment records both causes (build breaks May–Sep; expired/under-scoped Cloudflare token from 2026-09-20, fixed by rotating with `Pages:Edit` on `vox-docs`) |
| 2 | A forced deploy failure opens/updates exactly one de-duplicated issue (assignee pinged, no repeat comments); no success within N days raises a ci-liveness alert | ✓ VERIFIED | **Liveness:** `ci-liveness.yml` "Deploy docs" row live. **Escalation (live drill):** dispatch runs 37975768983 and 37975892538 opened #639 once (assignee brbrainerd) and edited it in place to `consecutive: 2` with 0 comments; green push run 37982158404 (298bbe08d, PR #640) closed #639 with one "Recovered:" comment; 0 open `docs-deploy-broken` issues afterwards |
| 3 | Post-deploy smoke checks: home and `/voxup` respond, every llms.txt URL resolves, sampled pages free of retired syntax; a broken llms link fails the check | ✓ VERIFIED | `smoke-test` job runs `tests/smoke.spec.ts` after the Cloudflare deploy (main only). Re-run by the verifier against the live site: **31/31 passed**. `tests/unit/llms-links.test.mjs` asserts that a 404 in the URL list fails `checkUrls` (one of 744 unit tests, all passing locally) |
| 4 | "Last updated" on the deployed site matches the last substantive commit (fmt, regen, bulk ignored), verified through the content symlink | ✓ VERIFIED | `git-dates.mjs` reads `git log -M --name-status -- docs/src` from the repo root (not the symlinked mirror). It skips `.git-blame-ignore-revs` SHAs, `chore(ssot): auto-regenerate`/`style:`/fmt subjects and >100-file commits, and follows renames. Recomputed locally: 236 pages differ from naive last-commit dates. Live samples match the computed dates, not the naive ones. Example: `/adr/001-burn-backend-selection/` shows 2026-03-25T02:55:18Z, not 2026-07-23 (`4e98a0f87`, the 337-file frontmatter strip). The same holds for `/reference/orchestration-unified/`, `/explanation/mcp_serverless_research/` and `/architecture/mesh-phase3-vcs-gossip-plan-2026/` |
| 5 | Status `research/roadmap/deprecated/legacy` → banner + noindex from frontmatter; Internals sidebar only; excluded from every llms variant; overlapping designs superseded | ✓ VERIFIED | Live: a research page has the "Internals — research note" banner and `robots noindex`; deprecated `/api/vox-codegen-ts/` has the "Deprecated:" banner and noindex. All rules come from `page-status.mjs` via `routeData.ts`. A structural sidebar parse of a live page found all 133 research/roadmap pages only inside the `Internals` group. None of the 133 Internals titles appears as a page in live `/llms-full.txt` or `/llms-small.txt`; `/llms.txt`, `/.well-known/llms.txt` and `/.well-known/llms-full.txt` contain none of them (some current pages still cross-link to Internals pages, which is expected). They are absent from `sitemap-0.xml` and `feed.xml`. Both specs carry `status: "deprecated"` and a "Superseded (2026-10-08) by the docs-freshness workstream" notice; `docs-reality-audit-program.md` is `status: "roadmap"` and marked "Dormant", to be reactivated by phase 21 |
| 6 | tut-getting-started links to a working install page; relative `.md` links resolve; repo Markdown renders; source/contract links go to GitHub; CI fails on a broken-link fixture | ✓ VERIFIED | Live tut-getting-started `<main>` links `/reference/installation/` and `#quick-install-from-source` (200, anchor present). In a 46-page live sample, no `<main>` contains a relative `.md` href, and all 175 internal hrefs resolve to 200 (after trailing-slash redirects). `/repo/agents-md/` renders AGENTS.md ("Agents Policy"); contract/source links are `github.com/.../blob/main/...` (200). docs-quality run 37960645034 on main ran the fixture self-test (lychee must exit 2 naming the dead route and the dead anchor) and then the blocking `fail: true` built-site check, both green. Nightly `link_checker.yml` handles external links with a lychee cache |
| 7 | Every tutorial audited against current code, recording the commit it was verified against | ✓ VERIFIED | `contracts/documentation/tutorial-verification.v1.json` lists all 7 tutorials as all-pass with `verified_at_commit: b10e20dfd`. Each `blob_sha` equals HEAD's blob, enforced by `tutorial-record.test.mjs`. Independent check: every `vox <cmd>` in the 7 tutorials resolves to an `active` row in `command-registry.yaml`, and tut-getting-started's Rust 1.98.1 matches `rust-toolchain.toml` and `installation.md` (Node/pnpm minimums link to that page). Snippets are compiled by `ci doctest-md --strict`, green on main. All 7 tutorials carry `status:` |

**Score:** 7/7 truths verified

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | 64 research/findings-named pages ship as `status: current` (WR-06), so the status rule correctly gives them no treatment | Phase 21 | Measure & Prune: user decides every disposition from evidence |
| 2 | Tutorial record freshness keyed only on the tutorial blob (IN-05) | Phase 22 | Deterministic drift gate at PR time |

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `.github/workflows/docs-deploy.yml` | Push deploy, pinned wrangler, least privilege, main-only deploys, edit-in-place notifier, recovery closer | ✓ VERIFIED | `fetch-depth: 0`; `wrangler` 4.146.0 exact pin via `pnpm exec`; deploy/smoke jobs gated to `refs/heads/main` (WR-02 fix); recovery closes only on `push` |
| `.github/workflows/ci-liveness.yml` | Deploy-staleness alert | ✓ VERIFIED | Exercised live (#636) |
| `.github/workflows/docs-quality.yml` | Blocking built-site link check with self-test | ✓ VERIFIED | Green on main (37960645034) |
| `docs-astro/src/utils/git-dates.mjs`, `routeData.ts` | Repo-root git dates through the mirror | ✓ VERIFIED | Live dates match the computation |
| `docs-astro/src/utils/page-status.mjs` | Single status policy | ✓ VERIFIED | Read by routeData, sidebar, sitemap, feed and the llms patch |
| `docs-astro/src/plugins/remark-doc-links.mjs` | Build-time relative-link rewrite and strict gate | ✓ VERIFIED | No relative `.md` hrefs live; source Markdown stays repo-relative |
| `docs-astro/src/utils/repo-mounts.mjs` + mount contract | `/repo/<route>/` pages from a single source | ✓ VERIFIED | `/repo/agents-md/` live |
| `docs-astro/tests/smoke.spec.ts` + `tests/lib/llms-links.mjs` | Live smoke | ✓ VERIFIED | 31/31 live |
| `contracts/documentation/tutorial-verification.v1.json` + `scripts/docs/tutorial-verify.vox` | Generated tutorial record | ✓ VERIFIED | See truth 7 caveat below |

### Key Link Verification

| From | To | Via | Status |
|------|----|-----|--------|
| `build-docs` | `deploy-cloudflare` | `docs-dist` artifact (hidden files included) | WIRED (green live run) |
| `deploy-cloudflare` | `smoke-test` | `needs:` + `BASE_URL=https://voxlang.org` | WIRED |
| any failed job | `notify-on-failure` | `needs: [all four]` + `failure()` | WIRED, never run live |
| green push run | `notify-on-success` | closes `docs-deploy-broken` issues | WIRED, fired live (closed #462) |
| `page-status.mjs` | sidebar / routeData / sitemap / feed / llms patch | imports | WIRED (live output consistent) |
| `git-dates.mjs` | rendered `<time datetime>` | `routeData.ts` middleware | WIRED / FLOWING (live) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Docs unit tests | `node --test --test-reporter=tap "tests/unit/*.test.mjs"` | 744 pass, 0 fail | ✓ PASS |
| Live smoke suite | `BASE_URL=https://voxlang.org npx playwright test tests/smoke.spec.ts` | 31 passed | ✓ PASS |
| Notifier logic (stubbed `gh`, three failures) | extracted `notify-on-failure` script | 1 create, 2 edits, 0 comments, `consecutive: 3` | ✓ PASS (logic only) |
| Live link sample | curl over 46 sitemap pages, 175 internal hrefs | 0 non-200, 0 relative `.md` | ✓ PASS |
| Archive redirect | `curl /archive/vox-lexer/` | 301 → `/retired/` (noindex) | ✓ PASS |
| Live escalation drill | dispatch `simulate_failure` ×2 | not run (user skipped; verifier read-only) | ? SKIP → human |

### Probe Execution

Step 7c: SKIPPED. No `scripts/*/tests/probe-*.sh` exist and no plan declares one.

### Requirements Coverage

| Requirement | Source Plan | Status | Evidence |
|-------------|-------------|--------|----------|
| DEPLOY-01 | 20-01, 20-11 | ✓ SATISFIED | Truth 1 |
| DEPLOY-02 | 20-01, 20-11 | ✓ SATISFIED | Truth 2 (liveness + live escalation drill) |
| DEPLOY-03 | 20-07, 20-09, 20-11 | ✓ SATISFIED | Truth 3 |
| DEPLOY-04 | 20-02, 20-03 | ✓ SATISFIED | Truth 4 |
| HONEST-01 | 20-06 | ✓ SATISFIED | Truth 5 |
| HONEST-02 | 20-03, 20-06, 20-07 | ✓ SATISFIED | Truth 5. Internals scope is status-driven per decision P20-D1 |
| HONEST-03 | 20-03 | ✓ SATISFIED | Truth 5 |
| LINKS-01 | 20-05, 20-08 | ✓ SATISFIED | Truth 6 |
| LINKS-02 | 20-08, 20-09, 20-12 | ✓ SATISFIED | Truth 6 |
| LINKS-03 | 20-05, 20-10 | ✓ SATISFIED | Truth 6 |
| TUT-01 | 20-04 | ✓ SATISFIED | Truth 7 |

No orphaned requirements: all 11 phase-20 IDs are claimed by at least one plan.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| (phase diff `43a74412b`, 90 files) | — | No `TBD`/`FIXME`/`XXX` added; no `TODO`/`HACK`/placeholder outside docs content | — | None |
| `docs-astro/src/plugins/remark-doc-links.mjs` | — | WR-03: warm content-layer cache lets a local rebuild pass the dead-link gate | ⚠️ Warning | Local builds only; CI builds clean (in deferred-items.md) |
| `.github/workflows/docs-deploy.yml`, `docs-quality.yml` | 6–32 | WR-05: hand-copied mount path filters can drift from `discoverMounts()`; `docs-sidebar-section-order.v1.json` does not trigger a deploy | ⚠️ Warning | A newly mounted file's edits could go stale silently (staleness, not a failed deploy). In deferred-items.md |
| `contracts/documentation/tutorial-verification.v1.json` | 4 | Record generated at `c0b7fc9cc`, before the WR-04 fail-open fix (`81458ca5b`), and not regenerated since | ℹ️ Info | Blob SHAs still match; commands independently confirmed registered/active. Flags were not re-checked by the verifier (no `vox-cli` build in the worktree) |
| `.github/workflows/link_checker.yml` | — | The nightly external link check has failed on its scheduled runs (#578 open; last scheduled run 37930815687 on `33642200d`, before the phase merge). The phase's cached version has not had a scheduled run yet | ℹ️ Info | Non-blocking by design (LINKS-03). Watch the first post-merge nightly |
| `docs-astro/tests/smoke.spec.ts` | 11 | Live Internals-exclusion check covers 3 of 5 llms files (`/llms-full.txt`, `/llms-small.txt` are covered only by the build-output `llms.spec.ts`) | ℹ️ Info | Verified clean live by the verifier |

### Gaps Summary

No gaps. All seven success criteria are verified against main, CI runs on main, and the live site. The escalation half of DEPLOY-02, previously human_needed, passed its live drill (see truth 2 and 20-UAT.md). Review warnings WR-03 and WR-05 were recorded in deferred-items.md; WR-05 was fixed after phase close (PR #640). WR-06 (research notes still marked `current`) and IN-05 (tutorial record freshness) are covered by phases 21 and 22.

---

_Verified: 2026-10-09T17:33:23Z; re-verified 2026-10-09T20:10:00Z_
_Verifier: Claude (gsd-verifier)_
