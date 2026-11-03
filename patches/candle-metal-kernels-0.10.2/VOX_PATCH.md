# Vox fork notes (candle-metal-kernels 0.10.2)

Upstream: `crates.io` candle-metal-kernels 0.10.2 (verbatim copy, one fix).

## Fix: rank>4 strided reduce read the wrong elements

`src/metal_src/reduce.metal`, `get_strided_idx_fallback`: for tensors of rank
5+, the fallback loop meant to peel the innermost `num_dims - 4` dims indexed
`dims[num_dims - 1 - d]` for `d = 4..num_dims`, i.e. the **outer** dims, and
then ran `strided_indexer<4>` over `dims[0..4)` again. Every strided
`sum`/`max`/`min`/`argmax` over a rank-5+ tensor on Metal returned garbage
(ranks 1-4 take the specialized `switch` arms and were correct).

Impact on Vox: `repeat_kv` (GQA) builds a rank-5 `expand`; its backward is a
`sum_keepdim` over a middle axis of that rank-5 tensor. On Metal, every K/V
LoRA gradient was wrong, grew ~1.5x per layer back toward layer 0, and
overflowed to inf/NaN within the first micro-steps. AdamW then wrote NaN into
every LoRA var at the first optimizer step (even at `lr = 0`, since
`0 * NaN = NaN`), and Metal's NaN-skipping `max` reduce hid the NaNs in the
loss, which read as a finite ~250.

Regression test (runs in the workspace, needs a Metal device):
`cargo test -p vox-plugin-mens-candle-metal --lib metal_backward_tests`.

Drop this patch when upgrading candle to a release whose
`get_strided_idx_fallback` peels `dims[num_dims - 1 - (d - D)]` (or equivalent).
