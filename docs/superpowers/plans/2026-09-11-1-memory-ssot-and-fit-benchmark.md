---
title: "MENS memory SSOT and the codified fit benchmark"
description: "Measure the lanes Vox actually has, replace eleven hand-written ladders with one measured model, and keep the CUDA lane working throughout."
category: "Implementation Plans"
status: "planned"
---

# Plan 1 — Memory SSOT and the Codified Fit Benchmark

> **For agentic workers:** one fresh subagent per task. **Step 0 of every task is to read the Executor Preamble below.** Steps use checkbox (`- [ ]`) syntax.

**Goal:** One measured memory model — `peak = artifact_bytes + a_lane · layers · hidden · tokens` — that sizes training to the memory a machine actually has, on the lanes Vox actually ships, without ever refusing a run the old ladders would have allowed.

**Architecture:** Measure first, model second, delete last. The previous draft of this plan inverted that and would have deleted every working sizing path in the tree while calibrating a lane the workspace does not contain. Here, Task 2 builds a measurement harness, Task 3 **runs** it on `candle-metal`, and only then does Task 4 build a model with a constant that came from this codebase. The CUDA lane is seeded from the constants already in `memory_budget.rs` and is never left unable to plan.

**Tech Stack:** Rust (`vox-populi`, `vox-ml-cli`, `vox-plugin-mens-candle-metal`), `objc2-metal` 0.3.2 (already in `Cargo.lock` via candle-core), `vox-plugin-nvml-probe` (already in-tree).

**Spec:** `docs/superpowers/plans/2026-09-11-mens-mac-lane-hardening.md` (locked invariants L1–L15)

**Prerequisite:** Plan 0 (`2026-09-11-0-serving-defects.md`) owns the `hub.rs` `@<sha>` download break and the `VOX_MENS_FORCE_TRAIN` override. **This plan must not also carry them.**

---

## What changed from the previous draft, and why

An eight-track audit ran this plan's commands rather than reasoning about them. Six findings were fatal; all are fixed here.

1. **Every `cargo test -p vox-populi --lib` in the old draft ran zero tests and exited 0.** `vox-populi` has `default = []` and `pub mod mens` is `#[cfg(feature = "mens")]`. Twelve commands, every "Expected: FAIL to compile" step silently reporting success. Verified: `cargo test -p vox-populi --lib accel_budget` → `0 passed; 25 filtered out`.
2. **The only calibrated lane was `Mlx`, and MLX is not in this workspace** (`rg -li mlx crates/` → one unrelated hit). Meanwhile `candle-metal` cannot even set `gradient_checkpointing: true` — it warns that the flag is CUDA-only and trains unsegmented. So every reachable lane was uncalibrated, and Task 6 then deleted the ladders serving them. **A CUDA machine would have been unable to plan any run at all.**
3. **The rank-clamp fix targeted the wrong size classes.** `detect_qwen_size_class` matches the literal substrings `"32b"`, `"14b"`, `"0.6b"`, `"8b"`; lowercased, `"qwen3.8-27b"` contains none of them, so the 27B falls through to `QwenSizeClass::Other`. The clamps are at **four** sites — `:130`, `:148`, `:174`, `:191` — and the old draft fixed the two the 27B never reaches.
4. **`preset_for` does not exist.** The old draft asserted "this one compiles and fails on the assertion, because `preset_for` already exists." The real symbol is the private `apply_qwen_size_ladder_policy(p, class, vram_mb, model_hint)` — four arguments, `preset_schema.rs:108`.
5. **`safety_headroom_is_applied_exactly_once` was a tautology.** It asserted `plan.usable_bytes == working_set * SAFETY_FRACTION`, a verbatim restatement of `plan_for`'s own line, and passed whether or not the call sites were rewired.
6. **The benchmark had no measuring half.** It built `best_record`, `fit_a_lane` and `sweep_stop_index` — every consumer of a calibration record — and nothing that produces one. `run_probe` only prints an example training command.

---

## Executor Preamble — every task's Step 0 is to read this

**You are a fresh subagent.** You have not read the other tasks and do not need to. Everything your task consumes is stated inside it. If you need a symbol, signature, file, or decision that is not written down in your task, that is **a defect in the task**, not a gap for you to fill by inference: stop and report. Do not guess a signature, do not invent a helper, do not adapt the code to match the plan.

**Checkout.** Work in a branch off `origin/main` `7295b4470`:

```bash
git merge-base --is-ancestor 7295b4470 HEAD && echo OK || echo "WRONG CHECKOUT — stop and report"
```

Every line number below was verified at that commit. If a cited line is off by more than ±5, or a symbol is absent, **stop and report**.

**Feature flags — the single most dangerous trap in this plan.** `vox-populi` has `default = []`. Nothing in `mens/` compiles without a feature. Use exactly what your task specifies:

| code under test | flag |
|---|---|
| `mens/tensor/*` | `--features mens` |
| `mens/hub.rs` | `--features mens-hf-hub` |
| `mens/cloud/*` | `--features mens-cloud` |
| `tests/training_presets_yaml_contract.rs` | `--features mens-train` |
| `vox-ml-cli` serving | `--features execution-api` |
| `vox-ml-cli` GPU paths | `--features gpu` |

**A `0 passed` result is never success.** It means the code was not compiled. Diagnose which flag is missing before touching anything.

**Other house rules that will bite you:**

- `cargo test` takes **exactly one** positional TESTNAME filter. Two is a hard error.
- **Never** `cargo fmt --all`. Use `cargo fmt -p <crate>`.
- Every new `pub fn` needs a same-file `#[test]` or `tdd-guard` blocks the commit.
- No new `.py` / `.sh` / `.ps1` glue. Automation is VoxScript.
- A new CLI subcommand needs BOTH `contracts/operations/catalog.v1.yaml` (via `vox ci operations-sync --target cli --write`) AND `contracts/cli/command-registry.yaml` (via `vox ci command-sync --write`), plus a literal `vox <path>` line in `docs/src/reference/cli.md`. All three run inside `ssot-drift`, which runs in the **fast pre-push tier** — miss one and the first `git push` fails with an unrelated-looking error.
- Do **not** create or edit `docs/src/architecture/research-index.md` — retired 2026-09-06.
- Do **not** `git add -A`. Commit only the paths your task's **Files** block names.

**Timeouts.** Unbounded commands are killed at 120 s. Bound them: `timeout 900s` for scoped tests, `timeout 1800s` for `pre-push --complete`, `timeout 2700s` for release runs. Exit **124** is a timeout, not a test failure. Cargo builds are serialized machine-wide by the build broker.

