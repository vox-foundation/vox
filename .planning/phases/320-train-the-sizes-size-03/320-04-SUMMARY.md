---
phase: 320-train-the-sizes-size-03
plan: 04
subsystem: mens-training
tags: [mens, qlora, metal, plan-only, corpus, d-01, d-02, d-03, d-09, size-03]
status: complete
requires: [320-03]
provides:
  - "release vox, vox-ml-cli (gpu,execution-api) and the --features metal plugin, all from 900ceb5bf"
  - "installed mens-candle-metal 0.6.0 byte-identical to the metal build (sha256 68b80d73…)"
  - "target/dogfood/train_mixed.jsonl (1166 rows, QA clean) with training_contract.yaml pointing at it"
  - "320-S0-EVIDENCE.md (Stage S0: PASS) and 320-S1-CORPUS.md (Stage S1: PASS)"
affects: [320-05, 320-06, 320-07]
tech-stack:
  added: []
  patterns: ["ops runs through target/release/vox mens (argv forwarded to the sibling release vox-ml-cli)"]
key-files:
  created:
    - .planning/phases/320-train-the-sizes-size-03/320-S0-EVIDENCE.md
    - .planning/phases/320-train-the-sizes-size-03/320-S1-CORPUS.md
    - .planning/phases/320-train-the-sizes-size-03/evidence/s0-*
    - .planning/phases/320-train-the-sizes-size-03/evidence/s1-*
  modified: []
decisions:
  - "Training file is train_mixed.jsonl (mix output, 1166 rows); target/dogfood/training_contract.yaml = `train_path: train_mixed.jsonl`"
  - "32B plan-only uses the existing VOX_MENS_FORCE_TRAIN=1 after its estimated-floor refusal, which is recorded in s1-plan-32B.refusal.txt"
metrics:
  duration: "~27 min (01:02-01:29 UTC 2026-10-28)"
  completed: 2026-10-28
actuals:
  tokens: 20700
  tasks: 3
  commits: 2
plan_head_before: 900ceb5bf35e5895e7f4d01ed0ee70400dc0ef06
plan_head_after: a67c977d5a90ce558c0f76a69c71a627db3332b0
---

# Phase 320 Plan 04: S0 binaries, plugin and plan-only; S1 corpus and per-size gate Summary

This plan rebuilt the release `vox`, the release `vox-ml-cli` (`gpu,execution-api`) and the `--features metal` plugin from `900ceb5bf`. It reinstalled `mens-candle-metal` 0.6.0, which is now byte-identical to the build, so the stale ABI-mismatched plugin is replaced. The plan-only run recorded the live automatic pick from the Metal working set and showed refusal parity. It then built a 1166-row QA-clean corpus and gated every ladder size with `--plan-only --json`. There was no GPU training and no download.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | c6d254f21 | docs(320-04): S0 binaries, plugin and plan-only evidence |
| 2+3 | a67c977d5 | docs(320-04): S1 corpus and plan-only gate evidence |

Task 2 produces only gitignored corpus files and `evidence/s1-*`. The plan commits those with Task 3.

## S0 (Stage S0: PASS)

- **Builds.** All three logs end in `Finished`: vox-ml-cli in 4m55s, the plugin in 2m34s, vox in 10m59s.
- **Versions.** `vox-ml-cli 0.6.0 (900ceb5bf)` and `vox 0.6.0+build.7421 (900ceb5bf)`. `vox doctor`: `vox-ml-cli build ... matches vox build 900ceb5bf`, and no other training feature set is named.
- **Plugin.**
  - The previous install is backed up at `$TMPDIR/gsd-320-04/plugin-backup/0.6.0/` (old dylib sha256 `47fc1208…`).
  - `vox plugin install --path <stage> --yes` succeeded.
  - Built and installed dylib sha256 are both `68b80d73f45f494e983b2f108f7846740e4e2938ff88dcb3290e64cee24419e4`, and `diff -q` shows them identical.
  - `vox plugin doctor` reports `✓ mens-candle-metal v0.6.0 — ok` with host ABI 13. Its only failures are on the unrelated `mens-candle-cuda` plugin (ABI 12, needs nvidia-gpu).
- **Lane prefix.** 12/12 `training_text` tests pass, including `direct_answer_puts_empty_think_in_the_prompt_not_the_loss`.
- **Live auto pick.** Run with an empty data dir and no `--model`:
  - plan-only: `size: picked 32B: largest size whose floor 60000 MB fits 60094 MB free (Metal working set)`
  - real run: the same pick at 60005 MB free
  - Both exit 1 with the identical `Corpus file not found: .../s0-empty/train.jsonl`. Neither stderr contains `Downloading base model`.
  - The real run's train-record is `not_trained` with null pin and size.

## S1 (Stage S1: PASS)

- **Corpus counts.**
  - extract: 278 rows (324 files, 46 failed)
  - validate-batch `--no-recheck`: 210 rows after dedup (1 compiler reject)
  - pairs `--docs docs/src`: **1166** (703 declaration, 47 whole-file, 279 error-correction, 145 docs)
  - mix: **1166** rows (`vox_codegen` 1021, `vox_docs_qa` 145; no optional mix source exists)
