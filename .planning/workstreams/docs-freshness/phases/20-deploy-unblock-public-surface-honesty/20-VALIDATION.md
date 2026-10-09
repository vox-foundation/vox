---
phase: "20"
slug: "deploy-unblock-public-surface-honesty"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: validated
nyquist_compliant: true
wave_0_complete: true
created: "2026-10-08"
---

# Phase 20 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `20-RESEARCH.md` §Validation Architecture, adjusted for decisions in `20-CONTEXT.md`.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Playwright `@playwright/test` ^1.63 + `node --test` unit tests (docs-astro); `cargo test -p vox-doc-pipeline`; `vox ci` gates |
| **Config file** | `docs-astro/playwright.config.ts` (testDir `./tests`); build-output specs live in `tests/build-output/*.spec.ts` |
| **Quick run command** | `cd docs-astro && node --test tests/unit/ && npx playwright test tests/build-output/` (after `pnpm build`) |
| **Full suite command** | `cd docs-astro && pnpm build && node --test tests/unit/ && npx playwright test tests/build-output/ && BASE_URL=https://voxlang.org npx playwright test tests/smoke.spec.ts` |
| **Estimated runtime** | unit ~5 s; build ~140 s; build-output spec ~20 s; live smoke ~60 s |

---

## Sampling Rate

- **After every task commit:** `vox ci pre-push` (fast tier) + the unit/build-output spec for the touched area.
- **After every plan wave:** `pnpm build` + `build-output.spec.ts`; manual dispatch for workflow changes.
- **Before `/gsd-verify-work`:** a green **push-triggered** `docs-deploy.yml` run including the extended smoke job, plus the forced-failure dispatch check.
- **Max feedback latency:** ~180 s locally (build-bound).

---

## Requirement → Verification Map

The planner assigns task IDs; each task's `<automated>` verify must use one of these commands.

