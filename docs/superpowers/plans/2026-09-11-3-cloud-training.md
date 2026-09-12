# Plan 3 — Cloud GPU Training as a Gated Contingency

> **For agentic workers:** one fresh subagent per task. **Step 0 of every task is to read the Executor Preamble below.** Steps use checkbox (`- [ ]`) syntax.

**Goal:** Keep MENS training **local by default**, and make the cloud a *bounded, gated, inspectable* fallback rather than a peer option. This plan ships two things worth having regardless of where a run lands: a suitability filter that refuses structurally-wrong offers before money moves, and a read-only estimator that prints the ranked bill without provisioning. It also records the 2026-09-11 platform survey with per-row provenance so a guess never gets budgeted against as a fact.

**The position this plan encodes:** 128 GB of unified memory already outdoes a 96 GB GPU for this workload. Cloud is only worth entering when it beats local on a specific, named axis — and for three of the four triggers below that axis is **width, not depth**.

---

## The local baseline — measured, not modelled

| Metric | Measured value |
|---|---|
| Run | Qwen3.8-27B-8bit, LoRA, 800 iterations |
| Corpus | 306,484 tokens |
| Peak memory | **41.825 GB** |
| Throughput | **38.96 tok/s** |
| Wall time | **~2 h 10 m** |
| Val loss | **2.974 → 0.403** |
| Cost | **$0** |
| OOM events | **zero** |
| Machine envelope | 107.52 GiB Metal working set, 80.64 GiB max single buffer |

41.8 GB peak against a 107.52 GiB working set is **39 % utilisation**. There is no capacity problem to solve here, and therefore no cloud problem to solve here — until one of the four triggers below actually fires.

## Entry gate — the only four conditions under which cloud wins

**Do not provision anything unless you can name which of these fired.**

1. **Capacity wall.** Dense ~70 B+ at 8-bit; any full fine-tune above ~13 B; anything >100 B. This is binary arithmetic against the 107.52 GiB working set, not a judgement call.
2. **Parallel sweeps — the most likely near-term trigger.** Six LoRA arms serialize to ~13 h locally. Six concurrent pods finish in ~15 min for ~$3 total. The cloud's real product is **width**: many runs at once, not one run faster. **Threshold: ≥4 concurrent runs.**
3. **Single run >12 h local.** Below ~6 h the setup, upload, download and teardown overhead eats the entire speedup. 6–12 h is a coin flip. Above 12 h the overhead amortizes.
4. **Sequence length, not parameter count.** A 27 B LoRA at 32 K context can exceed the **80.64 GiB single-buffer cap** while the same model at 2 K sits at 41.8 GB. The cap is a per-allocation ceiling, so it bites on context, not on weights.

If none of these fired: **run it locally and close this plan.** Tasks 1–3 still ship — the filter and the estimator are how you find out cheaply *whether* a trigger justifies the bill — but nothing here authorizes a purchase on its own.

**Prerequisite before any cloud run:** local training and local quantization must be verified end-to-end first. A cloud run that produces an adapter you cannot quantize or serve has bought nothing.

---

**Architecture:** No new lane, no new ladder, no new price table in code. The cloud path already has everything except judgement:

- `CloudResolver::resolve` (`crates/vox-populi/src/mens/cloud/resolver.rs`) already queries Vast / RunPod / Local in parallel, costs each offer through `TimeEstimator`, and ranks by cost.
- `GpuOffer` (`crates/vox-populi/src/mens/cloud/mod.rs:239`) already carries `gpu_count`, `vram_mb`, `reliability_pct`, `price_per_hour_usd`, and `fetched_at`.

What is missing is that **none of those fields gate anything**. `gpu_count` is written by every provider and read by nothing. `reliability_pct` is only a sort tiebreaker. So this plan adds one pure predicate, wires it into the one ranking site, and exposes that ranking as a read-only command.

**Tech Stack:** Rust (`vox-populi/mens/cloud`, `vox-ml-cli`), the existing Vast.ai and RunPod provider clients, `mens/config/gpu-specs.yaml` (the existing GPU-spec SSOT), Markdown for the survey.

---

## Executor Preamble — every task's Step 0 is to read this

**You are a fresh subagent.** You have not read the other tasks and do not need to. Everything your task consumes is stated inside it. If you need a symbol, signature, file, or decision that is not written down in your task, that is **a defect in the task**, not a gap for you to fill by inference: stop and report what is missing. Do not guess a signature, do not invent a helper, do not adapt the code to match the plan.

**Checkout.** Work in a branch off `origin/main` `7295b4470`. Confirm before Step 1:

```bash
git merge-base --is-ancestor 7295b4470 HEAD && echo OK || echo "WRONG CHECKOUT — stop and report"
```

Every line number below was verified at that commit. If a cited line is off by more than ±5, or a cited symbol is absent, **stop and report** — you are in the wrong tree or the plan is stale.

**Read before you edit.** For every existing file your task modifies, read the cited region first (`sed -n 'START,ENDp' <file>`) and confirm the symbol is where the task says. Never edit a region you have not read in this session.

**House rules that will bite you** (from `AGENTS.md`):

- `cargo test` takes **exactly one** positional TESTNAME filter. Two is a hard error (`unexpected argument found`).
- **Never** `cargo fmt --all` — it overflows the Windows command-line limit. Use `cargo fmt -p <crate>`.
- **Cargo features are per-package and nothing is on by default.** `vox-populi` has `default = []` and `pub mod mens` is `#[cfg(feature = "mens")]`. `vox-ml-cli`'s serving module is behind `execution-api`, which `gpu` does **not** imply. A test run without the right `--features` compiles none of your work and exits 0.
- Every new `pub fn` needs a same-file `#[test]` or the `tdd-guard` pre-commit hook blocks the commit.
- No new `.py` / `.sh` / `.ps1` glue. Automation is VoxScript (`.vox`, via `vox run`).
- Do **not** create or edit `docs/src/architecture/research-index.md` — retired 2026-09-06. Frontmatter alone makes a doc discoverable.
- Do **not** run `git add -A` or `git commit -am`. Commit only the paths your task's **Files** block names; other agents are editing neighbouring files.

**Timeouts.** Unbounded commands are killed at 120 s with no output, which looks like a failure. Bound them:

```bash
timeout 900s  cargo test -p <crate> --features <f> --lib <one-filter>
timeout 1800s cargo run -p vox-cli -- ci pre-push --complete
```

`/opt/homebrew/bin/timeout` is GNU coreutils; an expired command exits **124** — that is a timeout, not a test failure. Cargo builds are serialized machine-wide by the build broker, so a cold build is slow even when nothing is wrong. On a 124, re-run **once** with double the budget, then report.

**You cannot watch remote CI.** `gh pr checks`, `gh run watch`, and polling loops are blocked by a hook. You cannot run interactive terminal dialogs (`/permissions`, `/config`, `/hooks`) or drive another machine's console.

**Verification discipline — the part that matters most.**

1. Run the exact command the step gives. Do not substitute a broader or narrower one.
2. Compare output **literally**. `PASS` is not an expectation; `2 passed; 0 failed` is.
3. **Zero tests run is never success.** `0 passed` means the code was not compiled — almost always a missing `--features`.
4. **A test that should not compile must not compile.** If a step says "Expected: FAIL to compile — `cannot find function X`" and you get a clean run or an assertion failure instead, **the premise is wrong.** Stop and report. Do not proceed to implement against a test you never saw fail.
5. **When reality and the step disagree, reality wins and you stop.** Report the step number, the command, the actual output, the promised output, and your one-line reading. Do not repair the plan by rewriting the test to match the code, loosening an assertion, or adding a wrapper that makes a missing symbol resolve.
6. **Verify each guard by mutation.** Break it deliberately, confirm red, restore, confirm green. A test that passes against both fixed and unfixed code is worse than none. Confirm the file is actually restored — a concurrent `rustfmt` can revert your edit and hand you a meaningless pass.

**Report back:** `STATUS: DONE | BLOCKED | PREMISE-FALSE`, files changed, each test with whether it failed-as-expected and then passed, every command run verbatim with exit status, and any deviation from the plan. A report that says "all steps complete, tests pass" without the counts is not a report.

---

## Hard dependency — the memory-SSOT plan must land first

Task 3 consumes `plan_for`, `AccelBudget`, `MemoryModels`, `CalKey`, `ModelShape`, `Request`, `TrainPlan` and `Verdict` from [`2026-09-11-1-memory-ssot-and-fit-benchmark.md`](2026-09-11-1-memory-ssot-and-fit-benchmark.md), all in `crates/vox-populi/src/mens/tensor/memory_model.rs`.

**None of those symbols exist at `origin/main` 7295b4470** (verified: `rg -n "enum Lane|fn plan_for" --glob '*.rs' crates/` returns only an unrelated `vox-skill-runtime::plan_for_min_tier`). **Do not start Task 3 until that plan has landed.** Tasks 1, 2 and 4 have no such dependency and can proceed immediately.

**`plan_for` is infallible.** Its exact signature, from that plan:

```rust
pub fn plan_for(
    budget: &AccelBudget,
    models: &MemoryModels,
    key: &CalKey,
    shape: &ModelShape,
    want: &Request,
) -> TrainPlan;
```

It returns a `TrainPlan` carrying a `Verdict`, **not** a `Result`. There is no `MemoryBudget` type; the budget type is `AccelBudget`. Task 3 therefore fails closed by matching `Verdict::Uncalibrated { lane }`, never with `?`.

**There is no measured CUDA `a_lane`.** The memory model is fitted from MLX points only. Every cloud offer in this plan runs CUDA, so an uncalibrated CUDA lane must produce a refusal, not a borrowed constant (Task 3, Step 5).

**Interface boundary, deliberate.** This plan's filter takes `required_vram_mb: u64`, a plain number — *not* a `&TrainPlan`. The caller derives that number from `plan_for`. Only `u64` bytes cross this seam. That keeps every symbol in Tasks 1–2 verifiable against today's tree and means a field rename in the SSOT plan cannot silently invalidate this one.

## Precondition — owned by the memory-SSOT plan, not by this one

The memory-SSOT plan **owns**:

- deleting `preset_for_vram` — `crates/vox-populi/src/mens/cloud/resolver.rs:58`
- deleting `min_vram_mb: 24000` — `crates/vox-populi/src/mens/cloud/resolver.rs:285`
- **re-sourcing `ResolvedOffer.effective_preset`** (`resolver.rs:29`), which today is constructed only from `preset_for_vram(offer.vram_mb)` at `resolver.rs:250` — the single call site.

