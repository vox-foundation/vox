# Phase 320 — Stage S2: 0.6B smoke run (D-05)

Two attempts. Attempt 2 (below) re-ran Plan 05 from the start on the fixed binaries. Attempt 1 is kept as history at the end. Its raw evidence moved to `evidence/s2-attempt1/`.

## Attempt 2 — 2026-10-27 19:56-20:12 local (2026-10-28T02:56Z-03:12Z)

**Binaries.** `target/release/vox --version` printed `vox 0.6.0+build.7428 (55a338c53)`. The sibling `vox-ml-cli` was built from the same tree. The Metal plugin at `~/Library/Application Support/vox/plugins/mens-candle-metal/0.6.0/` has sha1 `e1eb82871845611b84c96631142e4e8bf41a79e4`, which matches `target/release/libvox_plugin_mens_candle_metal.dylib`. The fixes through `67c1d8a8d` are in, including per-layer checkpointing, the chunked head, preset batch/accum, the calibrated estimate and signal reporting. See `320-S2-DEBUG.md`.

**Command (smoke).** `timeout --signal=INT --kill-after=300s 3600s target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-0.6B --output-dir mens/runs/320-smoke-0.6b --max-runtime-secs 480 --force-restart`. `vox` forwards to `vox-ml-cli`. The shell allowlist blocks calling `vox-ml-cli` directly.

**Result: FAIL at check 5 (held-out answer).** The adapter does not load for inference. Checks 1, 2, 3 and 6 pass, and check 4 refused at the gate as expected. Code must be fixed before S3. This plan does not patch code.

### 1. Loss recorded and falling — PASS

- Exit `0` (`evidence/s2-smoke.exit`). The run hit the cap: `run cap reached after 490 s at optimizer step 29; finalizing` (`evidence/s2-smoke-log.txt`).
- Batch/accum were 1/1 and the effective batch was 1. The log says `activation/gradient checkpointing ENABLED for this run explicit=false` and `gradient checkpointing on: one layer per segment`. `Training started: first backward pass done after 24.1s (1108 optimizer steps planned, grad_accum=1)`.
- Steps: 29 optimizer steps and 29 micro-steps, about 17-21 s per row.
- **Telemetry scope.** `--force-restart` does not truncate `telemetry.jsonl`. The run dir's file still starts with attempt 1's `train_start` (run `20261028T013029Z`) and its step-0 row, which carries the 374 GB peak. `evidence/s2-loss.json` therefore uses only the rows after the last `train_start` (run `20261028T025651Z`). This is a hygiene finding, not a check failure. The plan's own `jq` peak check (max over all step rows) would also pass on the stale row alone.
- `evidence/s2-loss.json`: n = 29, q = 7, `any_non_finite: false`. Per-row loss is noisy at batch 1, so both a quarter window and a 5-step window are recorded:

  | window | first | last |
  |---|---|---|
  | quarter (q = 7) | 1.6225 | 1.2988 |
  | 5 steps | 1.8224 | 1.6266 |

  Step 1 loss was 3.1694, the same as attempt 1's step 0. `final_loss_ema` was 1.4059 and `avg_train_loss` 1.4556.
- Task 1's automated verify passed in full (`TASK1-VERIFY-PASS`).

### 2. Manifest names the pinned base — PASS

- `run_summary.json` (`evidence/s2-run-summary.json`): `"base_model": "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"`.
- `adapter_manifest.json` `base_model` (`evidence/s2-adapter-base.txt`): `/Users/brbrainerd/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca`.
- `train-record.json` (`evidence/s2-train-record.json`):
  - `pin` is `Qwen/Qwen3-0.6B@c1899de…`
  - `status` is `capped`
  - `preset` is `qwen3_dev_cpu`
  - `seq_len` is 3072 and `epochs` is 1
  - `truncated_rows` and `prompt_trimmed_rows` are both 0
- **"Downloading base model" on a cache hit.** The log prints `📥 Downloading base model from Hugging Face: Qwen/Qwen3-0.6B@c1899de…` and then `✓ Cached at …/snapshots/c1899de…`. Nothing was fetched. The blob listings before and after the run are identical (`evidence/s2-cache-before.txt`, `evidence/s2-cache-after.txt`): every blob is dated 2026-09-05 or 2026-09-11. The line also prints on a cache hit, so the acceptance check "no `Downloading base model` line" fails as written even though nothing downloaded.

