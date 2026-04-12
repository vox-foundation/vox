---
title: "Vox shell operations boundaries"
description: "Where host PowerShell, `vox shell`, and `.vox` std I/O/process primitives each belong — and what Vox is not building (shell emulator)."
category: "architecture"
status: "current"
last_updated: 2026-04-02
training_eligible: true

schema_type: "TechArticle"
---

# Vox shell operations boundaries

Vox is a **language and toolchain**. It does **not** ship a **general-purpose shell emulator** as a product surface. This page names the three lanes agents and contributors should use so responsibilities stay clear.

## Three lanes

| Lane | Use when | Mechanism |
|------|----------|-----------|
| **Host shell** | You are typing or pasting commands in a terminal (IDE, CI step, local automation harness). | Real **`pwsh`** (or the platform shell your workflow uses). Prefer validating risky PowerShell with **`vox shell check`** against [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml). |
| **`vox shell`** | Quick manual smoke of the CLI **or** validating a PowerShell fragment against exec-policy. | Subcommands: **`repl`** (micro-REPL, dev-only) and **`check`** (AST + policy). **`repl`** is **not** a substitute for `pwsh` and does **not** implement pipelines, session `cd`, or robust quoting. |
| **`.vox` programs** | Logic lives in the Vox language (scripts, apps, generated Rust). | Typed **`std.fs`**, **`std.path`**, **`std.process`** (argv-first). Do **not** rely on parsing arbitrary shell command strings in `.vox` as the default pattern. |

## Design principles (LLM-friendly, Vox-native)

1. **Argv-first subprocesses** — `std.process.run` / `run_ex` / `run_capture` take a program name and argument list, not a shell line. This avoids quoting and injection hazards common in generated shell.
2. **Explicit path operations** — compose paths with `std.path.*`; probe kind with `std.fs.exists` / `is_file` / `is_dir`; normalize with `std.fs.canonicalize` when comparing locations.
3. **Resolve tools before spawning** — `std.process.which` resolves an executable on `PATH` to an absolute path when you need deterministic spawn behavior.
4. **Policy at the host boundary** — exec-policy applies to **PowerShell source** checked by `vox shell check`, not to the `repl` passthrough path.

## Explicit non-goals

- A Vox-owned **interpreter** for bash/PowerShell syntax inside `.vox`.
- Growing **`vox shell repl`** into a session-aware shell with pipelines, job control, or policy-gated arbitrary execution.
- Duplicating exec-policy with a second allowlist unless a future product requirement is approved.

## Related references

- CLI: [`docs/src/reference/cli.md`](../reference/cli.md) — `vox shell`.
- Std surfaces: [`docs/src/reference/std-surfaces.md`](../reference/std-surfaces.md).
- Script primitives: [`docs/src/architecture/vox-automation-primitives.md`](vox-automation-primitives.md).
- Policy research: [`terminal-exec-policy-research-findings-2026.md`](terminal-exec-policy-research-findings-2026.md), [`terminal-ast-validation-research-2026.md`](terminal-ast-validation-research-2026.md).