**Do not delete or re-source any of the three in this plan.** Assume they are already handled and that `ResolveRequest.min_vram_mb` (`resolver.rs:35`) is now populated from `plan_for`. This plan's resolver work is **additive only**.

If you reach Task 2 and `resolver.rs:285` still reads `min_vram_mb: 24000`, **stop and report** — the plans have been executed out of order. (Tripwire retained deliberately.)

## Global Constraints

- **Local first, always.** No task in this plan provisions anything. Task 3's command is read-only by construction.
- **Burst, never persistent.** Any cloud run is a single job. The workload is data-limited (306 K tokens), not compute-limited, so no reserved commitment reaches break-even.
- **Never pay for interconnect.** A 27–32 B QLoRA trains on one GPU. Renting an 8-GPU node to use one of them is the most expensive mistake available here, and it is the mistake the codebase is currently unable to refuse — see Task 1.
- **The training corpus is the user's proprietary codebase.** This is the single most important constraint on provider choice, and it is why the recommendation is a single-tenant operator rather than the cheapest marketplace row. See Task 4 §Gotchas.
- **Always `terminate`, never `stop`.** Storage bills until deletion, and a stopped pod frequently cannot restart. See Task 4 §Gotchas 2–3.
- **Prices stay out of the code.** `GpuOffer.price_per_hour_usd` is documented in-tree as "Live price per hour in USD — always from provider API, never hardcoded" (`mod.rs:251-252`). This plan adds no price table to any contract file. Surveyed prices live in the dated research doc (Task 4), flagged V / S / U.
- `cargo test` takes **exactly one** positional TESTNAME filter — verified: `cargo test -p vox-populi --features mens-cloud --lib aaa bbb` → `error: unexpected argument 'bbb' found`.
- Never `cargo fmt --all`. Use `cargo fmt -p <crate>`.
- Features are per-package. The CLI feature is **`cloud`**, not `mens-cloud` (`crates/vox-ml-cli/Cargo.toml`: `cloud = ["gpu", "vox-populi/mens-cloud"]`). `mens-cloud` is `vox-populi`'s name for the same thing. Both verified.
- Attribution: `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>`.

## What was deleted from the previous revision of this plan, and why

| Deleted | Reason |
|---|---|
| **Cloud as a peer to local** | The measured local baseline (41.8 GB peak, 39 % utilisation, $0, zero OOM) settles the default. Cloud is now a gated contingency with four named triggers. |
| `UnsuitableReason::StalePrice`, its match arm, its `Display` arm and its test | **Unreachable in production.** Both providers TTL-gate their cache (`vast.rs:158`, `runpod_provider.rs:173`) and stamp `fetched_at = Some(now)` on the fetch they return (`vast.rs:209`, `runpod_provider.rs:131`, `local_provider.rs:46`). Any offer reaching `rank_offers` is by construction fresher than `price_cache_ttl_secs`. Only a hand-built test `Instant` could fire it — a test asserting a branch that no caller can reach. `GpuOffer::is_stale` stays uncalled; that is now a **stated** finding rather than a hidden one. |
| The `${x:>7.2}` padding in `format_row` | **Failed its own tests, 3 of 3.** Verified by running the format strings: `format!("${usd_per_hr:>7.2}", 2.09)` renders `"$   2.09"`, so `row.contains("$2.09")` is `false`; `${total_usd:>8.2}` with 27.16 renders `"$   27.16"`, so `contains("$27.16")` is `false`; test 2's `.find("$2.09").expect(...)` **panics**. Only the `"13.0 h"` / `"313.0 h"` assertions passed. Fixed by padding *outside* the `$`. |
| `plan_for(lane, params_b).map_err(...)?` | **Wrong arity and wrong fallibility.** `plan_for` takes five arguments and returns `TrainPlan`, not `Result`. `MemoryBudget` does not exist; the type is `AccelBudget`. Task 3 now matches `Verdict::Uncalibrated`. |
| Deleting `preset_for_vram` / `min_vram_mb: 24000` / re-sourcing `effective_preset` | All three owned by the memory-SSOT plan (see Precondition). |
| Three near-identical 12-field `GpuOffer` fixtures | Replaced by one `#[cfg(test)] pub(crate) fn test_offer()` in `cloud/mod.rs`, reused by struct-update syntax. |
| The claim "no surveyed provider published a numeric uptime SLA" | **Wrong — three do.** Voltage Park (≥99.5 % per VM with a credit ladder), Nebius (99.50 %) and Hyperstack (published but self-contradictory). Corrected in Task 4 §5. |
| All of old **Task 1** — `contracts/mens/cloud-platforms.v1.yaml`, `PlatformRow`, `load_platforms`, ~40 hand-transcribed GPU prices | Contradicts the in-tree rule on `GpuOffer.price_per_hour_usd` ("never hardcoded"), and was a 40-value hand-transcription surface with no test pinning any value. |
| All of old **Task 3** — `resolve_checkpoint_every`, `latest_checkpoint_step` | Resume is **already implemented**. `CheckpointState` (`crates/vox-populi/src/mens/tensor/checkpoint_state.rs`) saves atomically and is loaded at `crates/vox-plugin-mens-candle-cuda/src/candle_qlora_train/training_loop/checkpoint.rs:27` and the `-metal` sibling. The invented function scanned `checkpoint-<step>/` directories; the real artifact is a single `checkpoint_state.json`. |
| The `CloudOffer` type | **Does not exist.** The real type is `GpuOffer` (`mod.rs:239`) and zero field names overlapped. |
| Filters on `egress_usd_per_gb`, `disk_gib`, `node_granular` | **No such fields on `GpuOffer`.** (Egress remains the single biggest cost risk — it is handled in the survey's gotcha list, not by a field that does not exist.) |
| `OfferConstraints` as a new public struct | Every threshold it wanted already exists on `CloudProviderConfig` (`mod.rs:128`): `min_reliability` (`:132`), `price_cache_ttl_secs` (`:136`). |
| `status: "accepted"` in the doc frontmatter | Not a valid status. `VALID_STATUS` (`crates/vox-doc-pipeline/src/pipeline/lint.rs:37-45`) is `approved, current, experimental, legacy, research, roadmap, deprecated`. |
| Adding a row to `docs/src/architecture/research-index.md` | **That file no longer exists** (retired 2026-09-06). `AGENTS.md` forbids recreating it. |
| `cargo run -p vox-ml-cli --features mens-cloud` | Hard error — no such feature on that package. Correct flag is `--features cloud`. |
| `cargo test … --lib platform_tests cost` | Hard error — two positional filters. |

**Kept deliberately:** `training_eligible: false` in the Task 4 frontmatter. It **is** a known key — `crates/vox-doc-pipeline/src/pipeline/lint.rs:428` parses it, and only `training_eligible: true` on a research/roadmap page requires a companion `training_rationale:` (`lint.rs:458`, `pipeline/mod.rs:350`). `false` is inert and explicit.

## File structure

| File | Change |
|---|---|
| `crates/vox-populi/src/mens/cloud/offer_filter.rs` | **new** — one pure predicate + its reason enum. |
| `crates/vox-populi/src/mens/cloud/mod.rs` | **modify** — `pub mod offer_filter;`, re-export, shared `#[cfg(test)] test_offer()`. |
| `crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs` | **modify** — its local `test_offer()` moves out; tests import the shared one. |
| `crates/vox-populi/src/mens/cloud/resolver.rs` | **modify** — extract `rank_offers`, call the filter, `resolve` returns rejections too. |
| `crates/vox-speech/src/backends/cloud_offload.rs` | **modify** — one line, for the new `resolve` return type. |
| `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` | **modify** — one line, same reason. |
| `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs` | **modify** — `CloudEstimate` variant. |
| `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` | **modify** — dispatch arm. |
| `crates/vox-ml-cli/src/commands/mens/cloud_estimate.rs` | **new** — the printer. |
| `contracts/operations/catalog.v1.yaml` | **modify** — `mens.cloud-estimate` row (SSOT). |
| `contracts/cli/command-registry.yaml`, `docs/src/reference/cli-command-surface.generated.md` | **regenerated**, never hand-edited. |
| `docs/src/reference/cli.md` | **modify** — doc-parity mention. |
| `docs/src/architecture/mens-cloud-training-economics-2026-09-11.md` | **new** — the survey. |

---

### Task 1: The offer filter — encode the rules the resolver cannot currently express

- [ ] **Step 0: Read the Executor Preamble at the top of this file.**

**Files:**
- Create: `crates/vox-populi/src/mens/cloud/offer_filter.rs`
- Modify: `crates/vox-populi/src/mens/cloud/mod.rs` (module declaration, re-export, shared test fixture)
- Modify: `crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs` (drop its private fixture, import the shared one)

**Interfaces:**

- Consumes: `GpuOffer` (`mod.rs:239`), `CloudProviderConfig` (`mod.rs:128`).
- Consumes: `required_vram_mb: u64`, derived by the caller from `plan_for` (memory-SSOT plan). Only a `u64` crosses this seam.
- Produces:
  ```rust
  pub enum UnsuitableReason {
      MultiGpuNode { gpu_count: u32, price_per_hour_usd: f64 },
      InsufficientVram { per_gpu_mb: u64, required_mb: u64 },
      Unreliable { reliability_pct: f32, floor_pct: f32 },
  }
  pub fn offer_is_suitable(
      offer: &GpuOffer,
      config: &CloudProviderConfig,
      required_vram_mb: u64,
  ) -> Result<(), UnsuitableReason>;
  ```

**Why three variants and not four.** A `StalePrice` variant is **unreachable in production**: `vast.rs:158` and `runpod_provider.rs:173` both return cached offers only while `elapsed() < price_cache_ttl_secs`, and a fresh fetch stamps `fetched_at = Some(now)` (`vast.rs:209`, `runpod_provider.rs:131`, `local_provider.rs:46`). Every offer reaching the ranker is younger than the TTL by construction. `GpuOffer::is_stale` (`mod.rs:273`) therefore stays with **zero call sites**, and that is recorded here rather than papered over with a test that fabricates an old `Instant`.

**Why no `OfferConstraints` struct:** `config.min_reliability` (f32, **[0.0, 1.0]**, default 0.90 at `mod.rs:190`) and `config.price_cache_ttl_secs` already exist and are already the knobs the Vast client reads. A parallel struct is a second source of truth for the same numbers.

- [ ] **Step 1: Hoist the shared test fixture**

`crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs:593` has a private `fn test_offer() -> GpuOffer` used by five tests (`:716`, `:736`, `:909`, `:1089`, `:1125`). Tasks 1 and 2 need the same shape. Move it, **values unchanged**, into `crates/vox-populi/src/mens/cloud/mod.rs` at module scope:

```rust
/// Shared `GpuOffer` fixture. Values are the ones `pipeline_dispatch`'s tests were
/// written against; vary fields with struct-update syntax rather than cloning the
/// whole 12-field literal.
#[cfg(test)]
pub(crate) fn test_offer() -> GpuOffer {
    GpuOffer {
        provider: ProviderKind::RunPod,
        offer_id: "offer-1".into(),
        gpu_name: "rtx 4090".into(),
        gpu_count: 1,
        vram_mb: 24576,
        price_per_hour_usd: 1.0,
        is_spot: true,
        reliability_pct: 95.0,
        auto_terminate: false,
        fetched_at: Some(std::time::Instant::now()),
        datacenter_region: None,
        cuda_max: None,
    }
}
```

Delete the copy at `pipeline_dispatch.rs:593-607` and add `test_offer` to that module's `use crate::mens::cloud::{...}` import. **Its five tests must still pass unchanged** — that is the regression check for this step.

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib pipeline_dispatch
```
Expected: the same pass count as before the move. Record the number before you start.

- [ ] **Step 2: Write the failing tests**

Create `crates/vox-populi/src/mens/cloud/offer_filter.rs` containing **only** the test module below, and add `pub mod offer_filter;` to `crates/vox-populi/src/mens/cloud/mod.rs` beside the existing `pub mod estimator;` (`mod.rs:29`).

```rust
//! Suitability filter for cloud GPU offers.
//!
//! `CloudResolver` ranks offers by cost. Cost alone cannot express "this offer is
//! structurally wrong for the job": an 8-GPU node reports 640 GB of VRAM and bills
//! for eight GPUs to run a single-device QLoRA, and a marketplace host with a 12%
//! reliability score will evict you.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mens::cloud::{CloudProviderConfig, GpuOffer, ProviderKind, test_offer};

    /// An offer that should pass every arm: 1x H100 SXM 80 GB, 97% reliable.
    fn h100() -> GpuOffer {
        GpuOffer {
            provider: ProviderKind::Vast,
            gpu_name: "h100 sxm".into(),
            vram_mb: 81_920,
            price_per_hour_usd: 2.09,
            reliability_pct: 97.0,
            auto_terminate: true,
            ..test_offer()
        }
    }

    /// MUTATION CAUGHT: deleting the `gpu_count` arm.
    /// An 8x H100 node reports vram_mb = 655_360, so every VRAM-only check waves it
    /// through -- and you then pay 8x $2.09/hr to keep seven GPUs idle. `gpu_count`
    /// is written by every provider today and read by nothing.
    #[test]
    fn rejects_an_eight_gpu_node_for_a_single_gpu_job() {
        let offer = GpuOffer {
            gpu_count: 8,
            vram_mb: 655_360,
            price_per_hour_usd: 16.72,
            ..h100()
        };
        assert!(matches!(
            offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).unwrap_err(),
            UnsuitableReason::MultiGpuNode { gpu_count: 8, .. }
        ));
    }

    /// MUTATION CAUGHT: comparing `offer.vram_mb` instead of `vram_mb / gpu_count`.
    /// `GpuOffer.vram_mb` is documented as "Total VRAM in MB across all GPUs"
    /// (mod.rs:249). A 4x A100-40G node totals 163_840 MB and would satisfy a
    /// 96 GB requirement on paper while no single device holds more than 40 GB --
    /// the job OOMs on step 1 after the instance is already billing.
    #[test]
    fn vram_is_checked_per_gpu_not_per_node() {
        let offer = GpuOffer {
            gpu_count: 4,
            vram_mb: 163_840,
            gpu_name: "a100-sxm4-40gb".into(),
            price_per_hour_usd: 0.0, // free: isolates the VRAM arm from the node arm
            ..h100()
        };
        assert!(matches!(
            offer_is_suitable(&offer, &CloudProviderConfig::default(), 98_304).unwrap_err(),
            UnsuitableReason::InsufficientVram { per_gpu_mb: 40_960, required_mb: 98_304 }
        ));
    }

    /// MUTATION CAUGHT: comparing `reliability_pct >= config.min_reliability` directly.
    /// `config.min_reliability` is f32 on [0.0, 1.0] (mod.rs:131-132, default 0.90);
    /// `GpuOffer.reliability_pct` is f32 on [0, 100] (mod.rs:255-256). Comparing them
    /// unscaled passes a host with a 1% score. This matters for RunPod and Local
    /// offers specifically: only `vast.rs:195-198` filters reliability provider-side,
    /// so a RunPod offer reaches the resolver ungated.
    #[test]
    fn reliability_floor_is_applied_on_the_offers_own_scale() {
        let offer = GpuOffer {
            provider: ProviderKind::RunPod,
            reliability_pct: 12.0,
            ..h100()
        };
        let err = offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).unwrap_err();
        assert!(
            matches!(err, UnsuitableReason::Unreliable { floor_pct, .. } if (floor_pct - 90.0).abs() < 1e-3),
            "floor must be 90.0 pct, not 0.90; got {err:?}"
        );
    }

    /// MUTATION CAUGHT: applying the single-GPU rule unconditionally.
    /// `LocalProvider` emits one offer with `price_per_hour_usd: 0.0` and the host's
    /// real `gpu_count` (local_provider.rs:40-46). That row exists purely so the
    /// operator can compare "rent" against "run it here" -- and under this plan's
    /// entry gate, that comparison is the whole decision. A blanket gpu_count == 1
    /// rule silently deletes the local baseline on any multi-GPU workstation.
    /// The rule is about *paying* for idle silicon, so it is priced, not counted.
    #[test]
    fn a_free_local_multi_gpu_box_is_not_rejected_as_a_node() {
        let offer = GpuOffer {
            provider: ProviderKind::Local,
            gpu_count: 2,
            vram_mb: 163_840,
            price_per_hour_usd: 0.0,
            reliability_pct: 100.0,
            ..h100()
        };
        assert!(offer_is_suitable(&offer, &CloudProviderConfig::default(), 81_920).is_ok());
    }

    /// MUTATION CAUGHT: a filter that fails closed on everything (e.g. an inverted
    /// comparison, or a `required_mb` unit mix-up between MB and GiB). Without this
    /// the three reject tests above all still pass while the resolver returns an
    /// empty ranking and the operator sees "No GPU offers found".
    #[test]
    fn accepts_the_recommended_configuration() {
        assert!(offer_is_suitable(&h100(), &CloudProviderConfig::default(), 81_920).is_ok());
    }
}
```

- [ ] **Step 3: Run to verify failure — state the exact failure mode**

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib offer_filter
```

