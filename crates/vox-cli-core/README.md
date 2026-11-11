# `vox-cli-core`

Shared lightweight CLI core (`L1`) for the `vox` command-line binary: argv parsing helpers, exit-code policy, output formatting primitives, and common command types.

Extracted from `vox-cli` (`Phase 394`) so subcommand crates (`vox-cli-ci`, `vox-cli-diagnostics`, `vox-cli-research`, `vox-cli-review`, `vox-cli-share`) compile in parallel without circular dependencies on the top-level binary crate.

- **Workspace documentation:** [`README.md`](../../README.md) · [`where-things-live.md`](../../docs/src/architecture/where-things-live.md)
- **License:** [Apache-2.0](../../LICENSE)
