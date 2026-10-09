---
phase: 20-deploy-unblock-public-surface-honesty
plan: 01
subsystem: docs-deploy
tags: [ci, deploy, cloudflare, permissions, escalation, liveness, supply-chain]
status: complete

requires: []
provides:
  - "docs-deploy.yml: top-level permissions contents: read; pages/id-token write only on deploy-pages; issues: write only on notify-on-failure / notify-on-success"
  - "docs-deploy.yml: build-docs full-history checkout (fetch-depth 0, no filter), simulate_failure dispatch input, docs-dist artifact (include-hidden-files, 3-day retention); deploy-cloudflare deploys the downloaded artifact with `pnpm exec wrangler` (no Rust/Astro rebuild)"
  - "docs-deploy.yml: notify-on-failure edits ONE open docs-deploy-broken issue in place (managed section between docs-deploy-status markers, first-failure + consecutive markers, assignee brbrainerd, label nightly-failure) with zero comments; notify-on-success closes it with one 'Recovered' comment on a green push run to main only"
  - "ci-liveness.yml: step 'Deploys succeeded recently' opens 'Deploy stale: Documentation' (label nightly-failure, assignee brbrainerd) when docs-deploy.yml has no successful main run in 14 days, closes it when one lands; dispatch input deploy_max_age_days overrides (0 forces stale)"
  - "docs-astro: wrangler 4.146.0 exact devDependency (pnpm 11 lockfile); allowBuilds workerd: false"
affects: [20-11]

actuals:
  tokens: 16800
  tasks: 3
  commits: 2
plan_head_before: efb11f435e6f1c8e87483d2a5de2c19fcd203424

tech-stack:
  added:
    - "wrangler 4.146.0 (devDependency, exact pin; replaces run-time `npx wrangler` resolution)"
  patterns:
    - "Edit-in-place escalation: one issue per failure streak, a delimited managed body section rewritten each run, text outside the markers preserved (keeps #462's history)"
    - "Every dynamic workflow value passed through `env:`; no `${{ }}` inside any `run:` script"
    - "Build once, deploy the artifact: Cloudflare job consumes build-docs' docs-dist instead of rebuilding"

key-files:
  created: []
  modified:
    - .github/workflows/docs-deploy.yml
    - .github/workflows/ci-liveness.yml
    - docs-astro/package.json
    - docs-astro/pnpm-lock.yaml
    - docs-astro/pnpm-workspace.yaml

key-decisions:
  - "User approved the wrangler pin verbatim: \"approve wrangler@4.146.0\" (published 2026-10-01T16:34Z, newest release >=7 days old at 2026-10-09T08:31Z; 4.147.0 was 6d21h old, 4.149.0 latest was ~14h old)"
  - "allowBuilds workerd: false — the only build script pnpm reported for wrangler's tree; its postinstall fetches the local Workers runtime, which `pages deploy` never starts, and deploy-cloudflare carries the CF token. Proven unnecessary: a from-scratch frozen install in a clean copy ran `wrangler pages deploy --help` (exit 0) without it"
  - "esbuild stays true (pre-existing entry; allowBuilds is per package name, so it also covers wrangler's pinned esbuild 0.28.1)"
  - "notify-on-success requires github.event_name == 'push' (P20-D7): a manual dispatch never closes the tracking issue"

duration: "~57 min wall (2026-10-09T08:21Z → 09:18Z, including the version-approval wait)"
completed: 2026-10-09
---

# Phase 20 Plan 01: Deploy Unblock Workflow Summary

**Least-privilege docs deploy that builds once and hands `docs-dist` to a lockfile-pinned `pnpm exec wrangler` (4.146.0), escalates failures by editing a single `docs-deploy-broken` issue in place, auto-closes it on the next green push run, and is watched by a 14-day "Deploy stale" liveness check.**

## Performance

- Duration: ~57 min wall clock, including the Task 2 approval wait
- Tasks: 3/3 (Task 1 tracer, Task 2 user checkpoint, Task 3 auto)
- Files modified: 5

## Accomplishments

