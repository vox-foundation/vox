---
title: "CLI design rules (SSOT)"
category: architecture
last_updated: 2026-03-21
---

# CLI design rules

Single source for **shipped `vox` CLI** conventions (see also [`ref-cli.md`](../ref-cli.md), [`cli-scope-policy.md`](cli-scope-policy.md), [`cli-reachability-ssot.md`](cli-reachability-ssot.md)).

## Hierarchy and naming

- **One primary tree** of nouns/verbs; avoid near-synonyms (`update` vs `upgrade`) for the same action.
- **Latin-themed group commands** (`fabrica`, `mens`, `ars`, `recensio`) mirror the flat top-level commands for discoverability; legacy top-level names remain **active** (not hidden).
- **Subcommand depth** should stay ≤ 2 for most flows; deeper trees only for dense domains (e.g. `populi corpus`).
- **Retired / deprecated** commands stay in the registry with `status` and doc’d migration (see [`command-surface-duals.md`](../ci/command-surface-duals.md)).

## Help, output, and exit codes

- Every subcommand supports **`--help`**; root supports **`--version`** (via clap on `VoxCliRoot`).
- **Machine-readable / JSON** output belongs on **stdout** where a command documents it; **diagnostics and errors** on **stderr**.
- Prefer **`--json`**, **`--quiet`**, **`--verbose`** on subcommands that emit structured or noisy output; root sets hints via env (`VOX_CLI_GLOBAL_JSON`, `VOX_CLI_QUIET`) when using global flags.
- **Non-zero exits** must mean something actionable (document in help where non-obvious).

## Global flags (root)

- **`--color auto|always|never`** — forwarded to `vox_cli::diagnostics` (`NO_COLOR` still wins when set).
- **`--json`** — sets `VOX_CLI_GLOBAL_JSON=1` for subcommands that honor it.
- **`--verbose` / `-v`** — if `RUST_LOG` is unset, sets it to `debug` before tracing init.
- **`--quiet` / `-q`** — sets `VOX_CLI_QUIET=1` for supported commands.
- **`doctor --json`** is the subcommand’s own machine JSON; **`vox --json doctor`** only sets `VOX_CLI_GLOBAL_JSON` for code paths that read it — do not assume they are interchangeable.

## Completions

- **`vox completions <shell>`** — use **`clap_complete`**; shells: **bash**, **zsh**, **fish**, **powershell**, **elvish**. Install by redirecting stdout to the appropriate completion path for your shell (see [`ref-cli.md`](../ref-cli.md)).

## Adding or renaming commands

1. Implement in `crates/vox-cli` (and internal surfaces as needed).
2. Add or update rows in **`contracts/cli/command-registry.yaml`** (schema: **`contracts/cli/command-registry.schema.json`**).
3. Update **`docs/src/ref-cli.md`** and, for top-level reachability, **`cli-reachability-ssot.md`** when `reachability_required` is not `false`.
4. Run **`vox ci command-compliance`** before merge (also enforced in CI).