**You cannot watch remote CI.** `gh pr checks`, `gh run watch` and polling loops are blocked by a hook.

**Verification discipline.**

1. Run the exact command given. Do not substitute.
2. Compare output **literally**. `PASS` is not an expectation; `2 passed; 0 failed` is.
3. **A test that should not compile must not compile.** If a step says "Expected: FAIL to compile — `cannot find X`" and you get a clean run or an assertion failure, **the premise is wrong.** Stop and report. Do not implement against a test you never saw fail.
4. **When reality and the step disagree, reality wins and you stop.** Report the step, the command, the actual output, the promised output, and your reading. Do not rewrite the test to match the code, loosen an assertion, or widen a tolerance.
5. **Verify each guard by mutation.** Break it, confirm red, restore, confirm green, and confirm the file is actually restored.

**Report:** `STATUS: DONE | BLOCKED | PREMISE-FALSE`, files changed, each test's failed-then-passed evidence, every command verbatim with exit status, and any deviation.

---

## The model

```
peak_bytes = artifact_bytes + a_lane · layers · hidden · tokens_per_step
```

- `artifact_bytes` — sum of `*.safetensors` via `fs::metadata`. **Measured, never fitted.** The previous two-parameter fit's intercept (29.071 GB) turned out to be the 27B-8bit weight file (29.501 GB) to within 1.5%.
- `layers`, `hidden` — from `config.json` (`num_hidden_layers`, `hidden_size`; multimodal configs nest these under `text_config`).
- `tokens_per_step` — `batch_size · max_seq_length`.
- `a_lane` — **the only fitted constant, and it must be measured per lane.**

**Do not add a `sqrt(params)` term.** The implied exponent across measured points is 0.67, which `layers · hidden` already encodes. The existing `ACT_GIB_PER_KTOK_PER_SQRTB = 9.5` is wrong in both directions at once — it over-predicts absolute usage ~2.2× and under-predicts the cross-model ratio ~2×.

**Do not add a quantization term.** Quantization enters only through `artifact_bytes`.

### The external reference point, and the prediction it licenses

An MLX + gradient-checkpointing run on an Apple M5 Max measured `a = 75.662` bytes across two Qwen3.8-27B points (+1.0% / +0.8%, and +13.2% over-prediction on a 0.6B). **MLX is not a Vox lane** — it is an external tool, and this number is a reference, not a row in the model.

It licenses a falsifiable prediction that Task 3 must record **before** it measures:

> **`a_candle_metal` will be substantially larger than 75.662** — plausibly 2–3× — because the Rust QLoRA lane retains full BF16 base weights through the backward (~2 GiB per billion parameters, by its own doc comment), and because `candle-metal` does not implement gradient checkpointing at all.

If the measured value comes out near 75.662, one of those two beliefs is wrong and that is a finding worth more than the constant. **Write the prediction into the commit message before running the measurement.** A model that only ever describes data it was fitted to has not been tested.

---

## Task 1: `AccelBudget` — ask the driver, not the nameplate

**Files:**
- Create: `crates/vox-populi/src/mens/tensor/accel_budget.rs`
- Modify: `crates/vox-populi/src/mens/tensor/mod.rs` (add `pub mod accel_budget;` **in Step 1**)
- Modify: `crates/vox-populi/Cargo.toml`

**Interfaces:**
- Produces: `pub struct AccelBudget { pub device_name: String, pub total_bytes: u64, pub working_set_bytes: u64, pub max_alloc_bytes: u64, pub source: BudgetSource }`; `pub enum BudgetSource { Metal, Cuda }`; `pub fn query_accel_budget() -> Option<AccelBudget>`; `impl AccelBudget { pub fn host_key(&self) -> String; pub fn single_alloc_fits(&self, bytes: u64) -> bool }`

### Why three numbers

A measured OOM on this M5 Max needed **122.1 GiB** on a machine with a **107.52 GiB** working set and an **80.64 GiB** max single buffer. One "how much memory" number cannot distinguish "too much in total" from "one allocation too large". Both OOM.

This replaces `vram_autodetect.rs:178`'s `query_apple_available_memory`, which shells to `vm_stat`, sums free+inactive+speculative, and **falls back silently on any parse failure**. Measured against the driver it under-reports ~2× (54.8 vs 107.52 GiB) and drifts with page cache.

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Dependency and module declaration**

`crates/vox-populi/Cargo.toml`:

```toml
[target.'cfg(target_os = "macos")'.dependencies]
objc2-metal = { version = "0.3.2", optional = true }
```

`objc2-metal 0.3.2` is already in `Cargo.lock:9352` via candle-core, so this adds no resolved dependency and does not touch the `crate-edges` ratchet (which governs *workspace* crate→crate edges only). It adds no cmake/nasm/Go/perl/libclang.

**Add `pub mod accel_budget;` to `mens/tensor/mod.rs` in this step.** A test in an undeclared module compiles green with zero tests.

- [ ] **Step 2: Write the failing tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn budget(working: u64, max_alloc: u64) -> AccelBudget {
        AccelBudget {
            device_name: "Apple M5 Max".into(),
            total_bytes: 137_438_953_472,
            working_set_bytes: working,
            max_alloc_bytes: max_alloc,
            source: BudgetSource::Metal,
        }
    }

    #[test]
    fn max_alloc_can_bind_before_the_working_set() {
        let b = budget(115_448_725_504, 86_587_244_544); // measured M5 Max
        assert!(!b.single_alloc_fits(90 * 1024 * 1024 * 1024));
        assert!(b.single_alloc_fits(70 * 1024 * 1024 * 1024));
    }

    #[test]
    fn host_key_carries_exact_bytes_so_it_cannot_drift_by_rounding() {
        let b = budget(115_448_725_504, 86_587_244_544);
        assert!(b.host_key().contains("115448725504"), "key must pin exact bytes: {}", b.host_key());
        assert_ne!(b.host_key(), budget(57_724_362_752, 43_293_622_272).host_key());
    }

    #[test]
    fn query_accel_budget_is_callable_on_every_platform() {
        // tdd-guard requires a same-file test for this pub fn, and a None on a
        // GPU-less host is a valid answer, not a skip.
        let _ = query_accel_budget();
    }
}
```

*Mutations caught:* (1) comparing `single_alloc_fits` against `working_set_bytes` instead of `max_alloc_bytes`; (2) reintroducing GiB rounding into `host_key` — `115_448_725_504 / 2^30 = 107.5200` rounds to **108** while seeded records say **107**, which silently disables every calibration lookup. Exact bytes make the whole class impossible.

- [ ] **Step 3: Run and confirm failure**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib accel_budget
```