Expected: **a compile error, not an assertion failure.** The test module references `offer_is_suitable` and `UnsuitableReason` via `use super::*`, and neither exists yet:

```
error[E0425]: cannot find function `offer_is_suitable` in this scope
error[E0433]: failed to resolve: use of undeclared type `UnsuitableReason`
```

Do not proceed until you have seen those two errors. A run that reports `0 passed; 0 failed` means you forgot `pub mod offer_filter;` in `mod.rs` — the filter name matched nothing and the suite passed vacuously.

- [ ] **Step 4: Minimal implementation**

Prepend to `offer_filter.rs`, above the test module:

```rust
use super::{CloudProviderConfig, GpuOffer};

/// Why an offer cannot run this job. Carries the measured values so the CLI can
/// print a reason the operator can act on rather than "no offers found".
#[derive(Debug, Clone, PartialEq)]
pub enum UnsuitableReason {
    /// A paid multi-GPU node for a single-device job: you are billed for silicon
    /// a single-device QLoRA will never touch.
    MultiGpuNode {
        /// GPUs in the offer.
        gpu_count: u32,
        /// Hourly price for the whole node.
        price_per_hour_usd: f64,
    },
    /// No single device in the offer holds the working set.
    InsufficientVram {
        /// `vram_mb / gpu_count` — what one device actually has.
        per_gpu_mb: u64,
        /// What `plan_for` says the job needs.
        required_mb: u64,
    },
    /// Host reliability below the configured floor.
    Unreliable {
        /// The offer's score, [0, 100].
        reliability_pct: f32,
        /// `config.min_reliability * 100.0`.
        floor_pct: f32,
    },
}

impl std::fmt::Display for UnsuitableReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MultiGpuNode { gpu_count, price_per_hour_usd } => write!(
                f,
                "{gpu_count}-GPU node at ${price_per_hour_usd:.2}/hr; this job uses one GPU"
            ),
            Self::InsufficientVram { per_gpu_mb, required_mb } => {
                write!(f, "{per_gpu_mb} MB per GPU < {required_mb} MB required")
            }
            Self::Unreliable { reliability_pct, floor_pct } => {
                write!(f, "reliability {reliability_pct:.1}% < floor {floor_pct:.1}%")
            }
        }
    }
}

/// Can this offer run a single-device job needing `required_vram_mb` on one GPU?
///
/// `required_vram_mb` comes from `plan_for` (memory-SSOT plan), not from a VRAM
/// ladder: the question is "does my model fit on this box", not "which canned
/// preset is nearest this number".
///
/// Thresholds come from [`CloudProviderConfig`] so this filter and the Vast client
/// read the same knobs.
pub fn offer_is_suitable(
    offer: &GpuOffer,
    config: &CloudProviderConfig,
    required_vram_mb: u64,
) -> Result<(), UnsuitableReason> {
    // Paid multi-GPU nodes only. A free local box with two GPUs is the comparison
    // baseline, not a purchase.
    if offer.gpu_count > 1 && offer.price_per_hour_usd > 0.0 {
        return Err(UnsuitableReason::MultiGpuNode {
            gpu_count: offer.gpu_count,
            price_per_hour_usd: offer.price_per_hour_usd,
        });
    }

    // `vram_mb` is the total across all GPUs (mod.rs:249). A single-device job
    // sees one device's share.
    let per_gpu_mb = offer.vram_mb / u64::from(offer.gpu_count.max(1));
    if per_gpu_mb < required_vram_mb {
        return Err(UnsuitableReason::InsufficientVram {
            per_gpu_mb,
            required_mb: required_vram_mb,
        });
    }

    // config.min_reliability is [0.0, 1.0]; reliability_pct is [0, 100].
    let floor_pct = config.min_reliability * 100.0;
    if offer.reliability_pct < floor_pct {
        return Err(UnsuitableReason::Unreliable {
            reliability_pct: offer.reliability_pct,
            floor_pct,
        });
    }

    Ok(())
}
```

