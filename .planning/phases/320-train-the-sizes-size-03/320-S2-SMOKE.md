# Phase 320 — Stage S2: 0.6B smoke run (D-05)

Three attempts. Attempt 3 (below) re-ran Plan 05 on the binaries carrying the gap-closure fixes (`320-S2-DEBUG.md` "Gap closure"). Attempts 2 and 1 are kept as history after it; their raw evidence moved to `evidence/s2-attempt2/` and `evidence/s2-attempt1/`, so the `evidence/s2-*` paths quoted inside those history sections now live in those directories.

## Attempt 3 — 2026-10-27 21:44-22:19 local (2026-10-28T04:44Z-05:19Z)

**Binaries.** `target/release/vox --version` printed `vox 0.6.0+build.7442 (cdc8fdc3a)`. `f40481471` and `afb5ef543` are ancestors of `cdc8fdc3a`, and the only later commit, `6d326bf74`, changes `320-S2-DEBUG.md` alone. The installed Metal plugin (`~/Library/Application Support/vox/plugins/mens-candle-metal/0.6.0/libvox_plugin_mens_candle_metal.dylib`) and `target/release/libvox_plugin_mens_candle_metal.dylib` are identical (`diff -q`), sha256 `73e98f3dcd216e04c6117dae3aad3601bfbfbb421f44f2bed45ad3f7af5cfe6f`. The plugin was built at 20:50, after the last plugin-crate commit (`afb5ef543`, 20:46); `cdc8fdc3a` touches only the estimate in `vox-ml-cli`, which was rebuilt at 21:36.

**Command (smoke).** `timeout --signal=INT --kill-after=300s 3600s target/release/vox mens train --backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --model Qwen3-0.6B --output-dir mens/runs/320-smoke-0.6b-a3 --max-runtime-secs 480 --force-restart`, backgrounded, one GPU job at a time. `VOX_MENS_FORCE_TRAIN` was **not** set for any run in this attempt: the estimate (`VRAM: est. ~20725 MB / 110101 MB available`) did not refuse.

**Result: PASS on the plan's six D-05 checks**, with two caveats stated under checks 1 and 5: the 5-step loss window does not fall (the quarter windows do), and eval-local persists only a 240-character `completion_preview`, so no complete answer text can be quoted, only the scored outcome of the three complete answers.

### 1. Loss recorded and falling — PASS (quarter windows; 5-step windows flat)

- Exit `0` (`evidence/s2-smoke.exit`). `run cap reached after 485 s at optimizer step 60; finalizing` (`evidence/s2-smoke-log.txt`). `Training started: first backward pass done after 9.7s (1108 optimizer steps planned, grad_accum=1)`.
- Telemetry is clean: `telemetry.jsonl` holds exactly one `train_start`, 60 `step` rows and one `train_complete` (the `--force-restart` purge, `f78bbc6eb`, works). All 60 rows are used.
- `evidence/s2-loss.json`: n = 60, q = 15, `any_non_finite: false`.

  | window | first | last |
  |---|---|---|
  | quarter (q = 15) | 1.7491 | 1.1632 |
  | 5 steps | 1.8251 | 1.8373 |

  Quarter means in order: 1.7491, 1.1539, 0.9350, 1.1632. Step 1 loss 3.1743 (the chunked-attention value recorded in the debug log), step 60 loss 1.8686. `final_loss_ema` 1.3123, `avg_train_loss` 1.2503. The log's EMA column goes 3.1743 → 2.2993 (step 9) → 1.3868 (step 26) → 0.8712 (step 47) → 1.2505 (step 59).
- **Caveat.** At batch 1 each step is one row, and per-row loss ranges from 0.005 to 3.65 in this run, so a 5-row window measures row difficulty more than training. The last five rows (56-60) average 0.012 above the first five. The plan's criterion (first-quarter mean above last-quarter mean, n ≥ 8, all finite) holds by 0.586; the 5-step comparison the orchestrator also asked for does not fall.

### 2. Manifest names the pinned base; checkpoint kept after the cap — PASS