Expected: **FAIL to compile** — `cannot find struct 'AccelBudget'`. If you see `0 passed`, the feature flag is missing.

- [ ] **Step 4: Implement**

```rust
//! What the accelerator will actually give us, asked of the driver.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetSource { Metal, Cuda }

#[derive(Debug, Clone)]
pub struct AccelBudget {
    pub device_name: String,
    pub total_bytes: u64,
    /// Metal: `recommendedMaxWorkingSetSize`. CUDA: total VRAM.
    pub working_set_bytes: u64,
    /// Largest single allocation. Metal: `maxBufferLength`. Binds before
    /// `working_set_bytes` on large-batch runs.
    pub max_alloc_bytes: u64,
    pub source: BudgetSource,
}

impl AccelBudget {
    /// Stable identity for calibration lookups. Exact bytes, never rounded.
    pub fn host_key(&self) -> String {
        let compact: String = self.device_name.chars().filter(char::is_ascii_alphanumeric).collect();
        format!("{compact}-{}", self.working_set_bytes)
    }

    pub fn single_alloc_fits(&self, bytes: u64) -> bool {
        bytes <= self.max_alloc_bytes
    }
}

#[cfg(target_os = "macos")]
pub fn query_accel_budget() -> Option<AccelBudget> {
    use objc2_metal::{MTLCreateSystemDefaultDevice, MTLDevice};
    let device = MTLCreateSystemDefaultDevice()?;
    let working = device.recommendedMaxWorkingSetSize();
    Some(AccelBudget {
        device_name: device.name().to_string(),
        total_bytes: working,
        working_set_bytes: working,
        // NSUInteger is usize; the struct field is u64.
        max_alloc_bytes: device.maxBufferLength() as u64,
        source: BudgetSource::Metal,
    })
}
```

**On accessor names:** if `recommendedMaxWorkingSetSize()` or `maxBufferLength()` do not resolve, find the real spelling — do **not** guess and do **not** fall back to a constant:

```bash
rg -n 'recommendedMaxWorkingSet|maxBufferLength|fn name' ~/.cargo/registry/src/*/objc2-metal-0.3.2/src/generated/MTLDevice.rs | head
```

An unavailable accessor means `query_accel_budget` returns `None` and callers fail closed.

- [ ] **Step 5: CUDA — wrap the plugin that already exists**

Do **not** add a second NVML integration. `crates/vox-plugin-nvml-probe/src/probe.rs` already reports `vram_total_mb` and `vram_free_mb` per device (`probe_summary()`, `:53`). Add a `#[cfg(not(target_os = "macos"))]` `query_accel_budget` that routes through it, with `working_set_bytes` from total VRAM and `max_alloc_bytes` equal to it (CUDA has no separate single-buffer cap), `source: BudgetSource::Cuda`.

If the plugin is not loadable at runtime, return `None`. **`None` is honest; a fabricated number is not.**

- [ ] **Step 6: Run and confirm 3 passed**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib accel_budget
```

- [ ] **Step 7: Sanity-check the live device**

```rust
#[test]
#[ignore = "reads the live device; run manually on a Metal host"]
fn live_device_reports_a_plausible_budget() {
    let b = query_accel_budget().expect("a device on this host");
    assert!(b.working_set_bytes > 0 && b.max_alloc_bytes > 0);
    assert!(b.max_alloc_bytes <= b.working_set_bytes);
    eprintln!("{} working_set={} max_alloc={}", b.device_name, b.working_set_bytes, b.max_alloc_bytes);
}
```

```bash
timeout 900s cargo test -p vox-populi --features mens --lib live_device_reports_a_plausible_budget -- --ignored --nocapture
```

On this M5 Max expect `working_set=115448725504` and `max_alloc=86587244544`. **If they differ, stop and record the real numbers** — every calibration record is keyed to them.

- [ ] **Step 8: Commit**

---

## Task 2: The measurement harness — the half the benchmark was missing

Without this task there is nothing that produces a calibration record, and the model has no constant that came from this codebase.

**Files:**
- Create: `crates/vox-populi/src/mens/tensor/calibration.rs`
- Modify: `crates/vox-populi/src/mens/tensor/mod.rs`
- Modify: `crates/vox-plugin-mens-candle-metal/src/candle_qlora_train/mod.rs` (peak sampling)

**Interfaces:**
- Consumes: `AccelBudget`, `BudgetSource` (Task 1)
- Produces: `pub struct CalibrationRecord { pub host_key: String, pub lane: String, pub model: String, pub artifact_bytes: u64, pub layers: u32, pub hidden: u32, pub tokens_per_step: u64, pub outcome: FitOutcome, pub peak_bytes: Option<u64> }`; `pub enum FitOutcome { Fits, Oom }`; `pub fn best_record<'a>(records: &'a [CalibrationRecord], host_key: &str, lane: &str) -> Option<&'a CalibrationRecord>`; `pub fn fit_a_lane(records: &[CalibrationRecord]) -> Option<f64>`; `pub struct PeakSampler` with `pub fn observed_peak_bytes(&self) -> u64`

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn best_record_ignores_oom_rows_even_when_they_peaked_higher() {
    // A near-OOM run really does peak highest just before it dies, so the OOM
    // fixture carries the LARGEST peak. A max_by_key that forgets the outcome
    // filter will pick it.
    let rows = vec![
        rec("AppleM5Max-115448725504", "candle-metal", FitOutcome::Fits, 512, Some(41_765_000_000)),
        rec("AppleM5Max-115448725504", "candle-metal", FitOutcome::Oom, 4096, Some(119_000_000_000)),
    ];
    let best = best_record(&rows, "AppleM5Max-115448725504", "candle-metal").unwrap();
    assert_eq!(best.tokens_per_step, 512, "an OOM row must never be selected as best");
}

#[test]
fn a_record_from_another_host_is_not_reused() {
    let rows = vec![rec("OtherMac-57724362752", "candle-metal", FitOutcome::Fits, 512, Some(1_000))];
    assert!(best_record(&rows, "AppleM5Max-115448725504", "candle-metal").is_none());
}

#[test]
fn a_record_from_another_lane_is_not_reused() {
    let rows = vec![rec("AppleM5Max-115448725504", "mlx", FitOutcome::Fits, 512, Some(41_765_000_000))];
    assert!(best_record(&rows, "AppleM5Max-115448725504", "candle-metal").is_none(),
        "an MLX measurement must not stand in for a candle-metal one");
}

