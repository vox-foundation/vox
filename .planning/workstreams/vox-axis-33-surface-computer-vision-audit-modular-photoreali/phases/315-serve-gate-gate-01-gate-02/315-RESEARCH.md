# Phase 315: Serve Gate - Research

**Researched:** 2026-10-26
**Domain:** Rust, `vox-ml-cli` collateral-damage writer and `vox mens serve` gate; `vox-eval` pure scoring helpers
**Confidence:** HIGH (every value below was read from source this session; the model-dependent parts are out of scope by decision)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Compare accuracy and think-initiation separately on each reasoning slice.
- Both use the existing 5% relative bar (`max_degradation_rate` 0.05).
- A drop on either key fails the adapter.
- Non-reasoning benches keep their single score.
- A missing slice report, a `not_scored` report, or any `n` other than 32 is a hard error. Serve refuses. Those cases are not stored as accuracy 0.0 or think-initiation 0.0.
- A non-finite accuracy or think-initiation refuses the adapter. A reasoning pre-score of zero refuses the adapter. The current "pre-score near zero means no drop" path does not apply to these reasoning keys. That generic path may stay for non-reasoning benches. The existing zero-pre test on `eval_collateral_damage` is not a license to pass a reasoning key whose pre-score is zero.
- The collateral report says these benches are 32-item slices, not full GSM8K or MATH-500, and it records `n`.
- Serve only reads the report already on disk. No model run in this phase. No before/after numbers in this phase (phase 316).

### Claude's Discretion
- Whether each reasoning slice is two baseline keys (`gsm8k_think` and `gsm8k_think_init`) or one key the gate expands, as long as both scores are compared and a drop on either fails.
- The exact refusal text, as long as serve does not bind and the report names the benches as slices and records `n`.

### Deferred Ideas (OUT OF SCOPE)
- Live base-versus-adapter comparison, or a "snapshot absent" record (phase 316).
- A report marker for an unclosed think block (WR-02, phase 316).
- MATH-500 equivalent surface forms (WR-03). The grader and the slice stay locked.
- Swapping the pinned Qwen3-8B base, or a new training technique.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| GATE-01 | Serve refuses an adapter when the collateral report is missing, or when accuracy or think-initiation on the lane benches drops past the configured threshold. | Serve already refuses a missing report; it does not read any per-key data, so the new work is a strict reasoning evaluator (writer side) plus a read-only re-check of the on-disk report (serve side). See Patterns 1-3. |
| GATE-02 | The report says these benches are slices, so the 5% relative bar is not read as a full GSM8K or MATH-500 result. | Add an additive `reasoning_slices` block and per-entry `slice`/`n` fields to `collateral_damage_report.json`. See Pattern 2. |
</phase_requirements>

## Summary

Today's gate is thinner than the context file suggests. `dispatch.rs` checks only that `collateral_damage_report.json` exists and that `status == "pass"`; it never reads a per-bench value. The writer (`run_collateral_damage_with`) already scores `_init` keys correctly through `score_bench`, but it (a) drops `ReasoningReport.n` on the floor (`#[allow(dead_code)]`), (b) routes reasoning keys through the generic `eval_collateral_damage`, which treats a zero pre-score as "no drop", and (c) writes no slice label. A baseline that lists only `gsm8k_think` (no `_init` partner) is silently accepted and never checks think-initiation.

The cheapest correct design: keep the generic path for non-reasoning benches untouched; add one pure strict evaluator for reasoning keys (in `vox-eval`, tested without a GPU); make the writer split the baseline into reasoning and other keys, carry `n` out of `score_bench`, and write an additive `reasoning_slices` block; make serve call one extracted, tempdir-testable function that re-validates the on-disk report instead of trusting `status` alone, and cross-checks the per-slice `reasoning_report.json` files phase 314 already writes. No model runs at serve time. No new crate edges are needed.

