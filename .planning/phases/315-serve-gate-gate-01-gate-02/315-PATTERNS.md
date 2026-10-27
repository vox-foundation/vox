# Phase 315: Serve Gate - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 6 (3 modified for behavior, 1 new, 1 one-line wiring, 1 compat-only)
**Analogs found:** 6 / 6 (every excerpt below was read from the tracked file this session; all analog paths pass `git ls-files`)

All paths are repo-relative. Line numbers are from the working tree on 2026-10-26.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/vox-eval/src/reasoning_slice.rs` (NEW; or append to `reasoning_bench.rs`) | utility (pure evaluator) | transform | `crates/vox-eval/src/lib.rs:287-334` (`eval_collateral_damage` + suite) and `crates/vox-eval/src/reasoning_bench.rs:100-154` (`BenchOutcome`, `String` errors) | exact (role + flow) |
| `crates/vox-eval/src/lib.rs` (one `pub mod` line only) | config (module wiring) | n/a | `crates/vox-eval/src/lib.rs:9-17` (`pub mod reasoning_bench;`) | exact |
| `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (writer) | service (report writer) | batch + file-I/O | itself: `run_collateral_damage_with` (17-96), `score_bench` (143-186), `mod tests` (189-289) | exact (modify in place) |
| `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (drop `#[allow(dead_code)]` on `n`, keep behavior) | service (scorer) | file-I/O | itself: `ReasoningReport` (20-31), `score_reasoning_file_with` (48-95), `write_outcome` (237-276) | exact (modify in place) |
| `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (extract `assert_collateral_gate` + `collateral_gate_tests`) | middleware (serve guard) | request-response + file-I/O (read-only) | `assert_serve_preconditions` (816-850) and `serve_preconditions_tests` (852-889) | exact |
| `crates/vox-gui/src/commands/mens_run_reports.rs` (NO edit; compat only) | utility (report reader) | file-I/O | `parse_collateral` (196-208), test (602-615) | exact (unchanged) |

Out of scope, do not edit: `eval_collateral_damage` body (`vox-eval/src/lib.rs:295-317`) and its zero-pre test (879-888); `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` (stays unstaged); `.planning/STATE.md`, `ROADMAP.md`.

## Pattern Assignments

### `crates/vox-eval/src/reasoning_slice.rs` (NEW, utility, transform)

**Placement.** `lib.rs` is already 1006 lines, so do not append there. A sibling module keeps the file small and makes the validated filter `cargo test -p vox-eval --lib reasoning_slice` match by module path (`reasoning_slice::tests::...`) as well as by test name. Alternative if the planner wants zero new files: append to `reasoning_bench.rs` (540 lines, already owns `SLICE_SIZE`/`SLICE_NAMES`); then every test name must contain `reasoning_slice`.

**Analog 1: the arithmetic to reuse, not copy** (`crates/vox-eval/src/lib.rs`, lines 255-317):
```rust
/// Result of evaluating collateral damage on a held-out benchmark.
#[derive(Debug, Clone)]
pub struct CollateralDamageReport {
    pub benchmark_name: String,
    pub pre_training_score: f64,
    pub post_training_score: f64,
    pub degradation: f64,
    pub degradation_rate: f64,
    pub exceeds_threshold: bool,
}

#[derive(Debug, Clone)]
pub struct CollateralDamageConfig {
    /// Maximum allowed degradation rate before blocking model promotion (default 0.05 = 5%).
    pub max_degradation_rate: f64,
}
// Default => max_degradation_rate: 0.05

