---
phase: 20-deploy-unblock-public-surface-honesty
plan: 11
subsystem: docs-site
tags: [docs, deploy, smoke, llms, cloudflare-pages, ci-liveness, escalation]
status: complete

requires:
  - 20-01 (docs-deploy.yml notify jobs + simulate_failure input; ci-liveness deploy_max_age_days input)
  - 20-12 (public/_redirects /archive/* -> /retired/ 301)
  - 20-02..20-10 (everything the live smoke checks assert about)
provides:
  - "docs-astro/tests/lib/llms-links.mjs: extractLlmsUrls(text, {origin, baseUrl}), checkUrls(urls, fetcher) -> { ok, failures[] }"
  - "docs-astro/tests/smoke.spec.ts: /voxup + /voxup.ps1, llms URL resolution for /llms.txt, /.well-known/llms.txt, /.well-known/llms-full.txt, retired-syntax scan of 12 sampled reader pages, Internals routes absent from llms files, `archive redirect` (live only)"
  - "First green push-triggered docs deploy since 2026-05-12; #462 closed with root cause"
affects: [21, 22]

actuals:
  tokens: 2800
  tasks: 3
  commits: 2
plan_head_before: 5245bd54132f044e83b8a06b5785726f7c24018d

tech-stack:
  added: []
  patterns:
    - "llms link checker is a pure helper with an injected fetcher, so the 404/throw cases are unit-tested without a network"
    - "Live-only assertions skip on localhost BASE_URL rather than fail, because astro preview never applies _redirects"

key-files:
  created:
    - docs-astro/tests/lib/llms-links.mjs
    - docs-astro/tests/unit/llms-links.test.mjs
  modified:
    - docs-astro/tests/smoke.spec.ts

key-decisions:
  - "DEPLOY-02 stays Pending: the user skipped the two-forced-failure drill, which the requirement's 'escalates once ... no repeat comments' and this plan's must_have both require live"
  - "DEPLOY-01 and DEPLOY-03 marked complete on live evidence (push run 37960645178, smoke 31 passed incl. archive redirect, #462 closed with root-cause comment)"
---

# Phase 20 Plan 11: Live smoke checks and real-world phase close Summary

Post-deploy smoke now checks the installers, every same-origin URL in all three llms files, retired syntax on 12 reader pages, Internals routes absent from llms, and the `/archive/` 301. The first green push-triggered deploy since 2026-05-12 landed, #462 closed with its root cause recorded, and the liveness drill opened and closed its alert. The failure-escalation drill was not run.

## What was built

- **Task 1 (`d8b886f9b`)**: `tests/lib/llms-links.mjs`. `extractLlmsUrls` collects same-origin Markdown and bare URLs, dedupes them, and rewrites the origin to `BASE_URL`. `checkUrls` runs at most 8 requests at once and reports non-2xx responses and thrown errors. A unit test proves that one 404 fails the check. Smoke tests cover `/voxup` (200, starts with `#!`), `/voxup.ps1` (200), and link resolution for `/llms.txt`, `/.well-known/llms.txt` and `/.well-known/llms-full.txt`.
- **Task 2 (`83bf3e5bd`)**: The smoke test scans 12 sampled pages for retired syntax: `/`, the 7 tutorials, and 4 reference pages. Each page's `<main>` text is checked line by line against the 22 `retired-symbols.v1.yaml` patterns plus the `@endpoint(kind` and `@py.import` spellings, using the same carve-outs as the CI check. Each llms file must list no Internals route. The `archive redirect` test asserts `/archive/<x>` returns 301 to `/retired/`, and it skips on localhost.
- **Task 3 (human-action, live)**: The phase deployed from `main` and the live drills ran. Evidence follows.

These commits were made on `docs-freshness/phase-20`. The whole branch was squash-merged as PR [#635](https://github.com/vox-foundation/vox/pull/635) to `main` at `43a74412b`, and the tree is identical to the old branch head. This SUMMARY is committed on `docs-freshness/phase-20-close`, which is based on `43a74412b`. `commits: 2` counts the plan's own commits, measured as `git rev-list --no-merges 5245bd541..docs-freshness/phase-20 --grep='(20-11)'`. The raw ledger range also includes review fixes and a `main` merge, and since the squash `5245bd541` is no longer an ancestor of `HEAD`.

## Verification

### Local

- `node --test tests/unit/llms-links.test.mjs`: 7 passed, 0 failed (re-run at SUMMARY time). This includes the 404 case, the thrown-error case and the concurrency cap of 8.

### Pre-merge (PR #635)

- Review fixes landed before the merge: `b619027f4` (noindex pages kept out of RSS), `6c5da2ce6` (production deploy only from `main`), `81458ca5b` (the tutorial flag check fails when it cannot run), and `62388d41a`. The last one renamed a `path` local in `tutorial-verify.vox`. stdlib-coverage had misread it as the unregistered `path` stdlib module, which failed CI's stdlib-coverage parity check and the vox-audit no-regression test.
- `ea0c02ba7` regenerated doc-inventory. `f029780f3` (README/index readme-sync) came from another agent session.
- All PR checks were green, and the merge queue merged it as `43a74412b` at 2026-10-09T16:39Z.

### Step 1: push-triggered deploy (DEPLOY-01, DEPLOY-03)

- Run [37960645178](https://github.com/vox-foundation/vox/actions/runs/37960645178) (push, `43a74412b` == `origin/main`) concluded **success**.
  - Build, Deploy to GitHub Pages, Deploy to Cloudflare Pages: success.
  - Smoke test live site: **31 passed**. `archive redirect` **ran and passed** (156 ms), so it was not skipped.
  - Close docs deploy issue on recovery: success. Notify on failure: skipped.
- `curl https://voxlang.org/archive/p20-redirect-probe/` returns `301 https://voxlang.org/retired/` (re-checked at SUMMARY time). `/repo/agents-md/` returns 200.
- This is the first successful push-triggered deploy since 2026-05-12, out of 476 runs since 2026-03-04.

### Step 2: #462 (DEPLOY-01)

- [#462](https://github.com/vox-foundation/vox/issues/462) auto-closed at 2026-10-09T16:56:12Z. The bot comment reads "Recovered: …37960645178 (push, 43a74412b…)". State re-checked: `CLOSED`.
- The approved root-cause comment is posted: [issuecomment-6085846292](https://github.com/vox-foundation/vox/issues/462#issuecomment-6085846292).

### Step 3: failure drill (DEPLOY-02 escalation) was NOT exercised

- The user explicitly chose to skip this drill. Its steps were two `simulate_failure=true` dispatches, then checks for one open issue, an unchanged comment count, `consecutive: 2`, the assignee and the labels, and finally a recovery run.
- Evidence for the edit-in-place escalation is limited to 20-01's local and unit tests, plus the live auto-close on recovery observed in Step 2. Single-issue de-duplication, the absence of repeat comments, and the `consecutive` counter have **not** been verified live.
- Open `docs-deploy-broken` issues: 0 (re-checked).

### Step 4: liveness drill (DEPLOY-02 liveness half)

- Dispatch `deploy_max_age_days=0`, run [37966035022](https://github.com/vox-foundation/vox/actions/runs/37966035022): success. It opened [#636](https://github.com/vox-foundation/vox/issues/636) "Deploy stale: Documentation" with the `nightly-failure` label, assigned to brbrainerd.
- Default dispatch, run [37966245145](https://github.com/vox-foundation/vox/actions/runs/37966245145): success. #636 **CLOSED** at 2026-10-09T17:27:30Z with the comment "Successful deploy landed at 2026-10-09T16:39:07Z." (re-checked).

## Requirement status

| Req | Status | Basis |
|---|---|---|
| DEPLOY-01 | Complete | Green push deploy from `main` (37960645178), #462 closed, root-cause comment posted |
| DEPLOY-02 | **Pending (partial)** | The liveness half is verified live (#636 opened, then closed). The "escalates once, single de-duplicated issue, no repeat comments" half needs the failure drill, which the user skipped. |
| DEPLOY-03 | Complete | The live smoke job (31 passed) checks the home page, `/voxup`, llms URL resolution and retired syntax on sampled pages |

## Deviations from Plan

- **[User decision] Failure drill skipped (Task 3, step 3).** The must_have "Two forced failures yield exactly one open issue with no new comments, `consecutive` = 2 …" is unmet live, so DEPLOY-02 is not marked complete. To close it, run step 3 as written in this plan (two `gh workflow run docs-deploy.yml --ref main -f simulate_failure=true`, then a normal dispatch) and record the results.
- **[Process] Branch change after squash-merge.** Plan commits live on `docs-freshness/phase-20`. The SUMMARY and state updates are committed on `docs-freshness/phase-20-close`, based on `43a74412b`.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: `docs-astro/tests/lib/llms-links.mjs`, `docs-astro/tests/unit/llms-links.test.mjs`, `docs-astro/tests/smoke.spec.ts`
- FOUND commits: `d8b886f9b`, `83bf3e5bd` (on `docs-freshness/phase-20`), `43a74412b` (on `main`)