**Primary recommendation:** One plan (`315-01`), three tasks, test-first: (1) `vox-eval` strict evaluator, (2) writer: expand/validate keys, carry `n`, write slice label, delete any stale report before scoring, (3) serve: extract `assert_collateral_gate` and re-validate the report plus slice reports. Use two explicit baseline keys per slice (`<slice>` and `<slice>_init`), and make a missing partner key a hard error.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Pure reasoning-slice comparison (finite, pre>0, n==32, 5% bar) | `vox-eval` (L0-ish pure scoring) | — | Already owns `SLICE_SIZE`, `SLICE_NAMES`, `eval_collateral_damage`; testable with `cargo test -p vox-eval --lib`, no `gpu` feature. |
| Baseline expansion, scoring loop, report writing | `vox-ml-cli` `eval_collateral.rs` | — | Existing writer; owns `score_bench`. |
| Serve refusal | `vox-ml-cli` `populi/dispatch.rs` | — | Existing gate site; read-only. |
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
The arm is `#[cfg(feature = "gpu")]` (`dispatch.rs:~312`), and the local-serve gate is inline, so it has no unit test today. Only the cloud path (`cloud != "local"`) returns before it. Pre-existing and out of scope: a directory with neither `training_manifest.json` nor `candle_qlora_adapter.safetensors` skips the gate entirely.

**Writer** `[VERIFIED: crates/vox-ml-cli/src/commands/mens/eval_collateral.rs:11-103]`: reads `{"benchmarks": {name: f64}}` into a `BTreeMap<String,f64>` (so benches score in sorted order), scores every key with the closure `score_bench: &mut dyn FnMut(&str) -> Result<f64>` (an `Err` propagates, no report written), then calls `eval_collateral_damage_suite(&borrowed, &cfg)`. The suite returns `Err(first failing)`, so a failing report holds `reports: vec![bad]` only. The envelope is `{"status","failed_on","threshold","reports":[{"benchmark","pre","post","degradation","degradation_rate","exceeds_threshold"}]}`. It writes to `run_dir.join("collateral_damage_report.json")` with `run_dir` = the adapter path.

**`score_bench`** `[VERIFIED: eval_collateral.rs:134-152]`
```rust
let (dir_name, want_think_rate) = match bench.strip_suffix("_init") {
    Some(base) => (base, true),
    None => (bench, false),
};
let reasoning = Path::new(BENCH_ROOT).join(dir_name).join("reasoning.jsonl");
if reasoning.is_file() {
    let report = cached_reasoning_report(dir_name, &reasoning, adapter)?;
    return Ok(if want_think_rate { report.think_initiation_rate } else { report.accuracy });
}
```
`BENCH_ROOT` is the relative `"mens/data"`, so reasoning detection is cwd-dependent. The gate must therefore classify reasoning keys **by name** (against `SLICE_NAMES`), not by `is_file()`. A baseline key that is `gsm8k_think` run from a wrong cwd currently falls through to the `manifest.json` branch and errors with a "no manifest" message, which is a fail-closed outcome but a confusing one.

**`ReasoningReport`** `[VERIFIED: reasoning_eval.rs:22-31]`
```rust
pub(crate) struct ReasoningReport {
    pub(crate) accuracy: f64,
    pub(crate) think_initiation_rate: f64,
    /// Items actually generated. Written into the JSON report; the gate reads
    /// accuracy and think rate off this struct.
    #[allow(dead_code)]
    pub(crate) n: usize,
}
```
`n` exists but `score_bench` discards it. It is always 32 on `Ok`: `load_slice(items_path, SLICE_SIZE)` refuses any other count, and `BenchOutcome::from_score` returns `Scored` only when `score.n == n_requested`. `NotScored` becomes `Err` (`reasoning bench '{bench}' not scored: {reason}`), so a not-scored slice already aborts the writer with no collateral report.

**`BenchOutcome` JSON** `[VERIFIED: crates/vox-eval/src/reasoning_bench.rs:119-141]`
```rust
BenchOutcome::Scored(s) => serde_json::json!({
    "status": "scored", "n": s.n, "accuracy": s.accuracy,
    "think_initiation_rate": s.think_initiation_rate,
}),
BenchOutcome::NotScored { reason, n_requested } => serde_json::json!({
    "status": "not_scored", "n": 0, "n_requested": n_requested, "reason": reason,
}),
```
`reasoning_eval::write_outcome` writes this (plus `benchmark`, and for scored: `temperature`, `prompt`, `items`) to `<adapter>/collateral_eval/<bench>/reasoning_report.json` `[VERIFIED: reasoning_eval.rs:239-247 path, 313-style tests read it]`. `fail_closed` writes the `not_scored` form and **replaces** a stale scored one (test `a_not_scored_write_replaces_a_stale_scored_report`).