#[test]
fn fitting_uses_the_difference_quotient_between_two_token_counts() {
    // Two points at the same shape: a = (peak2 - peak1) / ((T2 - T1) * L * H).
    // NOT a least-squares slope forced through the origin, which would give
    // 74.126 for these points and silently disagree with the model.
    let rows = vec![
        rec27("AppleM5Max-115448725504", "mlx", FitOutcome::Fits, 512, Some(41_765_000_000)),
        rec27("AppleM5Max-115448725504", "mlx", FitOutcome::Fits, 1024, Some(54_459_000_000)),
    ];
    let a = fit_a_lane(&rows).expect("two distinct token counts are enough");
    assert!((a - 75.662).abs() < 0.01, "fitted a = {a}, expected 75.662");
}

#[test]
fn fitting_refuses_a_single_point() {
    let rows = vec![rec27("AppleM5Max-115448725504", "mlx", FitOutcome::Fits, 512, Some(41_765_000_000))];
    assert!(fit_a_lane(&rows).is_none(),
        "one point cannot determine a slope; fitting it invents a degree of freedom");
}
```

*Mutations caught:* (1) dropping the `outcome == Fits` filter — the fixture is built so `max_by_key` picks the OOM row without it; (2) ignoring `host_key`; (3) **ignoring `lane`, which is how an external MLX number would leak into a Vox lane's model**; (4) using a through-origin least-squares fit instead of the difference quotient — the two disagree by 2% on these exact points, and the old draft's spec and test disagreed on which was meant; (5) fitting a slope from one point, which is precisely the statistical error that produced the discredited earlier model.

- [ ] **Step 2: Run and confirm failure**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib calibration
```

Expected: **FAIL to compile** — `cannot find function 'best_record'`.

- [ ] **Step 3: Implement the record store and the fit**

`best_record` filters on `host_key` equality **and** `lane` equality **and** `outcome == Fits`, then `max_by_key(peak_bytes)`. `fit_a_lane` requires at least two records with distinct `tokens_per_step` at the same `(model, layers, hidden)`, and returns `(p2 - p1) as f64 / (((t2 - t1) * layers as u64 * hidden as u64) as f64)`. Subtract nothing — `artifact_bytes` cancels in the difference, which is exactly why the difference quotient is the right estimator and why the intercept is never fitted.

- [ ] **Step 4: Add peak sampling to the Metal trainer**

`PeakSampler` wraps `MTLDevice.currentAllocatedSize`, sampled on a background thread at a fixed interval, retaining the maximum. Start it before the first step and read it after the last.

This is the piece that makes the benchmark reusable: it observes a **real `candle-metal` training run**, in Rust, with no external tooling. A Python probe is a house-rule violation and must not be reintroduced.

- [ ] **Step 5: Run and confirm 5 passed**

- [ ] **Step 6: Commit**

---

## Task 3: Measure `candle-metal` — and record the prediction first

**This task produces a number, not code.** It is the reason Task 4 can exist.

**Files:**
- Create: `docs/src/architecture/measurements/2026-09-11-candle-metal-calibration.json`

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Write down the prediction before measuring**

Record in the commit message, before running anything:

> Predicted `a_candle_metal` ≈ **150–230** bytes per (layer × hidden × token) — 2–3× the MLX reference of 75.662 — because this lane retains full BF16 base weights through the backward (~2 GiB/B params by its own doc comment) and does not implement gradient checkpointing at all (`candle_qlora_train/mod.rs:311-318` warns that the flag is CUDA-only and the forward runs unsegmented).

A prediction written after the fact is not a prediction.

- [ ] **Step 2: Measure — smallest models first**

Run each through the real `candle-metal` trainer with `PeakSampler` active. **Use the smallest models that resolve the slope; the 27B is the slowest point and adds the least information.**

| # | model | batch × seq | what it settles |
|---|---|---|---|
| 1 | Qwen3-0.6B | 1 × 512 | anchor |
| 2 | Qwen3-0.6B | 1 × 1024 | slope in tokens |
| 3 | Qwen3-0.6B | 4 × 512 | **the load-bearing one** — same tokens as #2, different shape |
| 4 | Qwen3-8B | 1 × 512 | cross-model anchor |
| 5 | Qwen3-8B | 1 × 1024 | cross-model slope |

**If #2 and #3 differ by more than 10%, `tokens_per_step` is the wrong axis** and the whole model form needs revisiting — attention working memory can grow as `B·S²` while checkpoint storage grows as `B·S`. Stop and report; do not average them. The earlier MLX reference never tested this, because both of its points held `seq = 512` and varied only batch — it was a batch-scaling law wearing a token-scaling label.

- [ ] **Step 3: Fit, and compare against the prediction**

Run `fit_a_lane` over #1/#2 and again over #4/#5. **If the two disagree by more than 15%, `layers · hidden` does not capture the cross-model scaling** — report rather than averaging.

Record the outcome of Step 1's prediction explicitly, including if it was wrong. A measured `a` near 75.662 would mean the BF16-retention belief is wrong, which is a more valuable finding than the constant.

- [ ] **Step 4: Write the record with `peak_bytes` canonical**

Store `peak_bytes` as an integer. Derived units are presentation only. **A mislabelled GiB has already cost this project a 7.4% systematic error in committed data** — mlx-lm's `Peak mem` is decimal GB (`mx.get_peak_memory() / 1e9`), which an earlier record stored under a `peak_gib` key.

- [ ] **Step 5: Commit** — with the prediction, the measurement, and the verdict on the prediction.

---

## Task 4: `memory_model` — one measured constant per lane, fail closed

**Depends on Task 3's measured constant. Do not start it earlier.**

**Files:**
- Create: `crates/vox-populi/src/mens/tensor/memory_model.rs`
- Create: `contracts/mens/memory-model.v1.yaml`
- Modify: `crates/vox-populi/src/mens/tensor/mod.rs`, `contracts/index.yaml`

**Interfaces:**
- Produces: `pub enum Lane { CandleCuda, CandleMetal }`; `pub struct CalKey`; `pub struct MemoryModel { pub act_bytes_per_lht: f64, pub source: CalSource }`; `pub enum CalSource { Measured, Seeded }`; `pub struct ModelShape { pub artifact_bytes: u64, pub layers: u32, pub hidden: u32 }`; `pub struct MemoryModels` with `load_from_str`, `load_default`, `get`; `predict_bytes`; `ModelShape::from_model_dir`

### Two lanes, both real

`Lane` has **two** variants, because Vox has two training backends. **MLX is not a lane** — it is an external reference in `measurements/`, and `best_record` refuses to serve its rows to a Vox lane.

`CalKey::new(lane, gradient_checkpointing)` **returns an error for `(CandleMetal, true)`**, because `candle-metal` does not implement checkpointing. Making the impossible cell unconstructible is better than returning `Uncalibrated` at runtime — the tree already ships a `train_log::warn` apologising that a budget was planned assuming a feature the backend lacks, and that warning is the canonical case of a runtime check where the type system would do.

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Write the failing tests**

