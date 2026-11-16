# Phase 427 Summary: Metal Memory Model Slope Calibration & Ladder Floor Measurement

**Status:** Complete  
**Workstream:** `mens-nov2026-v73` (`Milestone v7.3`)  
**Phase:** `427-metal-calibration`

## Delivered Scope

### Wave 3A — Kernel Vectorization & Calibration Step Control
- **`crates/vox-plugin-mens-candle-{core,metal,cuda}` (`training_loop/mod.rs`, `finalize.rs`, `model.rs`):**
  - Wired `max_runtime_secs` and `capped: bool` cleanly through QLoRA training summaries for deterministic short calibration probes.
  - Replaced scalar inner loops in Qwen3.5/Qwen3.8 causal depthwise `conv1d` and Gated DeltaNet linear attention with vectorized tensor ops and chunked recurrence (`DELTANET_CHUNK_SIZE = 64`).
  - Measured speedup on `Qwen/Qwen3.5-2B` (`seq_len = 512`): first backward step dropped from `~21.3s/step` to `~2.80s/step`.

### Wave 3B — Sequential Metal Calibration & Ladder Floor Probes (`target/mens-cal/*`)
Executed 7 sequential single-GPU Metal probes on Apple M5 Max (`recommendedMaxWorkingSetSize = 115,448,725,504` B / `110,101` MiB, `maxBufferLength = 86,586,540,032` B):
- `target/mens-cal/2b-s512`: `Qwen/Qwen3.5-2B` (`24x2048`, r16, `seq=512`) -> `9,633,126,196` B (`9,187` MiB, `2.80 s/step`)
- `target/mens-cal/2b-preset`: `Qwen/Qwen3.5-2B` (`24x2048`, r16, `seq=1024`) -> `19,649,785,102` B (`18,740` MiB, safe `floor_mb = 19,500`, `3.20 s/step`)
- `target/mens-cal/2b-s2048`: `Qwen/Qwen3.5-2B` (`24x2048`, r16, `seq=2048`) -> `21,900,136,876` B (`20,886` MiB, `12.72 s/step`)
- `target/mens-cal/4b-preset`: `Qwen/Qwen3.5-4B` (`32x2560`, r16, `seq=1024`) -> `34,697,153,390` B (`33,090` MiB, safe `floor_mb = 34,000`, `10.22 s/step`)
- `target/mens-cal/9b-preset`: `Qwen/Qwen3.5-9B` (`32x4096`, r16, `seq=1024`, untied 248k `lm_head`) -> `35,281,560,652` B (`33,648` MiB, safe `floor_mb = 36,000`, `14.01 s/step`)
- `target/mens-cal/27b-s1024`: `Qwen/Qwen3.8-27B` (`64x5120`, r32, `seq=1024`, NF4 linear + BF16 embed/head) -> `77,073,260,172` B (`73,503` MiB, safe `floor_mb = 74,000`, `41.40 s/step`)
- `target/mens-cal/27b-preset` (spill ref): `Qwen/Qwen3.8-27B` (`64x5120`, r32, `seq=2048`) -> `118,280,866,252` B (`112,802` MiB > `110,101` MiB static Metal limit, `143.42 s/step`)

### Wave 3C — Slope Calibration, Config Contracts & Pre-428 Seams
- **Two-point `candle-metal` activation slope fit (`fit_a_lane`):** `(21,900,136,876 - 9,633,126,196) / ((2048 - 512) * 24 * 2048) = 162.48240311940512` B/LHT persisted with `source: measured` in `contracts/mens/memory-model.v1.yaml` and `crates/vox-populi/src/mens/tensor/memory_model.rs` / `calibration.rs`.
- **Portable measurement records:** Written to `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.json` and `docs/src/architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.md` using relative paths only.
- **Preset & floor updates:** Updated `crates/vox-populi/src/mens/tensor/preset_schema.rs` (`qwen38_27b` `seq_len = 1024`), `mens/config/gpu-specs.yaml` (`qwen3_5_code`, `strong_code_default`, `agentic_default` with `floor_measured: true`, `measured_on: silicon-gpu-metal`), `crates/vox-ml-cli/src/commands/mens/{probe.rs,size_train.rs}`, and `crates/vox-ml-cli/src/commands/schola/train/gpu.rs`.
- **Pre-428 seams:** Added `qwen3_5_code` explicit pin fallback in `crates/vox-ml-cli/src/commands/schola/train/run_train.rs` (`pick_train_size`) so `train-record.json` populates `pin`, `size`, and `preset`, and updated `crates/vox-populi/src/mens/hub_card/facts.rs` (`RunRecords::load` + unit test) so `reasoning_compare` reports for `qwen3.5-*` and `qwen3.8-*` run directories are retained cleanly.

## Verification
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/measurements/2026-11-13-candle-metal-qwen35-calibration.md` -> `exit 0` (`no hard errors`)
- `cargo test -p vox-populi --features mens --lib` -> `270 passed; 0 failed; 1 ignored` (`exit 0`)
- `cargo test -p vox-populi --features mens --test training_presets_yaml_contract` -> `4 passed; 0 failed` (`exit 0`)
- `cargo test -p vox-ml-cli --features mens-candle-metal --lib` -> `495 passed; 0 failed; 2 ignored` (`exit 0`)
- `cargo build --release -p vox-ml-cli --features mens-candle-metal` -> `exit 0`
- `./target/release/vox-ml-cli mens probe --detailed --model Qwen/Qwen3.5-2B@15852e8c16360a2fea060d615a32b45270f8a8fc` -> `verdict: FITS` (`exit 0`)
- `./target/release/vox-ml-cli mens probe --detailed --model Qwen/Qwen3.8-27B@37561816e66b5b9153b1b64c10968c18c1e6f4a0` -> `verdict: FITS` (`exit 0`)
