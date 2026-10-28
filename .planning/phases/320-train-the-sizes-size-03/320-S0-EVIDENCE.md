# Phase 320 Stage S0: binaries, Metal plugin, plan-only on the live reading

Recorded 2026-10-28 (UTC) by Plan 320-04 Task 1. Raw files are in `evidence/s0-*`.

## Build provenance (D-09)

- Build commit: `900ceb5bf35e5895e7f4d01ed0ee70400dc0ef06` (`evidence/s0-build-commit.txt`), written before any build.
- All three release builds end in `Finished` (`evidence/s0-build-{ml,plugin,vox}.txt`, last 200 lines each):
  - `cargo build -p vox-ml-cli --release --features gpu,execution-api`: Finished in 4m 55s.
  - `cargo build -p vox-plugin-mens-candle-metal --release --features metal`: Finished in 2m 34s.
  - `cargo build -p vox-cli --release --bin vox`: Finished in 10m 59s. The `vox` build did not fail, so the copy-in fallback was not needed.
- Versions:
  - `vox-ml-cli 0.6.0 (900ceb5bf)` (`evidence/s0-ml-version.txt`)
  - `vox 0.6.0+build.7421 (900ceb5bf)` (`evidence/s0-vox-version.txt`)
- sha256 of the binaries:
  - `target/release/vox-ml-cli`: `be0c1a2fdd65cac7e67d6f74b08995f40225e7e29f3ef5f4139ff15df83dd2e5`
  - `target/release/vox`: `54fb67ab6b6780f2ac2df2a7e8c0e15a13764926f274fa07bfcc59785ddd198f`
- `vox doctor` (`evidence/s0-doctor.txt`, exit 1 on unrelated checks: Docker daemon, Graphify cache):
  - `✓ vox-ml-cli build  .../target/release/vox-ml-cli matches vox build 900ceb5bf`
  - `✓ Binary freshness  installed vox build 7421 matches the working tree`
  - `✓ GPU Discovery  Delegated to vox-ml-cli`
  - It names no other feature set for training, so no rebuild was needed.

## Metal plugin reinstall (D-09)

- Backup of the previous install: `$TMPDIR/gsd-320-04/plugin-backup/0.6.0/`, i.e. `/var/folders/lp/cmsx7ph161vgpqb09z7mxs6r0000gn/T/gsd-320-04/plugin-backup/0.6.0/`. That is the stale build: dylib sha256 `47fc1208ef23cde14d6e6c97891044ad82c113b88b990cf8a8196fb97e8aee3d`, 32,600,032 bytes, dated 2026-09-22. To restore it, copy it back into `~/Library/Application Support/vox/plugins/mens-candle-metal/`.
- The staging directory held `crates/vox-plugin-mens-candle-metal/Plugin.toml` (id `mens-candle-metal`, version `0.6.0`, `abi-version = 13`) and `target/release/libvox_plugin_mens_candle_metal.dylib` (the `--features metal` build).
- `vox plugin install --path <stage> --yes` printed `✓ Installed plugin 'mens-candle-metal' v0.6.0 (2 files) → .../plugins/mens-candle-metal/0.6.0`. No remove step was needed (`evidence/s0-plugin-install.txt`).
- Byte check (`evidence/s0-plugin-sha256.txt`). `cmp` is blocked by the shell allowlist, so this used `diff -q` plus sha256:
  - built: `68b80d73f45f494e983b2f108f7846740e4e2938ff88dcb3290e64cee24419e4`
  - installed: `68b80d73f45f494e983b2f108f7846740e4e2938ff88dcb3290e64cee24419e4`
  - `diff -q` exit 0, so the two are IDENTICAL (32,849,136 bytes).
- `vox plugin doctor` (`evidence/s0-plugin-doctor.txt`) reports host ABI 13 and `✓ mens-candle-metal v0.6.0 — ok`. Its exit 1 comes only from the installed `mens-candle-cuda` plugin (ABI 12, and it requires `nvidia-gpu`). That plugin is out of scope here.

## Lane prefix in the built code (Phase 313)

The run was `cargo nextest run -p vox-plugin-mens-candle-core --lib -E 'test(training_text)'`. Result: **12 tests run, 12 passed**, 68 skipped, 0 failed (`evidence/s0-lane-prefix-tests.txt`). This includes `training_text::tests::direct_answer_puts_empty_think_in_the_prompt_not_the_loss`.

## S0 automatic pick and refusal parity (D-02)

There is no `--model`. The data dir is empty: `$TMPDIR/gsd-320-04/s0-empty`. Both runs used `--backend qlora --tokenizer hf --device metal --require-gpu --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal`.

| Run | Exit | Pick line (stderr) | Refusal |
|-----|------|--------------------|---------|
| `--plan-only --json` | 1 | `size: picked 32B: largest size whose floor 60000 MB fits 60094 MB free (Metal working set)` | `Corpus file not found: .../s0-empty/train.jsonl` |
| real run | 1 | `size: picked 32B: largest size whose floor 60000 MB fits 60005 MB free (Metal working set)` | `Corpus file not found: .../s0-empty/train.jsonl` |

- Live reading: about 60,000 MB free. The source is `Metal working set`, which is live, not a fallback. The automatic pick is **32B** (floor 60,000 MB). The margin is under 100 MB, so the pick depends on the moment: the reading moved 89 MB between the two runs.
- The two `Corpus file not found: <path>` lines are identical (`diff -q`). Neither stderr contains `Downloading base model` (count 0 in each), so the refusal comes before any download. Neither run wrote stdout.
- The real run also wrote the Plan 03 `train-record.json` with `status: not_trained`, the same corpus error, and `size`/`pin` null (`evidence/s0-auto-real-train-record.json`).

This closes Phase 318's carried human item 1: the live automatic pick on this Mac is on record.

Stage S0: PASS
