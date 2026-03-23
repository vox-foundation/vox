---
title: "Crate: vox-wasm"
category: api
last_updated: 2026-03-23
---

# vox-wasm

Workspace **excluded** crate (see root `Cargo.toml` `[workspace].exclude`). Regenerate the JavaScript / WASM package with **wasm-pack** (or your chosen bindgen flow) into `pkg/` when developing this target.

The `pkg/` directory is intentionally ignored by git (see `pkg/.gitignore`); do not commit bindgen output here unless the project policy changes.
