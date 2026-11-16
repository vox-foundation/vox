# Cargo `[patch.crates-io]` overlays

## `aegis-0.9.8`

Upstream `aegis` runs a `cc` build (preferring `clang-cl` on `x86_64-pc-windows-msvc`) for native AES paths. This fails in minimal Windows dev/CI environments without LLVM.

The fork only changes **default features** to include `pure-rust` (Rust `softaes` backend), matching the approach `turso_core` already uses for Android.

When upgrading the workspace lockfile’s `aegis` version, refresh this tree from crates.io and re-apply the `default` feature line in `Cargo.toml`.

## `candle-core-0.10.2`

Fixes Metal buffer pool unbounded memory growth (`private_buffers` pruning, exact-bucket reuse, and unified-memory cache cap) and skips autograd backward allocation/matmuls for untracked (frozen) operands while preserving exact input/LoRA gradients (`with_no_grad` + `sorted_nodes_and_tracked` + `GradStore::accumulate`). See [`candle-core-0.10.2/VOX_PATCH.md`](candle-core-0.10.2/VOX_PATCH.md).

## `candle-metal-kernels-0.10.2`

Fixes rank>4 strided reduce indexing on Metal (required for GQA `repeat_kv` backward without NaN/Inf). See [`candle-metal-kernels-0.10.2/VOX_PATCH.md`](candle-metal-kernels-0.10.2/VOX_PATCH.md).