- `run_summary.json` (`evidence/s2-run-summary.json`): `"base_model": "Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca"`, `"capped": true`, `"wall_secs": 486.27`.
- `adapter_manifest.json` `base_model` (`evidence/s2-adapter-base.txt`): `/Users/brbrainerd/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B/snapshots/c1899de289a04d12100db370d81485cdf75e47ca`.
- `train-record.json` (`evidence/s2-train-record.json`): `status` `capped`, `pin` `Qwen/Qwen3-0.6B@c1899de…`, `preset` `qwen3_dev_cpu`, `seq_len` 3072, `epochs` 1, `max_runtime_secs` 480.
- **Checkpoint state kept after the cap** (`afb5ef543`). The run dir still holds `checkpoint_state.json` and `checkpoint_step_60.safetensors` next to the final adapter (`evidence/s2-checkpoint-state.json`: `epoch` 1, `global_step` 60, `pair_offset` 60, `base_model` `Qwen/Qwen3-0.6B@c1899de…`, adapter `checkpoint_step_60.safetensors`). The step adapter and the final adapter have the same sha1 `1c69b700…` (`evidence/s2-ckpt-shasum-before.txt`).
- The `📥 Downloading base model … ✓ Cached at …/snapshots/c1899de…` pair still prints on a cache hit; nothing was fetched (same finding as attempt 2).

### 3. Resume refuses a different base — PASS, fast, no download

- Instead of a separate SIGINT'd Run B, the refusal was tested against this run's own kept checkpoint, which is now the state a capped run leaves behind. Command: `target/release/vox mens train … --seq-len 3072 … --model Qwen3-1.7B --resume mens/runs/320-smoke-0.6b-a3 --output-dir mens/runs/320-resume-1.7b-a3` (no `--force-restart`, no `VOX_MENS_FORCE_TRAIN`).
- Exit `1` (`evidence/s2-resume-c.exit`) after **0.43 s** wall (touch-marker mtimes `1793163255.275` → `1793163255.705`, `evidence/s2-resume-c.time`). The log (`evidence/s2-resume-c.txt`) has no `Downloading base model` line (count 0) and stops right after the floor note:
  ```text
  Error: Checkpoint in /Users/brbrainerd/dev/vox-mens-compare/mens/runs/320-smoke-0.6b-a3 was trained on base Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca, but this run's base is Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e. Resuming would attach that adapter to a different base model. Pass --model Qwen/Qwen3-0.6B@c1899de289a04d12100db370d81485cdf75e47ca to keep training it, or --force-restart to discard the checkpoint and start fresh on Qwen/Qwen3-1.7B@70d244cc86ccca08cf5af4e1e306ecf908b1ad5e.
  ```
- The 1.7B base was already in the HF cache from attempt 2; its blob count was 9 before and after. The refusal comes before the download step (`dfeb3237f`) and before the memory estimate, so the attempt-2 ordering finding is closed.
- The checkpoint survived: `evidence/s2-ckpt-shasum-before.txt` and `-after.txt` are identical (`checkpoint_state.json` `f4bcc3b4…`, `checkpoint_step_60.safetensors` and `candle_qlora_adapter.safetensors` `1c69b700…`), and so are `evidence/s2-ckpt-base-{before,after}.txt`. `evidence/s2-resume-c-record.json` has `status` `not_trained` and the refusal as `reason`.

### 4. `vox mens serve` loads the adapter — REFUSED AT THE GATE (expected)

- `timeout 120s target/release/vox mens serve --model mens/runs/320-smoke-0.6b-a3 --host 127.0.0.1 --port 11436` exited `1`, not 124 (`evidence/s2-serve.exit`):
  ```text
  Error: eval_collateral_damage check not found! Run `vox mens eval-collateral-damage --pre-score <baseline.json> --post-adapter <adapter>` before serving this adapter.
  ```
- No `collateral_damage_report.json` or `gate_receipt.json` exists under the run dir; serve was not retried with the adapter file path (P-320-09). W1 stays carried to Phase 321.

### 5. One held-out prompt answered (eval-local) — PASS (score recorded; full text not persisted)