**Generic evaluator** `[VERIFIED: crates/vox-eval/src/lib.rs:295-318]`
```rust
let degradation = (pre_training_score - post_training_score).max(0.0);
let degradation_rate = if pre_training_score > f64::EPSILON {
    degradation / pre_training_score
} else {
    0.0
};
let exceeds_threshold = degradation_rate > config.max_degradation_rate;
```
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
**Why a typed error and not a failing report:** the locked rule is that these cases are hard errors that refuse, not stored scores. A typed `Err` makes the writer `bail!` (no report written) exactly like an unloadable adapter.
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
(Numbers above are shape placeholders in a schema sketch, not scores.) Also add `"slice": true, "n": 32` to each reasoning entry in `reports[]`. Do not claim a full-dataset row count in the note; the slice files are pinned at 32 rows `[VERIFIED: reasoning_bench.rs test checked_in_gsm8k_and_math500_slices_have_32_pinned_rows]`.
**Write the `reasoning_slices` block on both pass and fail.** The generic suite stops at the first failing bench and writes only that one in `reports`; the reasoning block should list every compared slice regardless, so a failure on `vox_think` is not hidden by an earlier non-reasoning failure. Score every key first (current behavior), then evaluate both groups.

### Pattern 3: Serve re-validation (read-only)
Extract the inline block into `#[cfg(feature = "gpu")] fn assert_collateral_gate(model: &Path) -> Result<()>` in `dispatch.rs` (next to `assert_serve_preconditions`, which it precedes), so it is unit-testable with a tempdir like `serve_preconditions_tests`. It must:
1. Bail on a missing `collateral_damage_report.json` (keep the existing message; GATE-01 clause 1).
2. Bail unless `status == "pass"` (existing).
3. Require `reasoning_slices` with all three `SLICE_NAMES`; for each: `n == 32`, four finite values, `pre > 0`; recompute `degradation_rate` with the same `vox-eval` function and refuse if `> threshold` **regardless of the stored `status`/`exceeds_threshold`**. This is what makes "serve refuses when accuracy or think-initiation drops" true even for a hand-edited or stale `pass`.
4. Require `collateral_eval/<slice>/reasoning_report.json` to exist with `status == "scored"` and `n == 32` (the literal "missing slice report" / "not_scored" cases from the locked decision). Do not require numeric equality with the collateral report (optional hardening).
5. Message text: name the slice and the cause; keep the `eval_collateral_damage` prefix so existing greps still match.

### Pattern 4: Delete the stale report before scoring
`run_collateral_damage_with` never removes an existing `collateral_damage_report.json`. If a rerun fails (adapter now unloadable, slice not scored), `score_bench` returns `Err`, no new report is written, and **the previous passing report stays on disk** while `reasoning_report.json` is overwritten with `not_scored`. Remove the old report (`remove_file`, ignore `NotFound`) after the baseline parses and before the scoring loop. Serve step 4 above is the second line of defense. The existing test `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` uses a fresh dir and cannot catch this; add a pre-seeded-stale-report variant.

### Decision: two explicit keys, not expansion
Recommend **two explicit baseline keys per slice** (`gsm8k_think`, `gsm8k_think_init`), matching the shipped doc format (`mens-reasoning-preservation-research-2026-09-21.md`, line ~39) and `score_bench`'s existing suffix handling, **plus a pairing rule**: for each slice named in the baseline, the partner key must also be present, else hard error (`gsm8k_think` alone currently skips think-initiation silently). Expansion would invent a baseline value the operator never supplied.
**Resolved:** both the serve side and the writer require all six keys (three slices, accuracy and think-initiation). This is the stricter reading of the locked "each reasoning slice" decision. Old-format passing reports are refused until re-evaluated. None are present in this checkout (`mens/runs` holds only `logs`).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| 5% relative degradation arithmetic | A second formula | `vox_eval::eval_collateral_damage` after the guards | One implementation; the `>` boundary is already tested. |
| Slice size and names | Literal `32` / string lists | `SLICE_SIZE`, `SLICE_NAMES` | Single source; the loader already enforces it. |
| Per-slice outcome shape | A new report format | Existing `reasoning_report.json` (`BenchOutcome::to_json`) | Serve reads what phase 314 already writes. |
| Serve-side scoring | Re-running the model | Read the on-disk report only | Locked; phase 316 owns live runs. |

## Common Pitfalls

