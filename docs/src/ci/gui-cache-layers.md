---
title: "GUI cache layers (main-only save)"
description: "How vox-gui UI CI restores pnpm/Vite caches without writing PR-scoped Actions caches (cache-key-lint compliant)."
category: "CI & Quality"
status: "current"
topic: "ci-guards"
domain: "platform"
authorship: "llm-assisted"
authorship_model: "claude-opus-4-6"
---

# GUI cache layers (main-only save)

<!-- kb:summary -->
Describes GUI CI cache layers for pnpm, Vite, and Playwright artifacts on hosted runners. Explains restore versus save policy, main-only cache writes, and how agents should avoid thrashing the Actions cache budget. Reference for GUI pipeline changes and cache-key lint.
<!-- /kb:summary -->


## Policy

Rust caches (`Swatinem/rust-cache`) and Node package-manager caches save **only from `main`**. PR and `merge_group` jobs may **restore** but must not **save** — enforced by `vox ci cache-key-lint`. The Actions cache budget is hard-capped (~10 GB); PR-scoped keys are write-only garbage.

## `ui` job pattern (`ci.yml`)

- `actions/setup-node` uses `package-manager-cache: false` on PRs.
- pnpm store / Vite cache: restore from a **main-scoped** key when present; never `actions/cache` with `save` on pull_request.
- Nightly / `cache-seed` (or push-to-main) jobs are the writers for layered `pnpm` + Vite artifact keys.

## Local agents

Do not add `cache: pnpm` / `cache: npm` to PR workflows. Prefer:

```yaml
- uses: actions/setup-node@v4
  with:
    node-version-file: crates/vox-gui/ui/package.json
    package-manager-cache: false
```

See [runner-contract.md](runner-contract.md) and AGENTS.md §CI Contract.
