---
title: "vox-dei (workspace-excluded)"
description: "Official documentation for vox-dei (workspace-excluded) for the Vox language. Detailed technical reference, architecture guides, and impl"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# vox-dei (workspace-excluded)

Fragment sources under `src/` are **not** wired into a full library graph yet. A minimal **`Cargo.toml`** + **`src/lib.rs`** exists so:

- Socrates-aligned research floors stay type-checked:  
  `cargo check --manifest-path crates/vox-dei/Cargo.toml`
- `research/orchestrator.rs` can reference [`vox_socrates_policy::ConfidencePolicy`] when that tree is reattached to the crate root.

Do not add `vox-dei` as a dependency of workspace members (`vox-cli`, etc.); see **AGENTS.md**.
