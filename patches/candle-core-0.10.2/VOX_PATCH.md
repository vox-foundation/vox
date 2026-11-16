# Vox Patch: `candle-core-0.10.2`

1. **`src/metal_backend/device.rs`**:
   - Exact bucket lookup (`buf_size(size)`) in `find_available_buffer` so small persistent tensors (e.g. LoRA grad accumulators) never steal and pin multi-hundred-MB `lm_head` / MLP buffers; aligns buffers `> 16 MiB` to `16 MiB` multiples instead of `next_power_of_two` (preventing `2.25 GiB` attention buffers from rounding up to `4.00 GiB`).
   - Prunes unused `private_buffers` (`Arc::strong_count == 1`) alongside `buffers` when Metal allocation exceeds `cache_limit_bytes` (default 78% of `recommendedMaxWorkingSetSize`, overridable via `CANDLE_METAL_CACHE_LIMIT_MB`), preventing unbounded `MTLStorageModePrivate` pool growth and macOS SSD swap thrashing on 32B+ QLoRA training.
   - Adds `MetalDevice::live_allocated_size()` to sum live `Arc<Buffer>` allocations (`Arc::strong_count > 1`) directly across `buffers` and `private_buffers` (returning `None` on allocator lock contention), excluding both cached free-list entries and pool-evicted `MTLBuffer`s awaiting async `MTLCommandBuffer` retirement.
2. **`src/backprop.rs`, `src/custom_op.rs`, `src/tensor.rs` & `src/lib.rs`**:
   - Skips computing and storing gradients for non-`track_grad` operands (e.g. frozen NF4/BF16 base weights in `Op::Matmul`, frozen `RmsNorm` scales, RoPE `cos`/`sin`, causal masks) while preserving exact `dX` cotangent propagation.
   - `GradStore::accumulate` avoids allocating a redundant `zeros_like()` buffer + `0 + grad` kernel when inserting the first gradient for a tensor; `GradStore::retain_vars` drops non-leaf entries immediately after final-segment backward.
   - Runs 1st-order `Tensor::backward` under `candle_core::with_no_grad` so backward ops never construct transient 2nd-order `BackpropOp` edges.
   - Adds `Tensor::softmax_last_dim` (`FusedSoftmaxLastDim` `CustomOp1` backed by `candle_metal_kernels::call_last_softmax` + analytical backward) so causal attention retains 1 output buffer instead of 4 decomposed softmax intermediates.