`Display` is kept because Task 3 prints these reasons to the operator. If Task 3 is cut, delete `Display` with it — `#[derive(Debug)]` already covers the `tracing` path.

Add the re-export beside the existing `pub use resolver::{CloudResolver, ResolveRequest};` (`mod.rs:44`):

```rust
pub use offer_filter::{UnsuitableReason, offer_is_suitable};
```

- [ ] **Step 5: Run, mutate, commit**

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib offer_filter
```
Expected: `5 passed; 0 failed`.

Mutation check (Preamble rule 6): delete the `gpu_count > 1` arm, confirm `rejects_an_eight_gpu_node_for_a_single_gpu_job` **fails**, restore, confirm green. Then change `floor_pct` to `config.min_reliability` (unscaled), confirm `reliability_floor_is_applied_on_the_offers_own_scale` fails, restore.

```
cargo fmt -p vox-populi
```

```bash
git add crates/vox-populi/src/mens/cloud/offer_filter.rs \
        crates/vox-populi/src/mens/cloud/mod.rs \
        crates/vox-populi/src/mens/cloud/pipeline_dispatch.rs
git commit -m "$(cat <<'EOF'
feat(mens/cloud): suitability filter over the fields GpuOffer already carries

GpuOffer records gpu_count and reliability_pct, and today neither gates
anything -- gpu_count is written by every provider and read by nothing, and
reliability_pct is only a sort tiebreaker.

offer_is_suitable refuses the three offers that are structurally wrong for a
single-device QLoRA burst: a paid multi-GPU node (vram_mb is the total across
all GPUs, so an 8x H100 node sails past any VRAM-only check while billing for
seven idle GPUs), an offer whose per-GPU share does not hold the working set,
and a host under the configured reliability floor.

There is deliberately no StalePrice arm. Both providers TTL-gate their cache
and stamp fetched_at on every fetch they return, so an offer reaching the
ranker is fresher than price_cache_ttl_secs by construction; the variant would
have been reachable only from a test that fabricates an old Instant.
GpuOffer::is_stale accordingly still has zero call sites, which is now a
stated finding rather than a hidden one.

Thresholds come from CloudProviderConfig rather than a new struct, so this
filter and vast.rs read the same knobs. Note min_reliability is [0.0, 1.0]
while reliability_pct is [0, 100]; the scale conversion is tested.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Wire the filter into the one ranking site, and surface the rejections

- [ ] **Step 0: Read the Executor Preamble at the top of this file.**

A validator with no call site is the failure mode `AGENTS.md` §PR & Review Discipline names explicitly ("a validator unit-tested directly still passed with every call site deleted"). Task 1's tests all pass with `offer_is_suitable` never called.

**Decision made here so the executor does not have to make it:** `CloudResolver::resolve` **changes its return type** to `(Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>)`. Without that, rejections die in a `tracing::debug!` and Task 3 cannot print them — a green test over a feature unreachable from the CLI. `resolve` has exactly **two** call sites (verified below); each needs one line changed.

**Files:**
- Modify: `crates/vox-populi/src/mens/cloud/resolver.rs`
- Modify: `crates/vox-speech/src/backends/cloud_offload.rs:58` (one line)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:170-183` (one line)

**Interfaces:**
- Consumes: `offer_is_suitable`, `UnsuitableReason` (Task 1).
- Consumes: `ResolveRequest.min_vram_mb` (`resolver.rs:35`) — populated from `plan_for` by the memory-SSOT plan.
- Produces:
  ```rust
  pub(crate) fn rank_offers(
      offers: Vec<GpuOffer>,
      req: &ResolveRequest,
      config: &CloudProviderConfig,
      estimator: &TimeEstimator,
      remaining_usd: f64,
  ) -> (Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>);
  ```
- Changes: `pub async fn resolve(&self, req: &ResolveRequest) -> anyhow::Result<(Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>)>`

**Why extract a function:** `resolve()` is `async` and needs live provider clients. `rank_offers` is pure and testable. The residual "someone deletes the `rank_offers` call from `resolve`" mutation is caught by the compiler — `ResolvedOffer` has no other construction site.

- [ ] **Step 1: Write the failing test**

Append to `crates/vox-populi/src/mens/cloud/resolver.rs`:

```rust
#[cfg(test)]
mod rank_tests {
    use super::*;
    use crate::mens::cloud::{ProviderKind, test_offer};

    fn offer(id: &str, gpu_count: u32, vram_mb: u64, usd: f64) -> GpuOffer {
        GpuOffer {
            provider: ProviderKind::Vast,
            offer_id: id.into(),
            gpu_name: "h100 sxm".into(),
            gpu_count,
            vram_mb,
            price_per_hour_usd: usd,
            reliability_pct: 97.0,
            auto_terminate: true,
            ..test_offer()
        }
    }

    /// Same shape as `pipeline_dispatch::tests::zero_estimator`: an empty specs file
    /// keeps the estimator on its conservative fallback tier, which is deterministic.
    fn test_estimator() -> TimeEstimator {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("gpu-specs.yaml");
        std::fs::write(&path, "gpus: {}\npresets: {}\n").unwrap();
        TimeEstimator::new(&path, vec![]).unwrap()
    }

    fn req(min_vram_mb: u64) -> ResolveRequest {
        ResolveRequest {
            min_vram_mb,
            seq_len: 512,
            batch_size: 1,
            num_samples: 8,
            epochs: 1,
            // Budget must never be the thing that drops an offer in this test --
            // otherwise a deleted filter still yields a "correct-looking" ranking.
            max_acceptable_cost: f64::MAX,
            target: CloudTarget::Auto,
        }
    }

    /// MUTATION CAUGHT: deleting the `offer_is_suitable` call from `rank_offers`.
    /// Without it the 8-GPU node survives -- and because it is *not* the cheapest
    /// row, a test that only asserted "the H100 ranks first" would still pass. This
    /// asserts the node is absent, which is the only assertion the mutant fails.
    #[test]
    fn ranking_drops_a_paid_multi_gpu_node_and_keeps_the_single_gpu_offer() {
        let offers = vec![
            offer("node-8x", 8, 655_360, 16.72),
            offer("single-h100", 1, 81_920, 2.09),
        ];
        let (ranked, rejected) = rank_offers(
            offers,
            &req(81_920),
            &CloudProviderConfig::default(),
            &test_estimator(),
            f64::MAX,
        );

        let ids: Vec<&str> = ranked.iter().map(|r| r.offer.offer_id.as_str()).collect();
        assert_eq!(ids, ["single-h100"], "the 8-GPU node must not be rankable");
        assert!(
            rejected.iter().any(|(id, r)| id == "node-8x"
                && matches!(r, UnsuitableReason::MultiGpuNode { .. })),
            "the rejection must be reported with its reason, got {rejected:?}"
        );
    }

    /// MUTATION CAUGHT: swallowing rejections silently (returning only the ranked
    /// vec). An operator whose whole board was filtered out needs to see why, or
    /// the failure reads as "the provider has no capacity" and they go buy the
    /// wrong thing somewhere else.
    #[test]
    fn every_rejected_offer_is_reported_with_a_reason() {
        let offers = vec![offer("too-small", 1, 24_576, 0.44)];
        let (ranked, rejected) = rank_offers(
            offers,
            &req(81_920),
            &CloudProviderConfig::default(),
            &test_estimator(),
            f64::MAX,
        );
        assert!(ranked.is_empty());
        assert_eq!(rejected.len(), 1);
        assert!(matches!(
            rejected[0].1,
            UnsuitableReason::InsufficientVram { per_gpu_mb: 24_576, required_mb: 81_920 }
        ));
    }
}
```

- [ ] **Step 2: Run to verify failure — state the exact failure mode**

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib rank_tests
```

Expected: **compile error**, `error[E0425]: cannot find function 'rank_offers' in this scope`. Not an assertion failure — the function does not exist yet.

If instead you see `error[E0433]: failed to resolve: use of undeclared crate or module 'tempfile'`, verify `tempfile` is in `[dev-dependencies]` of `crates/vox-populi/Cargo.toml` before adding it — `pipeline_dispatch`'s tests already use it, so it should be there.

- [ ] **Step 3: Extract `rank_offers` and call the filter from it**

Read `resolver.rs` from `let mut ranked: Vec<ResolvedOffer> = all` (`:227`) through `Ok(ranked)` (`:273`) first. Replace that whole block with:

```rust
        let (ranked, rejected) = rank_offers(all, req, &self.config, &self.estimator, remaining);
        for (id, reason) in &rejected {
            tracing::debug!(offer_id = %id, reason = %reason, "offer filtered out");
        }
        Ok((ranked, rejected))
```

and widen `resolve`'s signature to return `anyhow::Result<(Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>)>`.

Add the extracted function at module scope:

```rust
/// Cost, filter, and rank offers. Pure: no I/O, no clock.
///
/// Returns `(ranked, rejected)`. Rejections carry their reason so the caller can tell
/// the operator *why* the board is empty instead of "no offers found".
pub(crate) fn rank_offers(
    offers: Vec<GpuOffer>,
    req: &ResolveRequest,
    config: &CloudProviderConfig,
    estimator: &TimeEstimator,
    remaining_usd: f64,
) -> (Vec<ResolvedOffer>, Vec<(String, UnsuitableReason)>) {
    let mut rejected: Vec<(String, UnsuitableReason)> = vec![];
    let mut ranked: Vec<ResolvedOffer> = vec![];

    for offer in offers {
        if let Err(reason) = offer_is_suitable(&offer, config, req.min_vram_mb) {
            rejected.push((offer.offer_id.clone(), reason));
            continue;
        }

        let overhead = if offer.auto_terminate {
            OVERHEAD_AUTO_TERMINATE
        } else {
            OVERHEAD_POLL_TERMINATE
        };
        let (est_secs, source) = estimator.estimate(
            &offer.gpu_name,
            req.seq_len,
            req.batch_size,
            req.num_samples,
            req.epochs,
        );
        let total_secs = est_secs * overhead;
        let cost = (total_secs / 3600.0) * offer.price_per_hour_usd;

        if cost > remaining_usd || cost > req.max_acceptable_cost {
            continue;
        }

        ranked.push(ResolvedOffer {
            effective_preset: preset_for_vram(offer.vram_mb),
            estimated_secs: total_secs,
            estimated_cost_usd: cost,
            estimate_source: source,
            offer,
        });
    }

    // Sort: cheapest → prefer auto_terminate → higher reliability.
    ranked.sort_by(|a, b| {
        a.estimated_cost_usd
            .partial_cmp(&b.estimated_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(b.offer.auto_terminate.cmp(&a.offer.auto_terminate))
            .then(
                b.offer
                    .reliability_pct
                    .partial_cmp(&a.offer.reliability_pct)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    (ranked, rejected)
}
```

