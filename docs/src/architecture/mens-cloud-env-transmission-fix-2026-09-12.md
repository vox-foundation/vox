---
title: "Cloud Env Transmission Fix"
description: "How batch_size/seq_len reach rented cloud training workers, and why the old silent-divergence gap existed."
category: "Architecture SSOTs"
status: "current"
---

## What I found (Step 1: the real remote execution contract)

The remote side is **not** external to this repo. `infra/containers/entrypoints/populi-entrypoint.vox`
is the actual script baked into the Docker image (`Dockerfile.populi`) as `/entrypoint.sh`'s
CMD/ENTRYPOINT (per the comment header in `vast.rs`: "the `onstart` script calls `/entrypoint.sh`").
Both `vast.rs`'s `build_onstart_script` and `runpod_provider.rs` rely on this same image/entrypoint —
`vast.rs`'s onstart body is literally `exec /entrypoint.sh`, which just runs whatever env vars were
injected through.

`populi-entrypoint.vox`'s `"train"` match arm invokes the real remote training command directly:

```
process.run("vox", [
    "mens", "train",
    "--backend", "qlora", "--tokenizer", "hf",
    "--model", model_id, "--preset", "auto",
    "--data-dir", "/workspace/data", "--output-dir", "/workspace/output",
    "--device", "cuda"
])
```

Before this fix, it passed no sizing flags at all, so `vox mens train`'s own `resolve_effective_profile`
(`crates/vox-populi/src/mens/tensor/preset_schema.rs`) re-derived batch_size/seq_len from its own
VRAM-tiered preset ladder — which is exactly the "estimate and dispatch can never disagree" gap a
code review flagged: the local dispatch decision (`min_vram_mb_for_training` in
`crates/vox-populi/src/mens/cloud/resolver.rs`, which sizes the GPU rental via `spec.batch_size`/
`spec.seq_len`) and the value the rented worker actually trains with could diverge.

`vox mens train`'s CLI (`crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs`) already
exposes `--batch-size <usize>` and `--seq-len <usize>` as `Option<usize>` overrides that win over the
preset (`crates/vox-ml-cli/src/commands/schola/train/gpu.rs`: `cli_overrides.seq_len`/`batch_size` fill
the slot before falling back to the profile). So the correct fix is a real command-line construction
point in this repo, not a brand-new external contract.

Gradient-checkpointing needed no equivalent fix: the local sizing decision
(`min_vram_mb_for_training` → `min_vram_mb_for_cuda` → `CalKey::new(Lane::CandleCuda, true)`) already
hardcodes gradient-checkpointing **on** for its VRAM estimate — the local decision is already sized
against the more conservative assumption regardless of what the real training run picks. There is no
`gradient_checkpointing` field on `CloudJobSpec`, and the actual runtime toggle
(`VOX_MENS_GRADIENT_CHECKPOINTING`) can only ever shrink memory use versus the sizing assumption, so
it can't reproduce the "remote picks something LARGER than verified" bug.

## What was fixed

1. `crates/vox-populi/src/mens/cloud/vast.rs` (`build_env_map`): for `JobKind::Train`, injects
   `VOX_MENS_BATCH_SIZE` and `VOX_MENS_SEQ_LEN` from `spec.batch_size`/`spec.seq_len` (mirroring the
   existing `VOX_MENS_*` env-var convention already used elsewhere for training knobs).
2. `crates/vox-populi/src/mens/cloud/runpod_provider.rs` (`build_env`): identical fix for RunPod, same
   env var names, same `JobKind::Train` guard, for provider parity.
3. `infra/containers/entrypoints/populi-entrypoint.vox`: the `"train"` arm now reads
   `VOX_MENS_BATCH_SIZE`/`VOX_MENS_SEQ_LEN` and appends `--batch-size <n>`/`--seq-len <n>` to the
   `vox mens train` invocation when present, so the value the local dispatch decision verified is
   exactly what the rented worker uses.

## Tests

Added to both provider files: `build_env_map_transmits_batch_size_and_seq_len_for_train_jobs`
(and the RunPod equivalent) build a `CloudJobSpec::new_train` with `batch_size = 7`, `seq_len = 999`,
and assert the built env map contains `VOX_MENS_BATCH_SIZE = "7"`/`VOX_MENS_SEQ_LEN = "999"`; a
companion test asserts both keys are absent for a `new_serve` spec.

## Verification

- `cargo test -p vox-populi --features mens-cloud --lib`: 311 passed, 0 failed, 1 ignored.
- `cargo clippy -p vox-populi --features mens-cloud --lib -- -D warnings`: 4 pre-existing errors,
  unchanged class and files, no new errors.

## Remaining gap

None on the local-repo side — the remote consumer (`populi-entrypoint.vox`) lives in this repo and
was updated in the same change, so both halves of the fix (transmission and consumption) are closed.
A Docker image already built and pushed from an older copy of `Dockerfile.populi`/
`populi-entrypoint.vox` would need to be rebuilt for the fix to take effect on real rentals — an
operational/CI concern, not a code gap.
