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
- Layer SSOT drift (D-09): **resolved** in `276852014` and `2a2a7e477` (2026-09-27). The two maps used
  different scales (L0-5 vs L0-4) and disagreed on 89 of 119 shared crates; 9 workspace crates were missing
  from the JSON. `layers.toml` is the only layer map now. vox-arch-check, the WTL parity rule, drift-check and
  the scripts already read it. `vox ci crate-edges` now reads its `[crates]` and `[[known_inversions]]`, and
  `contracts/ci/crate-layers.v1.json` is deleted. `vox-plugin-mens-candle-core` moved L3 -> L4. It is an rlib
  linked only by the two L4 Candle plugins, and its Candle deps are unconditional. The 03-02 CORE Candle scan,
  re-keyed on `layers.toml`, prints only the three L4 `vox-plugin-mens-candle-*` crates (core_count=0).
  `vox-speech` (L3), `vox-quantize` (L2) and `vox-populi` (L3) keep their `layers.toml` values; the live graph
  needs them. **Still open:** under the one map, `vox-orchestrator (L3) -> vox-plugin-nvml-probe (L4)` is an
  upward edge. vox-arch-check already failed on it (layer inversion + linked cdylib), and crate-edges now
  reports it too. The fix is a code change (load it through vox-plugin-host) or a user-authorized ledger entry.
  Inventory: `target/d09-inventory.txt`.
- Stale crate-edges baseline entry `vox-populi -> vox-grammar-export`: **fixed** in `07a681ca7` (2026-09-27).
  `--tighten` refuses while the three NEW EDGE violations stand, so the one pair was removed by hand
  (removal-only diff); the stale warning is gone. The three violations themselves remain open.
- **Still open — `docs/src/reference/cli.md:970`** names the retired `VOX_MCP_ORCHESTRATOR_RPC_WRITES`; the file
  carries another session's uncommitted edits, so the one-line fix waits for that session.
- **Still open — webhook auth for GitHub/Slack/Discord:** the listener accepts only `Authorization: Bearer`;
  those providers need a relay that adds the header, or per-source HMAC verification as a second path.
