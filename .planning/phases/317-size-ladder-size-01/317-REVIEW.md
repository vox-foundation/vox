---
phase: 317-size-ladder-size-01
reviewed: 2026-10-27T00:00:00Z
depth: deep
files_reviewed: 8
files_reviewed_list:
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/schola/train/gpu.rs
  - crates/vox-populi/src/mens/tensor/preset_schema.rs
  - crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs
  - crates/vox-populi/tests/training_presets_yaml_contract.rs
  - docs/src/how-to/how-to-train-mens-macos-metal.md
  - mens/config/gpu-specs.yaml
findings:
  critical: 0
  warning: 4
  info: 6
  total: 10
status: issues_found
---

# Phase 317: Code Review Report

**Reviewed:** 2026-10-27
**Depth:** deep
**Files Reviewed:** 8
**Status:** issues_found
**Scope:** `git diff 2824de851..8862cd02d -- . ':(exclude).planning'` (da1680f4e, 5de0c3250, 50527bec1, 443e46a46)

## Summary

The ladder pins are correct. I re-ran `git ls-remote https://huggingface.co/Qwen/<repo> refs/heads/main` for all six sizes during review, and every `refs/heads/main` equals its pin. The `LICENSE` at the 1.7B and 4B pins is Apache 2.0. The 4B floor arithmetic reconciles exactly with the architecture: 36 layers x 100,930,816 + final norm = 3,633,511,936 non-embedding params.

**Does requiring `floor_measured` break any reader?** No. I checked every reader of `mens/config/gpu-specs.yaml`:

- `train_bases` is deserialized only by `spoke_base_resolver::parse_overlay`.
- `preset_schema::GpuSpecsFile` is the reader used by the cloud estimator and resolver. It does not set `deny_unknown_fields` and has no `train_bases` field, so it ignores the section.
- No `.vox`, TypeScript, Python, plugin or fixture file parses `train_bases`.
- The only in-repo YAML fixtures are the two new tests.
- The embedded `include_str!` copy is the same file, and `floor_notes(None, …)` exercises it.

The real cost of the strictness is different. Three callers already swallow overlay parse errors, so a single missing key now degrades silently or produces a misleading error (WR-01).

**Size-class fix.** `detect_qwen_size_class` is correct for `owner/name@sha` ids:

- `Qwen/Qwen3-0.6B@…` maps to S0p6, `Qwen3-14B` to S14, `Qwen3-32B` to S32, and `Qwen3-8B@b968…` to S8.
- `Qwen3-1.7B@…` and `Qwen3-4B@…` map to Other.
- There is no `4b`/`14b` collision, because 4B has no class and `14b` is tested first.

`resolve_base_pin` suffix matching is also safe: `/Qwen3-4B` does not match `Qwen/Qwen3-14B`. However, `VOX_BASE_MODEL` is documented as "Path or ID", and the fix does not cover HF-cache snapshot paths (WR-02).

**Floor estimates.** The 1.7B derivation comment double-counts the embedding. The claim that the estimates "err high" is contradicted by the file's own 0.6B rung (WR-03).

## Warnings

### WR-01: Required `floor_measured` turns one missing key into silent fallbacks and a misleading license error