```rust
const SEED_YAML: &str = r#"
schema: vox.mens.memory-model.v1
lanes:
  - lane: candle-metal
    gradient_checkpointing: false
    act_bytes_per_lht: <FROM TASK 3>
    source: measured
  - lane: candle-cuda
    gradient_checkpointing: true
    act_bytes_per_lht: <DERIVED FROM RESIDENT_GIB_PER_B_PARAMS — see Step 3>
    source: seeded
"#;

#[test]
fn the_weights_term_is_read_not_fitted() {
    let m = MemoryModels::load_from_str(SEED_YAML).unwrap()
        .get(&CalKey::new(Lane::CandleMetal, false).unwrap()).unwrap().clone();
    let a = ModelShape { artifact_bytes: 10_000_000_000, layers: 64, hidden: 5120 };
    let b = ModelShape { artifact_bytes: 20_000_000_000, layers: 64, hidden: 5120 };
    assert_eq!(m.predict_bytes(&b, 512) - m.predict_bytes(&a, 512), 10_000_000_000,
        "artifact_bytes must pass through one-for-one");
}

#[test]
fn activations_are_linear_in_tokens() {
    let models = MemoryModels::load_from_str(SEED_YAML).unwrap();
    let m = models.get(&CalKey::new(Lane::CandleMetal, false).unwrap()).unwrap();
    let s = ModelShape { artifact_bytes: 29_501_218_479, layers: 64, hidden: 5120 };
    let a1 = m.predict_bytes(&s, 512) - s.artifact_bytes;
    let a2 = m.predict_bytes(&s, 1024) - s.artifact_bytes;
    assert!((a2 as f64 / a1 as f64 - 2.0).abs() < 0.01,
        "doubling tokens must double activations; a sqrt or table breaks this");
}

#[test]
fn metal_cannot_claim_gradient_checkpointing() {
    assert!(CalKey::new(Lane::CandleMetal, true).is_err(),
        "candle-metal does not implement checkpointing; the key must be unconstructible, \
         not merely uncalibrated at runtime");
    assert!(CalKey::new(Lane::CandleCuda, true).is_ok());
}

#[test]
fn an_uncalibrated_lane_is_an_error_not_a_borrowed_constant() {
    let yaml = "schema: vox.mens.memory-model.v1\nlanes: []\n";
    let err = MemoryModels::load_from_str(yaml).unwrap()
        .get(&CalKey::new(Lane::CandleMetal, false).unwrap()).expect_err("no rows");
    let msg = err.to_string();
    assert!(msg.contains("candle-metal"), "the error must name the lane: {msg}");
    assert!(msg.contains("probe --measure"), "the error must name the fix: {msg}");
}

#[test]
fn model_shape_reads_a_multimodal_config() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("config.json"),
        r#"{"text_config":{"num_hidden_layers":64,"hidden_size":5120}}"#).unwrap();
    std::fs::write(dir.path().join("model.safetensors"), vec![0u8; 1024]).unwrap();
    let s = ModelShape::from_model_dir(dir.path()).unwrap();
    assert_eq!((s.layers, s.hidden, s.artifact_bytes), (64, 5120, 1024));
}
```

*Mutations caught, one per test:* fitting the intercept; replacing the linear token term with anything else; keying on `lane` alone so an impossible regime becomes constructible; falling back to a default row for a missing lane; reading `num_hidden_layers` only at the top level, which is `null` on every multimodal Qwen config.

- [ ] **Step 2: Run and confirm failure.** Expected: `cannot find struct 'MemoryModels'`.

- [ ] **Step 3: Seed the CUDA row from the constants already in the tree**

**This is what keeps the 4080 Super lane working.** `memory_budget.rs` holds `RESIDENT_GIB_PER_B_PARAMS = 3.5` (`:31`), `FIXED_OVERHEAD_GIB = 1.6` (`:35`, documented "CUDA context, cuBLAS workspaces, allocator slack") and `ACT_GIB_PER_KTOK_PER_SQRTB = 9.5` (`:46`). Derive an `act_bytes_per_lht` that reproduces the old model's predictions on the rungs that lane serves today, and mark it `source: seeded`.

**The seeded row must not silently claim to be measured.** `CalSource::Seeded` exists so `vox mens probe` can say "this lane's constant is inherited from the pre-measurement estimator; measure it with `--measure`."

Do **not** guess. Reproduce the old arithmetic on at least two existing rungs and assert the seeded row lands within 10% of what `plan_with_resident` returns today. If it cannot, **stop and report** — a CUDA row that changes the 4080's answers is a regression dressed as an SSOT.

- [ ] **Step 4: Write the contract + `contracts/index.yaml` row**

Entries in `contracts/index.yaml` are 6–8-line blocks (`id/path/owner/kind/description/enforced_by`), not one-liners. It is append-only, so the append merges safely.

- [ ] **Step 5: Run and confirm 5 passed.** Commit.

---

## Task 5: `plan_for` — one entry point, headroom once, refusal over retreat

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/memory_model.rs`
- Modify: `crates/vox-populi/src/mens/tensor/preset_schema.rs` (**four** clamp sites)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs`

### The two defects

**The safety fraction is applied twice** — `train_arm.rs` and `preset_schema.rs` each scale the budget, so effective headroom is the product of two fractions.

**The rank clamps fire before any memory check, at four sites.** `preset_schema.rs:130`, `:148`, `:174`, `:191` all run `p.rank = p.rank.min(8); p.alpha = p.alpha.min(16.0);` **above** the `vram_mb <=` tests that were meant to gate them.

**And the 27B reaches the sites an earlier draft did not fix.** `detect_qwen_size_class` (`:86-106`) matches the literal substrings `"32b"`, `"14b"`, `"0.6b"`, `"8b"`. Lowercased, `"qwen3.8-27b"` contains **none** of them — it falls through to `QwenSizeClass::Other`, whose clamps are at `:174` and `:191`. Fixing only S14 (`:130`) and S32 (`:148`) leaves the headline model on the unfixed path.

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Read before editing**

```bash
sed -n '86,106p;120,200p' crates/vox-populi/src/mens/tensor/preset_schema.rs
```

Confirm: `apply_qwen_size_ladder_policy` at `:108` takes **four** arguments — `(mut p: TrainPresetProfile, class: QwenSizeClass, vram_mb: u64, model_hint: Option<&str>)` — and both it and `QwenSizeClass` are **private**. Record the exact `TrainPresetProfile` construction the sibling tests use; you will reuse it verbatim.