pub fn eval_collateral_damage(
    benchmark_name: &str,
    pre_training_score: f64,
    post_training_score: f64,
    config: &CollateralDamageConfig,
) -> CollateralDamageReport {
    let degradation = (pre_training_score - post_training_score).max(0.0);
    let degradation_rate = if pre_training_score > f64::EPSILON {
        degradation / pre_training_score
    } else {
        0.0
    };
    let exceeds_threshold = degradation_rate > config.max_degradation_rate;
    CollateralDamageReport { /* fields as above */ }
}
```
Holes the new guards close (reasoning keys only): `f64::max` ignores NaN so a NaN post gives `degradation == 0.0`; `pre <= EPSILON` gives rate 0.0. Guard first, then call this function so there is one arithmetic implementation. `exceeds_threshold` is a strict `>`, so exactly 5% passes.

**Analog 2: crate idiom for refusals and module imports** (`crates/vox-eval/src/reasoning_bench.rs`, lines 8-21 and 40-41, 90-96):
```rust
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::qwen_reasoning::ReasoningScore;

/// Number of items in every held-out slice.
pub const SLICE_SIZE: usize = 32;

/// Directory names under `mens/data/` that hold the three slices.
pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];

pub fn load_slice(path: &Path, expected_n: usize) -> Result<Vec<ReasoningItem>, String> { ... }
    if items.len() != expected_n {
        return Err(format!(
            "{}: found {} items, expected {expected_n}",
            path.display(),
            items.len()
        ));
    }
```
`vox-eval` has no `thiserror` and no hand-written `Error` impls (searched `crates/vox-eval/src`). Match the crate: `Result<_, String>` (or a small enum with a manual `Display`). The consumer (`vox-ml-cli`) wraps with `anyhow::bail!`, so a `String` error is enough. Import for the new file: `use crate::reasoning_bench::{SLICE_NAMES, SLICE_SIZE}; use crate::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage};`.

**Core pattern to write** (shape from RESEARCH Pattern 1; values only from verified quotes):
```rust
fn guard(label: &str, bench: &str, pre: f64, post: f64) -> Result<(), String> {
    if !pre.is_finite() || !post.is_finite() {
        return Err(format!("{bench} {label}: non-finite score (pre={pre}, post={post})"));
    }
    if pre <= 0.0 {
        return Err(format!("{bench} {label}: baseline {pre} gives no relative bar; re-baseline or choose a different slice"));
    }
    Ok(())
}
// n != SLICE_SIZE => Err(format!("{bench}: n={n}, expected {SLICE_SIZE}"))
// then: eval_collateral_damage(&format!("{bench}/accuracy"), pre, post, cfg)
//       eval_collateral_damage(&format!("{bench}/think_initiation"), pre, post, cfg)
// verdict.exceeds = accuracy.exceeds_threshold || think.exceeds_threshold
```
Guards run before any arithmetic and return `Err`, never a passing or zero-valued verdict (locked: not stored as 0.0). Optional `[0,1]` range guard is flagged `[ASSUMED]` in RESEARCH (A2).

**Testing pattern** (same file, same `mod tests` style; `crates/vox-eval/src/lib.rs`, lines 420-443):
```rust
#[cfg(test)]
mod collateral_damage_tests {
    use super::*;