**File:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:201-214`, `:111-125`; `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:825-829`

**Issue:** `parse_overlay` now fails on any rung that lacks `floor_measured`. That is the intended strictness. But three callers turn the parse error into something else:

- `license_for_hf_id` does `load_overlay(root).ok()?`. Without `--license-class`, `vox mens train` therefore fails with "it has no `license:` entry in … train_bases (or isn't a known train_bases rung at all)". That is false: the rung and its license are present, and the file simply did not parse. With `--license-class`, training proceeds and the parse error is never surfaced.
- `floor_notes` returns an empty Vec on error. The train start then prints no `floor unmeasured` line, which is the exact honesty signal this phase adds. Its doc comment claims "`resolve_license_class` already fails closed on a bad overlay", which is false when an override is passed.
- `derive_quantization_for_base` (`let Ok(overlay) = load_overlay(root)`) falls back to `"qlora"`. That mislabels the 48 GB un-quantized LoRA rung.

All three swallow patterns predate this phase. What changed is that a contributor adding a rung and forgetting one key now reaches them.

**Fix:** Propagate the parse error from `license_for_hf_id`, and keep `None` only for "not found":

```rust
fn license_for_hf_id(root: Option<&Path>, hf_id: &str)
    -> anyhow::Result<Option<(Option<String>, bool)>> {
    let overlay = match root { Some(r) => load_overlay(r)?, None => load_embedded_overlay()? };
    Ok(overlay.values().flatten().find(|b| b.hf_id == hf_id)
        .map(|b| (b.license.clone(), b.attribution_required)))
}
```

Make `floor_notes` return `anyhow::Result<Vec<String>>`, or a single `"floor notes unavailable: <err>"` line, rather than an empty Vec.

### WR-02: Size-class fix covers only the `@sha` form; documented local-path hints still misclassify

**File:** `crates/vox-populi/src/mens/tensor/preset_schema.rs:87-112`

**Issue:** The hint is `VOX_BASE_MODEL`, which `operator_registry.rs:810` documents as "Path or ID of the base model". `run_train.rs:328` copies `--model` into it.

Stripping after the last `@` fixes `Qwen/Qwen3-4B@1cfa…`. An HF-cache snapshot path still carries the revision hex as a path segment, for example `…/models--Qwen--Qwen3-1.7B/snapshots/70d244cc86ccca08cf5af4e1e306ecf908b1ad5e`. That hex contains `908b1`, so the hint still classifies as S8, which is the same bug this phase claims to fix. The 4B snapshot hex `…e8b04…` behaves the same way.

The failure can also go the other way. Any 8B path whose segment hex happens to contain `14b` or `32b` would receive the S14/S32 small-card clamps: rank 8, alpha 16, seq 256, lr ≤ 1e-4.

**Fix:** Classify on a delimited size token of the model name, never on arbitrary substrings. One option is to reuse the existing parser on the repo-name segment:

```rust
let name = hint.rsplit_once('@').map_or(hint, |(r, _)| r);
let name = name.rsplit('/').find(|s| s.to_ascii_lowercase().contains("qwen"))?; // also covers models--Qwen--Qwen3-1.7B
let params = crate::mens::tensor::memory_budget::params_b_from_model_hint(name);
let near = |x: f64| params.is_some_and(|p| (p - x).abs() < 0.05);
if near(32.0) { S32 } else if near(14.0) { S14 } else if near(8.0) { S8 } else if near(0.6) { S0p6 } else { Other }
```

Add a test case with a `models--Qwen--Qwen3-1.7B/snapshots/<pin>` path.

### WR-03: 1.7B floor derivation double-counts the embedding, and "errs high" is contradicted by the ladder's own small rung

**File:** `mens/config/gpu-specs.yaml:265-279`; `crates/vox-populi/tests/training_presets_yaml_contract.rs:169-170`

**Issue (arithmetic):** At pin `70d244cc…`, `config.json` has `tie_word_embeddings: true`. The checkpoint nevertheless ships a separate `lm_head.weight`, because `model.safetensors.index.json` maps it to shard 00002 and `total_size` is 4,063,479,808 B = 2,031,739,904 params. The comment subtracts one embedding and calls the remaining 1,720,574,976 params "the other … at NF4". By the config, the real non-embedding count is 28 layers x 50,336,000 + 2,048 = 1,409,410,048. The "other" figure therefore includes the 311,164,928-param lm_head copy. The derivation counts the embedding once at bf16 and again at NF4, while asserting tied embeddings.

`vox-hf-layout` honors `tie_word_embeddings`, so the tied bare size is 622,329,856 + 704,705,024 = 1,327,034,880 B (1,266 MiB). At x2.96 that is 3,746, not 4,185. The rounded 4200 is still above that, but the evidence text is wrong. The contract-test doc line ("HF `safetensors.total` 2,031,739,904") repeats the inflated count as the model's size.

**Issue (claim):** The comment says x2.96 is "top of the 2.25x-2.96x band, so an unmeasured floor errs high". That band comes from the 8B, 14B and 32B rungs. The same method applied to the file's own 0.6B rung (tied, 440.4M non-embedding + 155.6M embedding gives a bare 507 MiB) gives 2000 / 507 ≈ 3.95x. Fixed costs dominate small models, mainly the 151,936-vocab logits buffer, which does not scale with parameters.

Interpolated from the nearest rungs, 4B at 3.95x would be ≈ 9,770 MB, not 7,400. The 4B "8 GB class" rung may therefore under-state its floor, and the 8 GB card it targets has about 800 MB of margin. `floor_measured: false` keeps this honest, but the "errs high" sentence is not true.

**Fix:**
- Rewrite the 1.7B derivation with the tied, config-derived non-embedding count (1,409,410,048), and note that the checkpoint ships a redundant `lm_head.weight`.
- Drop "so an unmeasured floor errs high". Instead state that small rungs have historically carried about 3.9x (0.6B), or derive the multiplier from the 0.6B rung, which gives roughly 5,000 / 9,800 MB.
- Either way, have Phase 320 measure 4B before an 8 GB host relies on 7400.

### WR-04: Measured-floor evidence check does not relate `peak_mb` to `floor_mb`

**File:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:153-175`