- [ ] **Step 2: Write the failing tests**

The clamp test **must live in `preset_schema.rs`'s own `#[cfg(test)] mod tests`** — both symbols are private and unreachable from `memory_model.rs`.

```rust
// in preset_schema.rs
#[test]
fn a_large_machine_does_not_get_the_small_card_clamp_in_any_size_class() {
    let base = /* the TrainPresetProfile fixture the sibling tests build */;
    for (class, hint) in [
        (QwenSizeClass::S14,   Some("Qwen3-14B")),
        (QwenSizeClass::S32,   Some("Qwen3-32B")),
        (QwenSizeClass::Other, Some("Qwen3.8-27B")), // the model this program is about
    ] {
        let big = apply_qwen_size_ladder_policy(base.clone(), class, 110_000, hint);
        assert!(big.rank > 8, "{class:?}: a 110 GB budget must not get the 16 GB card's rank, got {}", big.rank);
    }
    let small = apply_qwen_size_ladder_policy(base, QwenSizeClass::S32, 16_384, Some("Qwen3-32B"));
    assert_eq!(small.rank, 8, "a 16 GB card still gets the tight envelope");
}

#[test]
fn the_27b_is_classified_other_not_s32() {
    // If this ever changes, the clamp fix above must move with it.
    assert!(matches!(detect_qwen_size_class(Some("Qwen3.8-27B")), Some(QwenSizeClass::Other)));
}
```

```rust
// in memory_model.rs
#[test]
fn an_over_large_request_is_refused_with_a_reason_not_shrunk_to_a_smaller_model() {
    let plan = plan_for(&m5max(), &models(), &metal_key(), &shape_27b(),
                        &Request { batch_size: 4, seq_len: 1024 });
    assert!(!matches!(plan.verdict, Verdict::Fits));
    assert_eq!(plan.tokens_per_step, 4096, "plan_for must report what was ASKED, not a substitute");
}

#[test]
fn safety_headroom_reaches_the_call_sites_exactly_once() {
    // NOT `plan.usable_bytes == working_set * SAFETY_FRACTION` — that restates
    // plan_for's own line and passes whether or not the call sites were rewired.
    // Assert through budget_gate, which is where the second application lived.
    let b = m5max();
    let once = (b.working_set_bytes as f64 * SAFETY_FRACTION).round() as u64;
    assert_eq!(budget_gate_usable_bytes(&b), once,
        "budget_gate must consume plan_for's usable_bytes, not scale it again");
}
```

*Mutations caught:* leaving any of the four clamps above its memory conditional; a reclassification of the 27B silently moving it to an unfixed arm; reintroducing ladder retreat; **and the actual double-application, asserted through the call site rather than restated from the definition.**

- [ ] **Step 3: Run and confirm failure**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib a_large_machine_does_not_get_the_small_card_clamp_in_any_size_class
```

Expected: **FAIL on the assertion** — `Other: a 110 GB budget must not get the 16 GB card's rank, got 8`. This one compiles, because `apply_qwen_size_ladder_policy` exists. A `cannot find function` means you put the test in the wrong file.

- [ ] **Step 4: Implement `plan_for`**

```rust
/// The one place headroom is taken. Peaks land above the running average, and
/// on a unified-memory machine the window server competes for the same pool.
pub const SAFETY_FRACTION: f64 = 0.88;
```

`plan_for` **reports**; it does not choose. Picking a shape that fits is `sweep` (Task 6). Keeping them separate is what stops a refusal from becoming a silent substitution.

- [ ] **Step 5: Move all four clamp pairs inside their memory conditionals.**

- [ ] **Step 6: Rewire both planners.** Neither may scale the budget itself.

- [ ] **Step 7: Run the suites**

```bash
timeout 1800s cargo test -p vox-populi --features mens --lib
timeout 1800s cargo test -p vox-ml-cli --features gpu --lib
```

Existing tests asserting ladder retreat will fail. **Read each before updating it** — a test asserting that a 27B request becomes a 9B is asserting the bug.

- [ ] **Step 8: Commit**

---

## Task 6: `probe --measure` / `--sweep` and auto-as-default

**Files:**
- Modify: `crates/vox-ml-cli/src/commands/mens/probe.rs` — **note the path: `mens/probe.rs`, not `mens/populi/probe.rs`, which does not exist**
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/action_populi_enum.rs` (`Probe`, `:357`), `dispatch.rs` (`:408` destructures `Probe { detailed }` — an undeclared third claimant in the old draft)
- Modify: `contracts/operations/catalog.v1.yaml`, `contracts/cli/command-registry.yaml`, `docs/src/reference/cli.md`

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn sweep_picks_the_largest_shape_that_fits_not_the_first() {
    let picked = sweep(&m5max(), &models(), &metal_key(), &shape_27b(), 512)
        .expect("something fits");
    // Compute the expectation from the model, do not hardcode an index:
    // every larger candidate must be refused and this one must fit.
    assert!(matches!(plan_for(&m5max(), &models(), &metal_key(), &shape_27b(), &picked).verdict,
                     Verdict::Fits));
    let bigger = Request { batch_size: picked.batch_size * 2, seq_len: picked.seq_len };
    assert!(!matches!(plan_for(&m5max(), &models(), &metal_key(), &shape_27b(), &bigger).verdict,
                      Verdict::Fits), "sweep stopped early: {bigger:?} also fits");
}
```

*Mutation caught:* a sweep returning the first or smallest fitting shape. **Note the expectation is derived from the model rather than hardcoded** — an earlier draft asserted a stop index of 1 when the model actually predicts 80.28 GB at 2048 tokens, which fits both caps, making the correct answer 2. A hardcoded index makes the test fail against a correct implementation.

- [ ] **Step 2: Run and confirm failure.**

- [ ] **Step 3: Implement `sweep`** as `take_while` over `plan_for`. One implementation, reused by auto mode — do not write a second loop.

- [ ] **Step 4: Auto is the default path**

`vox mens train --model <name>` with no sizing flags runs `sweep` and prints what it chose and why. Explicit `--batch-size`/`--seq-len` are **gated, never adjusted**.

**Delete the `retreated_from_b` branch at `train_arm.rs:539-558`** — a model the user named is never silently substituted.

- [ ] **Step 5: Register the CLI surface — all three, not one**

```bash
timeout 1800s cargo run -p vox-cli -- ci operations-sync --target cli --write
timeout 1800s cargo run -p vox-cli -- ci command-sync --write
```

Then add the literal `vox mens probe` line to `docs/src/reference/cli.md` (the list at `:673`) — `check_ref_cli` (`validators.rs:604-611`) requires it. Confirm:

```bash
timeout 1800s cargo run -p vox-cli -- ci ssot-drift
```

- [ ] **Step 6: Commit**

---

## Task 7: Pin the catalogue against the measured model

**Files:**
- Modify: `crates/vox-populi/tests/training_presets_yaml_contract.rs`

The file is `#![cfg(feature = "mens-train")]` — **without that flag the whole target is empty and passes.**

`workspace_root()` in `spoke_base_resolver.rs` is `#[cfg(test)]`-private and unreachable here; the existing file computes the root inline. Follow that. `train_bases_rungs()` and `known_shape()` **do not exist** — you must write them, or inline a literal `&[(repo_id, ModelShape)]` table in the test.

- [ ] **Step 0–2:** Preamble; write the test; run with `--features mens-train`.

- [ ] **Step 3: Read failures before changing anything**

**The catalogue is not automatically wrong.** The 27B's `floor_mb: 34000` is within 1.26× of a measured MLX peak — the catalogue was right and the Rust trainer was the outlier. Do **not** mass-edit `floor_mb` to make the test green. An earlier proposal would have moved Qwen3-8B from 12,000 to ~35,700 MB (a 16 GB Mac then trains nothing) and Qwen3-32B from 60,000 to ~130,000 MB (a 128 GB Mac loses the 32B); seven of eight rungs failed its own drift guard, and it was withdrawn.

Also note `gpu-specs.yaml` deliberately leaves three `Qwen2.5-Coder-*` rungs **unpinned** (`:265`, `:268`, `:300`), which the file's own comment at `:248-254` explains. That is not drift.

- [ ] **Step 4: Commit**

---

## Task 8: Delete the ladders — nine sites, not five

**Single owner. Not a fan-out task.** Run verification after **each** file.

**Files:**
- `memory_budget.rs` — `QWEN3_LADDER`, `QWEN35_LADDER`, `QWEN25CODER_LADDER`, `SEQ_LADDER`, `RESIDENT_GIB_PER_B_PARAMS`, `FIXED_OVERHEAD_GIB`, `ACT_GIB_PER_KTOK_PER_SQRTB`, `QWEN35_GC_OFFSET_SLOPE_PER_B`, `get_resident_per_b`
- `vram_autodetect.rs` — `query_apple_available_memory` + `parse_vm_stat`; **`usable_unified_memory_gb` (`:54`) and `DEFAULT_UNIFIED_MEM_RESERVE_GIB = 12.0` (`:49`)**; **`auto_preset` (`:270`) and `auto_preset_for` (`:321`) with the `METAL_*_GIB` constants (`:311-315`)**
- `preset_schema.rs` — **`TrainingPreset::best_for_vram` (`:666`)**, live at `:438`
- `mens/config/gpu-specs.yaml` — the `presets.*.max_vram_mb` ladder (`:186-217`), the YAML half of `best_for_vram`
- `cloud/resolver.rs` — `preset_for_vram` (`:58-69`), `min_vram_mb: 24000` (`:285`), **and re-source `ResolvedOffer.effective_preset` (`:250`), which is constructed only from `preset_for_vram`**
- `cloud/estimator.rs` — `preset_for_vram` (`:190`) and `vram_mb_for` (`:185`), both with **zero callers**
- `train_arm.rs` — `resolve_training_sizing` (`:811`) and its tests (`:1009-1063`)
- **`cohort/planner.rs:230`** — `memory_budget::plan(node.vram_gib, target_params_b).over_budget`, a caller the old draft never listed

**`METAL_*_GIB` is invariant L15 and the old draft dropped it.** The constants cover 8B, 14B-QLoRA, 14B-LoRA and 32B — **there is no 27B rung**, so `auto_preset_for` hands the 27B the 14B preset, with both tests green and a comment claiming alignment.

- [ ] **Step 1: Enumerate callers first**

```bash
timeout 1800s cargo run -q -p vox-cli -- graph refresh --auto
cargo run -q -p vox-cli -- graph query "usable_unified_memory_gb auto_preset_for best_for_vram preset_for_vram"
```

`vox graph` answers reachability where grep answers mentions. Write the caller list into the commit message.

- [ ] **Step 2: Delete one surface, verify, repeat.** After **each** file:

```bash
cargo check -p vox-populi --features mens
cargo check -p vox-populi --features mens-cloud   # cloud/ is behind this, NOT mens
cargo check -p vox-ml-cli --features gpu
cargo check -p vox-ml-cli --features cloud
```

**The `mens-cloud` / `cloud` checks are not optional.** `preset_for_vram` lives in `cloud/`, which `mens` does not enable — an earlier draft's verification could not have caught the break it was about to cause.

- [ ] **Step 3: Full suite, then `timeout 1800s cargo run -p vox-cli -- ci pre-push --complete`.** The fast tier omits clippy and tests.

- [ ] **Step 4: Commit**

---

## Task 9: Surface the plan where the model is chosen

The demand was that this work reach the GUI, not only the CLI, and that picking a model be enough. **It is cheaper than it looks, because the GUI already runs the CLI** — one Rust renderer serves both surfaces and there is nothing to keep in sync.

**Files:**
- Modify: `crates/vox-populi/src/mens/tensor/memory_model.rs` (add `render_verdict`)
- Modify: `crates/vox-ml-cli/src/commands/mens/probe.rs`
- Modify: `crates/vox-gui/ui/src/components/surfaces/CommandCardsView.tsx` (`:50`)
- Modify: `crates/vox-gui/ui/src/components/surfaces/decoratorRegistry.ts` (`:42-46`)

**Interfaces:**
- Consumes: `TrainPlan`, `Verdict`, `AccelBudget` (Tasks 1, 5)
- Produces: `pub fn render_verdict(plan: &TrainPlan, budget: &AccelBudget, shape: &ModelShape) -> String`

### The defect, and why it is a one-line diff

`decoratorRegistry.ts:45` describes the GPU Probe card as **"Detected accelerators + LoRA fit"**. `CommandCardsView.tsx:50` invokes `execute_command` with `args: { __argv: [] }` — hardcoded empty. `probe.rs` gates the entire fit block on `verbose`. **So the card promises a fit and renders a VRAM number.**

`crates/vox-gui/src/commands/execute.rs:22` already forwards `__argv` to the CLI, and `vox-gui` already ships the `vox` binary as a Tauri `externalBin` sidecar (`tauri.conf.json:52`), the same way `daemon.rs` and `scientia.rs` reach it. **No new Tauri command, no new crate edge, no `crate-edges` exception.** The renderer must live in `vox-populi` so nothing crosses a crate boundary that does not already exist.

- [ ] **Step 0:** Read the Executor Preamble.