Add to the `use super::{...}` block at the top of `resolver.rs`:
`offer_filter::{UnsuitableReason, offer_is_suitable},`

> **Note on `effective_preset`.** The line above still calls `preset_for_vram`. The memory-SSOT plan **owns** re-sourcing `ResolvedOffer.effective_preset` (`resolver.rs:29`), whose only construction site is `resolver.rs:250`. If that plan has landed, use whatever it put there and do not reintroduce the ladder. If `preset_for_vram` still exists at `resolver.rs:58` and `resolver.rs:285` still reads `min_vram_mb: 24000`, **stop and report** — the plans are out of order.

- [ ] **Step 4: Update the two callers**

Both are a single line, verified by `rg -n "\.resolve\(" --glob '*.rs' crates/` filtered to `CloudResolver`:

- `crates/vox-speech/src/backends/cloud_offload.rs:58` — `let ranked = resolver.resolve(&req).await?;` → `let (ranked, _rejected) = resolver.resolve(&req).await?;`
- `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:170` — `let ranked = resolver.resolve(...).await?;` → `let (ranked, _rejected) = resolver.resolve(...).await?;`

Both then pass `&ranked` to `dispatch_top` unchanged.

- [ ] **Step 5: Run, mutate, commit**

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib rank_tests
```
Expected: `2 passed; 0 failed`.

```
timeout 900s cargo test -p vox-populi --features mens-cloud --lib cloud
```
Expected: all pre-existing cloud tests still pass (regression check on the extraction).

Mutation check: delete the `if let Err(reason) = offer_is_suitable(...)` block from `rank_offers`, confirm **both** `rank_tests` fail, restore, confirm green.

```
cargo fmt -p vox-populi
timeout 1800s cargo clippy -p vox-populi --features mens-cloud --all-targets -- -D warnings
```

```bash
git add crates/vox-populi/src/mens/cloud/resolver.rs \
        crates/vox-speech/src/backends/cloud_offload.rs \
        crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs
git commit -m "$(cat <<'EOF'
feat(mens/cloud): rank_offers applies the suitability filter, and says why

Extracts the ranking block out of the async resolve() so the filter has a
testable call site. A filter with no call site is the exact failure AGENTS.md
warns about -- Task 1's unit tests all pass with offer_is_suitable never
invoked.

resolve() now returns the rejections alongside the ranking rather than logging
them at debug and discarding them, so the reason can actually reach an
operator. Both call sites (vox-speech cloud_offload, vox-ml-cli train_arm)
destructure and ignore the second element; only `vox mens cloud-estimate`
prints it.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: `vox mens cloud-estimate` — see the bill before provisioning

- [ ] **Step 0: Read the Executor Preamble at the top of this file.**

**Blocked on the memory-SSOT plan.** Do not start until `plan_for` exists in `crates/vox-populi/src/mens/tensor/memory_model.rs`.

Today `vox mens train --cloud <provider>` resolves, ranks, **and dispatches**. There is no `--dry-run` (verified: `rg -n "dry_run" crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` returns nothing). So the only way to see the ranked cost is to buy it — which is exactly backwards for a plan whose default is "don't buy it".

This command is the same resolve, printed instead of dispatched. It adds no cost model of its own — `ResolvedOffer.estimated_cost_usd` already exists.

**Files:**
- Create: `crates/vox-ml-cli/src/commands/mens/cloud_estimate.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/mod.rs` (module declaration)
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs`
- Modify: `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs`
- Modify: `contracts/operations/catalog.v1.yaml`, `docs/src/reference/cli.md`
- Regenerated: `contracts/cli/command-registry.yaml`, `docs/src/reference/cli-command-surface.generated.md`

**Interfaces:**
- Consumes: `CloudResolver::new_from_env`, `ResolveRequest`, `ResolvedOffer`, `UnsuitableReason` (vox-populi).
- Consumes: `plan_for`, `AccelBudget`, `MemoryModels`, `CalKey`, `ModelShape`, `Request`, `TrainPlan`, `Verdict` (memory-SSOT plan).
- Produces: `pub fn format_row(...) -> String` and `pub async fn run(...) -> anyhow::Result<()>`

- [ ] **Step 1: Register the command in the catalog SSOT — do this FIRST**

This is not bookkeeping you can defer. `vox ci ssot-drift` runs `operations_catalog::verify` and `command_compliance::run` (`crates/vox-cli/src/commands/ci/run_body_helpers/docs.rs:601-611`), and `ssot-drift` is an unconditional step in the **fast** pre-push tier (`crates/vox-cli/src/commands/ci/pre_push.rs:512-516`). An unregistered command makes your first `git push` fail with an error that looks unrelated to the work.

Insert into `contracts/operations/catalog.v1.yaml` **in `id` order** — between `mens.check` (currently line 7564) and `mens.corpus` (currently line 7591):

```yaml
- id: mens.cloud-estimate
  title: Mens Cloud-estimate
  description: CLI operation `vox mens cloud-estimate`
  description_human: null
  product_lane: ai
  intent_tags: []
  side_effect_class: null
  scope_kind: null
  reversible: null
  requires_repo: null
  preferred_for_models: null
  human_takeover_friendly: null
  mens_planner_visible: null
  canonical_name: null
  latin_aliases: null
  mcp: null
  cli:
    path:
    - mens
    - cloud-estimate
    status: active
    latin_ns: mens
    handler_rust: null
    feature_gate: cloud
    catalog_group: null
    ref_cli_required: true
    reachability_required: null
```

`ref_cli_required: true` means `check_ref_cli` (`crates/vox-cli/src/commands/ci/command_compliance/validators.rs:604-611`) requires the literal string `vox mens cloud-estimate` in `docs/src/reference/cli.md`. Add it to the existing **"Doc parity (`vox ci command-compliance`)"** list at `docs/src/reference/cli.md:673`.

Regenerate the two derived files — never hand-edit them. **Each of these is a workspace build; bound them:**

```bash
timeout 1800s cargo run -p vox-cli -- ci operations-sync --target cli --write
timeout 1800s cargo run -p vox-cli -- ci command-sync --write
timeout 1800s cargo run -p vox-cli -- ci ssot-drift
```

Exit **124** is a timeout, not a failure — re-run once at double the budget before reporting. Verify `ssot-drift` is green before writing any code.

- [ ] **Step 2: Write the failing test**

`vox-ml-cli` is a binary crate; the useful unit here is the row formatter, not the async resolve. Create `crates/vox-ml-cli/src/commands/mens/cloud_estimate.rs` with the test module only, and add `#[cfg(feature = "cloud")] pub mod cloud_estimate;` to `crates/vox-ml-cli/src/commands/mens/mod.rs` beside the existing gated modules (`:24-37`).

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// MUTATION CAUGHT: printing `estimated_secs` raw, or dividing by 60 instead of
    /// 3600. A 13-hour run shown as "13 min" or "46800" is the number the operator
    /// uses to decide whether the >12h trigger fired. Getting it wrong by 60x
    /// produces a confident, wrong purchase.
    #[test]
    fn a_thirteen_hour_run_prints_as_hours_not_seconds() {
        let row = format_row("vast", "h100 sxm", 1, 46_800.0, 27.16, 2.09);
        assert!(row.contains("13.0 h"), "expected hours in {row:?}");
        assert!(row.contains("$27.16"), "expected total cost in {row:?}");
    }

    /// MUTATION CAUGHT: printing $/hr where total cost belongs (or vice versa), and
    /// padding *inside* the `$` -- `format!("${x:>7.2}", 2.09)` renders "$   2.09",
    /// which contains neither "$2.09" nor any greppable amount. $2.09 and $27.16 are
    /// both plausible numbers on a rental table; a swap makes the cheapest row look
    /// 13x cheaper than it is.
    #[test]
    fn hourly_rate_and_run_total_are_both_shown_and_not_swapped() {
        let row = format_row("vast", "h100 sxm", 1, 46_800.0, 27.16, 2.09);
        let rate_at = row.find("$2.09").expect("hourly rate missing");
        let total_at = row.find("$27.16").expect("run total missing");
        assert!(rate_at < total_at, "rate must precede total in {row:?}");
    }

    /// MUTATION CAUGHT: dropping the free local row, or dividing by its $0.00 rate.
    /// The local row is the entire point of the comparison under this plan's entry
    /// gate -- "$0 and 2h10m here" vs "$27 and 13h there" is the decision, and a NaN
    /// or a panic on the zero-price row removes it from the table silently.
    #[test]
    fn the_free_local_row_renders_without_dividing_by_zero() {
        let row = format_row("local", "apple m5 max", 1, 1_126_800.0, 0.0, 0.0);
        assert!(row.contains("$0.00"), "expected a free row in {row:?}");
        assert!(row.contains("313.0 h"), "expected 313 h in {row:?}");
        assert!(!row.contains("NaN") && !row.contains("inf"), "bad math in {row:?}");
    }
}
```

- [ ] **Step 3: Run to verify failure**

```
timeout 900s cargo test -p vox-ml-cli --features cloud --lib cloud_estimate
```

Expected: **compile error**, `error[E0425]: cannot find function 'format_row' in this scope` (three times, once per test).

Note the feature flag: `--features cloud`. `--features mens-cloud` is a hard error on this package — `mens-cloud` belongs to `vox-populi`.

- [ ] **Step 4: Implement the formatter**

The padding goes **outside** the `$`, over a pre-formatted amount. Verified by running both variants: the old `${usd_per_hr:>7.2}` renders `"$   2.09"` (fails `contains("$2.09")`); the version below renders `"    $2.09"` (passes, and still column-aligns).

```rust
//! `vox mens cloud-estimate` — rank live GPU offers and print the bill, without
//! provisioning anything.
//!
//! This adds no cost model. `CloudResolver::resolve` already costs every offer
//! through `TimeEstimator`; this command prints that ranking instead of
//! dispatching the top row. It exists because the plan's default is "run it
//! locally" and you should be able to price the alternative for free.

