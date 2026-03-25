---
title: "vox-bootstrap"
description: "Official documentation for vox-bootstrap for the Vox language. Detailed technical reference, architecture guides, and implementation patt"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# vox-bootstrap

**Single place** for machine bootstrap logic used by `scripts/install.sh` and `scripts/install.ps1`.

- Probes Rust, MSVC/C compiler, and **clang / clang-cl** (needed for `turso` → `aegis` native builds).
- Optional **`--apply`**: `rustup component add` (with `--dev`), `winget install LLVM.LLVM` on Windows (with `--install-clang`).
- Optional **`--install`**: installs `vox` after probe success.
  - Default path is **binary-first** from GitHub Releases: fetches **`/releases/latest`** JSON for `tag_name`, downloads `vox-<tag>-<host-triple>.tar.gz` or `.zip` plus `checksums.txt`, verifies SHA-256, then writes **`~/.cargo/bin/vox`** atomically (HTTP requests time out after **120s**).
  - **Source fallback:** `cargo install --path crates/vox-cli` from repo root discovered via **`VOX_REPO_ROOT`** or by walking up from the current directory until `crates/vox-cli/Cargo.toml` exists.
  - `--source-only` skips the binary attempt.
  - `--version <tag>` pins the release tag (for example `v1.2.3`); when omitted, “latest” still uses real `tag_name` in the asset basename (not `vox-latest-…`). Contract: [binary release contract](../ci/binary-release-contract.md).
- **`plan --json`**: stable machine-readable manifest for CI/docs tooling.

Full project setup (API keys, wasm target, Codex) remains **`vox setup`** in the main CLI when that binary is built.

```bash
cargo run -p vox-bootstrap -- --help
cargo run -p vox-bootstrap -- plan --json
```
