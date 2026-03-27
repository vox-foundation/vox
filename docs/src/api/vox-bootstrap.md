---
title: "vox-bootstrap"
description: "Official documentation for vox-bootstrap for the Vox language. Detailed technical reference, architecture guides, and implementation patt"
category: "reference"
last_updated: 2026-03-24
training_eligible: true
---

# vox-bootstrap

**Single place** for machine bootstrap logic used by `scripts/install.sh` and `scripts/install.ps1` (both repository and cargo-free install paths).

- Probes Rust, MSVC/C compiler, and **clang / clang-cl** (needed for `turso` → `aegis` native builds).
- Optional **`--apply`**: `rustup component add` (with `--dev`), `winget install LLVM.LLVM` on Windows (with `--install-clang`).
- Optional **`--install`**: installs `vox` after probe success.
  - Default path is **binary-first** from GitHub Releases: fetches **`/releases/latest`** JSON for `tag_name`, downloads `vox-<tag>-<host-triple>.tar.gz` or `.zip` plus `checksums.txt`, verifies SHA-256, then writes **`~/.cargo/bin/vox`** atomically (HTTP requests time out after **120s**).
  - **Source fallback:** `cargo install --locked --path crates/vox-cli` from repo root discovered via **`VOX_REPO_ROOT`** or by walking up from the current directory until `crates/vox-cli/Cargo.toml` exists. If only standalone `vox-bootstrap` is present (no local repo + Cargo), source fallback cannot run.
  - `--source-only` skips the binary attempt.
  - `--version <tag>` pins the release tag (for example `v1.2.3`); when omitted, “latest” still uses real `tag_name` in the asset basename (not `vox-latest-…`). Contract: [binary release contract](../ci/binary-release-contract.md).
- **`plan --json`**: stable machine-readable manifest for CI/docs tooling.

Full project setup (API keys, wasm target, Codex) remains **`vox setup`** in the main CLI when that binary is built.

Install scripts use a tiered launcher strategy:

1. repo checkout + Cargo available: `cargo run --locked -p vox-bootstrap -- ...`
2. `vox-bootstrap` already on `PATH`: execute it directly
3. otherwise: download `vox-bootstrap-<tag>-<triple>.*`, verify checksum, run it

```bash
cargo run -p vox-bootstrap -- --help
cargo run -p vox-bootstrap -- plan --json
```