### 3. Resume refuses a different base — PASS (forced past the estimate)

- **Run B.** `--model Qwen3-0.6B --output-dir mens/runs/320-resume-0.6b --force-restart` printed `Training started …` after 17.5 s. Exactly one trainer PID matched (`87627`), and one `kill -INT` went to it. The log then shows `Training paused at step 2. Resume with 'vox mens train --resume …/320-resume-0.6b'` (`evidence/s2-resume-b.txt`). Exit 0. `checkpoint_state.json` `base_model` is `Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca` (`evidence/s2-ckpt-base-before.txt`).
- **Run C, first try.** `--model Qwen3-1.7B --resume mens/runs/320-resume-0.6b`. It downloaded the 1.7B base at pin `70d244cc…` and was then refused by the new Metal memory estimate before the base check could run: `Error: estimated peak 65896 MB (every row at --seq-len 3072, gradient checkpointing true) exceeds 65502 MB free (Metal working set); lower --seq-len or free memory, or set VOX_MENS_FORCE_TRAIN=1 to try anyway`. Evidence: `evidence/s2-resume-c-estimate-refusal.txt`, `.exit` and `-record.json`.
- **Run C, second try.** The same command with `VOX_MENS_FORCE_TRAIN=1`, so the base check is reached. `apply_checkpoint_resume` runs before the CE preflight and the first step (`candle_qlora_train/mod.rs:1069`). A watcher was ready to SIGINT the run if `Masked CE numeric preflight` or `Training started` appeared (P-320-10), and it never fired. Exit 1 (`evidence/s2-resume-c.exit`), with:
  ```text
  Error: mens-candle-metal training failed: training failed: QLoRA checkpoint resume: Checkpoint in /Users/brbrainerd/dev/vox-mens-compare/mens/runs/320-resume-0.6b was trained on base Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca, but this run's base is Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e. Resuming would attach that adapter to a different base model. ...
  ```
- **The checkpoint survived.** `evidence/s2-ckpt-base-after.txt` is identical to the before file. The sha1 of `pause_step_2.safetensors` (`740e3ee7…`) and of `checkpoint_state.json` (`687c6fe4…`) are unchanged (`evidence/s2-ckpt-shasum-{before,after}.txt`). `evidence/s2-resume-c-record.json` has `status` `not_trained` and the refusal as `reason`.
- **Observed ordering.** The memory estimate runs before the cheap base-mismatch check, so a mismatched resume downloads about 4 GB and is then refused for memory rather than for the base. It is not a correctness bug: the checkpoint is safe either way.

### 4. `vox mens serve` loads the adapter — REFUSED AT THE GATE (expected)

- `timeout 120s target/release/vox mens serve --model mens/runs/320-smoke-0.6b --host 127.0.0.1 --port 11436` exited `1`, which is not 124 (`evidence/s2-serve.exit`). Output (`evidence/s2-serve.txt`):
  ```text
  Error: eval_collateral_damage check not found! Run `vox mens eval-collateral-damage --pre-score <baseline.json> --post-adapter <adapter>` before serving this adapter.
  ```
- No `collateral_damage_report.json` or `gate_receipt.json` exists under the run dir. Serve was not retried with the adapter file path (P-320-09).

### 5. One held-out prompt answered (eval-local) — FAIL (code bug)

- `timeout 3600s target/release/vox mens eval-local --model mens/runs/320-smoke-0.6b --bench mens/data/heldout_bench --max-tokens 128 -o evidence/s2-eval-local.json` exited `1` (`evidence/s2-eval.exit`). No JSON was written. The plugin loaded (`plugin.loaded id="mens-candle-metal"`), and then the run failed with (`evidence/s2-eval-local-log.txt`):
  ```text
  Error: load_model(mens/runs/320-smoke-0.6b): 1 trained LoRA delta(s) were never applied to any weight: ["model.embed_tokens.weight"]. The adapter's base_key_map names tensors this model never loads as a linear projection, so the trained layers would be served as base weights. Next: check base_key_map in adapter_manifest.json against this model's config.json.
  ```
