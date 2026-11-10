# Phase 337 Plan 11 Summary: Opt-in BF16 Decode Protocol and Row Padding

## Overview
Implemented Tasks 1 & 2 of Plan 337-11 for opt-in `bf16` decode protocol with fixed row padding on Metal/CPU (Task 3 Metal GPU benchmarking deferred while 0.6B GPU training runs in the background):
- Protocol handling and selection in `crates/vox-plugin-mens-candle-metal/src/inference.rs`:
  - `compute_dtype_for_protocol`: resolves `DecodeProtocol::F32` to the platform's default compute dtype (F32 on CPU/Metal) and `DecodeProtocol::Bf16 { .. }` to `qlora_rs::ComputeDType::BF16`.
  - `supported_protocol`: now accepts both `DecodeProtocol::F32` and `DecodeProtocol::Bf16 { gemm_rows }`.
  - `InferenceEngine::load`: configures block linears under the resolved protocol compute dtype, while keeping the LM head in `ComputeDType::F32` and unpadded (`gemm_rows: None`).
  - Records protocol in `InferenceEngine.protocol`.
- Row padding in `crates/vox-plugin-mens-candle-metal/src/model.rs`:
  - Added `gemm_rows: Option<usize>` field to `QuantizedLinear::Dense`.
  - Implemented `padded_rows(rows, gemm_rows)`: returns `gemm_rows` when `rows < gemm_rows` (e.g., decode steps), or `rows` unchanged when `rows >= gemm_rows` (prefill) or when `gemm_rows` is `None` (f32).
  - Updated `into_inference(self, gemm_rows: Option<usize>)`: propagates `gemm_rows` into `QuantizedLinear::Dense`.
  - In `Dense::forward`: pads inputs to `padded_rows` along row dimension with zeros, executes GEMM, narrows back to live rows, and returns outputs in F32. On CPU, falls back to F32 matmul execution to support non-Metal test environments while preserving BF16 precision truncation on the weights.
- Batch width bound in `crates/vox-plugin-mens-candle-metal/src/inference/batch.rs`:
  - In `InferenceEngine::batch_start`: clamps admitted session width to `gemm_rows` under `DecodeProtocol::Bf16 { gemm_rows }` so concurrent sequences never exceed the padded row bound.
- Ladder shape extension in `crates/vox-plugin-mens-candle-metal/src/decode_dtype_bench.rs`:
  - Extended `SHAPES` to length 5, adding Qwen3-4B (`(2560, 9728, 4096, 1024, 36, 151_936)`), Qwen3-8B (`(4096, 12288, 4096, 1024, 36, 151_936)`), and Qwen3-14B (`(5120, 17408, 5120, 1024, 40, 151_936)`).

## Acceptance Criteria
- `grep -c 'fn compute_dtype_for_protocol' crates/vox-plugin-mens-candle-metal/src/inference.rs` -> 2 (definition + test)
- `grep -c 'implemented by plan 337-11' crates/vox-plugin-mens-candle-metal/src/inference.rs` -> 0 (placeholder replaced)
- `grep -c 'fn padded_rows' crates/vox-plugin-mens-candle-metal/src/model.rs` -> 1
- `grep -c 'Qwen3-4B\|Qwen3-8B\|Qwen3-14B' crates/vox-plugin-mens-candle-metal/src/decode_dtype_bench.rs` -> 3
- `grep -c 'fn metal_bf16_batched_equals_single' crates/vox-plugin-mens-candle-metal/src/model.rs` -> 1

## Verification & Tests
- `cargo test -p vox-plugin-mens-candle-metal --lib -- compute_dtype_for_protocol supported_protocol dense_linear batch::tests bf16_rows`:
  - `supported_protocol_accepts_f32_and_bf16`: PASSED
  - `compute_dtype_for_protocol_picks_bf16_only_for_the_tag`: PASSED
  - `a_bf16_dense_linear_returns_f32`: PASSED (returns F32, within 1e-2 relative error of F32 baseline)
  - `bf16_rows_pad_to_gemm_rows`: PASSED (verifies padding logic and bitwise equality for slice subsets)
  - `bf16_gemm_rows_bounds_batch_width`: PASSED (verifies batch width clamp to `gemm_rows`)
  - All 13 filtered unit tests PASSED.
- `cargo check -p vox-plugin-mens-candle-metal --features metal --tests`:
  - Compiles cleanly with zero errors/warnings.

## Mutation Proof
- Metric: presence of `fn padded_rows` in `crates/vox-plugin-mens-candle-metal/src/model.rs`.
- Count before: 1.
- Mutation: modified `padded_rows` to return `rows` unchanged without padding.
- Observed failure:
  - `model::bf16_dense_tests::bf16_rows_pad_to_gemm_rows` FAILED (`left: 1, right: 8`).
- Restored: returned `padded_rows` to original matching logic.
- Recount after: 1.
- Re-run verification: `model::bf16_dense_tests::bf16_rows_pad_to_gemm_rows` passed ok.

## Deferrals
- Task 3 (Metal GPU benchmarking `decode_linears_by_weight_dtype` and 0.6B baseline comparison) is deferred because a Metal GPU training job (`vox-ml-cli mens train`) is currently active on the GPU.
