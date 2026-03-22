---
title: "CLI scope policy (minimal shipped binary)"
category: architecture
last_updated: 2026-03-21
---

# CLI scope policy

## Shipped binary

The **`vox`** executable built from `crates/vox-cli` is the **minimal compiler CLI**. Its command surface is defined in code (`Cli` in `src/lib.rs`, invoked from `src/main.rs`) and documented in [ref-cli.md](../ref-cli.md). The legacy monolithic dispatch source file was removed to avoid drift; extend the shipped surface only via `lib.rs` / `commands/mod.rs` and feature flags.

**Canonical decision:** The product ships this **minimal** surface by default. A larger command tree under `crates/vox-cli/src/commands/**` exists for future integration; most of it stays **out of** `commands/mod.rs` until wired into `lib.rs` / `main.rs`. **`commands::runtime`** (dev / info / tree / run+test shims / shell) and **`commands::info`** are compiled as **library-visible** modules for reuse; they do **not** add subcommands to the minimal `Cli` until explicitly dispatched.

## Feature-gated commands (minimal `Cli`)

Some variants exist only when Cargo features are enabled (see `crates/vox-cli/Cargo.toml`):

- **`ars`** — `vox openclaw` / `oc` (OpenClaw gateway client; `vox-ars`) and `vox skill` (ARS registry / promote / context). Build with `cargo build -p vox-cli --features ars`.
- **`extras-ludus`** — `vox ludus` (gamification; `vox-ludus`). Build with `cargo build -p vox-cli --features extras-ludus`.
- **`live`** — `vox live` (orchestrator demo bus).
- **`mesh`** — `vox mesh status` / `vox mesh serve` (`vox-mesh` registry + HTTP control plane). Build with `cargo build -p vox-cli --features mesh`.
- **`workflow-runtime`** — interpreted `vox populi workflow run` + `commands::workflow` when enabled; implies **`populi-dei`**. Build with `cargo build -p vox-cli --features workflow-runtime`.

## Documentation

- **Shipped commands** — `ref-cli.md` must match `lib.rs` (`Cli`) / `commands/mod.rs`.
- **Registry + parity** — `contracts/cli/command-registry.yaml` is the machine SSOT; run **`vox ci command-compliance`** (see [`cli-design-rules-ssot.md`](cli-design-rules-ssot.md), [`command-compliance-ssot.md`](../ci/command-compliance-ssot.md)).
- **Broader narrative** — `how-to-cli-ecosystem.md` may describe workspace-wide or planned tooling; it must state clearly when a command is **not** in the minimal binary.

## Tests and scripts

Integration tests and scripts must not assume subcommands that are absent from the minimal `Cli` enum. Prefer `cargo run -p vox-cli -- …` against documented commands only.

## Script migration exceptions

- **Allowed in GitHub workflows without Rust rewrite:** paths under `scripts/` that are **data artifacts** or **explicitly allowlisted** in `docs/agents/workflow-script-allowlist.txt`. CI enforces this via `vox ci workflow-scripts`.
- **Thin shell / PowerShell shims** (`scripts/check_*.sh`, `scripts/populi/*_gate.*`, …) are **delegates** to `cargo run -p vox-cli -- ci …` — keep them one-liners to avoid drift.
- **Host-only tooling** (GPU installers, external marketplace actions, third-party ML stacks) may stay outside `vox ci`; record them in [`docs/agents/script-registry.json`](../../agents/script-registry.json) with `status: "external"` when added.

## Governance

- New **`scripts/...` references** in `.github/workflows/*.yml` must either match the allowlist or the PR must update `workflow-script-allowlist.txt` with an owner note.
- Prefer extending **`vox ci`** for new guards instead of adding long bash matrices.