| Requirement | Secure behavior / threat ref | Test type | Automated command | Exists |
|-------------|------------------------------|-----------|-------------------|--------|
| DEPLOY-01 | — | ops | `gh run list --repo vox-foundation/vox --workflow docs-deploy.yml --branch main --event push --limit 1 --json conclusion -q '.[0].conclusion'` = `success`; `gh issue view 462 --repo vox-foundation/vox --json state -q .state` = `CLOSED` | n/a |
| DEPLOY-02 | Issue body built only from `env:`-passed run URLs (V5); `issues: write` job-scoped (V4) | workflow e2e | Two `gh workflow run docs-deploy.yml -f simulate_failure=true` → exactly one open issue labelled `nightly-failure`, assigned `brbrainerd`, comment count unchanged; next green run closes it. `ci-liveness.yml` with a test override of N=14 raises then clears a stale-deploy issue. `vox ci ssot-drift` stays green. | ✅ (live drill, 20-UAT.md) |
| DEPLOY-03 | — | live e2e + negative | `BASE_URL=https://voxlang.org npx playwright test tests/smoke.spec.ts` (home, `/voxup`, every llms.txt URL, retired-symbol scan on sampled pages); negative fixture with a 404 URL must fail | ✅ |
| DEPLOY-04 | — | unit + build-output | `node --test tests/unit/git-dates.test.mjs` (ignored revs, autoregen commits, renames); build-output: ≥90% of `dist/**/index.html` have `<time datetime` | ✅ |
| HONEST-01 | — | build-output | research page has banner + `<meta name="robots" content="noindex">`; current page has neither | ✅ |
| HONEST-02 | — | build-output | research/roadmap links appear only under `Internals` in the sidebar; no Internals URL in any `dist/**/llms*.txt`; Internals pages present in the pagefind index and labelled (P20-D2); `dist/robots.txt` has a valid `Sitemap:` | ✅ |
| HONEST-03 | — | lint + grep | `cargo run -p vox-doc-pipeline -- --lint-only --paths <changed>`; the 2 specs + 1 plan carry `status: deprecated` + superseded note; Docs Reality Audit page `status: roadmap` (P20-D6) | n/a |
| LINKS-01 | — | unit + build-output | `node --test tests/unit/remark-doc-links.test.mjs` (sibling, `../x/y.md#a`, `index.md`, `README.md`, `qwen-3.7`→`qwen-37`, symlink realpath); `dist/tutorials/tut-getting-started/index.html` contains `href="/reference/installation/"`; no `href="….md"` in `dist/**/*.html` outside code blocks | ✅ |
| LINKS-02 | — | unit + build-output + smoke | repo Markdown rendered under `/repo/` (P20-D9), e.g. `dist/repo/agents-md/index.html` from root `AGENTS.md`; `.rs` / contract links → `https://github.com/vox-foundation/vox/blob/main/…` (dirs `/tree/main/`, `file.rs:N` → `#LN`); live `/repo/agents-md/` 200 | ✅ |
| LINKS-03 | — | CI step + mutation | `docs-quality.yml` lychee `--offline --root-dir docs-astro/dist --include-fragments` with `fail: true`; same command over dist + `tests/fixtures/broken-link/` exits non-zero | ✅ |
| TUT-01 | — | doctest + scripted | `cargo run -p vox-cli -- ci doctest-md --strict` over `docs/src/tutorials`; each command/flag in the per-tutorial punch list checked via `vox <cmd> --help`; the generated tutorial verification record (P20-D10) lists all 7 tutorials with a commit sha | ✅ |
| Security | Top-level `permissions: contents: read`; `pages`/`id-token` only on the Pages deploy job; `wrangler` pinned in lockfile | config check | `vox ci ssot-drift` (workflow-permissions guard); `rg -n 'npx wrangler' .github/workflows` uses the pinned devDependency | n/a |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [x] `docs-astro/tests/build-output/*.spec.ts` (archive, dates, honesty, links, llms, repo-docs): fs assertions over `dist/`, wired into `docs-quality.yml` after "Build Starlight" (that job needs `fetch-depth: 0`).
- [x] `docs-astro/tests/unit/git-dates.test.mjs`: extract a pure `parseGitLog(text, ignore)` from `git-dates.mjs`.
- [x] `docs-astro/tests/unit/remark-doc-links.test.mjs` + fixture tree; shared `src/utils/doc-slug.mjs`.
- [x] `docs-astro/tests/lib/llms-links.mjs` + `tests/unit/llms-links.test.mjs`.
- [x] `docs-astro/tests/fixtures/broken-link/` + lychee mutation step.
- [x] `simulate_failure` dispatch input in `docs-deploy.yml`.
- [x] Fix the ~70 already-dead link targets before lychee becomes blocking.

---

## Manual-Only Verifications

| Behavior | Requirement | Why manual | Test instructions |
|----------|-------------|------------|-------------------|
| Cloudflare token scope/expiry | DEPLOY-01 / Security | Dashboard-only | Confirm the token has only `Account › Cloudflare Pages › Edit` on one account; set an expiry reminder |
| Visual check of banners and the Internals group | HONEST-01/02 | Rendering judgement | Open one research page and one current page on the deployed site in light and dark themes |

---

## Validation Sign-Off

- [x] All tasks have `<automated>` verify or Wave 0 dependencies
- [x] Sampling continuity: no 3 consecutive tasks without automated verify
- [x] Wave 0 covers all MISSING references
- [x] No watch-mode flags
- [x] Feedback latency < 180 s
- [x] `nyquist_compliant: true` set in frontmatter

**Approval:** validated 2026-10-09

---

## Validation Audit 2026-10-09

| Metric | Count |
|--------|-------|
| Gaps found | 0 |
| Resolved | 0 |
| Escalated | 0 |

Evidence: `node --test "tests/unit/*.test.mjs"` 748/748 locally; docs-quality run 37982158338 on main `298bbe08d` green (build, lychee self-test + assert, internal links, unit tests, `tests/build-output/`); docs-deploy push run 37982158404 green incl. live smoke; DEPLOY-02 escalation proven by the live drill (20-UAT.md). Manual-only rows unchanged.
