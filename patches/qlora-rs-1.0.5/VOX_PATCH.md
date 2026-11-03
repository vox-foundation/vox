# Vox fork notes (qlora-rs 1.0.5)

Upstream: `crates.io` qlora-rs 1.0.5.

Changes in `src/training.rs`:

1. **`training_step_lm` — middle layers (pre-norm residual)**: For every `QuantizedLinear` except the last (LM head), compute `h <- h + (1/√n_mid) * F(RMSNorm(h))` with `rms_norm_slow` (γ = 1). The per-block `1/√n_mid` factor dampens deep stacks; this approximates a residual path so proxy stacks do not behave like a pure product of linear maps.
2. **`training_step_lm` — depth scale before LM head**: When `n_mid = layers.len() - 1 > 0`, multiply activations by `1/sqrt(n_mid)` again immediately before the final LM-head forward (combined with (1) for stronger magnitude control).
3. **`training_step_lm` — gradient accumulation**: Match `training_step` — scale loss by `1 / gradient_accumulation_steps` and only `backward_step` every N micro-batches.
4. **Gradient accumulation actually accumulates**: candle returns a fresh `GradStore` from every `backward()` and never sums into `Var`s, so upstream's `let _ = loss.backward()` on non-step micro-steps discarded those gradients (with `grad_accum = 8`, 7/8 of rows never trained). All step entry points (`backward_step`, `optimizer_step_with_grads`, `training_step`, `training_step_lm`) now route through `accumulate_and_maybe_step`, which sums per-`Var` grads across the window and clips + steps once. The paged-AdamW path also writes the updated param back (`training_step` previously lost it). Tests: `grad_accumulation_keeps_earlier_micro_step_grads`, `optimizer_step_with_grads_accumulates_across_micro_steps`. Run them from a copy with an empty `[workspace]` table — this crate is not a workspace member.
5. **Debug**: Set `VOX_QLORA_DEBUG_NORMS=1` to `eprintln!` mean absolute activation after each middle block (stderr; for local CUDA ablations only).

Reconcile with upstream when bumping qlora-rs.