- [ ] **Step 1: Write the failing tests**

```rust
#[test]
fn a_fitting_plan_states_what_was_chosen_and_that_it_was_automatic() {
    let s = render_verdict(&fits_plan_auto(), &m5max(), &shape_27b());
    assert!(s.contains("batch 1"), "name the shape it picked: {s}");
    assert!(s.contains("512"), "name the seq len: {s}");
    assert!(s.contains("[auto]"), "say the user did not choose this: {s}");
    assert!(s.contains("GiB"), "state predicted AND budget on the happy path too: {s}");
}

#[test]
fn a_refusal_states_the_cap_it_broke_and_the_knob_that_fixes_it() {
    // The measured b4x1024 OOM: 122.1 GiB needed against an 80.6 GiB buffer cap.
    let s = render_verdict(&single_alloc_too_large_plan(), &m5max(), &shape_27b());
    assert!(s.contains("122.1"), "state what it needs: {s}");
    assert!(s.contains("80.6"), "state the cap it broke: {s}");
    assert!(s.contains("batch"), "name the knob that fixes it: {s}");
}
```

*Mutation caught:* a renderer that speaks only on failure. **The first test is the important one** — a `render_verdict` returning `""` for `Verdict::Fits` passes any refusal-only test, and that build is one where picking a model tells the user nothing. That is the exact failure mode of this demand.

- [ ] **Step 2: Run and confirm failure**

```bash
timeout 900s cargo test -p vox-populi --features mens --lib render_verdict
```

Expected: **FAIL to compile** — `cannot find function 'render_verdict'`.

- [ ] **Step 3: Implement one renderer**, printing on every path: model and artifact size, device and both caps, usable budget with the safety fraction named, the chosen shape with `[auto]` or `[you asked for this]`, predicted peak, headroom, and the verdict. On refusal, name the cap that bound and the largest shape that would fit.

- [ ] **Step 4: Call it from `probe.rs`**, replacing the `verbose`-gated `recommend_config` block. Add `model: Option<String>` to the `Probe` variant so `vox mens probe --model X` is the dry run of `vox mens train --model X` with the same renderer. **Register the flag** per the Executor Preamble's three-step CLI rule.

- [ ] **Step 5: Thread `argv` through the GUI card**

Add `argv?: string[]` to `SurfaceCard`, pass `card.argv ?? []` into the existing `__argv`, and set the probe card to `argv: ['--detailed']`. That one line makes the card show the fit it already advertises.

- [ ] **Step 6: Test the seam, not the pixels**

Beside the existing `crates/vox-gui/ui/src/components/surfaces/Models/ModelsView.test.tsx`:

```ts
it('asks the CLI for the fit of the model the user actually picked', async () => {
  mockInvoke('get_active_model', 'Qwen/Qwen3-8B');   // models.rs:258, already exists
  render(<MensTrainingView pushToast={noop} />);
  await waitFor(() => expect(invoke).toHaveBeenCalledWith('execute_command', {
    path: ['mens', 'probe'],
    args: { __argv: ['--detailed', '--model', 'Qwen/Qwen3-8B'] },
  }));
});
```

*Mutation caught:* dropping `--model` (the card probes the machine, not the model — today's behaviour) or `--detailed` (the fit block stays gated off). **Both mutations are what ships today, so this test fails on `main` before the change**, which is the point.

- [ ] **Step 7: Verify with a gate that can actually fail**

**Do not use `vox graph coverage --kind mens`.** It returns `{"entries": []}` and exits 0, because `mens` is not a node kind — `compute_coverage` (`crates/vox-graph-reader/src/coverage.rs:107`) filters `node["kind"] == kind`, and the real kinds are `fn`, `command`, `cli-command`, `cli-group`, `struct`, `surface`, `tool`. An earlier draft used it to self-certify this exact demand, and it verified nothing.

Use `vox ci gui-surface-registry` / `contracts/reports/gui-surface-coverage.v1.json` instead, and assert the `mens` surface's tier moved from `curated_decorator` to `live_backend` in `crates/vox-gui/ui/src/generated/surfaceRegistry.generated.ts` — the surface now reads backend state (`get_active_model`) rather than running arg-free commands. That is a real, failing-today assertion.

- [ ] **Step 8: Commit**

---

## Sequencing

```
Task 1 (accel_budget)        ─┐
Task 2 (measurement harness) ─┴─→ Task 3 (MEASURE candle-metal)
                                        ↓
                                  Task 4 (memory_model, measured + seeded CUDA)
                                        ↓
                             ┌──── Task 5 (plan_for + 4 clamps) ────┐
                             ↓                                       ↓
                       Task 6 (probe/sweep)                   Task 7 (pinning)
                             └──────────────┬────────────────────────┘
                                            ↓
                                      Task 8 (deletions — single owner, serial)
```

Tasks 1 and 2 are file-disjoint and run in parallel. **Nothing is deleted until a measured constant exists and the catalogue is pinned against it.**

Task 9 (surfacing) runs after Task 5 and shares `probe.rs` and `action_populi_enum.rs` with Task 6 — **serialize Task 6, then Task 9.**

Task 6 and Task 8 both touch `train_arm.rs`; Task 6 first.

## Relationship to the other plans

- **Plan 0 owns** the `hub.rs` download break and the `VOX_MENS_FORCE_TRAIN` override. Not here.
- **Plan 2** (quantization, Ollama) is independent and runs fully in parallel.
- **Plan 3** (cloud) consumes `Lane`, `MemoryModels` and `plan_for` from Tasks 4–5 and must land after them. **This plan owns** the `resolver.rs` deletions *and* the `effective_preset` re-sourcing; Plan 3's resolver work is additive only.
- Task 6 shares `action_populi_enum.rs` with Plan 2's `MergeQlora` work. **Serialize: Plan 2 first**, then Task 6. (An earlier draft named Plan 3 as the counterpart; that was wrong — Plan 3 adds `CloudEstimate` to `PopuliMensTail` in `mens_tail_subcommands.rs`, a different enum in a different file.)

## What this plan deliberately does not do

- **No `sqrt(params)` term** — excluded by the data at any non-negative coefficient.
- **No quantization term** — it enters only through `artifact_bytes`.
- **No MLX lane.** MLX is an external reference measurement, not a Vox backend. Its constant is never served to a Vox lane, and `best_record` has a test that enforces that.
- **No unmeasured CUDA claim.** The CUDA row is `source: Seeded` from the constants already in the tree, so the 4080 lane keeps planning, and `probe` says plainly that it is inherited rather than measured. Calibrating it needs the hardware, and both 4080 Super hosts have been offline 67 and 28 days.