use anyhow::Result;

/// One row of the estimate table.
///
/// `secs` is the estimated wall time for the whole run; `total_usd` is what the
/// run costs end to end; `usd_per_hr` is the offer's rate. All three are printed
/// because the operator is choosing between "free and slower here" and
/// "metered and faster there".
///
/// Amounts are formatted first and padded second: `format!("${x:>7.2}")` pads
/// between the sigil and the digits, producing `"$   2.09"`, which is both ugly
/// and unsearchable.
#[must_use]
pub fn format_row(
    provider: &str,
    gpu: &str,
    gpu_count: u32,
    secs: f64,
    total_usd: f64,
    usd_per_hr: f64,
) -> String {
    format!(
        "{provider:<8} {gpu:<16} x{gpu_count}  {:>9}/hr  {:>7.1} h  {:>10}",
        format!("${usd_per_hr:.2}"),
        secs / 3600.0,
        format!("${total_usd:.2}"),
    )
}
```

- [ ] **Step 5: Wire the command, and fail closed on an uncalibrated lane**

Add to `PopuliMensTail` in `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs`:

```rust
    /// Rank live cloud GPU offers for a training run and print the estimated bill.
    ///
    /// Read-only: resolves and ranks, never provisions. Requires VOX_VAST_API_KEY
    /// and/or VOX_RUNPOD_API_KEY for the rented rows; the local row needs neither.
    #[cfg(feature = "cloud")]
    #[command(name = "cloud-estimate")]
    CloudEstimate {
        /// Path to the model directory (read for layers / hidden / artifact bytes).
        #[arg(long)]
        model_dir: std::path::PathBuf,
        /// Provider to query: auto, vast, runpod, local.
        #[arg(long, default_value = "auto")]
        target: String,
        /// Refuse to list offers above this total run cost.
        #[arg(long, default_value_t = 100.0)]
        max_budget: f64,
    },
```

and the matching arm in `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` alongside the existing `PopuliMensTail::` arms (`:634-690`).

**`plan_for` is infallible — match the verdict, do not `?` it.** Every rented offer here is CUDA, and the memory model is fitted from **MLX** points only. An uncalibrated lane must bail with the reason rather than silently reusing the MLX constant:

```rust
    let shape = ModelShape::from_model_dir(&model_dir)?;
    let models = MemoryModels::load_default()?;
    let key = CalKey { lane: Lane::CandleCuda, gradient_checkpointing: true };
    let plan = plan_for(&budget, &models, &key, &shape, &Request { batch_size, seq_len });

    if let Verdict::Uncalibrated { lane } = &plan.verdict {
        anyhow::bail!(
            "cannot size a {lane} run: the memory model has no measured constant for \
             this lane. It is fitted from MLX measurements only; borrowing that number \
             for CUDA would produce a confident wrong estimate. Measure a CUDA point \
             first (see the memory-SSOT plan) or pass an explicit --min-vram-mb."
        );
    }

    // Only a u64 crosses the seam into the resolver (see Interface boundary).
    let min_vram_mb = plan.predicted_bytes.div_ceil(1_048_576);
```

> **Unit note, stated rather than assumed.** `GpuOffer.vram_mb` is populated from Vast's `gpu_ram` and RunPod's memory field, both of which are MiB in practice despite the field being named MB. `div_ceil(1_048_576)` matches that. If the memory-SSOT plan later documents a different convention, this is the one line to change.

The rest of `run` calls `CloudResolver::resolve`, prints `format_row` per survivor, then prints each `(offer_id, reason)` from the second element of the tuple using `UnsuitableReason`'s `Display`. **Always print the local row first** — the point of the command is the comparison, not the cheapest rental.

- [ ] **Step 6: Run, verify end to end, commit**

```
timeout 900s cargo test -p vox-ml-cli --features cloud --lib cloud_estimate
```
Expected: `3 passed; 0 failed`.

```
timeout 1800s cargo run -p vox-ml-cli --features cloud -- mens cloud-estimate --model-dir <path> --target local
```
Expected: the local row prints with `$0.00/hr`. Without `VOX_VAST_API_KEY` / `VOX_RUNPOD_API_KEY` the rented rows are absent — that is the resolver's existing behaviour (`resolver.rs:105-112` bails naming both env vars when no provider is available).

```
timeout 1800s cargo run -p vox-cli -- ci ssot-drift
cargo fmt -p vox-ml-cli
```

```bash
git add crates/vox-ml-cli/src/commands/mens/cloud_estimate.rs \
        crates/vox-ml-cli/src/commands/mens/mod.rs \
        crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs \
        crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs \
        contracts/operations/catalog.v1.yaml contracts/cli/command-registry.yaml \
        docs/src/reference/cli.md docs/src/reference/cli-command-surface.generated.md
git commit -m "$(cat <<'EOF'
feat(mens): vox mens cloud-estimate — price the alternative without buying it

Training is local by default. `vox mens train --cloud` resolves, ranks, and
dispatches in one step, so the only way to see what renting would cost was to
rent. cloud-estimate is the same resolve, printed, with the free local row
first -- the comparison is the point.

It adds no cost model (ResolvedOffer.estimated_cost_usd already exists) and
prints the rejection reasons alongside, so an empty board reads as a fit
problem rather than a capacity problem.

Fails closed when asked to size a CUDA run: plan_for returns
Verdict::Uncalibrated for a lane with no measured constant, and the memory
model is fitted from MLX points only. Borrowing that constant would produce a
confident wrong number.

Registered in contracts/operations/catalog.v1.yaml (feature_gate: cloud) with
the registry and generated command surface regenerated, and the doc-parity
mention added to docs/src/reference/cli.md.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Record the platform survey with per-row provenance

- [ ] **Step 0: Read the Executor Preamble at the top of this file.**

**Files:**
- Create: `docs/src/architecture/mens-cloud-training-economics-2026-09-11.md`

**Do not** add a row to `docs/src/architecture/research-index.md` — that file was retired 2026-09-06 and `AGENTS.md` forbids recreating it. Discoverability comes from frontmatter via the Starlight sidebar.

- [ ] **Step 1: Write the doc**

Frontmatter (`category` from `crates/vox-doc-pipeline/src/pipeline/lint.rs:20-34`, `status` from `:37-45`; **do not** add `last_updated` — the pipeline derives it and a hand-added one is a hard lint error at `:435-446`). `training_eligible: false` is a real key (`lint.rs:428`) and is inert at `false`:

```yaml
---
title: "MENS Cloud Training Economics (surveyed 2026-09-11)"
description: "When renting a GPU beats the 128 GB local machine for a 27-32B QLoRA, what it costs across twelve providers with per-row provenance, and the gotchas that dominate the bill."
category: "Architecture SSOTs"
status: "research"
training_eligible: false
---
```

Body sections, in order:

**1. The default is local.** Lead with the measured baseline from this plan's header (41.825 GB peak on a 107.52 GiB working set, 38.96 tok/s, ~2 h 10 m, val loss 2.974 → 0.403, $0, zero OOM) and the four trigger conditions. State plainly: **the cloud's product here is width, not depth.** Six arms at ~15 min for ~$3 beats 13 h serialized; one run 2× faster does not beat free.

**2. The shape of the decision.** A 27–32 B QLoRA trains on **one** GPU; the only all-reduce would be over LoRA parameters. Every 8-GPU-node-only offering is structurally wrong. This is the rule Task 1 encodes as `UnsuitableReason::MultiGpuNode`.

**3. The survey table.** Every row carries provider, SKU, on-demand $/hr, spot $/hr, billing granularity, source URL, and a confidence flag: **V** = provider's own pricing page or official API; **S** = third-party/secondary; **U** = unverified (page unreachable or figure not published). All figures fetched **2026-09-11**.

Single-GPU-capable, VRAM ≥ 80 GB, sorted by on-demand rate:

| Provider | SKU (VRAM) | On-demand $/hr | Spot $/hr | Billing | Conf. |
|---|---|---|---|---|---|
| Vast.ai | RTX PRO 6000 Max-Q (96 GB) | from $0.92, median **$1.55** | not published (**U**) | per-second | V |
| Vast.ai | RTX PRO 6000 WS (96 GB) | from $0.96, median **$1.39** | not published (**U**) | per-second | V |
| RunPod | RTX PRO 6000 | Community **$1.69** / Secure $2.09 | not published (**U**) | per-second | V |
| Vast.ai | H100 SXM (80 GB) | from $1.73, median **$2.09** | not published (**U**) | per-second | V |
| Hyperstack | RTX Pro 6000 SE (96 GB) | $1.85 | **$1.48** | per-minute | V |
| Vast.ai | H200 (141 GB) | from $1.97, median $4.61 | not published (**U**) | per-second | V |
| Voltage Park | H100 (Ethernet) | from **$1.99** | not offered | hourly | V |
| Vast.ai | A100 SXM4 (80 GB) | from $0.34, median $0.93 | not published (**U**) | per-second | V |
| Lambda | GH200 (96 GB) | $2.29 | none offered | per-minute | V |
| Prime Intellect | H100 (80 GB) | $2.43 | $0.94 | unverified | V (see caveat) |
| RunPod | H100 SXM (80 GB) | Community **$2.69** / Secure $3.49 | not published (**U**) | per-second | V |
| Hyperstack | H100 SXM | $3.20 | not offered on SXM | per-minute | V |
| Verda (ex-DataCrunch) | H100 SXM5 (80 GB) | $3.25 | **$1.63** | unverified | V |
| Nebius | H100 | $3.85 | **$2.15** | unverified | V |
| Together AI | HGX H100 | $3.99 | **preemptible $1.99** | hourly | V |
| Lambda | H100 SXM (80 GB) | $4.29 | none offered | per-minute | V |
| Google Cloud | a2-ultragpu-1g (1× A100 80 GB) | $5.0688 | $2.9279 | unverified | V |
| AWS | **p5.4xlarge (1× H100 80 GB)** | $6.88 (us-east-1) | $2.63 | unverified | V / **S** (spot) |