- **Timing.** A 2-task probe at `--max-tokens 1024` took 374.5 s (~187 s per task that runs to the token limit), so the 52-task bench projects to about 2.7 h. The CLI has no item limit, but `--bench` takes any directory with a `manifest.json`, so the scored run used the first 8 tasks of `mens/data/heldout_bench/manifest.json` (a `jq '.benchmarks |= .[0:8]'` copy in the session scratchpad).
- **Scored run.** `target/release/vox mens eval-local --model mens/runs/320-smoke-0.6b-a3 --bench <8-task subset> --max-tokens 1024 -o evidence/s2-eval-local.json`: exit 0, 978 s (`evidence/s2-eval-local.meta`, `evidence/s2-eval-local-log.txt`). The adapter loaded through `mens-candle-metal` and every task produced a completion with no `error`.
  ```text
  Overall: pass@1 3.00/8 (38%) | pass@1 3.00/8 (38%) | semantic 0%
  ```

  | task | pass (compile + typecheck) | code after `</think>` |
  |---|---|---|
  | workflow_fetch_save | no | none (`trivial_placeholder_output`) |
  | action_create_item | no | none |
  | action_delete | yes | yes, richness 0.125 |
  | test_smoke | yes | yes, richness 0.25 |
  | mutation_update | no | none |
  | page_index | yes | yes, richness 0.125 |
  | count_multiples_of_3_or_5 | no | none |
  | matrix_row_sums | no | none |

  `semantic_task_success` 0.0 (no answer contains the manifest's `semantic_expected_contains` strings, e.g. `test smoke`), `trivial_placeholder_event_rate` 0.625: five of eight completions were still inside `<think>` at 1024 tokens.
- **Complete answers.** Three completions closed their `<think>` block and the code after it parsed and type-checked (`checks.non_empty`, `parse_ok`, `typecheck_ok` all true; `metrics.rs::extract_code` returns an empty string for an unclosed `<think>`, so `non_empty` true means the reasoning ended and code followed). `test_smoke` also passed alone at `--max-tokens 4096` in 43 s (`evidence/s2-eval-local-test_smoke-4096.json`).
- **Caveat: no complete answer text can be quoted.** `eval_local.rs:288` writes `completion.chars().take(240)` as `completion_preview`, and no other output (`-o` JSON, `eval_results.json`, `benchmark_passatk.json`, the log) carries the full completion. Every preview is the opening of the `<think>` preamble, for example `page_index`:
  ```text
  <think>
  Okay, I need to create a Vox program that defines a page named `IndexPage` with a title 'Home' and displays a heading with the text 'Welcome'. Let me start by writing the code.

  First, I'll define the page as a component. The compon
  ```
  This is a reporting limit of the eval-local output, not a load or inference failure; quoting a full answer needs eval-local to persist the completion (a code change, not made here).
- The 2-task probe is kept as `evidence/s2-eval-local-subset2-1024.json` (0/2, both still thinking at 1024 tokens).

### 6. Peak memory recorded and above zero — PASS (flat after the first full-length row)

- `run_summary.json` `peak_metal_allocated_bytes` 24,824,217,600 B; `train-record.json` `peak_mb` 23675, `peak_source` `run_summary`.
- Step telemetry (`evidence/s2-loss.json` `peak_by_step`, a running maximum): 19.24 GB at step 1, 20.26 GB by step 5, 20.39 GB steps 10-27, 20.77 GB steps 28-32, then **24.82 GB from step 33 and flat through step 60**. Step 33 is the first row at the full 3071 tokens (`theoretical_tokens` jumps by 3071; earlier rows were ~2300-2500 tokens). The run truncated 2 rows and middle-trimmed 2 prompts at seq_len 3072.
- Against attempt 2 (45.99 GB at ~2300-token rows) the peak is roughly halved by the query-chunked attention.
- **Finding: the estimate under-predicts at a full-length row.** The estimate prices every row at 3072 tokens and gave 20,725 MiB; the measured peak with a 3071-token row is 23,675 MiB (ratio 1.14). Its calibration points were rows of ~2500 tokens (`320-S2-DEBUG.md` §2). Not a gate failure here (110 GB working set), but the 4B and larger estimates are extrapolations from the same fit, so they are likely low by a similar margin at full-length rows.

### Run record (attempt 3)

| field | value |
|---|---|
| status | `capped` (`run_summary.capped: true`) |
| optimizer / micro steps | 60 / 60 |
| final loss EMA | 1.3123 |
| avg train loss | 1.2503 |
| peak MB | 23675 (`run_summary`) |
| train wall | 486.3 s (cap 480 s plus the step in flight) |
| total wall | 491.3 s (launch to exit 491 s, `evidence/s2-smoke.start`/`.end`) |
| throughput | ~8.1 s per row (60 rows in 486 s), 145,261 tokens |
| truncated / prompt-trimmed rows | 2 / 2 |

### Carried items (attempt 3)

- W1: serve load of a 320 adapter is verified in Phase 321 once the gate files exist (unchanged).
- eval-local writes only a 240-character `completion_preview`; persisting the full completion would let S3/S4 quote answers.
- At `--max-tokens 1024`, 5 of 8 held-out answers are still inside `<think>` (every completion opens with a `<think>` block; whether eval-local can turn Qwen3 thinking off was not checked). Scores at 1024 tokens understate the adapter.
- The memory estimate is ~12% low at a full 3072-token row (above).
- The `Downloading base model` line still prints on a cache hit.
- `VRAM check unavailable (no memory-model calibration for lane candle-metal (gradient_checkpointing=true) …)` prints because that cell is uncalibrated.

## History — Attempt 2 (FAIL, 2026-10-27 19:56-20:12 local, 2026-10-28T02:56Z-03:12Z)

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

### Carried items (attempt 2; the last three were closed by the gap-closure commits)

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
Stage S2: PASS
