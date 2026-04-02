---
title: "Crate API: vox-dei"
description: "Minimal workspace crate for DeI-aligned constants; legacy sources on disk are not wired into the library graph."
category: "reference"
last_updated: 2026-04-02
training_eligible: true
---

# Crate API: vox-dei

`crates/vox-dei` is a **workspace member** with a minimal `src/lib.rs` (Socrates-aligned floors). Fragment directories under `src/` (for example `research/`, `selection/`) are **not** part of the compiled module tree yet.
The actual orchestrator logic lives in `vox-orchestrator`.

- Type-check: `cargo check -p vox-dei`
- When legacy modules are reattached, they should export through `lib.rs` deliberately.

## Modules

- `research_policy` - Constants for Socrates research and evidence policies.

Do not add `vox-dei` as a dependency of `vox-cli` or other shipped binaries without an explicit product decision; CI enforces `vox ci no-vox-dei-import` on `vox-cli` sources.
