# vox-bootstrap

**Single place** for machine bootstrap logic used by `scripts/install.sh` and `scripts/install.ps1`.

- Probes Rust, MSVC/C compiler, and **clang / clang-cl** (needed for `turso` → `aegis` native builds).
- Optional **`--apply`**: `rustup component add` (with `--dev`), `winget install LLVM.LLVM` on Windows (with `--install-clang`).
- **`plan --json`**: stable machine-readable manifest for CI/docs tooling.

Full project setup (API keys, wasm target, Codex) remains **`vox setup`** in the main CLI when that binary is built.

```bash
cargo run -p vox-bootstrap -- --help
cargo run -p vox-bootstrap -- plan --json
```
