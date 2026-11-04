---
phase: 317-size-ladder-size-01
fixed_at: 2026-10-27T00:00:00Z
review_path: .planning/phases/317-size-ladder-size-01/317-REVIEW.md
iteration: 1
findings_in_scope: 7
fixed: 7
skipped: 0
status: all_fixed
---

# Phase 317: Code Review Fix Report

**Fixed at:** 2026-10-27
**Source review:** .planning/phases/317-size-ladder-size-01/317-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 7 (WR-01 to WR-04, IN-01, IN-02, IN-04)
- Fixed: 7
- Skipped: 0 in scope. IN-03, IN-05 and IN-06 were out of scope and not folded in (see the end of this report).

**Where verification ran:** in the main checkout of the `gsd/mens-compare-316` worktree, `/Users/brbrainerd/dev/vox-mens-compare`. No temporary review-fix worktree was created, so every number below can be reproduced from that tree.

**Mutation protocol:** for each guard I backed up the file and counted the guard line with `grep -c` (1 before). I then mutated it with Edit (0 while mutated) and confirmed the named test panicked. Finally I restored the file from the backup (1 again).

## Fixed Issues

### WR-01: Required `floor_measured` turns one missing key into silent fallbacks and a misleading license error

**Files modified:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`, `crates/vox-ml-cli/src/commands/schola/train/gpu.rs`, `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`
**Commit:** d33009b3f
**Applied fix:**
- `license_for_hf_id` now returns `anyhow::Result<Option<..>>`, and `resolve_license_class` propagates the error.
- `floor_notes` now returns `anyhow::Result<Vec<String>>`, and the `gpu.rs` caller propagates it with `?`. The license call just before it already does the same.
- `derive_quantization_for_base` now loads the overlay up front and returns `anyhow::Result<String>`. Its caller propagates the error.
- A genuinely absent file still falls back to the embedded copy, because `load_overlay` is unchanged.

**Tests:**
- `spoke_base_resolver::tests::malformed_overlay_surfaces_its_parse_error` checks three things against a temp root whose rung lacks `floor_measured`. The license call fails with the `parse train_bases` error both with and without a `--license-class` override. `floor_notes` fails the same way. An absent file still yields the embedded note.
- `train_arm::quantization_surfaces_a_malformed_overlay`, which is `cfg(feature = "cloud")`.

**Red first:** the first test did not compile against the old `Vec` return.

**Mutations, all red:**
- Swallowing the error in `license_for_hf_id` reproduced the review's misleading "no license_class resolved" message.
- Swallowing it in `floor_notes` gave `Ok([])`.
- `let Ok(overlay) = .. else { return Ok("qlora") }` in `derive_quantization_for_base` returned `"qlora"`.

### WR-02: Size-class fix covers only the `@sha` form; documented local-path hints still misclassify

**Files modified:** `crates/vox-populi/src/mens/tensor/preset_schema.rs`
**Commit:** a6889c0bd
**Applied fix:** `detect_qwen_size_class` still strips `@rev`. It then takes the last `/`- or `\`-separated segment that contains `qwen`, which can never be hex, and classifies it with the existing `memory_budget::params_b_from_model_hint` size-token parser. The result maps to a class within ±0.05 of 32, 14, 8 or 0.6. A `models--Org--Name` segment needs no special case, because the parser returns its first `<n>b` token. No new dependency edge was added; it is the same crate and the same cfg gate.

**Tests:** five cases were added to `size_class_ignores_the_pinned_revision_text`:
- the HF-cache snapshot paths for 1.7B and 4B at their real pins, both containing `8b`, which must map to Other;
- an 8B snapshot whose hex is `14b32b…`, which must map to S8;
- a Windows-separator 14B snapshot whose hex contains `8b`, which must map to S14;
- a local `qwen3-0.6b-finetune/checkpoint-32b` path, which must map to S0p6.

**Red first:** S8 was returned for the 1.7B snapshot. **Mutation, red:** classifying on the last path segment (`.next()?` in place of the `qwen` segment `find`) returned S8.

### WR-03: 1.7B floor derivation double-counts the embedding, and "errs high" is contradicted by the ladder's own small rung

**Files modified:** `mens/config/gpu-specs.yaml`, `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs` (tests), `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` (test), `crates/vox-populi/tests/training_presets_yaml_contract.rs` (doc line)
**Commit:** db1cb7497
**Applied fix:** I rewrote the comment and dropped the "errs high" claim. The multiplier is now 3.95x, the highest ratio in the file. It comes from the 0.6B rung, whose bare size I derived from its architecture: 155,582,464 embedding params plus 440,467,456 non-embedding params, which is 506.8 MiB, and 2000 / 506.8 = 3.95x. I verified the arithmetic with awk.

- **1.7B:** non-embedding is 28 x 50,336,000 + 2,048 = 1,409,410,048 params, with the tied embedding counted once. The bare size is 1,265.6 MiB, and x3.95 = 4,999, so the floor goes from **4200 to 5000**. The comment notes the checkpoint's redundant `lm_head.weight` copy.
- **4B:** the bare size is 2,474.5 MiB, and x3.95 = 9,774, so the floor goes from **7400 to 9800**. That stays below 8B's 12000, so the ladder 2000 < 5000 < 9800 < 12000 is still strictly increasing. Its tier comment changed from "8 GB class" to "10-12 GB class".

The contract-test doc line now gives 1.7B as 1,720,574,976 params and explains the 2,031,739,904 safetensors total.

**Tests:**
- `qwen3_code_4b_rung_is_pinned_and_says_its_floor_is_unmeasured` now expects 9800, picks 4B at 10000 MB, and picks 1.7B at 8000 MB.
- The `floor_notes` expectations moved to 9800.
- `reasoning_compare` now expects `floor_mb == 9800`.

**Red first, and also the mutation proof:** these tests failed 3 of 32 against the old YAML values. They passed after the change.

### WR-04: Measured-floor evidence check does not relate `peak_mb` to `floor_mb`

**Files modified:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`
**Commit:** ffd4a5840
**Applied fix:** a measured rung now requires `peak_mb.is_some_and(|p| p > 0 && p <= b.floor_mb)`, and the error text says "a peak_mb in 1..=floor_mb".