- **QA of F = `train_mixed.jsonl`.**
  - 1166 rows.
  - Empty 0 and exact duplicates 0, so no `train_qa.jsonl`.
  - **Think 0, direct 1166.** Every row is `response_mode: code_only`.
  - Contamination removed: 0.
- **Contract.** `target/dogfood/training_contract.yaml` is `train_path: train_mixed.jsonl`. No `examples/mens/metal-e2e` file is used. The 18 rows sourced from `scripts/*metal-e2e*.vox` are compile-checked source-pool files, not the excluded ChatML rows.
- **Contamination.** Every plan reports `clean`, 3 slices, 96 held-out questions, 1166 rows checked.

**Per-size plan-only results**

| Size | Pin | Preset | Free MB | Cached | Fit |
|------|-----|--------|---------|--------|-----|
| 0.6B | c1899de… | qwen3_dev_cpu | 58547 | yes | floor 2000 fits |
| 1.7B | 70d244c… | qwen3_6g | 57521 | no | floor 5000 fits |
| 4B | 1cfa9a7… | qwen3_12g | 58147 | no | floor 9800 fits |
| 8B | b968826… | qwen3_16g | 58146 | no | floor 12000 fits |
| 14B | 40c0698… | qwen3_24g | 58402 | no | floor 20000 fits |
| 32B | 9216db5… | qwen3_96g | 57835 | no | **refused** without override (`explicit size 32B needs 60000 MB but 58138 MB is free (Metal working set); refusing rather than shrinking`); exit 0 with `VOX_MENS_FORCE_TRAIN=1`, `pick_reason` ends `; forced` |

- **All six plans.** `seq_len` 3072, `epochs` 1, `explicit` true, `license_class` apache-2.0, and `corpus_path` `.../target/dogfood/train_mixed.jsonl`.
- **Auto pick in S1.** `picked 14B: largest size whose floor 20000 MB fits 57902 MB free (Metal working set)`, re-checked as `14B still fits ... 58278 MB free now`. Free memory dropped about 2 GB between S0 and S1, so 32B (under 100 MB of margin at S0) no longer fits.
- **No output directories.** Plan-only created no `mens/runs/320-plan-*` directories.

## Deviations from Plan

**1. [Environment] `vox-ml-cli` cannot be invoked directly.** The lean-ctx shell allowlist blocks it (`'vox-ml-cli' is not in the shell allowlist`).
- `--version` ran through `cargo run --release -q -p vox-ml-cli --features gpu,execution-api --bin vox-ml-cli -- --version`. That relinked `target/release/vox-ml-cli` from the same HEAD; `vox doctor` afterwards confirms that it matches vox build 900ceb5bf.
- Every `mens train` / `mens corpus` run used `target/release/vox mens ...`. Per the plan's verified facts, it forwards argv unchanged to the sibling release `vox-ml-cli` after a build-id handshake, so the same binary runs.
- In the Task 1 verify, the saved `evidence/s0-ml-version.txt` replaces the live `vox-ml-cli --version`.

**2. [Environment] `cmp` is blocked.** Byte equality was checked with `diff -q` plus sha256, in the evidence and in the verify commands.

**3. [Environment] Scratch paths.**
- Scratch moved from `/tmp/320-04-*` to `$TMPDIR/gsd-320-04/`: the plugin backup, the plugin stage, the S0 empty data dir (`.../gsd-320-04/s0-empty`) and the full build logs.
- The S0 refusal line therefore names that path. Parity holds because both runs used the same directory.

**4. [Environment] zsh does not word-split `$COMMON`.** The first S0 attempt passed the flags as one argument (clap exit 2). It was re-run with `${=A}`. Only the corrected run's files are in the evidence.

No code was modified. There was no GPU training and no base download (P-320-07). The user's global `vox` was not replaced (P-320-08). Nothing under `/Users/brbrainerd/dev/vox` was touched.

## Findings for later plans

- **The corpus has no think rows.** S2-S4 train only the direct-answer path of the lane prefix.
- **The 0.6B preset is `qwen3_dev_cpu`** on a Metal run. The selector resolves it this way, and this plan does not change it. Plan 05 (S2 smoke) should note it.
- **32B is borderline on this Mac** (estimated floor 60000 MB against roughly 58-60 GB free). Plan 06 has the refusal file.
- **Restoring the old plugin.** The backup lives in `$TMPDIR`, which macOS may purge, but it is the stale build and is only needed for a rollback.

## Known Stubs

None.

## Threat Flags

None. T-320-12 is mitigated: the plugin comes from this build, is byte-checked, and the old one is backed up. T-320-13: every plan ran the contamination guard, and it reported clean. T-320-14: the corpus came from the source pool and `docs/src` only.

## Self-Check: PASSED

- FOUND: 320-S0-EVIDENCE.md (last line `Stage S0: PASS`), 320-S1-CORPUS.md (last line `Stage S1: PASS`), evidence/s1-corpus-qa.json, evidence/s1-plan-{0.6B,1.7B,4B,8B,14B,32B,auto}.json
- FOUND commits: c6d254f21, a67c977d5 (both ancestors of HEAD)
- The Task 1, 2 and 3 automated verifies passed, with the substitutions in deviations 1-2.
