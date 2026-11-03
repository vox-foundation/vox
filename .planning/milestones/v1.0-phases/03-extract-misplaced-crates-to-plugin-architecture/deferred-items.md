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
  needs them. Inventory: `target/d09-inventory.txt`. Two known_inversions that were no longer upward were
  dropped in `3ab57a7f2`, and ROADMAP SC#3/SC#4 now cite `layers.toml` (`9c2409dab`).
- `vox-orchestrator (L3) -> vox-plugin-nvml-probe (L4)`: **resolved** (2026-09-27). The user chose a
  host-registered probe.
  - `042ef4764` adds `vox_orchestrator::models::{VramProbe, register_vram_probe}` and switches `vram.rs`.
    vox-orchestrator-mcp's `server_state::vram_probe` loads the `nvml-probe` plugin and registers it. The
    callers are ServerState new_full/new_for_daemon, `vox chat --model auto` and vox-gui's auto-model
    recommendation.
  - The idle 2026-09-22 edit that blocked `auto_select.rs` was reviewed and committed as `158ad634a`, on user
    approval. `c3527f7ae` then switched `probe_discrete_vram_gb` to the registered probe.
  - `8c6befd34` drops the dependency. That commit also carries the one-line Cargo.lock hunk, the regenerated
    crate-graph, the build-map delta and the baseline edge removal. The build-map delta is vox-plugin-nvml-probe
    dependents 17->0, blast_s 374->1, fan_in 1->0; vox-plugin-sdk dependents 19->2, blast_s 374->1.
  - After it, vox-arch-check has no layer-inversion or cdylib-dep error (only the unrelated
    forbidden_pattern set remains), and `ci crate-edges` prints `OK (699 live in-tree edges within baseline)`.
  - The VRAM-fit signal now needs the nvml-probe plugin installed. It no longer comes from in-process NVML.
- Stale crate-edges baseline entry `vox-populi -> vox-grammar-export`: **fixed** in `07a681ca7` (2026-09-27).
  `--tighten` refuses while the three NEW EDGE violations stand, so the one pair was removed by hand
  (removal-only diff); the stale warning is gone.
- The three crate-edges NEW EDGE violations: **resolved** (2026-09-27).
  - `vox-gui -> vox-db-types`: dependency removed, no exception needed. `2285de1fe` switched the four uses to
    `vox_db`'s re-exports. `2c8c27348` dropped the dependency. That commit also carries the one-line Cargo.lock hunk,
    the regenerated crate-graph and the build-map delta (vox-db-types `fan_in` 3 -> 2).
  - `vox-research-shim -> vox-compiler` and `vox-gui -> vox-research-shim`: user-authorized exceptions,
    `7f1dadf43`.
  - After these, `ci crate-edges` reported only the `vox-orchestrator -> vox-plugin-nvml-probe` layer
    inversion above, and `8c6befd34` removed that edge.
- `docs/src/reference/cli.md:970` retired `VOX_MCP_ORCHESTRATOR_RPC_WRITES`: **fixed** in `7ba445c14`. The line
  now names the status-tool pilot and its RPC_READS umbrella. Only that hunk was committed. The file's other
  uncommitted line documents `vox graph history`. That command is wired only by the uncommitted vox-cli
  `graphify/mod.rs` + command-registry changes, which are still idle.
- **Still open — webhook auth for GitHub/Slack/Discord:** the listener accepts only `Authorization: Bearer`;
  those providers need a relay that adds the header, or per-source HMAC verification as a second path.