Node-granular only — listed so the arithmetic is on the record, not as candidates:

| Provider | SKU | $/hr (node) | $/hr per GPU | Single GPU? | Conf. |
|---|---|---|---|---|---|
| CoreWeave | HGX H100 (8×) | $49.24 | $6.16 | **No** | V |
| Google Cloud | a3-highgpu-8g (8× H100) | $88.49 (spot $50.46) | $11.06 | **No** | V |
| Azure | ND96isr H100 v5 (8×) | $98.32 (spot $18.17) | $12.29 | **No** | V |

> Renting an Azure ND96isr to run one QLoRA is **~$236 for a 13-hour run at the *spot* price** and ~$1,278 on demand, against ~$18–27 for the same silicon-class on a single-GPU provider. GCP's a3-highgpu-8g spot is worse still (~$656). These are not close calls.

**4. Throughput — what is measured and what is inferred.** Be explicit about the gap:

| Device | Memory bandwidth | Ratio vs M5 Max | Conf. |
|---|---|---|---|
| M5 Max 128 GB (40-core) | **614 GB/s** | 1.00× | **VERIFIED** (Apple tech specs) |
| RTX PRO 6000 Blackwell | **1,792 GB/s** | 2.92× | VERIFIED |
| H100 SXM | **3.35 TB/s** | 5.46× | VERIFIED |
| H200 | **4.8 TB/s** | 7.82× | VERIFIED |

One clean measured LoRA fine-tuning datapoint exists: **Llama-3.1-8B on 1× RTX PRO 6000 Server Edition = 4,962 tok/s** (Exxact, 2026-06-04, **VERIFIED**). Scaling to ~27 B by parameter count gives ≈1,500 tok/s against the measured 38–56 tok/s local — **≈27–39×, but EXTRAPOLATED across a different model family, framework and quantization.**

**State the honest range as 10–30×, inferred. No direct 27B-class benchmark exists on either side of the comparison.** A bandwidth ratio is not a throughput ratio, and published fine-tuning throughput numbers are unusually dirty — one widely-cited 46,000 tok/s headline figure turned out to have **zero gradient flow**.

**5. Gotchas, ranked by likelihood of biting a 2–6 h single-GPU LoRA run.** This section, not the $/hr table, is where the money actually goes.

1. **Egress can dominate the bill, and it is not in the sort key.** vast.ai egress is **host-set per GB**, rates ranging $0.00–$0.10/GB. A published itemized vast.ai bill: GPU 0.92 h × $0.27 = **$0.25**; download 106 GB × $0.04/GB = **$4.14** — **16× the compute cost**. Listings sort by $/hr, which excludes the dominant term. (**VERIFIED**, vast.ai billing documentation / itemized invoice, fetched 2026-09-11.)
2. **Storage bills until you DELETE, not STOP.** RunPod's volume rate *doubles* when a pod is stopped: $0.10 → **$0.20/GB/mo**. (**VERIFIED**, RunPod pricing page, 2026-09-11.)
3. **A stopped RunPod pod frequently cannot restart** — the GPU gets rented to someone else while you are stopped. RunPod maintains a dedicated troubleshooting page for this failure mode. (**VERIFIED**, RunPod docs, 2026-09-11.)
4. **No usable grace period on reclaim.** vast.ai documents none. RunPod's "5-second SIGTERM" is **REPORTED** only (third-party blog; not in current RunPod primary docs). Five seconds cannot flush a 27 B checkpoint. Checkpoint on a **fixed iteration cadence**, not on a termination signal.
5. **Data posture — the constraint that decides the provider.** vast.ai's own security FAQ states *"Provider security varies significantly"* and recommends restricting to certified providers (**VERIFIED**, vast.ai security FAQ, 2026-09-11). Container isolation defends against other renters; it does **not** defend against a host with physical root. **The training corpus is the user's proprietary codebase.** That is the deciding fact, not the $/hr column.

**6. Uptime SLAs — three providers publish one, contrary to the earlier draft.**

| Provider | Numeric SLA | Credits | Conf. |
|---|---|---|---|
| **Voltage Park** | **≥99.5 % per VM** | 10 % (95–99.5 %), 25 % (90–95 %), **100 % (<90 %)**, 30-day claim window | **VERIFIED** — the most concrete self-serve GPU SLA found |
| **Nebius** | **99.50 %** Compute Cloud | graduated 10–30 % compensation | **VERIFIED** |
| **Hyperstack** | published but **self-contradictory** — the same document states 99.5 % in one section and 100.0 % in another | — | **VERIFIED as unreliable** — do not budget against it |
| **vast.ai** | **NONE, explicitly disclaimed.** ToS: *"Company cannot guarantee the Website and that Company Services will be always available."* No credits, no uptime number, no tier that adds one; liability capped at 3 months of fees. "Secure Cloud" on vast.ai is a host-selection **filter**, not a contractual product. | none | **VERIFIED** |
| **RunPod** | **not published for self-serve.** ToS carves out explicitly: *"Runpod does not make any specific uptime warranties with respect to the Community Cloud Offerings."* The 99.99 % figure appears only on the enterprise sales page with no linked SLA document. | none self-serve | **VERIFIED** |
| **Lambda** | **none** — ToS is AS-IS. Third-party "99.9 %" claims are contradicted by it. | none | **VERIFIED** |
| **Prime Intellect** | **none**, stated outright | none | **VERIFIED** |
| **CoreWeave** | object storage only; **not compute** | n/a | **VERIFIED** |

All fetched 2026-09-11 from each provider's own ToS / SLA / pricing pages.

**7. Recommendation.**

- **Default: do not rent.** Run locally. 41.8 GB peak on a 107.52 GiB working set at $0 is not a problem looking for a purchase.
- **When a trigger fires: RunPod Secure Cloud, on-demand.** Not vast.ai, not spot. You are buying a **trust boundary and predictable egress**, not the lowest $/hr — and at single-digit dollars per run, the marketplace discount is noise against a proprietary-corpus exposure. RTX PRO 6000 at **$2.09/hr** Secure, or H100 SXM at $3.49.
- **Always `terminate`, never `stop`** (gotchas 2 and 3). Checkpoint on a fixed iteration cadence regardless of lane (gotcha 4).
- **If the marketplace is acceptable for a specific run** (public corpus only): Vast.ai 1× RTX PRO 6000 Blackwell 96 GB, median $1.39–1.55/hr, per-second billing — but **price the egress before you sort by $/hr** (gotcha 1).
- **If H100 specifically is required from a first-party operator: Voltage Park at $1.99/hr** — cheapest verified single-H100, and the only one in this survey with a real credit ladder behind a numeric SLA.
- **Budget $20–35 per run** in the sane tier; with egress and one restart in three, plan **~$45/run all-in**.
- **Spot is not worth it at this job size.** The discount is ~$10–15 on a ~$30 run, against a bid market with no published grace period and a documented restart failure mode.

**8. Rejected, with the reason.**

- **GCP A3, Azure ND H100 v5, CoreWeave** — no shape smaller than 8 GPUs. Structurally wrong; the arithmetic is in §3.
- **AWS p5.4xlarge** — a genuine single-H100 shape, but $6.88/hr on demand is 3–5× the neoclouds. Its spot price ($2.63, **SECONDARY**) is the only hyperscaler number in range.
- **GCP a2-ultragpu-1g** — the one GCP single-GPU escape hatch. Workable, but Ampere-slow and right at the VRAM floor at $5.07/hr.
- **RTX 5090 (32 GB) and RTX 4090 (24 GB)** — cheapest per hour on the board and **cannot hold the working set**. They appear only so the filter's `InsufficientVram` rejection has something to reject.

**9. Stated gaps — do not budget against these.**

- **Vast.ai interruptible $/hr: UNVERIFIED.** Per-host bid market, no published rate; several `docs.vast.ai` interruptible pages 404'd.
- **Vast.ai storage and bandwidth $/GB: UNVERIFIED as a single number** — host-set, $0.00–$0.10/GB observed. Part of the bill cannot be priced before choosing a host. See gotcha 1.
- **RunPod spot rate: UNVERIFIED.** No figure published anywhere reached.
- **Billing granularity** for GCP, AWS, Azure, Nebius, CoreWeave, Verda, Voltage Park and Prime Intellect was not confirmed from a fetched page.
- **Prime Intellect** — the homepage marketplace widget repeats `$3.14/HR` across mismatched GPU/VRAM labels. Treat everything except its H100 row as UNVERIFIED.
- **GCP region** — figures are the page's default region as rendered; the region selector could not be read back.
- **No 27B-class fine-tuning throughput benchmark exists** on either the local or the rented side. The 10–30× figure in §4 is inferred, not measured.
- **This whole table is a snapshot dated 2026-09-11.** The survey date is in the filename and the title precisely so a stale row is visible rather than silently trusted. The code holds no prices; `cloud-estimate` reads live rates from the provider APIs.

**10. Link back to the code.** The suitability rules in §2 and §8 are executable at `crates/vox-populi/src/mens/cloud/offer_filter.rs`, not prose. The command that prints the live version of this table — local row first — is `vox mens cloud-estimate`.

- [ ] **Step 2: Lint and commit**

```
timeout 900s cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/mens-cloud-training-economics-2026-09-11.md
```
Expected: no errors. A failure naming `MissingCategory` or `UnknownStatus` means the frontmatter block was dropped or `status` is outside the seven-value vocabulary.

