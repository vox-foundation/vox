# vox-bootstrap

**Single place** for machine bootstrap logic used by `scripts/install.sh` and `scripts/install.ps1`.

- Probes Rust, MSVC/C compiler, and **clang / clang-cl** (needed for `turso` → `aegis` native builds).
- Optional **`--apply`**: `rustup component add` (with `--dev`), `winget install LLVM.LLVM` on Windows (with `--install-clang`).
- Optional **`--install`**: installs `vox` after checks.
  - Binary-first from GitHub Releases: resolves **latest** `tag_name` via the GitHub API so downloaded assets are named `vox-<tag>-<triple>.*` (not `vox-latest-*`); verifies SHA-256 against `checksums.txt`; HTTP timeout **120s**; install uses a temp file + rename in `~/.cargo/bin`.
  - Source fallback: `cargo install --locked --path crates/vox-cli` from repo root (**`VOX_REPO_ROOT`** or upward search for `crates/vox-cli/Cargo.toml`).
  - Use `--source-only` to skip binary install.
  - Use `--version <tag>` to pin a specific release.
  - SSOT: [`docs/src/ci/binary-release-contract.md`](../../docs/src/ci/binary-release-contract.md).

`scripts/install.sh` / `scripts/install.ps1` now support a standalone launcher path: if not running from a repo checkout with Cargo, they download `vox-bootstrap-<tag>-<triple>.*`, verify checksums, and execute this same binary.
- **`plan --json`**: stable machine-readable manifest for CI/docs tooling.

Full project setup (API keys, wasm target, Codex) remains **`vox setup`** in the main CLI when that binary is built.

```bash
cargo run -p vox-bootstrap -- --help
cargo run -p vox-bootstrap -- plan --json
```
