315-RESEARCH.md 380L
# Phase 315: Serve Gate - Research
**Researched:** 2026-10-26
**Domain:** Rust, `vox-ml-cli` collateral-damage writer and `vox mens serve` gate; `vox-eval` pure scoring helpers
**Confidence:** HIGH (every value below was read from source this session; the model-dependent parts are out of scope by decision)
... [lean-ctx: omitted 1 lines]
## User Constraints (from CONTEXT.md)
### Locked Decisions
- Compare accuracy and think-initiation separately on each reasoning slice.
- Both use the existing 5% relative bar (`max_degradation_rate` 0.05).
... [lean-ctx: omitted 2 lines]
- A missing slice report, a `not_scored` report, or any `n` other than 32 is a hard error. Serve refuses. Those cases are not stored as accuracy 0.0 or think-initiation 0.0.
- A non-finite accuracy or think-initiation refuses the adapter. A reasoning pre-score of zero refuses the adapter. The current "pre-score near zero means no drop" path does not apply to these reasoning keys. That generic path may stay for non-reasoning benches. The existing zero-pre test on `eval_collateral_damage` is not a license to pass a reasoning key whose pre-score is zero.
... [lean-ctx: omitted 2 lines]
### Claude's Discretion
- Whether each reasoning slice is two baseline keys (`gsm8k_think` and `gsm8k_think_init`) or one key the gate expands, as long as both scores are compared and a drop on either fails.
- The exact refusal text, as long as serve does not bind and the report names the benches as slices and records `n`.
### Deferred Ideas (OUT OF SCOPE)
- Live base-versus-adapter comparison, or a "snapshot absent" record (phase 316).
... [lean-ctx: omitted 1 lines]
- MATH-500 equivalent surface forms (WR-03). The grader and the slice stay locked.
- Swapping the pinned Qwen3-8B base, or a new training technique.
... [lean-ctx: omitted 2 lines]
## Phase Requirements
| ID | Description | Research Support |
... [lean-ctx: omitted 1 lines]
| GATE-01 | Serve refuses an adapter when the collateral report is missing, or when accuracy or think-initiation on the lane benches drops past the configured threshold. | Serve already refuses a missing report; it does not read any per-key data, so the new work is a strict reasoning evaluator (writer side) plus a read-only re-check of the on-disk report (serve side). See Patterns 1-3. |
| GATE-02 | The report says these benches are slices, so the 5% relative bar is not read as a full GSM8K or MATH-500 result. | Add an additive `reasoning_slices` block and per-entry `slice`/`n` fields to `collateral_damage_report.json`. See Pattern 2. |
... [lean-ctx: omitted 1 lines]
## Summary
Today's gate is thinner than the context file suggests. `dispatch.rs` checks only that `collateral_damage_report.json` exists and that `status == "pass"`; it never reads a per-bench value. The writer (`run_collateral_damage_with`) already scores `_init` keys correctly through `score_bench`, but it (a) drops `ReasoningReport.n` on the floor (`#[allow(dead_code)]`), (b) routes reasoning keys through the generic `eval_collateral_damage`, which treats a zero pre-score as "no drop", and (c) writes no slice label. A baseline that lists only `gsm8k_think` (no `_init` partner) is silently accepted and never checks think-initiation.
The cheapest correct design: keep the generic path for non-reasoning benches untouched; add one pure strict evaluator for reasoning keys (in `vox-eval`, tested without a GPU); make the writer split the baseline into reasoning and other keys, carry `n` out of `score_bench`, and write an additive `reasoning_slices` block; make serve call one extracted, tempdir-testable function that re-validates the on-disk report instead of trusting `status` alone, and cross-checks the per-slice `reasoning_report.json` files phase 314 already writes. No model runs at serve time. No new crate edges are needed.
... [lean-ctx: omitted 1 lines]
## Architectural Responsibility Map
| Capability | Primary Tier | Secondary Tier | Rationale |
... [lean-ctx: omitted 1 lines]
| Pure reasoning-slice comparison (finite, pre>0, n==32, 5% bar) | `vox-eval` (L0-ish pure scoring) | — | Already owns `SLICE_SIZE`, `SLICE_NAMES`, `eval_collateral_damage`; testable with `cargo test -p vox-eval --lib`, no `gpu` feature. |
| Baseline expansion, scoring loop, report writing | `vox-ml-cli` `eval_collateral.rs` | — | Existing writer; owns `score_bench`. |
... [lean-ctx: omitted 1 lines]
| Report consumers | `vox-gui` `mens_run_reports.rs` | — | Reads only `status`, `failed_on`; additive fields are safe. |
## Standard Stack
No new dependencies. `[VERIFIED: crates/vox-eval/src/reasoning_bench.rs:17-21]`
```rust
/// Number of items in every held-out slice.
pub const SLICE_SIZE: usize = 32;

/// Directory names under `mens/data/` that hold the three slices.
pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];
```
Everything needed is already in the workspace: `serde_json`, `anyhow`, `tempfile` (dev), `vox_eval::reasoning_bench::{SLICE_SIZE, SLICE_NAMES, BenchOutcome}`. No packages are installed, so there is no Package Legitimacy Audit for this phase.
**Dependency discipline:** `vox-ml-cli` already imports `vox_eval` (`eval_collateral.rs:5` `use vox_eval::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage_suite};`) and `vox_eval::reasoning_bench` (`reasoning_eval.rs`: `use vox_eval::reasoning_bench::{BenchOutcome, ReasoningItem, SLICE_SIZE, load_slice};`). No new crate edge, so no `crate-edges` exception is needed.
## Verified Current Behaviour (what the plan changes)
**Serve gate** `[VERIFIED: crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:353-374]`
```rust
// Gate: Ensure collateral damage eval has run and passed if we're serving an adapter
let manifest_path = model.join("training_manifest.json");
let adapter_path = model.join("candle_qlora_adapter.safetensors");
if manifest_path.exists() || adapter_path.exists() {
    let report_path = model.join("collateral_damage_report.json");
    if !report_path.exists() {
        anyhow::bail!(
            "eval_collateral_damage check not found! Run `vox mens eval-collateral-damage --pre-score <baseline.json> --post-adapter <adapter>` before serving this adapter."
        );
    }
    let report_raw = std::fs::read_to_string(&report_path)?;
    let report_json: serde_json::Value = serde_json::from_str(&report_raw)?;
    let status = report_json.get("status").and_then(|s| s.as_str());
    if status != Some("pass") {
        anyhow::bail!(
            "eval_collateral_damage check FAILED. The adapter degraded performance beyond acceptable thresholds and cannot be served."
        );
    }
    assert_serve_preconditions(&model)?;
}
```
... [lean-ctx: omitted 1 lines]
**Writer** `[VERIFIED: crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:11-103]`: reads `{"benchmarks": {name: f64}}` into a `BTreeMap<String,f64>` (so benches score in sorted order), scores every key with the closure `score_bench: &mut dyn FnMut(&str) -> Result<f64>` (an `Err` propagates, no report written), then calls `eval_collateral_damage_suite(&borrowed, &cfg)`. The suite returns `Err(first failing)`, so a failing report holds `reports: vec![bad]` only. The envelope is `{"status","failed_on","threshold","reports":[{"benchmark","pre","post","degradation","degradation_rate","exceeds_threshold"}]}`. It writes to `run_dir.join("collateral_damage_report.json")` with `run_dir` = the adapter path.
... [lean-ctx: omitted 12 lines]
`BENCH_ROOT` is the relative `"mens/data"`, so reasoning detection is cwd-dependent. The gate must therefore classify reasoning keys **by name** (against `SLICE_NAMES`), not by `is_file()`. A baseline key that is `gsm8k_think` run from a wrong cwd currently falls through to the `manifest.json` branch and errors with a "no manifest" message, which is a fail-closed outcome but a confusing one.
... [lean-ctx: omitted 22 lines]
`reasoning_eval::write_outcome` writes this (plus `benchmark`, and for scored: `temperature`, `prompt`, `items`) to `<adapter>/collateral_eval/<bench>/reasoning_report.json` `[VERIFIED: reasoning_eval.rs:239-247 path, 313-style tests read it]`. `fail_closed` writes the `not_scored` form and **replaces** a stale scored one (test `a_not_scored_write_replaces_a_stale_scored_report`).
... [lean-ctx: omitted 10 lines]
Consequences for the reasoning keys: `NaN` post gives `degradation = max(NaN, 0.0)` = `0.0` in Rust (`f64::max` ignores NaN), so a NaN post-score **passes**; a zero pre passes. Both are the holes the locked decisions close. `exceeds` is a strict `>`, so exactly 5% passes (`semcov_wave37_tests::collateral_damage_exactly_at_threshold_does_not_exceed`). The zero-pre test is `[VERIFIED: lib.rs:879-887]` `collateral_damage_zero_pre_score_avoids_divide_by_zero` (module `semcov_wave37_tests`) asserting `r.degradation_rate.is_finite()` and `!r.exceeds_threshold` for `("bench", 0.0, 0.0)`. Do not edit `eval_collateral_damage`.
## Architecture Patterns
### Data flow
```
baseline.json {"benchmarks": {name: f64}}
        │ BTreeMap, sorted
        ▼
 run_collateral_damage_with ── classify each key by NAME ──┐
        │                                                   │
        │ non-reasoning                       reasoning (SLICE_NAMES, with/without `_init`)
        ▼                                                   ▼
 score_bench ─► f64                         validate baseline: both keys present per slice
        │                                   score_bench ─► (f64, n)   (Err on not_scored ⇒ no report)
        ▼                                                   ▼
 eval_collateral_damage_suite              eval_reasoning_slices (strict: finite, pre>0, n==32,
 (unchanged, first-failure)                 each of accuracy / think-init at 5%)
        └───────────────┬───────────────────────────────────┘
                        ▼
      collateral_damage_report.json  (status, failed_on, threshold, reports,
                                       + reasoning_slices{scope_note, slice_size, benches})
                        ▼   (on disk; no model)
 vox mens serve ─► assert_collateral_gate(model_dir): exists ► parses ► status==pass ►
      reasoning_slices complete (3 slices, n==32, finite, pre>0, recomputed rate ≤ threshold) ►
      collateral_eval/<slice>/reasoning_report.json scored, n==32 ► assert_serve_preconditions
```
### Pattern 1: Strict reasoning evaluator (pure, in `vox-eval`)
**What:** `pub fn eval_reasoning_slice(bench: &str, acc: (pre, post), think: (pre, post), n: usize, cfg) -> Result<ReasoningSliceVerdict, ReasoningGateError>`. It refuses before arithmetic: `n != SLICE_SIZE`; any of the four values non-finite; any `pre <= 0.0`; any value outside `[0.0, 1.0]` (recommended extra guard, the scorers cannot produce one; `[ASSUMED]` that rejecting it is wanted). After guards, call the existing `eval_collateral_damage` for each key so there is one arithmetic implementation. Return both per-key `CollateralDamageReport`s; `exceeds` is the OR.
... [lean-ctx: omitted 1 lines]
**Where `n` comes from:** change the writer's scorer closure to return a small `BenchScore { value: f64, n: Option<usize> }` (`n = Some(report.n)` only for reasoning keys), or add a second closure. This touches the four existing writer tests (they return `Ok(f64)`); update them mechanically. Defense in depth: `score_bench` should also `bail!` if `report.n != SLICE_SIZE` even though `load_slice` already guarantees it, so the check does not depend on a distant invariant.
### Pattern 2: Report schema (additive)
Keep every existing key (`status`, `failed_on`, `threshold`, `reports[]`), so `vox-gui`'s `parse_collateral` (reads only `status`, `failed_on`, `[VERIFIED: mens_run_reports.rs:196-208]`) keeps working. Add:
```json
"reasoning_slices": {
  "scope_note": "Reasoning benches are fixed 32-item slices (gsm8k_think, math500_think, vox_think), not full GSM8K or MATH-500 results. The 5% relative bar applies to these 32 items only.",
  "slice_size": 32,
  "benches": {
    "gsm8k_think": {
      "n": 32,
      "accuracy":        {"pre": 0.0, "post": 0.0, "degradation_rate": 0.0, "exceeds_threshold": false},
      "think_initiation":{"pre": 0.0, "post": 0.0, "degradation_rate": 0.0, "exceeds_threshold": false}
    }
  }
}
```
... [lean-ctx: omitted 2 lines]
### Pattern 3: Serve re-validation (read-only)
Extract the inline block into `#[cfg(feature = "gpu")] fn assert_collateral_gate(model: &Path) -> Result<()>` in `dispatch.rs` (next to `assert_serve_preconditions`, which it precedes), so it is unit-testable with a tempdir like `serve_preconditions_tests`. It must:
... [lean-ctx: omitted 2 lines]
3. Require `reasoning_slices` with all three `SLICE_NAMES`; for each: `n == 32`, four finite values, `pre > 0`; recompute `degradation_rate` with the same `vox-eval` function and refuse if `> threshold` **regardless of the stored `status`/`exceeds_threshold`**. This is what makes "serve refuses when accuracy or think-initiation drops" true even for a hand-edited or stale `pass`.
4. Require `collateral_eval/<slice>/reasoning_report.json` to exist with `status == "scored"` and `n == 32` (the literal "missing slice report" / "not_scored" cases from the locked decision). Do not require numeric equality with the collateral report (optional hardening).
... [lean-ctx: omitted 1 lines]
### Pattern 4: Delete the stale report before scoring
`run_collateral_damage_with` never removes an existing `collateral_damage_report.json`. If a rerun fails (adapter now unloadable, slice not scored), `score_bench` returns `Err`, no new report is written, and **the previous passing report stays on disk** while `reasoning_report.json` is overwritten with `not_scored`. Remove the old report (`remove_file`, ignore `NotFound`) after the baseline parses and before the scoring loop. Serve step 4 above is the second line of defense. The existing test `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` uses a fresh dir and cannot catch this; add a pre-seeded-stale-report variant.
### Decision: two explicit keys, not expansion
Recommend **two explicit baseline keys per slice** (`gsm8k_think`, `gsm8k_think_init`), matching the shipped doc format (`mens-reasoning-preservation-research-2026-09-21.md`, line ~39) and `score_bench`'s existing suffix handling, **plus a pairing rule**: for each slice named in the baseline, the partner key must also be present, else hard error (`gsm8k_think` alone currently skips think-initiation silently). Expansion would invent a baseline value the operator never supplied.
**Open question for the planner:** must the baseline contain all three slices, or only those it names? Recommended: the **serve** side requires all three (the on-disk report is the contract), and the **writer** refuses a baseline that lacks any of the six reasoning keys (three slices, two keys each). Both sides require all six. This is a stricter reading of GATE-01 than the literal text and it retires the current vacuous-pass shape (`mens-reasoning-preservation-research` §3.4). `[ASSUMED]` needs confirmation; the consequence is that existing run dirs with an old-format passing report are refused until re-evaluated (none are present on this machine; `mens/runs` holds only `logs`).
## Don't Hand-Roll
| Problem | Don't Build | Use Instead | Why |
... [lean-ctx: omitted 1 lines]
| 5% relative degradation arithmetic | A second formula | `vox_eval::eval_collateral_damage` after the guards | One implementation; the `>` boundary is already tested. |
| Slice size and names | Literal `32` / string lists | `SLICE_SIZE`, `SLICE_NAMES` | Single source; the loader already enforces it. |
| Per-slice outcome shape | A new report format | Existing `reasoning_report.json` (`BenchOutcome::to_json`) | Serve reads what phase 314 already writes. |
... [lean-ctx: omitted 1 lines]
## Common Pitfalls
[… truncated at ~4065 of 7135 tokens — use ctx_read with lines= parameter to see specific sections]
