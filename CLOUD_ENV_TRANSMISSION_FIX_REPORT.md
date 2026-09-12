# Cloud env transmission fix: batch_size/seq_len to rented workers

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
VRAM-tiered preset ladder — which is exactly the "estimate and dispatch can never disagree" gap the
review flagged: the local dispatch decision (`min_vram_mb_for_training` in
`crates/vox-populi/src/mens/cloud/resolver.rs`, which sizes the GPU rental via `spec.batch_size`/
`spec.seq_len`) and the value the rented worker actually trains with could diverge.

`vox mens train`'s CLI (`crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs`) already
exposes `--batch-size <usize>` and `--seq-len <usize>` as `Option<usize>` overrides that win over the
preset (`crates/vox-ml-cli/src/commands/schola/train/gpu.rs`: `cli_overrides.seq_len`/`batch_size` fill
the slot before falling back to the profile). So the correct fix is a real command-line construction
point in this repo, not a brand-new external contract.

I also checked whether gradient-checkpointing needed the same treatment. It does not: the local
sizing decision (`min_vram_mb_for_training` → `min_vram_mb_for_cuda` → `CalKey::new(Lane::CandleCuda,
true)`) already hardcodes gradient-checkpointing **on** for its VRAM estimate — i.e. the local
decision is already sized against the more conservative (safer) assumption regardless of what the
real training run picks. There is no `gradient_checkpointing` field on `CloudJobSpec` and the actual
runtime toggle (`VOX_MENS_GRADIENT_CHECKPOINTING`, set by `vox mens train --gradient-checkpointing`)
can only ever shrink memory use versus the sizing assumption, so it does not reproduce the
"remote picks something LARGER than verified" bug. No change made there.

## What I fixed (Steps 2/3)

1. **`crates/vox-populi/src/mens/cloud/vast.rs`** (`build_env_map`): for `JobKind::Train`, injects
   `VOX_MENS_BATCH_SIZE` and `VOX_MENS_SEQ_LEN` from `spec.batch_size`/`spec.seq_len` (mirroring the
   existing `VOX_MENS_*` env-var convention already used elsewhere for training knobs, e.g.
   `VOX_MENS_GRADIENT_CHECKPOINTING`, `VOX_MENS_GC_SEGMENTS`).
2. **`crates/vox-populi/src/mens/cloud/runpod_provider.rs`** (`build_env`): identical fix for RunPod,
   same env var names, same `JobKind::Train` guard, for provider parity.
3. **`infra/containers/entrypoints/populi-entrypoint.vox`**: the `"train"` arm now reads
   `VOX_MENS_BATCH_SIZE`/`VOX_MENS_SEQ_LEN` and appends `--batch-size <n>`/`--seq-len <n>` to the
   `vox mens train` invocation when present, so the value the local dispatch decision verified is
   exactly what the rented worker uses (`resolve_effective_profile`'s CLI override always wins over
   the auto preset). Verified with `vox check infra/containers/entrypoints/populi-entrypoint.vox`
   ("Check passed with 0 warning(s)").

## Tests (Step 4)

Added to both provider files (no test module existed there before):
- `build_env_map_transmits_batch_size_and_seq_len_for_train_jobs` (vast.rs) /
  `build_env_transmits_batch_size_and_seq_len_for_train_jobs` (runpod_provider.rs): builds a
  `CloudJobSpec::new_train` with `batch_size = 7`, `seq_len = 999`, asserts the built env map/list
  contains `VOX_MENS_BATCH_SIZE = "7"` and `VOX_MENS_SEQ_LEN = "999"`.
- `build_env_map_omits_sizing_vars_for_non_train_jobs` / `build_env_omits_sizing_vars_for_non_train_jobs`:
  same but with `CloudJobSpec::new_serve`, asserting the keys are absent (matches the existing
  `VOX_SERVE_PORT`-only-for-non-train branching pattern).

## Verification (Steps 5/6)

- `cargo test -p vox-populi --features mens-cloud --lib`: **311 passed, 0 failed, 1 ignored** (baseline
  was 307+; the 4 new tests account for the delta). The 4 new tests are all present and passing:
  `mens::cloud::vast::tests::build_env_map_transmits_batch_size_and_seq_len_for_train_jobs`,
  `mens::cloud::vast::tests::build_env_map_omits_sizing_vars_for_non_train_jobs`,
  `mens::cloud::runpod_provider::tests::build_env_transmits_batch_size_and_seq_len_for_train_jobs`,
  `mens::cloud::runpod_provider::tests::build_env_omits_sizing_vars_for_non_train_jobs`.
- `cargo clippy -p vox-populi --features mens-cloud --lib -- -D warnings`: still exactly 4 errors,
  same classes as the known pre-existing baseline (`estimator.rs`, `runpod_provider.rs`, `vast.rs`,
  `watchdog.rs` — all pre-existing `map_or`/`div_ceil` lints in `list_offers`/unrelated code, confirmed
  by inspecting the flagged lines, which shifted only because my additions land earlier in the file).
  No new clippy errors introduced.

## Remaining gap / limitations

None on the local-repo side — the remote consumer (`populi-entrypoint.vox`) lives in this repo and was
updated in the same change, so both halves of the fix (transmission + consumption) are closed. The one
thing this fix does **not** touch is any Docker image that was already built and pushed from an older
copy of `Dockerfile.populi`/`populi-entrypoint.vox` — a stale image on a registry would need to be
rebuilt for the fix to take effect on real rentals; that is an operational/CI concern, not a code gap.
