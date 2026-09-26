# Phase 03 deferred items (out-of-scope discoveries)

## From 03-02 (2026-09-26)

- **vox-populi `mens-hf-hub` does not compile on HEAD `03cf0e62e`.** `crates/vox-populi/src/mens/hub.rs:201` builds
  `DownloadedModelFiles` on the local-directory path without the `tokenizer_config` and `chat_template` fields
  (E0063). The fields were added in `b9f05601f`/`a7cdfdb8e` (2026-09-17) and this early return was missed. As a
  result `cargo check -p vox-populi --features mens-hf-hub` fails, and so do vox-ml-cli's default build (`mens-base`
  -> `vox-populi/mens-train` -> `mens-hf-hub`) and `vox-ml-cli --features quantize`. The failure does not depend on
  vox-quantize, which is not in the `mens-hf-hub` graph. Fix: fill both fields on the local-dir path, e.g. probe
  `tokenizer_config.json` / `chat_template.jinja` next to `config.json` the same way the tokenizer is probed.