    #[test]
    fn large_degradation_fails() {
        let r = eval_collateral_damage("gsm8k", 0.80, 0.70, &CollateralDamageConfig::default());
        assert!(r.exceeds_threshold);
        assert!(r.degradation_rate > 0.05);
    }
```
Name tests `reasoning_slice_*` or put them in `mod reasoning_slice_tests`. Required cases: accuracy drop above 5% fails; exactly 5% passes (copy the boundary idea from `semcov_wave37_tests::collateral_damage_exactly_at_threshold_does_not_exceed`); think-initiation drop fails with accuracy flat; NaN post, `f64::INFINITY`, pre 0.0, n 31 and n 33 each `Err`. Expect at least 6 passed on the filter. Test-First policy (`skeleton/untested-pub-api`): every new `pub fn` needs a same-file test.

**Do not touch** the zero-pre test that must stay green (`lib.rs`, lines 879-888):
```rust
    #[test]
    fn collateral_damage_zero_pre_score_avoids_divide_by_zero() {
        let r = eval_collateral_damage("bench", 0.0, 0.0, &CollateralDamageConfig::default());
        assert!(r.degradation_rate.is_finite(), "degradation_rate must be finite when pre=0");
        assert!(!r.exceeds_threshold);
    }
```
(module `semcov_wave37_tests`, line 784.) Validation command: `cargo test -p vox-eval --lib collateral_damage_zero_pre_score_avoids_divide_by_zero` must print exactly 1 passed.

---

### `crates/vox-eval/src/lib.rs` (one-line wiring)

**Analog:** the existing module list, lines 9-17:
```rust
pub mod compare;
pub mod corpus_score;
pub mod corpus_stats;
pub mod mens;
pub mod provenance;
pub mod qwen_reasoning;
pub mod reasoning_bench;
pub mod report;
pub mod trial_flywheel;
```
Insert `pub mod reasoning_slice;` after `pub mod reasoning_bench;`. Nothing else in this file changes. Skip entirely if the evaluator is appended to `reasoning_bench.rs`.

---

### `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (writer, service, batch + file-I/O)

**Analog:** itself. Module is `#[cfg(feature = "gpu")]` (`crates/vox-ml-cli/src/commands/mens/mod.rs:24-25`), so every test needs `--features gpu --lib`.

**Imports pattern** (lines 1-5):
```rust
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vox_eval::{CollateralDamageConfig, CollateralDamageReport, eval_collateral_damage_suite};
```
Add `use vox_eval::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};` and the new evaluator import. `reasoning_eval.rs:18` already imports from `vox_eval::reasoning_bench`, so no new crate edge.

**Scorer signature and baseline parse** (lines 17-45), the seam to change:
```rust
pub(crate) fn run_collateral_damage_with(
    pre_score_path: &Path,
    run_dir: &Path,
    score_bench: &mut dyn FnMut(&str) -> Result<f64>,
) -> Result<i32> {
    ...
    let pre: BTreeMap<String, f64> = serde_json::from_value(
        pre_file.get("benchmarks").filter(|v| v.is_object()).cloned().ok_or_else(|| { ... })?,
    )?;

    if pre.is_empty() {
        anyhow::bail!(
            "pre-score file {} has an empty \"benchmarks\" object — nothing to score",
            pre_score_path.display()
        );
    }
```
Planner choices (RESEARCH Pattern 1): make the closure return a small `BenchScore { value: f64, n: Option<usize> }` (`n = Some(report.n)` for reasoning keys) or add a second closure. Four existing tests use `Ok(f64)` closures and need a mechanical update (lines 192-289).

**Insert after the baseline parses and the empty check, before the scoring loop** (Pattern 4, stale report removal; new code, idiom from the file's own `std::fs` use at lines 78-79):
```rust
// remove_file on run_dir/collateral_damage_report.json; ignore io::ErrorKind::NotFound, propagate other errors
```
Also in this region: classify keys **by name** against `SLICE_NAMES` (strip `_init`), require all six keys (three slices x accuracy and `_init`), bail on any missing partner with a message naming the key. Do this before any scoring so a bad baseline writes no report.

**Scoring loop and suite (keep for non-reasoning keys)** (lines 47-62):
```rust
    let mut scores: Vec<(String, f64, f64)> = Vec::with_capacity(pre.len());
    for (bench, pre_score) in &pre {
        let post = score_bench(bench)?; // fails loudly; never falls back to `pre`
        scores.push((bench.clone(), *pre_score, post));
    }

    let borrowed: Vec<(&str, f64, f64)> = scores
        .iter()
        .map(|(n, a, b)| (n.as_str(), *a, *b))
        .collect();
    let cfg = CollateralDamageConfig::default();
    let (status, failed_on, reports) = match eval_collateral_damage_suite(&borrowed, &cfg) {
        Ok(rs) => ("pass", None, rs),
        Err(bad) => ("fail", Some(bad.benchmark_name.clone()), vec![bad]),
    };
```
Rule: score every key first (an `Err` aborts with no report), then evaluate two groups: non-reasoning through `eval_collateral_damage_suite` unchanged, reasoning through the new strict evaluator. `status` is `fail` if either group fails. `failed_on` must name the failing key (the existing test asserts `report["failed_on"] == "code_bench"`; for reasoning use the key, e.g. `gsm8k_think_init`).

**Report projection and write** (lines 63-95), extend additively:
```rust
    // `CollateralDamageReport` is not `Serialize`; project it by hand.
    let reports_json: Vec<_> = reports
        .iter()
        .map(|r: &CollateralDamageReport| {
            json!({
                "benchmark": r.benchmark_name,
                "pre": r.pre_training_score,
                "post": r.post_training_score,
                "degradation": r.degradation,
                "degradation_rate": r.degradation_rate,
                "exceeds_threshold": r.exceeds_threshold,
            })
        })
        .collect();

    std::fs::create_dir_all(run_dir)?;
    std::fs::write(
        run_dir.join("collateral_damage_report.json"),
        serde_json::to_string_pretty(&json!({
            "status": status,
            "failed_on": failed_on,
            "threshold": cfg.max_degradation_rate,
            "reports": reports_json,
        }))?,
    )?;
```
Add one key to this `json!`: `"reasoning_slices": {"scope_note", "slice_size": SLICE_SIZE, "benches": {<slice>: {"n", "accuracy": {pre, post, degradation_rate, exceeds_threshold}, "think_initiation": {...}}}}` (schema in RESEARCH Pattern 2). Keep `status`, `failed_on`, `threshold`, `reports` untouched. Write the block on pass **and** fail. Add `"slice": true, "n": 32` to reasoning entries in `reports[]`; non-reasoning entries get no `slice` field. Keep the hand projection: do not derive `Serialize` on the report type (data-storage policy, and the file's own comment at line 63). The `scope_note` must say "32-item" and "not full" (GATE-02 test asserts both). Use `serde_json::json!` only; no new struct with `Deserialize` for the report.

**`score_bench` reasoning branch** (lines 143-157), where `n` is currently discarded:
```rust
fn score_bench(bench: &str, adapter: &Path) -> Result<f64> {
    let (dir_name, want_think_rate) = match bench.strip_suffix("_init") {
        Some(base) => (base, true),
        None => (bench, false),
    };
    let reasoning = Path::new(BENCH_ROOT).join(dir_name).join("reasoning.jsonl");
    if reasoning.is_file() {
        let report = cached_reasoning_report(dir_name, &reasoning, adapter)?;
        return Ok(if want_think_rate {
            report.think_initiation_rate
        } else {
            report.accuracy
        });
    }
```
Return `n` here (`report.n`) and add defense in depth: `anyhow::bail!` if `report.n != SLICE_SIZE`. `BENCH_ROOT` is the cwd-relative `"mens/data"` (line 10), so `is_file()` is cwd-dependent; gating must classify by key name, not by this branch (RESEARCH Pitfall 1). `ReasoningReport` is `Copy` (derive at `reasoning_eval.rs:21`), which `cached_reasoning_report` relies on (`return Ok(*report)`, line ~124).

**Error handling pattern to copy** (a scorer `Err` propagates and nothing is written; `a_degraded_adapter...` and `an_unloadable_adapter...` tests):
```rust
    #[test]
    fn an_unloadable_adapter_fails_loudly_and_writes_no_passing_report() {
        let dir = tempfile::tempdir().unwrap();
        let pre = dir.path().join("pre.json");
        std::fs::write(&pre, r#"{"benchmarks":{"general_bench":0.85}}"#).unwrap();
        let mut scorer = |_: &str| anyhow::bail!("adapter not found");
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        assert!(
            !dir.path().join("collateral_damage_report.json").exists(),
            "a load failure must never leave a report the serve gate would accept"
        );
    }
```
**Testing pattern** for every new writer test: tempdir, write `pre.json` with `{"benchmarks":{...}}`, drive scores through the closure (no GPU, no model), assert on `run_collateral_damage_with(...)` return/Err and on `dir.path().join("collateral_damage_report.json")`. For the stale-report test, pre-seed `collateral_damage_report.json` with `{"status":"pass",...}` in `dir`, make the closure return `Err`, then assert the file is gone (existing test at lines 234-245 cannot catch this because it starts from a fresh dir). Sorted-order assertion idiom from lines 192-232 (`asked == ["code_bench", "general_bench"]`) applies because the baseline is a `BTreeMap`.

Tests to add (names must hit the filter `eval_collateral`; the filter matches the module path `...eval_collateral::tests::...`): missing partner key; zero reasoning pre; NaN scorer value; wrong `n`; stale report removed; accuracy-only drop writes `status: fail`, `failed_on` names the key, exit code 1; think-only drop likewise; `reasoning_slices.scope_note`/`slice_size`/per-bench `n == 32` on pass and on fail; non-reasoning bench has one score and no `slice` field.

---

### `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (service, file-I/O; minimal edit)

**Analog:** itself (lines 20-31 and 84-94).
```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct ReasoningReport {
    pub(crate) accuracy: f64,
    pub(crate) think_initiation_rate: f64,
    /// Items actually generated. Written into the JSON report; the gate reads
    /// accuracy and think rate off this struct.
    #[allow(dead_code)]
    pub(crate) n: usize,
}
...
    match outcome {
        BenchOutcome::Scored(scored) => Ok(ReasoningReport {
            accuracy: scored.accuracy,
            think_initiation_rate: scored.think_initiation_rate,
            n: scored.n,
        }),
        BenchOutcome::NotScored { reason, .. } => {
            anyhow::bail!("reasoning bench '{bench}' not scored: {reason}")
        }
    }
```
Once the writer reads `report.n`, remove the `#[allow(dead_code)]` (and update the doc comment). No behavior change here: `NotScored` is already an `Err`, so a not-scored slice already aborts the writer. Do not change `write_outcome` (lines 237-276); serve reads exactly its output.

**On-disk shape serve will read** (`crates/vox-eval/src/reasoning_bench.rs`, lines 135-152, written to `<adapter>/collateral_eval/<bench>/reasoning_report.json` by `write_outcome`, lines 240-265):
```rust
BenchOutcome::Scored(s) => serde_json::json!({
    "status": "scored", "n": s.n, "accuracy": s.accuracy,
    "think_initiation_rate": s.think_initiation_rate,
}),
BenchOutcome::NotScored { reason, n_requested } => serde_json::json!({
    "status": "not_scored", "n": 0, "n_requested": n_requested, "reason": reason,
}),
```
`write_outcome` also inserts `"benchmark"`; scored adds `temperature`, `prompt`, `items`. A `not_scored` write replaces a stale scored file (test `a_not_scored_write_replaces_a_stale_scored_report`, lines 456-482), which is why a stale passing collateral report is the gap (Pattern 4).

---

### `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (serve guard, middleware, request-response read-only)

**Analog:** `assert_serve_preconditions` and its test module sit at the bottom of the same file. Both are `#[cfg(feature = "gpu")]` / `#[cfg(all(test, feature = "gpu"))]`.

**Current inline gate to extract** (lines 353-374, inside `PopuliAction::Serve`, arm `#[cfg(feature = "gpu")]` at 312):
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
Replace the body of the `if` with `assert_collateral_gate(&model)?;` followed by the unchanged `assert_serve_preconditions(&model)?;`. Keep the `manifest_path.exists() || adapter_path.exists()` condition as is (the skip-when-neither behavior is pre-existing and out of scope). Keep both existing message strings verbatim as the first two checks and keep the `eval_collateral_damage` prefix on every new message so existing greps match.

**Function-shape analog** (lines 816-850): a free fn at file scope, `-> Result<()>`, takes `&std::path::Path`, uses `anyhow::Context` locally, builds an actionable message, `anyhow::ensure!` for invariants:
```rust
#[cfg(feature = "gpu")]
fn assert_serve_preconditions(run_dir: &std::path::Path) -> Result<()> {
    use anyhow::Context;

    let receipt_path = run_dir.join("gate_receipt.json");
    let receipt: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        &receipt_path,
    )
    .with_context(|| {
        format!(
            "no gate_receipt.json in {} — run `vox mens eval-gate --run-dir {} --policy <p>` first",
            run_dir.display(),
            run_dir.display()
        )
    })?)?;
    anyhow::ensure!(
        receipt.get("overall_passed") == Some(&serde_json::Value::Bool(true)),
        "eval-gate did not pass for this run; refusing to serve"
    );
```
Place `assert_collateral_gate(model: &std::path::Path) -> Result<()>` immediately above `assert_serve_preconditions` (its doc comment starts at 810). It must NOT call `assert_serve_preconditions` (the "valid fixture accepted" test relies on that).

**Re-validation steps** (RESEARCH Pattern 3; parse defensively, no `unwrap`, no panic on missing keys: use `.get(..).and_then(Value::as_f64 / as_u64 / as_str)` as in `mens_run_reports.rs:196-208`):
1. missing report: keep message. 2. `status != "pass"`: keep message. 3. require `reasoning_slices.benches` to contain every name in `SLICE_NAMES` (iterate the constant, never the report's keys, so a slice name from the report cannot influence a path); per slice `n == SLICE_SIZE`, four finite values, `pre > 0`; recompute with the same `vox-eval` function and refuse if the recomputed rate exceeds the stored `threshold`, **regardless of stored `status`/`exceeds_threshold`**. 4. `model.join("collateral_eval").join(slice).join("reasoning_report.json")` must exist with `status == "scored"` and `n == 32`. 5. Messages name the slice and cause; zero-pre message says baseline of 0 gives no relative bar. Path building uses `Path::join` with a fixed filename and `SLICE_NAMES` entries only. Note `serde_json` rejects `NaN` tokens and `1e999` handling is `[ASSUMED]` (RESEARCH Pitfall 3, A3): test both outcomes refuse.

**Testing pattern** (lines 852-889, new module `collateral_gate_tests` next to it):
```rust
#[cfg(all(test, feature = "gpu"))]
mod serve_preconditions_tests {
    use super::*;

    #[test]
    fn a_receipt_whose_every_gate_skipped_is_not_evidence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("gate_receipt.json"),
            r#"{ "overall_passed": true, "gates": [ ... ]}"#,
        )
        .unwrap();
        let err = assert_serve_preconditions(dir.path())
            .unwrap_err()
            .to_string();
        assert!(err.contains("0 substantive gates"), "got: {err}");
    }

    #[test]
    fn a_receipt_with_two_real_gates_is_accepted() {
        ...
        assert!(assert_serve_preconditions(dir.path()).is_ok());
    }
```
Use the identical shape: `#[cfg(all(test, feature = "gpu"))] mod collateral_gate_tests { use super::*; ... }` so the validated filter `cargo test -p vox-ml-cli --features gpu --lib -- collateral_gate` matches. Build a helper that writes a fully valid fixture (report plus three `collateral_eval/<slice>/reasoning_report.json`), then mutate one field per test: missing report; `status: fail`; hand-edited `pass` with a stored 6% accuracy drop; hand-edited `pass` with a 6% think-initiation drop; missing slice in `reasoning_slices`; missing slice report file; `not_scored` slice file; `n: 31`; non-finite/zero pre; plus one accepted case. Assert on `err.to_string()` substrings as above. Use fixtures only; do not call `vox_corpus::...find_workspace_root` or `run_eval_gate` (that pattern at lines 902-965 is for the throughput test and is not needed here).

---

### `crates/vox-gui/src/commands/mens_run_reports.rs` (compat only, no edit)

**Analog:** itself, lines 196-208 and the test at 602-615.
```rust
fn parse_collateral(v: &serde_json::Value) -> CollateralDamageReportDto {
    CollateralDamageReportDto {
        status: v.get("status").and_then(serde_json::Value::as_str).unwrap_or("unknown").to_string(),
        failed_on: v.get("failed_on").and_then(serde_json::Value::as_str).map(str::to_string),
    }
}
```
Reads only `status` and `failed_on`, so additive fields are safe. Existing test `a_failed_collateral_damage_report_names_the_degraded_benchmark` (fixture `{"status":"fail","failed_on":"rust_authoring","threshold":0.1,"reports":[]}`) keeps passing. Per RESEARCH, running `cargo test -p vox-gui --lib collateral` needs the `vox-gui` sidecar and `ui/dist`; skip with a stated reason if absent. No Playwright/GUI surface changes, so the GUI visual invariant does not trigger.

## Shared Patterns

### Fail closed: hard `Err`, never a stored zero
**Source:** `reasoning_bench.rs:100-154` (`NotScored` has no score keys) and `reasoning_eval.rs:97-110` (`fail_closed`).
**Apply to:** the new evaluator, the writer, and `assert_collateral_gate`.
Every refusal (non-finite, zero pre, `n != 32`, missing partner key, missing/`not_scored` slice report) returns `Err`/`bail!` and writes no passing report. There is no numeric stand-in for "not scored".

### One arithmetic implementation
**Source:** `vox-eval/src/lib.rs:295-317`.
**Apply to:** writer and serve recompute. Both call `eval_collateral_damage` after the guards; neither re-implements `degradation / pre`.

### Single source for slice names and size
**Source:** `reasoning_bench.rs:14-18`.
**Apply to:** evaluator, writer, serve. Use `SLICE_SIZE` and `SLICE_NAMES`; no literal `32` or hand-typed name lists outside tests and the human-readable `scope_note`.

### Report projection by hand with `serde_json::json!`
**Source:** `eval_collateral.rs:63-95`.
**Apply to:** writer additions. No `#[derive(Serialize, Deserialize)]` on a new report struct (data-storage policy); the report stays in the adapter dir, no new `.vox/` path, no new `VOX_*` env var.

### Tempdir unit tests, no GPU, no model
**Source:** `eval_collateral.rs:189-289`, `dispatch.rs:852-889`, `reasoning_eval.rs:279-293` (`read_report` helper).
**Apply to:** all new tests. Scores enter through the scorer closure or JSON fixtures. A green test is not a model score. Never load Qwen3-8B.

### Validation run rules
**Source:** `315-VALIDATION.md`, `315-RESEARCH.md` Validation Architecture.
`vox-ml-cli` filters must be `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`; a `0 passed` line is a failed run. Quick: `cargo test -p vox-eval --lib reasoning_slice`. Existing generic suite stays at 10 passed under `collateral_damage`. Route cargo through the broker; do not run `cargo fmt --all` (use `vox run scripts/fmt.vox`). Mutation-check each guard (finite, pre>0, n, think-init compare, serve recompute, slice-report `scored`, stale-report removal) and confirm the edit is present before trusting red/green.

## No Analog Found

None. Closest-to-greenfield pieces, with the partial analogs to use:

| File / piece | Role | Data Flow | Reason |
|--------------|------|-----------|--------|
| `reasoning_slices` JSON block and per-slice verdict struct | model | transform | No existing nested per-slice report; project by hand like `reports_json` (`eval_collateral.rs:64-76`) following the schema in RESEARCH Pattern 2. |
| Stale-report `remove_file` before scoring | utility | file-I/O | The writer has no delete step today; use `std::fs::remove_file`, ignore `NotFound`, mirroring the plain `std::fs` calls at `eval_collateral.rs:78-79`. |
| Serve-side read of per-slice `reasoning_report.json` | middleware | file-I/O | No existing reader of `collateral_eval/<bench>/reasoning_report.json` outside tests (`reasoning_eval.rs:286-293` `read_report` is the nearest shape). |

## Metadata

**Analog search scope:** `crates/vox-eval/src/`, `crates/vox-ml-cli/src/commands/mens/` (writer, scorer, `populi/dispatch.rs`, `mod.rs`), `crates/vox-gui/src/commands/mens_run_reports.rs`, workspace-wide search for `collateral_damage_report` (only the three Rust files above plus `contracts/kb/eval/corpus.jsonl`).
**Files scanned:** 8 read (6 analogs, `Cargo.toml` for `vox-eval`, `mod.rs` gating); tracked-source gate (`git ls-files`) passed for all six analog paths.
**Pattern extraction date:** 2026-10-26
