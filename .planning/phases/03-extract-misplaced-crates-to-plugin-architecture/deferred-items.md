# Phase 03 deferred items (out-of-scope discoveries)

## From 03-02 (2026-09-26)

- **vox-populi `mens-hf-hub` does not compile on HEAD `03cf0e62e`.** `crates/vox-populi/src/mens/hub.rs:201` builds
  `DownloadedModelFiles` on the local-directory path without the `tokenizer_config` and `chat_template` fields
  (E0063). The fields were added in `b9f05601f`/`a7cdfdb8e` (2026-09-17) and this early return was missed. As a
  result `cargo check -p vox-populi --features mens-hf-hub` fails, and so do vox-ml-cli's default build (`mens-base`
  -> `vox-populi/mens-train` -> `mens-hf-hub`) and `vox-ml-cli --features quantize`. The failure does not depend on
  vox-quantize, which is not in the `mens-hf-hub` graph. Fix: fill both fields on the local-dir path, e.g. probe
  `tokenizer_config.json` / `chat_template.jinja` next to `config.json` the same way the tokenizer is probed.

## From 03-05 (2026-09-26)

- **vox-gui's sidecar autobuild cannot build the `vox-ml-cli` sidecar.** `crates/vox-gui/build.rs`
  `autobuild_sidecar` always runs `cargo build -p vox-cli --release --bin <name>`, but the `vox-ml-cli`
  binary belongs to the `vox-ml-cli` package (`crates/vox-ml-cli/Cargo.toml` `[[bin]]`), so a fresh
  checkout fails with `error: no bin target named vox-ml-cli in vox-cli package`. The `vox` sidecar
  autobuilds fine. 03-05 built it by hand (`cargo build -p vox-ml-cli --release --bin vox-ml-cli`, copied
  to `target/release/vox-ml-cli-<triple>`). Fix: pick the package per binary name.

## Resolution status (2026-09-26, phase close-out)

- vox-populi `mens-hf-hub` E0063: **fixed** in `90f408a45` (local-dir branch fills both fields; two tests).
- vox-gui sidecar autobuild package: **fixed** in `4840ff1b7` (package chosen per binary).
- **Still open — layer SSOT drift (D-09):** `docs/src/architecture/layers.toml` and
  `contracts/ci/crate-layers.v1.json` disagree on several crates (e.g. `vox-plugin-mens-candle-core` L3 vs L4,
  and per the verifier also `vox-speech`, `vox-quantize`, `vox-populi`). Under `layers.toml`,
  `vox-plugin-mens-candle-core`'s unconditional Candle deps would be the one SC#4 violation. Needs a decision on
  which file is authoritative and a reconciliation pass.
- **Still open — stale crate-edges baseline entry** `vox-populi -> vox-grammar-export`: run
  `cargo run -q -p vox-cli -- ci crate-edges --tighten` once the three unrelated violations are cleared.
- **Still open — `docs/src/reference/cli.md:970`** names the retired `VOX_MCP_ORCHESTRATOR_RPC_WRITES`; the file
  carries another session's uncommitted edits, so the one-line fix waits for that session.
- **Still open — webhook auth for GitHub/Slack/Discord:** the listener accepts only `Authorization: Bearer`;
  those providers need a relay that adds the header, or per-source HMAC verification as a second path.
