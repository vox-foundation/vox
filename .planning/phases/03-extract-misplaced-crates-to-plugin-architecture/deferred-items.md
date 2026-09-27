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