### Pitfall 1: A key that falls out of the reasoning branch
`score_bench` detects reasoning by `reasoning.jsonl` on a cwd-relative path. Classify keys by name for gating, or a wrong cwd turns a reasoning key into a generic one.
### Pitfall 2: NaN passes the generic path
`(pre - NaN).max(0.0)` is `0.0`. Guard with `is_finite()` before calling the generic function (test with `f64::NAN` post and `f64::INFINITY`).
### Pitfall 3: JSON cannot carry NaN
`serde_json` rejects the `NaN` token, so a non-finite **pre** cannot arrive from a baseline file in standard JSON; a non-finite **post** can only come from the scorer. Tests must drive these through the scorer closure and a direct call to the evaluator, not through a baseline string. A huge exponent such as `1e999` may also parse as an error or as infinity depending on `serde_json`; `[ASSUMED]`, so test it rather than rely on it (either outcome must refuse).
### Pitfall 4: Stale pass report (see Pattern 4)
### Pitfall 5: Trusting `status`
`status` is a string a person or an older writer produced. Serve recomputes from stored numbers.
### Pitfall 6: Zero pre is plausible
A baseline zero on `vox_think` accuracy is a legitimate base-model result, but the locked rule refuses it, so the message must say "baseline of 0 gives no relative bar; re-baseline or choose a different slice" rather than a bare failure.
### Pitfall 7: Stale doc text
`docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` still says `gsm8k_think` is "first 64 rows" and documents `VOX_REASONING_EVAL_LIMIT`. Neither matches the code (slices are 32 rows; the env var appears nowhere in `crates/`; verified by search). Do not update that page in this phase. Do not edit this file in phase 315. It has unrelated uncommitted changes and stays unstaged. The 64-row and `VOX_REASONING_EVAL_LIMIT` sentences are a docs follow-up, not a gate task.
 `vox-doc-pipeline` lint applies to the file (`cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`).
### Pitfall 8: Merge/format hazards
Do not run `cargo fmt --all`; format dirty files with `vox run scripts/fmt.vox`. New `pub fn` in `crates/*/src/**` needs a same-file test (Test-First policy, `skeleton/untested-pub-api`).

## Project Constraints (from .cursor/rules/ and AGENTS.md)
- Test first for every new `pub fn`; same-file `#[test]`.
- Do not hard-code `.vox/` paths; the report lives in the adapter dir, as today.
- Do not add `#[derive(Serialize, Deserialize)]` to a `row/` module struct; use `serde_json::json!` projection as the writer does now.
- No new `VOX_*` env var (none are needed; the 5% bar stays `CollateralDamageConfig::default()`).
- Route cargo through the broker (plain `cargo` on PATH).
- No new crate edges; no `exceptions` ledger entries.
- Verify guards by mutation (AGENTS.md PR discipline): break each refusal branch, confirm its test fails, restore.
- Do not touch `.planning/STATE.md`, `ROADMAP.md`, `REQUIREMENTS.md` from implementation commits unless the executor workflow requires it; the tree is dirty.

## Code Examples

### Strict evaluator skeleton (value names only from the verified quotes above)
```rust
// In crates/vox-eval: guards first, then the unchanged generic arithmetic.
use crate::reasoning_bench::SLICE_SIZE;

fn guard(label: &str, bench: &str, pre: f64, post: f64) -> Result<(), String> {
    if !pre.is_finite() || !post.is_finite() {
        return Err(format!("{bench} {label}: non-finite score (pre={pre}, post={post})"));
    }
    if pre <= 0.0 {
        return Err(format!("{bench} {label}: baseline {pre} gives no relative bar; refusing"));
    }
    Ok(())
}
// n != SLICE_SIZE => Err(format!("{bench}: n={n}, expected {SLICE_SIZE}"))
// then: eval_collateral_damage(&format!("{bench}/accuracy"), pre, post, cfg)
```

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`cargo test`), nextest available but not required |
| Config file | none beyond workspace `Cargo.toml` |
| Quick run command | `cargo test -p vox-eval --lib reasoning_slice` |
| Full suite command | `cargo test -p vox-eval --lib && cargo test -p vox-ml-cli --features gpu --lib` |

