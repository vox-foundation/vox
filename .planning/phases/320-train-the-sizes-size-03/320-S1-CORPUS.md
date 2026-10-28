# Phase 320 Stage S1: corpus build, QA and the per-size plan-only gate

Recorded 2026-10-28 (UTC) by Plan 320-04 Tasks 2-3. The binaries are the S0 release builds of `900ceb5bf`. Raw files are in `evidence/s1-*`.

## Corpus build (D-03)

All commands ran from the repository root, through `target/release/vox mens ...`, which forwards to the sibling release `vox-ml-cli`. Every one exited 0. Logs: `evidence/s1-{extract,validate,pairs,mix}.txt`.

| Step | Command | Result |
|------|---------|--------|
| extract | `mens corpus extract -o target/dogfood/validated.jsonl` | 278/324 files extracted (46 failed; skipped: 474 not in roots, 29 excluded glob, 5 `training_eligible: false`) |
| validate | `mens corpus validate-batch -i target/dogfood/validated.jsonl -o target/dogfood/validated.jsonl --no-recheck` | 278 in, 277 accepted, 1 rejected by the compiler, **210 after dedup** |
| pairs | `mens corpus pairs target/dogfood/validated.jsonl -o target/dogfood/train.jsonl --docs docs/src` | **1166 pairs**: 703 verified declaration pairs, 47 whole-file pairs, 279 error-correction pairs, 145 docs pairs; 871 unique answers |
| mix | `mens corpus mix --config mens/config/mix.yaml` | **1166 rows** to `target/dogfood/train_mixed.jsonl`. Lanes: `vox_codegen` 1021, `vox_docs_qa` 145. Only the required pairs source exists; `mens/data/mix_sources/` is absent, so every optional source was skipped |

`extract-rs` and `extract-docs` were not run. D-03 names only extract, pairs and mix.

## QA (D-03)

The training file F is `train_mixed.jsonl`: 1166 non-blank rows, which is at least 100, so the mix output was chosen. QA used `jq` over F, with the rules from the plan (`evidence/s1-corpus-qa.json`):

| Count | Value |
|-------|-------|
| rows | 1166 |
| empty rows (blank prompt/instruction/user, or blank response/output/assistant) | 0, so none removed |
| exact duplicates (`jq -cS .`, key order ignored) | 0, so none removed |
| contaminated rows removed | 0 (the guard found none; see below) |
| think rows (response matches `<think>\s*[^<\s]`) | **0** |
| direct rows | **1166** |

- No `train_qa.jsonl` or `train_decontam.jsonl` was needed.
- **Every row is direct.** All 1166 rows have `response_mode: code_only`. This corpus has no think rows, so these runs train only the lane prefix's direct-answer path (empty think in the prompt, not in the loss).
- **Contract.** `target/dogfood/training_contract.yaml` is the single line `train_path: train_mixed.jsonl`. `resolve_train_input` reads the data-dir contract before the workspace `mens/config/training_contract.yaml`, and every plan below resolved `corpus_path` to `.../target/dogfood/train_mixed.jsonl`.
- **metal-e2e (D-03).**
  - No file from `examples/mens/metal-e2e` was copied or referenced, and 0 rows have a source under `examples/mens`.
  - 18 rows come from `scripts/axis-drive-metal-e2e.vox` and `scripts/mens-macos-metal-e2e.vox`. These are compile-checked Vox source files that the source pool (`mens/config/vox-source-pool.yaml`) admits. They are not the double-wrapped ChatML rows in `examples/mens/metal-e2e/dogfood-metal-e2e.jsonl` that D-03 excludes.

## Plan-only gate (D-01, D-02, D-04)

Every run used `--backend qlora --tokenizer hf --device metal --require-gpu --data-dir target/dogfood --epochs 1 --seq-len 3072 --fast-corpus --no-auto-heal --plan-only --json`, with `--model Qwen3-<size>` and `--output-dir mens/runs/320-plan-<size>`. JSON is in `evidence/s1-plan-<size>.json` and stderr in `evidence/s1-plan-<size>.stderr.txt`.

| Size | Pin | Preset | Free MB (Metal working set) | Base cached | Corpus rows | Contamination | Estimated floor refused without override |
|------|-----|--------|-----------------------------|-------------|-------------|---------------|------------------------------------------|
| 0.6B | `c1899de289a04d12100db370d81485cdf75e47ca` | `qwen3_dev_cpu` | 58547 | yes | 1166 | clean, 3 slices, 96 questions | no (floor 2000 MB) |
| 1.7B | `70d244cc86ccca08cf5af4e1e306ecf908b1ad5e` | `qwen3_6g` | 57521 | no | 1166 | clean, 3 slices, 96 questions | no (floor 5000 MB) |
| 4B | `1cfa9a7208912126459214e8b04321603b3df60c` | `qwen3_12g` | 58147 | no | 1166 | clean, 3 slices, 96 questions | no (floor 9800 MB) |
| 8B | `b968826d9c46dd6066d109eabc6255188de91218` | `qwen3_16g` | 58146 | no | 1166 | clean, 3 slices, 96 questions | no (floor 12000 MB) |
| 14B | `40c069824f4251a91eefaf281ebe4c544efd3e18` | `qwen3_24g` | 58402 | no | 1166 | clean, 3 slices, 96 questions | no (floor 20000 MB) |
| 32B | `9216db5781bf21249d130ec9da846c4624c16137` | `qwen3_96g` | 57835 | no | 1166 | clean, 3 slices, 96 questions | **yes** (floor 60000 MB) |

- Every plan has the following: `seq_len` 3072, `epochs` 1, `explicit` true, `license_class` `apache-2.0`, `contamination.rows_checked` 1166, and a `model` ending in its pin.
- **32B.** Without the override it exited 1 with:

  `explicit size 32B needs 60000 MB but 58138 MB is free (Metal working set); refusing rather than shrinking. Choose a smaller size (VOX_MENS_FORCE_TRAIN=1 overrides)`

  That is the selector working (`evidence/s1-plan-32B.refusal.txt`). Re-run with the existing `VOX_MENS_FORCE_TRAIN=1`, it exits 0 with `pick_reason` `32B (explicit): floor 60000 MB does not fit 57835 MB free (Metal working set); forced`. Plan 06 reads the refusal file when it reaches 32B.
- **0.6B preset.** The trainer resolved the preset `qwen3_dev_cpu` for 0.6B on Metal. This is recorded as resolved. It is not changed here (no code edits in an ops plan), and it is noted for the S2 smoke.
- **Plan-only wrote nothing.** No `mens/runs/320-plan-*` directory was created, so there is no `telemetry.jsonl` and no adapter. No stderr contains `Downloading base model`.

### Automatic pick (no `--model`)

`evidence/s1-plan-auto.json` has `explicit` false and `free_source` `Metal working set`.

- First pick: `size: picked 14B: largest size whose floor 20000 MB fits 57902 MB free (Metal working set)`.
- Re-check before download: `14B still fits: floor 20000 MB, 58278 MB free now` (this is `pick_reason`).
- `free_mb` 58278. The model is `Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18`, preset `qwen3_24g`.

At S0, about 30 minutes earlier, the reading was about 60,000 MB and the pick was 32B (floor 60,000 MB, under 100 MB of margin). Free memory has since dropped by about 2 GB, so 32B no longer fits and the selector steps down to 14B. That is the live reading at work.

Stage S1: PASS