- **Permissions (T-01-1):** the top level is exactly `contents: read`. `pages: write` and `id-token: write` are granted only under `jobs.deploy-pages.permissions`. `issues: write` is granted only on the two notify jobs. No job that runs `pnpm install` can mint an OIDC token.
- **Artifact hand-off:** build-docs uploads `docs-dist` with `include-hidden-files: true`, so `.well-known/llms.txt` survives the upload. deploy-cloudflare does a sparse checkout of `docs-astro`, runs a pnpm 11 frozen install, downloads the artifact, and fails if `index.html` or `.well-known/llms.txt` is missing. It no longer runs Rust, sccache, system deps, the doc pipeline or the Starlight build. That job previously took 13m42s because it rebuilt everything.
- **Build-docs:** it now checks out full history with `fetch-depth: 0` (needed for DEPLOY-04 dates) and has a `simulate_failure` step. The dead `Archive docs` tar step and the non-blocking lychee and guidance steps are removed. The `.github/workflows/docs-deploy.yml` path is added to `on.push.paths`.
- **Escalation (T-01-5):** the failure job builds the managed section from `env:` only, using `toJSON(needs)` to list the failed jobs. If no tracking issue is open, it creates one. Otherwise it replaces or appends the managed section, increments `consecutive`, keeps `first-failure`, adds assignee `brbrainerd` and adds the `nightly-failure` label. It never calls `gh issue comment`.
- **Recovery:** notify-on-success closes every open `docs-deploy-broken` issue with one "Recovered: RUN_URL (EVENT, SHA)." comment. It runs only for green push runs on main.
- **Liveness (T-01-6):** the new ci-liveness step has `if: always()` and lists `Documentation|docs-deploy.yml|14`. It looks up the last successful main run with `gh run list --branch main --status success`. A stale result is reported as an issue, not as a job failure. A non-numeric override fails the step.
- **Supply chain (T-01-2, T-01-3):** wrangler is pinned exactly at 4.146.0 in a pnpm 11 lockfile (`lockfileVersion: '9.0'`). The lockfile only adds 55 packages and removes none. Existing snapshots were re-keyed because wrangler's `supports-color` now satisfies `debug`'s optional peer. No existing version changed.

## Task Commits

1. **Task 1 (tracer): restructure docs-deploy.yml and add deploy liveness** (`f848c4fa6`)
2. **Task 2: user approves the wrangler version.** This was a checkpoint with no commit. The approval was "approve wrangler@4.146.0".
3. **Task 3: pin wrangler 4.146.0 and deploy with the lockfile binary** (`e5fdf388c`)

## Verification

- `actionlint` on both workflows: clean, both after Task 1 and after Task 3.
- yq checks:
  - Top-level permissions are `contents=read`.
  - deploy-pages `id-token` is `write`, and the build-docs checkout `fetch-depth` is `0`.
  - Rust, Starlight, doc-pipeline or sccache step names in deploy-cloudflare: `0`.
  - `gh issue comment` in the failure path: `0`.
  - `${{` inside `run:` across both workflows: `0`.
  - notify-on-success `if:` contains `github.event_name == 'push'`, and both notify jobs set `timeout-minutes: 5`.
  - ci-liveness keeps its top-level `permissions:`.
- Acceptance greps: `simulate_failure` type is `boolean`, and the workflow path is in `on.push.paths`. The failure job has the markers `docs-deploy-status:start`, `first-failure:`, `consecutive:`, `--add-assignee brbrainerd` and `--add-label nightly-failure`. ci-liveness has `Documentation|docs-deploy.yml|14`, `Deploy stale:`, `--status success` and `deploy_max_age_days`. `nightly-report.yml` is unchanged.
- Offline drill of the failure script with a stubbed `gh`:
  - The seed was an existing #462 body with CRLF line endings.
  - Run 1 appended the managed section with `consecutive: 1` and kept the old history.
  - Run 2 replaced the section rather than duplicating it, set `consecutive: 2` and kept `first-failure`.
  - The `gh` calls were label create ×2, issue list, issue view and issue edit, with no comment calls.
