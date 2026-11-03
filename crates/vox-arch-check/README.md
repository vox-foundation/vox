# `vox-arch-check`

Standalone Cargo workspace layered-architecture linter and dependency-boundary guard from the [Vox](https://github.com/vox-foundation/vox) project.

## What It Enforces

- **Layered dependency ordering (`layers.toml`):** Assigns every workspace crate to a numeric layer (`0..=5`) and rejects lower-to-higher layer dependency inversions unless explicitly ledgered.
- **Fan-in and LoC growth budgets:** Tracks reverse-dependent counts (`max_dependents`) and crate growth deltas (`max_loc`).
- **Orphan & metadata hygiene:** Verifies crate descriptions, documentation lookup tables (`where-things-live.md`), and forbidden direct dependencies (`[[forbidden_deps]]`).
- **Forbidden source patterns (`[[forbidden_pattern]]`):** Blocks unapproved subprocess spawns, raw `unsafe` blocks, or banned imports across the workspace.

## License

Licensed under the [Apache License, Version 2.0](../../LICENSE).
