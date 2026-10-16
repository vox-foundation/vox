---
phase: 12-code-gen-benchmark-plumbing-provenance
plan: 06
subsystem: training-provenance
tags: [adapter-card, lineage, vox-crypto, mens, eval-gate]
requires: []
provides:
  - "AdapterCard lineage fields: adapter_sha256, train_data_sha256, training_manifest_digest, training_syntax_fingerprint, syntax_fingerprint_kind (serde default)"
  - "adapter_card::{validate_lineage, sha256_file_hex, locate_adapter, TrainingLineage, read_training_lineage, EvalCardCheck, check_for_eval}"
  - "vox_compiler::llm_prompt::grammar_fingerprint() -> (String, &'static str)"
  - "train_arm::write_local_adapter_card, called after every successful local run"
affects: [12-07, 12-09]
status: complete
actuals:
  tokens: 14000
  tasks: 2
  commits: 2
plan_head_before: 850f9fcd7295b60a8cd6eb1f58c5a3acdcfd3c6d
tech-stack:
  added: []
  patterns:
    - "hashing only through vox_crypto facades (interop_sha256 for files, secure_hash/BLAKE3 for the manifest digest); no direct sha2/blake3"
    - "card is validated (validate + validate_lineage) before anything is written; an untraceable run leaves no card"
key-files:
  created: []
  modified:
    - crates/vox-populi/src/mens/tensor/adapter_card.rs
    - crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs
    - crates/vox-compiler/src/llm_prompt.rs
    - crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
key-decisions:
  - "read_training_lineage falls back to run_dir/training_manifest.json when the adapter sits in final/ and has no manifest beside it"
  - "Cloud cards keep empty lineage (recorded deviation in the plan); eval refuses to label cloud-trained rows until the pod can report the consumed file hash"
requirements-completed: []
---

# Phase 12 Plan 06: Adapter lineage card Summary

Every successful local `vox mens train` now leaves an `adapter_card.json` carrying SHA-256 of the adapter and of the train.jsonl the trainer consumed, a BLAKE3 digest of its manifest, and the EBNF syntax fingerprint; `check_for_eval` refuses anything it cannot tie back to that card by hash.

BENCH-05 is deliberately NOT marked complete: 12-07 delivers the eval-side refusal that calls `check_for_eval`.

## Accomplishments

- `AdapterCard` gained five `#[serde(default)]` lineage fields. `validate()` is unchanged (serve depends on it); `validate_lineage()` rejects empty fields, non-lowercase-hex or non-64-char digests, and a `train_data_sha256` equal to the SHA-256 of empty input. Legacy cards parse and then fail lineage.
- `read_training_lineage` hashes the adapter and the file named by `train_file` in the run's `training_manifest.json` (raw bytes, no normalization); the manifest digest is BLAKE3 over its raw bytes.
- `train_arm::run_train` writes the card in the post-success block; failure only prints "eval will refuse to label this run" and never fails training. An unpinned base model (no `@revision`) produces no card.
- `grammar_fingerprint()` returns `(compute_ebnf_hash(), "ebnf-sha256")` through the existing vox-ml-cli to vox-compiler edge; no new crate edge.
- `check_for_eval` refuses missing adapter, missing/unreadable/invalid card, invalid lineage and a re-hashed adapter that differs from the card; syntax drift is reported as `Some(true)` without refusing.

## Verification

- `cargo nextest run -p vox-compiler --lib -E 'test(grammar_fingerprint_is_stable_hex)'`: 1 passed.
- `cargo nextest run -p vox-ml-cli --lib --features gpu -E 'test(local_card_tests::)'`: 2 passed.
- `cargo nextest run -p vox-populi --lib --features mens -E 'test(adapter_card::)'`: 15 passed (9 existing + 6 new).
- Acceptance greps: `pub fn grammar_fingerprint` 1, `serde(default)` 5, `fn write_local_adapter_card` 1 and `write_local_adapter_card(` 4, `pub fn check_for_eval` 1, three named edge-case tests 3, non-comment `sha2::|blake3::` in adapter_card.rs 0.
- Not run (lean verification): clippy, the full vox-populi/vox-ml-cli suites, a real training run. Pre-commit hooks (`tdd-guard`, `fmt-fix`, `ci-status`) ran and passed on both commits. Touched files formatted with `rustfmt --config-path .` only.

## Mutation proof (T-12-06-01)

Guard: the adapter re-hash comparison arm `Ok(h) if h == card.adapter_sha256 => {}` in `check_for_eval`.

| Step | `grep -c 'if h == card.adapter_sha256 => {}'` | `swapped_adapter_is_refused` |
|---|---|---|
| before | 1 | pass (15/15 in module) |
| mutated (arm replaced by `Ok(_) => {}`) | 0 | FAIL (`problems` empty) |
| restored | 1 | pass (15/15 in module) |

## Deviations from Plan

- **Task split detail:** `check_for_eval`/`EvalCardCheck` were held back from the Task 1 commit and added in Task 2, so each commit contains only its task's behavior. No change in final content.
- **Manifest lookup fallback (Rule 2):** the plan says read the manifest "next to the adapter"; the adapter may be in `<run>/final/`, where no manifest is written, so the lookup also tries `run_dir`. No other deviation.

## Known limitation

Cloud-arm cards (`card_from_manifest`) carry empty lineage by design (the pod trains on a Hugging Face dataset, bytes unobservable host-side), so they fail `validate_lineage` and eval refuses to label cloud-trained rows. Lifting this needs the pod to report the consumed file hash or the dataset file's LFS oid.

## Known Stubs

None.

## Self-Check: PASSED

- Files: all four modified files exist and are in commits 9422838a4 and e24557a14 (FOUND in `git log`).
