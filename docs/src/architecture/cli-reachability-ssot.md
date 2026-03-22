---
title: "CLI command reachability (SSOT)"
category: architecture
last_updated: 2026-03-22
---

# CLI command reachability

This page maps **`vox` subcommands** in [`crates/vox-cli/src/lib.rs`](../../crates/vox-cli/src/lib.rs) to their **implementation modules** under [`crates/vox-cli/src/commands/`](../../crates/vox-cli/src/commands/).

## Reachable from default / feature matrix

| CLI variant | Feature gate | Handler module |
|-------------|--------------|----------------|
| `build` | default | `commands::build` |
| `check` | default | `commands::check` |
| `test` | default | `commands::test` |
| `run` | default | `commands::run` |
| `script` | `script-execution` | `commands::runtime::run::script` |
| `dev` | default | `commands::dev` |
| `live` | `live` | `commands::live` |
| `bundle` | default | `commands::bundle` |
| `fmt` | default | `commands::fmt` (not implemented; fails with doc pointer; see `ref-cli.md`) |
| `install` | default | `commands::install` (not implemented; fails with doc pointer; see `ref-cli.md`) |
| `lsp` | default | `commands::lsp` |
| `doctor` | default / `codex` | `commands::doctor` or `commands::diagnostics::doctor` |
| `architect` | `codex` or `stub-check` | `commands::diagnostics::tools::architect` |
| `snippet` | default | `commands::extras::snippet_cli` |
| `share` | default | `commands::extras::share_cli` |
| `codex` | default | `commands::codex` |
| `db` | default | `commands::db` + `commands::db_cli` dispatch |
| `scientia` | default | `commands::scientia` (facade over `db_cli` research helpers) |
| `openclaw` | `ars` | `commands::openclaw` |
| `skill` | `ars` | `commands::extras::skill_cmd` |
| `ludus` | `extras-ludus` | `commands::extras::ludus_cli` |
| `stub-check` | `stub-check` | `commands::stub_check` |
| `ci` | default | `commands::ci` |
| `populi` | `populi-base` or `gpu` | `commands::populi` |
| `populi oratio`, `ai oratio` | `populi-oratio` | `commands::populi::oratio_cmd`, `commands::ai::oratio` |
| `review` | `coderabbit` | `commands::review` |
| `island` | `island` | `commands::island` |
| `train` | `gpu` + `populi-dei` | `commands::ai::train` |
| `mesh` | `mesh` | `commands::mesh_cli` |

## `vox-compilerd` RPC (not CLI variants)

Daemon dispatch lives in [`crates/vox-cli/src/compilerd.rs`](../../crates/vox-cli/src/compilerd.rs). Methods call **`commands::build`**, **`check`**, **`bundle`**, **`fmt`**, **`doc`**, **`test`**, **`run`**, **`dev`** — not the removed `commands/compiler/` tree.

## Removed / non-compiled trees (historical)

The following directories under `commands/` were **not** referenced from `commands/mod.rs` or the CLI and have been **removed** to reduce dead surface:

- `commands/compiler/` — duplicate of canonical `build` / `check` / `doc` / `fmt` / `bundle` paths used by `compilerd` and CLI.
- `commands/pkg/` — unwired package manager experiment.
- `commands/serve_dashboard/` — superseded by `vox-codex-dashboard` / extension flows.
- `commands/infra/` — unwired deploy/execution subtree.
- `commands/learn.rs`, `commands/dashboard.rs` — orphan modules with no `mod` declaration.

## Shared subtrees

- `commands::runtime` — used by `run` (script lane), `dev` re-exports, and feature-gated script execution.
- `commands::extras` — snippet, share, skill, ludus, ARS helpers.