```bash
git add docs/src/architecture/mens-cloud-training-economics-2026-09-11.md
git commit -m "$(cat <<'EOF'
docs(mens): cloud training economics, surveyed 2026-09-11

Local is the default: 41.8 GB peak on a 107.52 GiB working set, 38.96 tok/s,
~2h10m, $0, zero OOM. This doc records when renting beats that, and it is a
narrower set than the $/hr tables suggest -- capacity wall, >=4 concurrent
runs, a single run over 12h, or a sequence length that breaches the 80.64 GiB
single-buffer cap. The cloud's product here is width, not depth.

Twelve providers priced, every row carrying a source URL and a V/S/U flag.
Corrects the earlier claim that no provider publishes a numeric uptime SLA:
Voltage Park (>=99.5% with a 10/25/100% credit ladder) and Nebius (99.50%) do,
Hyperstack publishes a self-contradictory one, and vast.ai and RunPod both
disclaim uptime in their ToS in so many words.

The gotchas section, not the rate table, is where the money goes: vast.ai
egress is host-set per GB and one published invoice shows $4.14 of download
against $0.25 of GPU. Storage bills until DELETE, not STOP, and RunPod's
volume rate doubles while stopped. A stopped pod frequently cannot restart.

Recommendation is RunPod Secure Cloud on-demand when a trigger fires -- buying
a trust boundary, not the lowest rate, because the corpus is a proprietary
codebase and container isolation does not defend against a host with root.

No prices enter the code. GpuOffer.price_per_hour_usd is documented as live-
from-API, and `vox mens cloud-estimate` reads the live rates.

Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>
EOF
)"
```

---

## Self-review

**The reframe.** Cloud is a gated contingency with four named triggers, not a peer to local. The measured local baseline is in the header, the entry gate is in the header, and Task 3's command prints the local row first. Tasks 1–3 still ship because they are how you *decide* cheaply — not because a purchase is planned.

**Spec coverage.** "Expansion to vast.ai/RunPod/Google Cloud" → Task 4 covers all three plus nine others. "Most appropriate currently available hardware" → the survey table, provenance-flagged, with the honest inferred-throughput caveat. "Cost and suitability across plans" → Tasks 1, 3, 4. "Uptime needed" → §6, corrected: three providers publish numeric SLAs. "Most appropriate config under the new VRAM limits" → `plan_for` feeds `min_vram_mb`, enforced per-GPU (Task 1). "Estimate costs before spending" → Task 3.

**Decisions made here so the executor does not have to make them.**
- `resolve` returns the rejection vec (Task 2) — otherwise Task 2's reporting test is green over a feature the CLI cannot reach.
- `StalePrice` deleted (Task 1) — unreachable in production; `is_stale` stays uncalled and that is stated.
- `Display` kept — Task 3 prints it to the operator. Cut Task 3, cut `Display`.
- One shared `test_offer()`, values unchanged, so the five existing `pipeline_dispatch` tests are an unmodified regression check on the move.

**Test quality.** Ten tests, each annotated with the single mutation it catches. The two that matter most: `vram_is_checked_per_gpu_not_per_node` (catches the total-vs-per-device confusion that `GpuOffer.vram_mb`'s documented "Total VRAM in MB across all GPUs" invites — nothing in the tree divides it, so an 8×H100 node sails through a naive check) and `ranking_drops_a_paid_multi_gpu_node_and_keeps_the_single_gpu_offer` (catches a filter that exists but is never called).

**Gates handled.** Catalog → registry → generated surface → `cli.md` doc parity (Task 3 Step 1, before any code, with `timeout 1800s` on all three workspace builds). Doc frontmatter with a valid `category` and `status`, no hand-added `last_updated` (Task 4). Every new `pub fn` has a same-file `#[test]` so `tdd-guard` does not block the commit. No new contract file, so no `contracts/index.yaml` row.

**Known gaps, stated.** Spot/interruptible rates for Vast and RunPod are unpublished; the recommendation does not depend on them. There is no measured CUDA `a_lane`, so `cloud-estimate` fails closed. No 27B-class throughput benchmark exists on either side; the 10–30× figure is inferred. Provider pricing is a 2026-09-11 snapshot; the code holds no prices.

---

## Verification log

Every command was run against the read-only checkout `/Users/brbrainerd/dev/vox-audit2` at `origin/main` **7295b4470** (`git log -1 --format=%H` → `7295b4470e322bbe02fe13f2f72492bc0975e411`).

| Claim | Command | Result |
|---|---|---|
| **`format_row`'s old implementation fails 3/3 of its own tests** | compiled and ran both format strings with `rustc -O` | old: `"vast     h100 sxm         x1  $   2.09/hr     13.0 h  $   27.16"` → `contains("$2.09")` **false**, `contains("$27.16")` **false**, so test 2's `.find(...).expect(...)` **panics**. New (pad outside the `$`): `"…      $2.09/hr     13.0 h      $27.16"` → both `true`, `"13.0 h"` true, order 34 < 59. Local row: `$0.00`, `313.0 h`, no NaN/inf |
| `price_per_hour_usd` doc + field line numbers | `grep -n "price_per_hour_usd" crates/vox-populi/src/mens/cloud/mod.rs` | field at **`:252`**, doc at `:251` — **the old plan's `:255-256` was wrong by 4** |
| `reliability_pct` doc + field line numbers | same grep | field at **`:256`**, doc at `:255` — old plan's `:257-258` wrong by 2 |
| `GpuOffer` at `:239`; "Total VRAM in MB across all GPUs" at `:249`, `vram_mb` at `:250`; `is_stale` at `:273` | `sed -n '236,280p' .../mod.rs` | all confirmed |
| `CloudProviderConfig` `:128`; `min_reliability` `:132` (default 0.90 at `:190`); `price_cache_ttl_secs` `:136` (default at `:192`) | `grep -n` on `mod.rs` | confirmed |
| `pub mod estimator;` at `:29`, `pub use resolver::{…}` at `:44` | `grep -n "^pub mod\|^pub use" .../mod.rs` | confirmed |
| **`StalePrice` is unreachable in production** | `grep -n "let ttl = Duration::from_secs"` + `grep -n "fetched_at: Some"` on all three providers | TTL gate at `vast.rs:158`, `runpod_provider.rs:173`; `fetched_at = Some(now)` at `vast.rs:209`, `runpod_provider.rs:131`, `local_provider.rs:46`. Cache is returned only while `elapsed() < ttl`, and the offers in it carry that same `now`. Confirmed unreachable |
| Vast filters reliability provider-side; RunPod does not | `sed -n '155,215p' .../vast.rs`; `sed -n '120,180p' .../runpod_provider.rs` | `vast.rs:195-198` filters on `min_reliability` + `min_cuda_version`; RunPod sets `reliability_pct` from the config default at `:129` and filters nothing. (Old plan cited `vast.rs:194-197` — off by one) |
| `LocalProvider`: `price_per_hour_usd: 0.0`, real `gpu_count`, `reliability_pct: 100.0` | `sed -n '35,50p' .../local_provider.rs` | confirmed at `:40-46` |
| `effective_preset` `:29`, `min_vram_mb` `:35`, `preset_for_vram` `:58`, its **only** call site `:250`, `min_vram_mb: 24000` `:285` | `grep -n "effective_preset\|preset_for_vram\|min_vram_mb" .../resolver.rs` | exactly as stated; `preset_for_vram` is called once, at `:250` |
| Ranking block `:227` (`let mut ranked`) through `Ok(ranked)` `:273`; no gpu_count or reliability gate | `sed -n '225,275p' .../resolver.rs` | confirmed — a cost gate and a sort tiebreaker, nothing else |
| **`CloudResolver::resolve` has exactly two call sites** | `rg -n "\.resolve\(" --glob '*.rs' crates/`, filtered to `CloudResolver` | `crates/vox-speech/src/backends/cloud_offload.rs:58` and `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs:170`. Both bind `let ranked = …` then pass `&ranked` to `dispatch_top` — one line each |
| **`plan_for` is infallible, five args, no `MemoryBudget`** | read `2026-09-11-1-memory-ssot-and-fit-benchmark.md:595`, `:699-732` | `plan_for(&AccelBudget, &MemoryModels, &CalKey, &ModelShape, &Request) -> TrainPlan`; `Verdict::Uncalibrated { lane: String }` returned for an unknown lane; `TrainPlan.predicted_bytes: u64`. `MemoryBudget` appears nowhere. Lives in `crates/vox-populi/src/mens/tensor/memory_model.rs` |
| `Lane`, `plan_for` do **not** exist at main | `rg -n "enum Lane\|fn plan_for" --glob '*.rs' crates/` | only `vox-skill-runtime::plan_for_min_tier` (unrelated) |
| **`training_eligible` IS a known frontmatter key** | `rg -n 'training_eligible' crates/vox-doc-pipeline/src/` | parsed at `lint.rs:428`; only `true` on a research/roadmap page requires `training_rationale` (`lint.rs:458`, `pipeline/mod.rs:350`). **Kept `false`** — the fix list's condition resolved in favour of keeping it |
| `VALID_STATUS` has no `accepted`; `VALID_CATEGORIES` includes `Architecture SSOTs` | `sed -n '20,46p' crates/vox-doc-pipeline/src/pipeline/lint.rs` | `approved, current, experimental, legacy, research, roadmap, deprecated`; categories confirmed |
| Existing `test_offer()` fixture and its five users | `grep -n "test_offer" .../pipeline_dispatch.rs` | defined `:593`, used at `:716`, `:736`, `:909`, `:1089`, `:1125`. (Old plan cited `:604`) |
| Catalog rows sorted by `id`; `mens.check` 7564, `mens.corpus` 7591 | `grep -n "^- id: mens\." contracts/operations/catalog.v1.yaml` | confirmed; `mens.cloud-estimate` sorts between them |
| Doc-parity list at `cli.md:673` | `grep -n "vox mens eval-gate" docs/src/reference/cli.md` | `:673` |
| `cargo test` accepts exactly one positional filter | `cargo test -p vox-populi --features mens-cloud --lib aaa bbb` | `error: unexpected argument 'bbb' found` |
| `vox-ml-cli` feature is `cloud`, not `mens-cloud` | `sed -n '/^\[features\]/,/^\[/p' crates/vox-ml-cli/Cargo.toml` | `cloud = ["gpu", "vox-populi/mens-cloud"]`; no `mens-cloud` |
| `research-index.md` is gone and must not be recreated | `find docs -name "research-index*"`; `rg -n "research-index" AGENTS.md` | only the archived copies; `AGENTS.md` forbids recreating it |

**Could not verify:** (1) Everything in Task 4 — all provider pricing, SLA text, gotcha figures and throughput numbers are external web claims fetched 2026-09-11, marked VERIFIED / REPORTED / UNVERIFIED per item, and are **not** checkable against this repository. (2) The exact runtime unit of `GpuOffer.vram_mb` (MB vs MiB) at the provider boundary — Task 3 Step 5 states the `div_ceil(1_048_576)` assumption and names the one line to change. (3) The memory-SSOT plan's symbols, which do not exist at `7295b4470` and are declared as a hard dependency.
