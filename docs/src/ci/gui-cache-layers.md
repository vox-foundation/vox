---
title: "GUI cache layers (main-only save)"
description: "How vox-gui UI CI restores pnpm/Vite caches without writing PR-scoped Actions caches (cache-key-lint compliant)."
category: "CI & Quality"
status: "current"
---

# GUI cache layers (main-only save)

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