**Rule for every command below:** `vox-ml-cli` tests need `--features gpu --lib` (the writer, `reasoning_eval`, and the serve arm are all `cfg(feature = "gpu")`). A filter that prints `running 0 tests` / `0 passed` and exits 0 is a **failed** run: read the `test result:` line and confirm the count equals the expected number below. Running a `gpu`-gated test without the feature silently selects zero tests.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| GATE-01 | Reasoning accuracy drop > 5% on any slice fails; exactly 5% passes | unit (pure) | `cargo test -p vox-eval --lib reasoning_slice` (expect ≥ 6 passed) | ❌ Wave 0 |
| GATE-01 | Think-initiation drop > 5% fails while accuracy is flat | unit (pure) | same filter | ❌ Wave 0 |
| GATE-01 | NaN/inf post, pre == 0, n ≠ 32 each return `Err`, never a passing verdict | unit (pure) | same filter | ❌ Wave 0 |
| GATE-01 | Generic zero-pre behavior unchanged | unit | `cargo test -p vox-eval --lib collateral_damage_zero_pre_score_avoids_divide_by_zero` (expect exactly 1 passed) | ✅ |
| GATE-01 | Existing generic suite unchanged | unit | `cargo test -p vox-eval --lib collateral_damage` (currently **10 passed**, observed this session) | ✅ |
| GATE-01 | Writer: partner key missing, zero reasoning pre, NaN scorer value, wrong `n` ⇒ `Err`, **no report file** | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_collateral` | ✅ file, ❌ new tests |
| GATE-01 | Writer: pre-seeded stale passing report is removed when scoring fails | unit | same filter | ❌ Wave 0 |
| GATE-01 | Writer: reasoning degradation (accuracy only, then think only) writes `status: fail`, exit 1, `failed_on` names the key | unit | same filter | ❌ Wave 0 |
| GATE-01 | Serve: missing report / `status: fail` / hand-edited `pass` with a 6% stored drop / missing slice / `not_scored` slice file / `n: 31` ⇒ refuses | unit (tempdir) | `cargo test -p vox-ml-cli --features gpu --lib collateral_gate` (new module `collateral_gate_tests` in `dispatch.rs`) | ❌ Wave 0 |
| GATE-01 | Serve: a fully valid fixture is accepted by the collateral gate (does not run `assert_serve_preconditions`) | unit (tempdir) | same filter | ❌ Wave 0 |
| GATE-02 | Report carries `reasoning_slices.scope_note` mentioning "32-item" and "not full", `slice_size == 32`, per-bench `n == 32` on both pass and fail | unit | `cargo test -p vox-ml-cli --features gpu --lib eval_collateral` | ❌ Wave 0 |
| GATE-02 | Non-reasoning bench entries have no `slice` field and keep one score | unit | same filter | ❌ Wave 0 |
| compat | GUI reader still parses an enriched report | unit | `cargo test -p vox-gui --lib collateral` (confirm count ≥ 1; build needs the `vox-gui` sidecar and `ui/dist` per AGENTS.md, so run only if that setup exists; otherwise note it was skipped) | ✅ |

Name the new tests so each filter above matches at least the stated count, and record the actual `test result: ok. N passed` in the plan SUMMARY.

### Sampling Rate
- **Per task commit:** the task's filter above.
- **Per wave merge:** `cargo test -p vox-eval --lib` and `cargo test -p vox-ml-cli --features gpu --lib eval_collateral collateral_gate` (cargo accepts one filter; run twice).
- **Phase gate:** `cargo check -p vox-ml-cli --features gpu`, the two crate suites above, and `cargo clippy -p vox-ml-cli --features gpu -p vox-eval -- -D warnings` on the touched crates only (hosted CI is the full gate).

### Mutation checks (required by AGENTS.md; run once, then restore)
Break each of: the `is_finite` guard, the `pre <= 0.0` guard, the `n != SLICE_SIZE` guard, the think-initiation comparison, the serve recompute, the slice-report `status == "scored"` check, and the stale-report removal. Each must turn exactly its test red. Check the edit is still present before trusting a red/green result.

### Wave 0 Gaps
- [ ] `crates/vox-eval/src/...` new `reasoning_slice*` tests (same file as the new `pub fn`).
- [ ] New writer tests in `eval_collateral.rs`; update the four existing closures if the scorer signature gains `n`.
- [ ] New `collateral_gate_tests` module in `dispatch.rs`.
- [ ] No framework install needed.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust toolchain via broker `cargo` | all tests | ✓ | workspace pin | — |
| Qwen3-8B weights / adapter on disk | live scoring | ✗ | — | Not needed: this phase runs no model; tests inject scores through the closure and tempdir fixtures. Do not invent a score. |
| `vox-gui` sidecar + `ui/dist` | `vox-gui` compat test | unverified | — | Skip with a stated reason; `mens_run_reports.rs` change is none (additive fields only). |

`~/.cache/huggingface/hub` holds `Qwen3-0.6B`, not `Qwen3-8B`, `[VERIFIED: directory listing this session]`; irrelevant here, relevant to phase 316.

## Runtime State Inventory
Not a rename/refactor phase. One note: existing `collateral_damage_report.json` files in older run dirs are in the old shape and will be refused by the new serve gate (by design, see the open question). None exist under this checkout's `mens/runs` (only `logs`).

## State of the Art

| Old Approach | Current Approach | Impact |
|--------------|------------------|--------|
| Gate on `status == "pass"` only | Gate re-validates numbers and slice reports | A stale or edited `pass` cannot serve. |
| Reasoning keys through the generic zero-pre-tolerant path | Strict evaluator | Zero/NaN cannot pass. |
| Report silent on what a bench is | `reasoning_slices.scope_note` + `n` | 5% is not read as a full-benchmark result. |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Serve and the writer require all three slices and both keys, refusing old-format passing reports | Decision section | RESOLVED 2026-10-26 from the phase 315 context. |
| A2 | A value outside `[0,1]` should refuse | Pattern 1 | Harmless extra guard; drop it if undesired. |
| A3 | `serde_json` handling of `1e999` (error vs infinity) | Pitfall 3 | Test covers both outcomes; no design impact. |
| A4 | `vox-gui` compat test builds locally | Validation | Skippable; additive fields. |

## Open Questions (RESOLVED)

1. **All three slices required, or only those named?** (RESOLVED) All three slices and both keys, on the writer and on serve. The locked set is `gsm8k_think`, `math500_think`, and `vox_think`. A baseline missing any of the six keys is a hard error. Old-format passing reports are refused until re-evaluated. None exist under this checkout's `mens/runs`.
2. **Cross-check numeric equality between the collateral report's post values and each slice `reasoning_report.json`?** (RESOLVED) Not required. Serve checks `status == "scored"` and `n == 32`. Numeric equality is out of this phase.
3. **Should the writer refuse to write `status: pass` when a reasoning key is absent from the baseline?** (RESOLVED) Yes. A baseline that does not name all six keys writes no report and does not pass.

## Security Domain
`security_enforcement` is not disabled, but this phase has no network, auth, crypto, or secret surface. Relevant ASVS items: V5 input validation (the report is untrusted JSON on disk: parse defensively, no `unwrap`, no panics on missing keys) and V14 config (fail closed). Threat: tampered or stale report → mitigated by recomputation and slice-report cross-check. Path handling uses `Path::join` on a fixed filename and a slice name taken from `SLICE_NAMES`, never from the report, so no path traversal from report content.

## Sources

### Primary (HIGH, read this session)
- `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (serve gate, `assert_serve_preconditions`, test module)
- `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (writer, `score_bench`, tests)
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (`ReasoningReport`, `write_outcome`, `fail_closed`)
- `crates/vox-eval/src/lib.rs` (lines 253-336, 420-475, 874-892)
- `crates/vox-eval/src/reasoning_bench.rs` (`SLICE_SIZE`, `SLICE_NAMES`, `BenchOutcome`, `load_slice`)
- `crates/vox-gui/src/commands/mens_run_reports.rs` (`parse_collateral`)
- `.planning/phases/315-serve-gate-gate-01-gate-02/315-CONTEXT.md`, `REQUIREMENTS.md` (GATE-01/02), `ROADMAP.md` phase 315, `STATE.md`
- `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`
- Observed: `cargo test -p vox-eval --lib collateral` ⇒ `10 passed; 0 failed`.

### Secondary / Tertiary
- None. No web research was needed; the phase is internal wiring.

## Metadata
**Confidence breakdown:**
- Current behavior and integration points: HIGH (source read).
- Design (schema, pairing rule, stale-report fix): HIGH for feasibility, MEDIUM for A1.
- Test commands: HIGH for `vox-eval`; the `vox-ml-cli --features gpu --lib` filters are the repo convention and compile-gated on `gpu`; counts depend on tests the plan adds.

**Research date:** 2026-10-26
**Valid until:** until `eval_collateral.rs`, `dispatch.rs` serve arm, or `reasoning_eval.rs` changes.
