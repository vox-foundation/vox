# vox-cli

The binary surface and command-line entrypoint for the Vox programming language (`v0.6.0` / `v6.8`).

## Modular CLI Architecture (`v6.7` / `v6.8`)

`vox-cli` acts as the thin top-level binary dispatcher delegating domain logic to focused subcommand crates and the declarative operations catalog ([`contracts/operations/catalog.v1.yaml`](../../contracts/operations/catalog.v1.yaml), 1,056 cataloged operations across CLI, MCP, GUI, and HTTP surfaces):

| Crate | Responsibility |
|-------|----------------|
| `vox-cli-core` | Shared CLI context, output formatting, configuration, and telemetry plumbing |
| `vox-cli-ci` | Workspace governance, SSOT drift gates, doc sync, and CI verification subcommands |
| `vox-cli-model` | Model registry inspection, OpenRouter/local MENS routing, and model lifecycle |
| `vox-cli-db` | `VoxDb` schema inspection, migrations (`BASELINE_VERSION = 97`), and database ops |
| `vox-cli-diagnostics` | `vox doctor`, `vox audit`, health checks, and environment diagnostics |
| `vox-cli-research` | Deep research, knowledge base ingestion, and corpus evaluation commands |
| `vox-ml-cli` | MENS QLoRA training (`Candle`), dataset preparation, and evaluation workflows |

## Discoverability & Tolerant Routing

Command discovery is clap-derived and backed by the canonical CLI command registry ([`contracts/cli/command-registry.yaml`](../../contracts/cli/command-registry.yaml)) and capability index ([`contracts/capabilities/capability-index.v1.json`](../../contracts/capabilities/capability-index.v1.json)):

```bash
vox commands --recommended
vox commands --format json --include-nested
vox explain <target>
vox features
```

- **`vox commands --recommended`**: Curated starter commands for new users and agents without hiding advanced subcommands.
- **`vox explain`**: Explains compiler diagnostics, language constructs, or CLI operations.
- **`vox features`**: Displays compiled feature flags and active runtime capabilities.
- **Tolerant CLI routing**: Resolves common agent/user synonym invocations via `contracts/cli/semantic-aliases.v1.json`.

Canonical references:
- CLI Reference: [`docs/src/reference/cli.md`](../../docs/src/reference/cli.md)
- Generated Command Surface: [`docs/src/reference/cli-command-surface.generated.md`](../../docs/src/reference/cli-command-surface.generated.md)
- Installation Guide: [`docs/src/reference/installation.md`](../../docs/src/reference/installation.md)

## Key Files

| Path | Purpose |
|------|---------|
| `src/main.rs` | Binary entrypoint → `vox_cli::run_vox_cli()` |
| `src/lib.rs` | Top-level clap parser, semantic alias resolution, and crate dispatch |
| `src/commands/` | Subcommand wiring and dispatch tables |
| `src/templates.rs` | Project scaffolding templates for `vox init` |