- Repo guards, run with the worktree build `cargo run -q -p vox-cli -- ci …`:
  - workflow-concurrency-guard: OK.
  - node-pnpm-ssot-guard: OK.
  - cache-key-lint: OK.
  - yaml-parse-check on the changed YAML files: OK.
  - ssot-drift: OK, including workflow-policy-guard and the nested SSOT guards.
  - workflow-scripts: exit 1 on known pre-existing workflows only (nightly.yml, release-installers.yml, os-compat-report.yml, ci.yml). docs-deploy.yml and ci-liveness.yml are not in its output.
- Task 3 checks:
  - `pnpm@11 install --frozen-lockfile` exits 0 with no `ERR_PNPM`, both in `docs-astro` and in a from-scratch copy (package.json, lockfile and workspace yaml only), as CI runs it.
  - `devDependencies.wrangler` is `4.146.0`, an exact `X.Y.Z` pin.
  - `pnpm exec wrangler --version` prints `4.146.0`, and `pages deploy --help` exits 0.
  - `grep -c 'npx wrangler'` is `0`.
  - The lockfile header is `lockfileVersion: '9.0'`.
  - `git status` shows nothing outside files_modified.
- Live behaviour is deferred to 20-11's final checkpoint after push: a forced failure should produce one issue and no comments, recovery should close it, and the liveness override should open a stale issue.

## Deviations from Plan

### Execution adjustments (no behaviour change)

**1. [Rule 3 - Blocking] The guards ran with the worktree-built `vox`, not the installed one.**
- **Found during:** Task 1 verify.
- **Issue:** the installed `vox` (9a2575123) refuses to run guards because it is stale relative to the working tree.
- **Fix:** I ran every `vox ci …` guard as `cargo run -q -p vox-cli -- ci …`, as the plan allows. `yaml-parse-check` requires file arguments, so I passed the changed YAML files.

**2. The ssot-drift throwaway-worktree step was skipped.**
- This phase worktree carries no other session's dirty files. At the time of the run, only the two workflow files were modified. So I ran ssot-drift directly in the worktree, and it passed.

**3. Shell-wrapper hazard: StrReplace saw a mangled view of ci-liveness.yml.**
- **Fix:** I rewrote the file whole from its verbatim content plus the additions. The diff was 41 insertions and 0 deletions. For the one-line Task 3 workflow change, I used an anchored `sed` replacement. Every staged diff was checked for injected wrapper banners (none), and line 1 of each file was checked.

**4. Extra verification beyond the plan.**
- I added the stubbed-`gh` drill of the edit-in-place script and the from-scratch frozen install that proves `workerd: false` is enough.

None of these change the plan's intended behaviour.

## Threat Model Coverage

- **T-01-1:** mitigated by job-scoped Pages and OIDC permissions.
- **T-01-2:** mitigated by the exact, user-approved lockfile pin and `pnpm exec`.
- **T-01-3:** mitigated by `workerd: false`. The CF secrets are only in the deploy step's `env:`.
- **T-01-4:** mitigated because every dynamic value goes through `env:`; the verify grep found 0.
- **T-01-5:** mitigated by edit-in-place with zero failure comments.
- **T-01-6:** mitigated by the deploy-success-age check with a dispatch override.
- **T-01-7:** accepted. The artifact is the public site, kept for 3 days.

## Known Stubs

None.

## Next Phase Readiness

- 20-11 can run the live drills:
  1. Dispatch with `simulate_failure=true` twice and expect one issue with no new comments.
  2. Wait for a green push run, which should close the issue, including #462 while it still carries `docs-deploy-broken`.
  3. Dispatch ci-liveness with `deploy_max_age_days=0` and expect a `Deploy stale: Documentation` issue.
- DEPLOY-01 itself closes in 20-11, after a push-triggered green run (P20-D7).

## Self-Check: PASSED

- FOUND: .github/workflows/docs-deploy.yml, .github/workflows/ci-liveness.yml, docs-astro/package.json, docs-astro/pnpm-lock.yaml, docs-astro/pnpm-workspace.yaml
- FOUND commits: f848c4fa6, e5fdf388c (`git rev-list --count efb11f435..HEAD` = 2 before this summary commit)
