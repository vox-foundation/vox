---
title: "vox-wasm"
description: "Official documentation for vox-wasm for the Vox language. Detailed technical reference, architecture guides, and implementation patterns "
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# vox-wasm

Workspace **excluded** crate (see root `Cargo.toml` `[workspace].exclude`). Regenerate the JavaScript / WASM package with **wasm-pack** (or your chosen bindgen flow) into `pkg/` when developing this target.

The `pkg/` directory is intentionally ignored by git (see `pkg/.gitignore`); do not commit bindgen output here unless the project policy changes.

## Scope boundary

`vox-wasm` is not the current product path for full Vox-on-phone parity.

- Primary mobile product path remains browser-compatible Vox app generation plus remote host control.
- WASM/WASI surfaces are complementary and capability-scoped.
- Any future direct on-device `.vox` runtime must be treated as a reduced subset with explicit unsupported features, not a workstation-equivalent guarantee.