**Issue:** For `floor_measured: true`, the loader requires only that `measured_on` is non-blank and that `peak_mb` is present. It accepts `peak_mb: 0`, and it accepts `peak_mb` greater than `floor_mb`. `pick_base` selects on `floor_mb <= vram_mb`. A Phase 320 entry with `floor_mb: 7400, peak_mb: 8100` (or a transposed typo) therefore loads cleanly, and an 8 GB host is handed a base that was measured not to fit. This loader is the only place the evidence contract is enforced.

**Fix:** In the `floor_measured` branch, also require `peak_mb.is_some_and(|p| p > 0 && p <= b.floor_mb)`, and add a `measured_floor_needs_evidence` case for each.

## Info

### IN-01: Evidence test misses the "estimate with `measured_on` only" case

**File:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:945-972`

**Issue:** The bad cases cover `false` with `peak_mb`, but not `floor_measured: false, measured_on: x` without a peak. Mutating the estimate branch to `!has_peak`, which drops `b.measured_on.is_none()`, survives the suite.

**Fix:** Add `"floor_measured: false, measured_on: m4-max-metal"` to the `bad` list.

### IN-02: Contract-test `floor_label` assertion is tautological

**File:** `crates/vox-populi/tests/training_presets_yaml_contract.rs:376-383`

**Issue:** For `!b.floor_measured` it asserts that `floor_label()` contains "floor unmeasured". `floor_label` returns exactly that text whenever `floor_measured` is false, so the assertion cannot fail for any YAML content.

**Fix:** Delete it, or assert something about the YAML, e.g. that every rung is `floor_measured: false` until Phase 320.

### IN-03: The 1.7B and 4B floors have no ratio guard

**File:** `crates/vox-populi/tests/training_presets_yaml_contract.rs:283`

**Issue:** The 2.0x-4.0x band applies only to `params_b >= 8.0`. The new rungs are checked only against `floor_mb > bare nominal weight`, so a typo such as `floor_mb: 1200` for 4B would pass.

**Fix:** Add a small-model band derived from the 0.6B rung, or assert the floors stated in the YAML comments.

### IN-04: The "floor unmeasured" wording and its placement do not match the docs

**File:** `crates/vox-ml-cli/src/commands/schola/train/gpu.rs:430-434`; `docs/src/how-to/how-to-train-mens-macos-metal.md:107-109`

**Issue:** The how-to says "the compare record and the train start print `floor unmeasured`". The JSON record carries `floor_measured: false`, not that text; only stderr prints it. In `gpu.rs` the note is emitted after the model download and the VRAM check, not at train start.

**Fix:**
- Reword the doc to: "the compare command and `vox mens train` print `floor unmeasured` on stderr; the record carries `floor_measured: false`".
- Optionally move the `floor_notes` print above the download and VRAM check.

### IN-05: `rungs_for_hf_id` output order is nondeterministic for inconsistent duplicates

**File:** `crates/vox-populi/src/mens/tensor/spoke_base_resolver.rs:97-104`

**Issue:** The sort key is `(floor_mb, methods)`, and the input order comes from `HashMap` iteration. `dedup_by` only removes adjacent equals. Take two copies of a rung that agree on floor and methods but differ elsewhere, e.g. license: `[A, B, A]` can survive with an order that varies between runs, so `base_rungs` in the JSON record is not reproducible. Today the contract test keeps duplicates consistent, so this is latent.

**Fix:** Iterate tags in sorted order (`let mut tags: Vec<_> = overlay.iter().collect(); tags.sort_by_key(|(k, _)| *k);`), or extend the sort key with `license` and `attribution_required`.

### IN-06: `base_rungs` is passed as an untyped `serde_json::Value`

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:181, 345`

**Issue:** `run_before_after_with` accepts any JSON value, such as an object, a string or `null`, and writes it verbatim into the record. That widens the contract for no gain, since there is only one caller.

**Fix:** Take `&[&TrainBase]` and serialize at the single write site. Alternatively, `debug_assert!(base_rungs.is_array())`.

---

_Reviewed: 2026-10-27_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