**Tests:** `measured_floor_needs_evidence` gained two bad cases, `peak_mb: 0` and `peak_mb: 7401` with `floor_mb` 7400. It also gained two good boundary cases, `peak_mb: 7400` and `peak_mb: 1`.

**Red first:** the `peak_mb: 0` case loaded. **Mutations, all red:**
- dropping `p > 0` let `peak_mb: 0` load;
- changing `<=` to `<` rejected `peak_mb: 7400`;
- dropping the upper bound let `peak_mb: 7401` load.

### IN-01: Evidence test misses the "estimate with `measured_on` only" case

**Files modified:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs`
**Commit:** ae274590f
**Applied fix:** added `"floor_measured: false, measured_on: m4-max-metal"` to the `bad` list. It passes against the current code. **Mutation, red:** reducing the estimate branch to `!has_peak` let the new case load.

### IN-02: Contract-test `floor_label` assertion is tautological

**Files modified:** `crates/vox-populi/tests/training_presets_yaml_contract.rs`
**Commit:** d710f6dae
**Applied fix:** I replaced the tautology with `assert!(!b.floor_measured, "{} claims a measured floor before Phase 320")`. That is an assertion about the YAML, and Phase 320 must update it together with the measurement.

**Mutation, red:** flipping the 0.6B rung to `floor_measured: true, measured_on: m4-max-metal, peak_mb: 1900` failed the test. The YAML was then restored, with no diff.

### IN-04: The "floor unmeasured" wording and its placement do not match the docs

**Files modified:** `docs/src/how-to/how-to-train-mens-macos-metal.md`
**Commit:** 9a67a92fe
**Applied fix:** the how-to now says: "For such a rung the compare command and `vox mens train` print `floor unmeasured` on stderr, and the compare record carries `floor_measured: false`." The doc no longer makes a placement claim. I did not move the print in `gpu.rs`, because the review marks that step optional and minimal was requested. This is prose only, so there is no test.

## Not in scope (deliberately not folded in)

- **IN-03** (no ratio guard on the 1.7B and 4B floors): not trivial. The contract test's nominal NF4-only `bare_weight_mib` does not reproduce the YAML's bf16-embedding derivation, so a band needs its own design.
- **IN-05** (`rungs_for_hf_id` order with inconsistent duplicates): latent today, because the contract test keeps duplicates consistent.
- **IN-06** (untyped `base_rungs: &serde_json::Value`): a signature change with only one caller, so there is no correctness gain now.

## Final verification

| Command (scoped, `timeout`-bounded) | Result |
|---|---|
| `cargo nextest run -p vox-populi --features mens-train --lib -E 'test(spoke_base_resolver) \| test(preset_schema)'` | 64 passed |
| `cargo nextest run -p vox-populi --features mens-train --test training_presets_yaml_contract` | 5 passed |
| `cargo nextest run -p vox-ml-cli --features cloud --lib -E 'test(reasoning_compare) \| test(train_arm)'` | 43 passed |
| `cargo clippy -p vox-populi --features mens-train --lib --tests` | Finished, no finding in touched files |
| `cargo clippy -p vox-ml-cli --features cloud --lib --tests` (`cloud` includes `gpu`) | Finished, no finding in touched files |

The doc lint (`vox-doc-pipeline`) was not run for the IN-04 prose edit. No `vox` commands were run.

---

_Fixed: 2026-10-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
