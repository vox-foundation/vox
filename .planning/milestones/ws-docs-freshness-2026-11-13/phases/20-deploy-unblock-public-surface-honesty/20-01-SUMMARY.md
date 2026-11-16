---
phase: 20-deploy-unblock-public-surface-honesty
plan: 01
subsystem: docs-deploy
tags: [ci, github-actions, cloudflare-pages, wrangler, liveness]

requires: []
provides:
  - "Least-privilege permissions in docs-deploy.yml (contents: read top-level, scoped write for pages and issues)"
  - "Separation of build and deploy: build-docs emits docs-dist artifact consumed by deploy-cloudflare without rebuilding"
  - "Single-issue edit-in-place notification on deploy failure (docs-deploy-broken label, consecutive streak tracking, 0 comments)"
  - "Auto-close of docs-deploy-broken issue on successful push deploy to main"
  - "Exact lockfile pin of approved wrangler@4.146.0 in docs-astro devDependencies and allowBuilds configuration"
  - "Deploy liveness check in ci-liveness.yml alerting if no successful docs-deploy landed within 14 days"
affects: [20-02, 20-09, 20-10, 20-11]

actuals:
  tokens: 15000
  tasks: 3
  commits: 0

tech-stack:
  added: ["wrangler@4.146.0"]
  patterns: ["Least-privilege workflow permissions with job-scoped write tokens", "Edit-in-place managed issue status blocks with HTML comments"]

key-files:
  created: []
  modified:
    - ".github/workflows/docs-deploy.yml"
    - ".github/workflows/ci-liveness.yml"
    - "docs-astro/package.json"
    - "docs-astro/pnpm-workspace.yaml"
    - "docs-astro/pnpm-lock.yaml"

key-decisions:
  - "Approved wrangler@4.146.0 exact devDependency pin per Task 2 human checkpoint."
  - "workerd postinstall explicitly denied (workerd: false in docs-astro/pnpm-workspace.yaml allowBuilds) since pages deploy does not require local workerd execution."
  - "Zero issue comments in deploy failure path to prevent notification noise on repeated failures (#462 resolution pattern)."

requirements-completed: [DEPLOY-01, DEPLOY-02]

coverage:
  - id: D1
    description: "Top-level permissions in docs-deploy.yml are contents: read; pages and id-token are scoped to deploy-pages"
    requirement: "DEPLOY-02"
    verification:
      - kind: automated
        ref: "yq -r '.permissions | to_entries | map(.key + \"=\" + .value) | join(\",\")' .github/workflows/docs-deploy.yml — contents=read"
        status: pass
  - id: D2
    description: "build-docs checks out full history and emits docs-dist artifact without rebuilding in deploy-cloudflare"
    requirement: "DEPLOY-01"
    verification:
      - kind: automated
        ref: "yq -r '.jobs[\"build-docs\"].steps[] | select(.uses == \"actions/checkout@v7\") | .with[\"fetch-depth\"]' — 0"
        status: pass
      - kind: automated
        ref: "yq -r '.jobs[\"deploy-cloudflare\"].steps[].name' .github/workflows/docs-deploy.yml | grep -ciE 'rust|starlight|doc.?s pipeline|sccache' — 0"
        status: pass
  - id: D3
    description: "deploy-cloudflare uses approved, pinned wrangler@4.146.0 via pnpm exec"
    requirement: "DEPLOY-01"
    verification:
      - kind: automated
        ref: "pnpm exec wrangler --version in docs-astro — 4.146.0"
        status: pass
  - id: D4
    description: "ci-liveness.yml monitors deploy freshness and alerts on 14+ day staleness"
    requirement: "DEPLOY-02"
    verification:
      - kind: automated
        ref: "actionlint and yaml-parse-check on .github/workflows/ci-liveness.yml — clean exit 0"
        status: pass
---

# Plan 20-01 Summary

## What Was Done
1. **Restructured `.github/workflows/docs-deploy.yml`**:
   - Scoped top-level permissions to `contents: read`. Added explicit `pages: write` and `id-token: write` only to `deploy-pages`.
   - Updated `build-docs` to checkout full history (`fetch-depth: 0`) and emit the `docs-dist` artifact with hidden files (`.well-known/`) included.
   - Replaced redundant Rust/Starlight compilation in `deploy-cloudflare` with artifact download (`docs-dist`) and verified presence of output assets.
   - Refactored `notify-on-failure` to update a single `docs-deploy-broken` tracking issue in-place using HTML comment markers (`<!-- docs-deploy-status:start -->`), tracking consecutive streaks and eliminating all repeat issue comments.
   - Added `notify-on-success` to automatically close open `docs-deploy-broken` issues on green push deploys.
   - Added `simulate_failure` boolean input to `workflow_dispatch` to allow manual verification of escalation paths.

2. **Approved & Pinned `wrangler@4.146.0`**:
   - Replaced unpinned `npx wrangler` invocation with `pnpm exec wrangler pages deploy dist ...`.
   - Pinned `wrangler@4.146.0` in `docs-astro/package.json` devDependencies.
   - Configured `workerd: false` in `docs-astro/pnpm-workspace.yaml` allowBuilds.
   - Updated `docs-astro/pnpm-lock.yaml` cleanly under pnpm 11.

3. **Enhanced CI Liveness in `.github/workflows/ci-liveness.yml`**:
   - Added "Deploys succeeded recently" monitoring step with a 14-day threshold for `docs-deploy.yml`.
   - Supported on-demand staleness simulation via `deploy_max_age_days` workflow input.

## Verification
- `actionlint` passed with 0 errors across modified workflows.
- `vox ci workflow-concurrency-guard`, `vox ci node-pnpm-ssot-guard`, `vox ci cache-key-lint`, and `vox ci yaml-parse-check` passed cleanly.
- `pnpm exec wrangler --version` confirmed `4.146.0`.