- **Cause, observed and not fixed.** Qwen3-0.6B has `tie_word_embeddings: true` (run dir `config.json`).
  - The Metal trainer trains a LoRA on the tied head. It labels it `lm_head` and maps it, via `resolve_lm_head_source`, to `base_key_map["lm_head"] = "model.embed_tokens.weight"` (`crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs:1006-1020`).
  - Metal inference never requests `model.embed_tokens.weight` as a linear projection. `assert_adapter_fully_applied` (`crates/vox-plugin-mens-candle-metal/src/inference.rs:830-850`) correctly refuses rather than serve base weights.
  - So every tied-embedding adapter from this trainer fails to load: Qwen3-0.6B, and also 1.7B, whose cached `config.json` at pin `70d244cc…` has `tie_word_embeddings: true`. The 4B base is not cached, so its config was not checked. This is the same class as that guard's own doc comment ("a tied-head spelling training stores that inference's fallback chain never tries").
  - It was not caught before because the debug proof runs never ran eval-local.
- Per D-05 and Plan 05 Task 1 step 4, a failed check is a code bug to fix before S3. No pass rate was recorded because no sample was generated.

### 6. Peak memory recorded and above zero — PASS

- `run_summary.json` `peak_metal_allocated_bytes`: 45,994,475,520, about 43,864 MiB.
- `train-record.json`: `peak_mb` 43864 with `peak_source` `run_summary`.
- Step telemetry for this run: 45,293,404,160 to 45,994,475,520 B, flat (`evidence/s2-loss.json` `peak_min_bytes` and `peak_max_bytes`).
- Estimate: `VRAM: est. ~59149 MB / 110101 MB available`. The size check read `64599 MB free (Metal working set)`. The estimate did not refuse the smoke run, so `VOX_MENS_FORCE_TRAIN=1` was **not** needed for it. The measured peak is 74% of the estimate and 68% of free memory.

### Run record

| field | value |
|---|---|
| status | `capped` |
| optimizer / micro steps | 29 / 29 |
| final loss EMA | 1.4059 |
| avg train loss | 1.4556 |
| peak MB | 43864 (`run_summary`) |
| train wall | 491.5 s (cap 480 s + one ~18 s step) |
| total wall | 511.1 s (launch to exit 513 s, `evidence/s2-smoke.start`/`.end`) |
| truncated / prompt-trimmed rows | 0 / 0 |
| total tokens | 67,909 |

### Other observations

- The stale message `VRAM check unavailable (candle-metal does not implement gradient checkpointing; …)` still prints on every Metal run. It is already listed as unresolved in `320-S2-DEBUG.md`.
- The trainer backs up the corpus to `mens/data/train_full_backup.jsonl`, which is gitignored.

## Carried items

- W1: serve load of a 320 adapter is verified in Phase 321 once the gate files exist.
- Fix before S3: Metal inference must apply the tied-head LoRA delta keyed `model.embed_tokens.weight`, or training must key it to what inference requests. Re-run check 5 after the fix.
- Hygiene: `--force-restart` should truncate `telemetry.jsonl`.
- Ordering: check the resume base before the memory estimate and the base download.

## History — Attempt 1 (FAIL, 2026-10-27 18:30-18:36 local)

**Binaries:** `900ceb5bf` (S0), Metal plugin sha256 `68b80d73…`. **Command:** the same smoke command as attempt 2. Evidence is in `evidence/s2-attempt1/`.

The trainer died after its first micro-step, 354 s after launch, with exit 1 and no error line. It wrote no adapter, `run_summary.json` or `train-record.json`.

- **Preset.** 0.6B resolved to `qwen3_dev_cpu`, but the 0.6B ladder rung raised it to batch 2 / grad_accum 4 (277 optimizer steps planned). Gradient checkpointing was off.
- **Loss.** One step row: `optimizer_step 0`, `loss 3.1694`. n = 1, so a falling loss could not be shown.
- **Manifest.** `training_manifest.json` named the pin. No adapter manifest was written. The `Downloading base model` line printed on a cache hit then too.
- **Resume, serve and eval-local.** Not run.
- **Peak.** `peak_metal_allocated_bytes: 374417047552`, about 374 GB, against an estimate of `~1618 MB` and a 110101 MB working set.
- **Diagnosis.** One row's eager autograd graph was fully resident, the Metal loop ignored checkpointing, and the head was computed over the whole row. The launcher mapped the signal death to exit 1. Fixed in `a6e959e1a`, `7febb6bde`, `55a338c53` and `41bcbd9b7` (`320-S2-DEBUG.md`).

D-05 serve decision: accept-eval-local (2026-10-27, user, relayed by the orchestrator)
Stage S2: FAIL — held-out answer (eval-local refuses the adapter: tied-head LoRA delta "model.embed_tokens.weight" never applied by Metal inference)
