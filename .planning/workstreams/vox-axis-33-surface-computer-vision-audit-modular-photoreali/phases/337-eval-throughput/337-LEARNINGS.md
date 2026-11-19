---
phase: 337
phase_name: "eval-throughput"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 2
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 337 Learnings: eval-throughput

## Decisions

### Create the two pure vox-eval modules every later 337 plan and phase 338 ...
Create the two pure vox-eval modules every later 337 plan and phase 338 build on: the decode protocol identity (THRU-07 types, D-05/D-06/D-24) and the exact sequential McNemar rule with its worst-case false-fail proof (THRU-10 core, D-10/D-11/D-22/D-23). **Ordering (D-14, D-26):** this plan edits `crates/vox-eval/src/lib.rs`, a file in the crate phase 335 is editing, so it starts only after 335 has landed (precondition on Task 1) and re-reads `lib.rs` and `reasoning_slice.rs` at execution time. It is the first plan of the phase: plans 337-02 and 337-03 build on it, and phase 338 may begin once 337-01..03 have SUMMARYs (D-26). **Shape is final for 338 (D-15):** the bf16 variant carries its GEMM row count now, even though the plugin implements bf16 in 337-11, so the checkpoint-key fields and the report `decode` object that 338-02 hashes do not change shape later. D-23: futility (early pass) and deterministic curtailment are not implemented; the module has no API for either. D-16: test-first for every new pub fn, mutation proofs for the size guard, no new crate edge (vox-eval keeps its existing dependencies: serde, serde_json). Purpose: one source of truth for "which numerics produced this text" and "which stopping rule judged this slice", proven exactly rather than simulated. Output: `decode_protocol.rs`, `sequential_mcnemar.rs`, two env-var registry entries, mutation proofs in the SUMMARY.

**Rationale:** Primary phase objective established in plan.
**Source:** 337-01-PLAN.md

---

### Wire the protocol identity from 337-01 through the host and the plugin l...
Wire the protocol identity from 337-01 through the host and the plugin load (THRU-07, and the "protocol in key" part of THRU-09): checkpoint inputs, the report `decode` object 338 hashes, the baseline-pairing refusal, and a plugin that refuses numerics it does not implement (D-05, D-07, D-24). **Ordering (D-14, D-26):** start only after phase 335 has landed. This plan edits `item_checkpoint.rs`, `reasoning_eval.rs`, `eval_collateral.rs`, `reasoning_compare.rs` and `baseline_from_compare.rs`, which 335 is changing; the executor re-reads each at execution time and works from its landed shape, not from the line numbers quoted here. This plan and 337-03 are what phase 338 needs from 337 (D-15); their SUMMARYs should exist before 338-02 starts. **Key stability is the hard constraint.** `hash_json` hashes `Value::to_string()`, and serde_json's `preserve_order` is on workspace-wide, so the key depends on field insertion order (the comment on `hash_json` that says keys are sorted is wrong; correct it). The f32 inputs literal stays byte-for-byte as it is; protocol fields are appended after it only for bf16. `REASONING_SCORER` and `model_identity` are not touched (research: changing either orphans every cached generation). Purpose: a generation, a report and a baseline each say which numerics produced them, and the gate cannot pair two different ones. Output: golden-key test, `decode` objects in both reports, the baseline refusal, plugin load refusal, mutation proofs; the SUMMARY states the 338-02 amendment (decode_blake3 hashes this `decode` object, which has no batch size).

**Rationale:** Primary phase objective established in plan.
**Source:** 337-02-PLAN.md

## Lessons

## Patterns

## Surprises

