# Phase 316: Before/After Comparison - Research

**Researched:** 2026-10-26
**Domain:** Rust, `vox-ml-cli` reasoning scorer reuse, Hugging Face cache layout detection, a new non-gate report
**Confidence:** HIGH for code seams and detection design (every value below was read from source this session). No model was loaded or run. No weights were downloaded.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Absent snapshot on this checkout
- This checkout records that the snapshot is absent and writes no accuracy and no think-initiation.
- Do not score Qwen3-0.6B, or any other model, as a stand-in for Qwen3-8B.
- Do not download weights as part of this phase.

#### Both branches exist in code
- If the pinned snapshot and an adapter are on disk, score the three checked-in slices (`gsm8k_think`, `math500_think`, `vox_think`) on both sides.
- If either is missing, write the absence record instead.
- Tests for the present branch do not call a model and do not invent numbers.
- The run on this checkout takes the absent branch.

#### Absence record shape
- The absence record names `Qwen/Qwen3-8B` and says the snapshot is not on disk.
- It has no `accuracy` key and no `think_initiation_rate` key.
- It is not a collateral pass and is not a serve-gate input.

#### Grader, golds, and token cap stay as they are
- Do not change the grader or the slice golds.
- Do not add a token-cap field to the report.
- The deferred token-cap marker (review WR-02 from phase 315), the unclosed-think marker, and MATH-500 equivalent surface forms stay deferred. This phase does not produce a comparison to attach them to.

### Claude's Discretion
- Where the absence record lives on disk and what its file is called, as long as it cannot be mistaken for `collateral_damage_report.json` or a per-slice `reasoning_report.json` that serve reads.
- How "snapshot on disk" is detected (Hugging Face cache layout, an explicit path, or both), as long as detection never loads or runs the model and the pinned revision is the one checked.
- Whether the present branch reuses the phase 314 scorer and the phase 315 per-slice report shape directly, as long as both sides use the same checked-in items and the same keys.

### Deferred Ideas (OUT OF SCOPE)
- Pick a hub and its spokes together from available VRAM. Today spoke training bases resolve through `pick_base` on `train_bases.agentic_default` in `mens/config/gpu-specs.yaml`, but every spoke hard-codes `preset: qwen3_16g`, and the hub model has no VRAM entry (the `hub:` block in `mens/config/domain-profiles.yaml` lists only the embedder). This needs its own phase. Note that the `Qwen/Qwen3.8-27B` rung has an unmeasured floor and no license entry.
- Store the token cap on the report (review WR-02 from phase 315).
- A report marker for an unclosed think block.
- MATH-500 equivalent surface forms.
- Swapping the pinned Qwen3-8B base, or trying a new training technique.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BENCH-01 | Base and adapter are scored on the same slices when the weights exist, and no score is invented when they do not. (Success criteria: one report holds accuracy and think-initiation for the GSM8K, MATH-500 and Vox slices on both sides when the pinned snapshot and an adapter are on disk; when the snapshot is absent the run records that and writes no numeric scores; both sides use the checked-in items.) | A new `vox mens` subcommand over a model-free detection step plus an injectable per-slice scorer. Present branch loads `load_slice` once and hands the same `Vec<ReasoningItem>` to both sides. Absent branch writes a key-free record. See Patterns 1-4, the Validation Architecture, and Pitfalls 1-6. |
</phase_requirements>

## Summary

Everything the present branch needs already exists except three things: (1) a pinned-revision snapshot detector (the existing helpers ignore the pin), (2) a scorer entry point that does **not** write into the model directory (the phase 314 `score_reasoning_file` writes `collateral_eval/<bench>/reasoning_report.json` *inside* the directory it is given, which is exactly what serve reads, and inside an HF cache snapshot it would write through symlinks), and (3) the report itself and its CLI entry. The grading and prompt path (`score_loaded`) is already a model-free seam: it takes an `infer: &mut dyn FnMut(&str) -> Result<String>` closure, and 12 existing tests drive it without a model.

The base side needs no new loader. The candle plugin's `InferenceEngine::load` treats a directory with neither `candle_qlora_adapter.safetensors` nor `adapter_manifest.json` as a base-only snapshot and globs its own `model*.safetensors` shards. So the same `load_model(<dir>)` call serves both sides: pass the pinned snapshot directory for "before" and the adapter run directory for "after".

The adapter side has two traps. The adapter's `adapter_manifest.json` `base_model` must be a **local directory** (a Hub id makes the plugin bail), and nothing today checks that this directory is the pinned snapshot, so a Qwen3-0.6B adapter could be "compared" against a Qwen3-8B base. Detection must canonicalize `base_model` and compare it to the pinned snapshot path. Also, no host code ever calls `MlBackend::unload_model`, so scoring base then adapter in one process would hold two 8B models at once unless the new code unloads the first handle.

**Primary recommendation:** One plan (`316-01`), test-first, three tasks: (1) pure report shapes plus absence record in `vox-eval` (fast tests, no `gpu` feature); (2) model-free detection plus the injectable `run_before_after_with(...)` orchestrator and a no-write scorer entry in `vox-ml-cli`; (3) the `vox mens eval-reasoning-compare` subcommand, a docs line, and one real run on this checkout that takes the absent branch and writes the record under `mens/runs/v1/`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Absence record and before/after JSON shape (no model, no FS) | `vox-eval` `reasoning_bench.rs` (sibling of `BenchOutcome::to_json`) | — | Same place the not-scored shape lives; tests run with `cargo test -p vox-eval --lib` in seconds. |
| Pinned snapshot detection (HF cache layout, completeness, revision) | `vox-ml-cli` new `reasoning_compare.rs` | — | Needs `DEFAULT_MODEL_ID` (`vox_populi`) and the filesystem; `vox-ml-cli` already depends on both. No new crate edge. |
| Adapter detection and base-pin check | `vox-ml-cli` `reasoning_compare.rs` | — | Reads two file names and one JSON string; never loads. |
| Scoring both sides | `vox-ml-cli` `reasoning_eval.rs` (`score_loaded`, `run_backend`) | `reasoning_compare.rs` (orchestration) | Reuse grader and prompt unchanged; add one no-write entry point. |
| CLI entry | `vox-ml-cli` `populi/mens_tail_subcommands.rs` + `populi/dispatch.rs` | — | Same tail enum as `eval-collateral-damage`; gated `#[cfg(feature = "gpu")]`. |
| Serve gate | `populi/dispatch.rs` `assert_collateral_gate` | — | **Unchanged.** It never reads the new file. |

## Standard Stack

No new dependencies and no new crate edges. `[VERIFIED: crates/vox-ml-cli/Cargo.toml]` `vox-eval`, `vox-populi` (optional, pulled by `gpu`), `dirs`, `serde_json`, `vox-crypto`, `chrono`, and dev-dep `tempfile` are already present.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox_eval::reasoning_bench` | workspace | `load_slice`, `SLICE_SIZE`, `SLICE_NAMES`, `BenchOutcome`, `ReasoningItem` | Phase 314 SSOT for the slices and the not-scored shape |
| `vox_eval::reasoning_slice` | workspace | `REASONING_SLICE_SCOPE_NOTE` | Reuse the 32-item scope text so GATE-02 wording is not forked |
| `vox_populi::mens::DEFAULT_MODEL_ID` | workspace | The pin | The only pin SSOT; do **not** use `default_model_id()` (it honors `VOX_MENS_DEFAULT_MODEL` and would silently change the "pinned" base) |
| `serde_json`, `std::fs`, `dirs` | workspace | Report and detection | Already used by sibling code |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| New pinned detector | `vox_populi::mens::hub::is_model_cached` | **Rejected.** It strips the revision and answers "any cached revision exists" (`repo_name_without_revision` then `r.repo_id == repo_id`). A different Qwen3-8B commit would pass. `[VERIFIED: crates/vox-populi/src/mens/hub.rs:475-497]` |
| New pinned detector | `vox-orchestrator` `resolve_base_model_context_length_internal` | **Rejected.** Private, and it falls back to "any snapshot" when the pinned SHA directory is missing. |
| Reuse `score_reasoning_file` for each side | — | **Rejected.** It writes `collateral_eval/<bench>/reasoning_report.json` under the model dir (Pitfall 1). |
| New subcommand | Fold into `eval-collateral-damage` | Rejected: that command's contract is the serve gate and needs a `--pre-score` baseline; mixing them risks the very confusion the phase must avoid. |

**Version verification:** not applicable, no packages are installed.

## Package Legitimacy Audit

No external packages are installed by this phase. **Packages removed due to [SLOP] verdict:** none. **Packages flagged as suspicious [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```text
vox mens eval-reasoning-compare [--adapter DIR] [--hub-root DIR] [--bench-root mens/data] [--out FILE]
        |
        v
resolve hub root (HF_HUB_CACHE -> HF_HOME/hub -> ~/.cache/huggingface/hub)   (pure env/path, no network)
        |
        v
detect_inputs(hub_root, pin = DEFAULT_MODEL_ID, adapter)          <-- never loads a model
   |-- snapshot: <hub>/models--Qwen--Qwen3-8B/snapshots/<pinned sha>/  complete? (config, tokenizer, every shard in index)
   |-- adapter : candle_qlora_adapter.safetensors AND adapter_manifest.json, base_model == that snapshot
        |
   any missing / incomplete / mismatched?
   |                                   |
  yes                                  no
   |                                   |
   v                                   v
write ABSENCE record             load_slice x3 (once) --> same Vec<ReasoningItem> for both sides
(no score keys, not_compared)           |
   |                                    v
   |                       score_side(base_snapshot_dir)  -> 3 x BenchOutcome   (score_loaded, then unload)
   |                       score_side(adapter_dir)        -> 3 x BenchOutcome   (score_loaded, then unload)
   |                                    |
   |                                    v
   |                       build report: per slice, per side = BenchOutcome::to_json
   |                       status "compared" only if all six Scored, else "incomplete" (+ Err)
   v                                    v
        --out (default mens/runs/v1/reasoning_before_after.json)
        NEVER: collateral_damage_report.json, collateral_eval/**, reasoning_report.json
```

### Recommended Project Structure
```
crates/vox-eval/src/reasoning_bench.rs      # + BeforeAfterReport / AbsenceReason pure JSON builders (+ tests)
crates/vox-ml-cli/src/commands/mens/
├── reasoning_compare.rs                     # NEW: detection, run_before_after_with, no-write scorer, CLI body, tests
├── reasoning_eval.rs                        # minimal edit: expose a no-write, load-once scorer; add unload
└── mod.rs                                   # + #[cfg(feature = "gpu")] pub(crate) mod reasoning_compare;
crates/vox-ml-cli/src/commands/mens/populi/
├── mens_tail_subcommands.rs                 # + EvalReasoningCompare variant
└── dispatch.rs                              # + one match arm (cfg gpu)
```
`reasoning_eval.rs` is 552 lines (God-object Error limit is 500 non-blank lines, `docs/agents/governance.md`), so put new logic in the new file and keep the `reasoning_eval.rs` edit to visibility plus one small function.

### Pattern 1: Model-free pinned snapshot detection
**What:** A pure function of `(hub_root, repo_id_with_pin)` that returns an enum, not a bool, so the record can say *why*.
**When to use:** Before anything is loaded. Takes an explicit `hub_root` so tests build fake trees in a tempdir.

The pin and layout, verified:
- `DEFAULT_MODEL_ID` is `"Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218"` `[VERIFIED: crates/vox-populi/src/mens/mod.rs:46]`; `mens/config/gpu-specs.yaml` carries the same string on its Qwen3-8B rung `[VERIFIED: gpu-specs.yaml:252]`.
- A cached repo is `<hub>/models--<owner>--<name>/snapshots/<commit sha>/`, entries are symlinks into `../../blobs/<hash>`, and `refs/main` holds the sha. `[VERIFIED: ls of ~/.cache/huggingface/hub/models--Qwen--Qwen3-0.6B on this host]` `hub.rs` `download_model` pins by passing `maybe_revision(revision)` so a pinned download lands in `snapshots/<pin sha>`.
- This host has `models--Qwen--Qwen3-0.6B` and no `models--Qwen--Qwen3-8B` directory `[VERIFIED: ls ~/.cache/huggingface/hub]`.
- Hub root order used elsewhere in this repo: `HF_HUB_CACHE`, then `HF_HOME` joined with `hub`, then `~/.cache/huggingface/hub` `[VERIFIED: crates/vox-ml-cli/src/commands/oratio_cmd.rs:816-819; crates/vox-orchestrator/src/catalog.rs:1090-1103]`. `HF_HUB_CACHE`/`HF_HOME` are allowed by the env-var audit detector `[VERIFIED: crates/vox-code-audit/src/detectors/unregistered_llm_env.rs:48-53]`, so no `env-vars.v1.yaml` row is needed. XDG handling of hf-hub 1.0 itself is `[ASSUMED]` not checked; follow the precedent above.

**Detection rule (recommended):**
1. Split the pin on the last `@` (same rule as `hub.rs` `repo_revision`). No `@` or a revision that is not 40 lowercase hex characters: state `PinUnverifiable` and write the absence record. Never fall back to "any revision".
2. `snap = hub/models--Qwen--Qwen3-8B/snapshots/<sha>`. Not a directory: `Absent`.
3. Require `config.json`, `tokenizer.json` (the plugin's `InferenceEngine::load` reads `<dir>/tokenizer.json` first `[VERIFIED: crates/vox-plugin-mens-candle-metal/src/inference.rs:177-180]`), and weights. Weights complete means: if `model.safetensors.index.json` exists, every filename in its `weight_map` values is `is_file()` (follows the symlink, so a dangling link from a partial download is false); otherwise a single `model.safetensors` `is_file()`. Anything less: `Incomplete`.
4. `Present(snap)`. Never open a weight file, never call the plugin.

`Incomplete` and `Absent` both produce the absence record; `Incomplete` is a distinct `reason` string so a half-finished download is not described as "absent".

### Pattern 2: Model-free adapter detection with a base-pin check
**What:** Adapter on disk means the pair the plugin requires. `resolve_weights` treats exactly one of `candle_qlora_adapter.safetensors` / `adapter_manifest.json` as a hard error ("inconsistent on-disk state") `[VERIFIED: crates/vox-plugin-mens-candle-core/src/weight_resolution.rs:114-124]`, so detection requires both as files.
**Base-pin check:** parse `adapter_manifest.json` as `serde_json::Value`, read `base_model` as a string, require it to be an existing directory (the plugin bails on a non-directory `[VERIFIED: weight_resolution.rs:91-101]`; training can write a Hub id there when no local dir exists, `finalize.rs` `resolved_serve_base_model`), canonicalize it and the detected snapshot dir, and require equality. Mismatch, missing key, or a Hub id: absence record with reason `adapter_base_not_pinned_snapshot`. Without this check "before" and "after" could be different models.
**Do not** use the plugin's `PopuliAdapterManifestV3` type (private to the plugin crate family); one JSON key is enough.

### Pattern 3: One injectable seam, mirroring `run_collateral_damage_with`
The 315 writer takes `score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>` precisely so tests need no GPU `[VERIFIED: eval_collateral.rs run_collateral_damage_with]`. Do the same:

```rust
// Source: pattern from crates/vox-ml-cli/src/commands/mens/eval_collateral.rs (run_collateral_damage_with)
pub(crate) enum Side { Base, Adapter }

/// Scores every slice for one side in one model load. Real impl loads `model_dir`
/// once, scores each (bench, items) through `score_loaded`, then unloads.
pub(crate) type SideScorer<'a> =
    &'a mut dyn FnMut(Side, &Path, &[(&str, Vec<ReasoningItem>)]) -> Vec<BenchOutcome>;

pub(crate) fn run_before_after_with(
    inputs: Detected,          // Present{snapshot, adapter} | Absent{reason,...}
    bench_root: &Path,         // "mens/data"
    out: &Path,
    score_side: SideScorer<'_>,
) -> anyhow::Result<RunStatus> { /* absent -> write record, Ok; present -> load_slice x3, call score_side twice, write report */ }
```

Tests inject a fake `score_side` that returns canned `BenchOutcome`s built from `ReasoningScore` literals and records the `(Side, items)` it received. No model, no numbers other than the test's own literals. The real scorer is a thin closure over a no-write variant of `run_backend`.

### Pattern 4: Report shapes
Reuse `BenchOutcome::to_json` verbatim for every per-slice, per-side entry (so the keys are exactly the phase 314/315 keys) and add only a wrapper:

```json
{
  "schema": "reasoning_before_after.v1",
  "status": "compared",
  "scope_note": "<REASONING_SLICE_SCOPE_NOTE>",
  "slice_size": 32,
  "base_model": "Qwen/Qwen3-8B",
  "base_revision": "b968826d9c46dd6066d109eabc6255188de91218",
  "slices": {
    "gsm8k_think":   { "base": { "status": "scored", "n": 32, "accuracy": 0.0, "think_initiation_rate": 0.0 }, "adapter": { } },
    "math500_think": { "base": { }, "adapter": { } },
    "vox_think":     { "base": { }, "adapter": { } }
  }
}
```
(Numbers above are placeholders for shape only; the code never has a literal default.) `status` is `"compared"` only when all six entries are `Scored`; any `not_scored` entry makes it `"incomplete"` and the command returns `Err` (fail closed, matches phase 314). A `not_scored` entry keeps `BenchOutcome`'s own shape (`status`, `n` 0, `n_requested`, `reason`, no score keys). Do **not** add a pass/fail verdict, a delta, or a `degradation` field: this report must not look like the gate's `collateral_damage_report.json`.

Do **not** use a top-level `benchmarks` object of floats: `run_collateral_damage_with` reads exactly that shape from `--pre-score`, so a comparison file shaped that way could be fed in as a baseline by mistake.

**Absence record (recommended, discretion area):**
```json
{
  "schema": "reasoning_before_after.v1",
  "status": "not_compared",
  "reason": "base_snapshot_absent",
  "base_model": "Qwen/Qwen3-8B",
  "base_revision": "b968826d9c46dd6066d109eabc6255188de91218",
  "snapshot_on_disk": false,
  "searched": "<hub_root>/models--Qwen--Qwen3-8B/snapshots/b968826d9c46dd6066d109eabc6255188de91218",
  "adapter": { "given": false, "on_disk": false },
  "slices": ["gsm8k_think", "math500_think", "vox_think"],
  "detail": "pinned Qwen/Qwen3-8B snapshot is not on disk; nothing was scored",
  "gate_input": false
}
```
`reason` values: `base_snapshot_absent`, `base_snapshot_incomplete`, `pin_unverifiable`, `adapter_absent`, `adapter_incomplete`, `adapter_base_not_pinned_snapshot`. No `accuracy`, no `think_initiation_rate`, no `n`, no `status: "scored"` anywhere in the document. Test it by walking every key recursively, not just the top level.

### Where the report lives
Default `--out` is `mens/runs/v1/reasoning_before_after.json` (`vox_scaling_policy::DEFAULT_MENS_RUNS_V1` is `"mens/runs/v1"` `[VERIFIED: crates/vox-scaling-policy/src/lib.rs:70]`; `mens/runs/` is git-ignored `[VERIFIED: git check-ignore -v → .gitignore:210]`). Rationale: it is a run result, it is local-only, and it is not inside any adapter directory. The filename differs from both files serve reads. Create the parent directory. Do not hard-code a `.vox/` path (data-storage policy).

`--out` guard: refuse (Err, nothing written) when the file name is `collateral_damage_report.json` or `reasoning_report.json`, or when any path component is `collateral_eval`. Test it.

### Anti-Patterns to Avoid
- **Calling `score_reasoning_file` for either side:** writes under `<dir>/collateral_eval/<bench>/` (Pitfall 1).
- **Writing anything under the HF cache or the adapter's `collateral_eval/`:** serve reads those.
- **Scoring base-only when the adapter is missing (or the reverse):** locked decision says any missing side writes the absence record.
- **Reading `VOX_MENS_DEFAULT_MODEL` to pick the pin:** use the const.
- **Putting `n: 0`/`accuracy: 0.0` in the absence record:** "no score is invented".
- **Adding a verdict/delta/threshold to the report.**

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Per-item generation, grading, think detection | A second scorer | `score_loaded` + `qwen_reasoning::{thinking_mode_prompt, grade_final_answer, opened_nonempty_think, score_completions}` | Grader/gold changes are locked out; same code = same keys |
| Not-scored shape | New "failed" JSON | `BenchOutcome::NotScored` + `to_json` | Absent score keys are already guaranteed and tested |
| Slice loading | New parser | `load_slice(path, SLICE_SIZE)` | Enforces 32 rows, unique ids, size cap |
| Scope wording | New sentence | `REASONING_SLICE_SCOPE_NOTE` | One source for "32-item slices, not full GSM8K or MATH-500" |
| Token cap | New env var or flag | `VOX_REASONING_EVAL_MAX_TOKENS` via the existing `parse_max_tokens`/`max_tokens_env` | Already registered; parse once and use the same cap for both sides; do not store it (locked) |
| Backend selection | Own plugin picker | `vox_populi::mens::select_mens_backend(DeviceKind::Best, &vox_plugin_host::probe())` then `vox_plugin_host::cached_code_plugin` | Exactly what `run_backend` does |

**Key insight:** the new surface is small because every hard part (grading, fail-closed scoring, slice loading, backend loading) exists. The new risk is only in *where things are written* and *what is checked before loading*.

## Runtime State Inventory

Not a rename/refactor phase. Omitted. (For the record: the new report is a local run artifact under a git-ignored directory; no stored data, service config, OS registration, secret, or build artifact carries a changed name.)

## Common Pitfalls

### Pitfall 1: Reusing `score_reasoning_file` writes into the model directory
**What goes wrong:** `write_outcome` does `adapter.join("collateral_eval").join(bench)`, `create_dir_all`, then `std::fs::write(... "reasoning_report.json")` `[VERIFIED: reasoning_eval.rs write_outcome]`. Given the base snapshot dir, that creates directories inside the HF cache snapshot; given the adapter dir, it overwrites the per-slice reports the 315 serve gate reads (`status` and `n` only, no numeric cross-check `[VERIFIED: 315-VERIFICATION.md Locked Limits]`).
**Why it happens:** the function was built for the collateral writer, where writing there is the point.
**How to avoid:** add a no-write entry that returns `BenchOutcome` and call it from the new code. Test: after a run with a fake scorer, assert the adapter tempdir contains no `collateral_eval` and no `collateral_damage_report.json`.
**Warning signs:** a new `collateral_eval` directory next to a snapshot or after a comparison run.

### Pitfall 2: Existing "is it cached" helpers ignore the pin
`is_model_cached` and the orchestrator context-length helper answer for *any* revision. A different Qwen3-8B commit would be scored as "the pinned base". **Avoid:** Pattern 1, revision directory exact match.

### Pitfall 3: Adapter trained on a different base
The adapter manifest `base_model` is whatever training resolved; nothing ties it to the pinned snapshot, and it may be a Hub id. **Avoid:** Pattern 2 canonical-path equality; absence record otherwise.

### Pitfall 4: No host-side unload, so two 8B models stay resident
No host code calls `unload_model` `[VERIFIED: rg "unload_model" outside plugin crates returns nothing]`; the API doc says the host must unload `[VERIFIED: crates/vox-plugin-api/src/extensions/ml_backend.rs]`, and the plugin frees the box only in `unload_model`. Today's `run_backend` drops the handle without unloading (a pre-existing leak, out of scope to fix elsewhere). **Avoid:** in the new no-write scorer, call `backend.unload_model(&handle)` after scoring each side, before the next `load_model`. Load once per side and score all three slices in that load (the 314 path loads once per slice). Keep the existing comment that the cached plugin reference must outlive the handle.

### Pitfall 5: cwd-relative slice paths
`eval_collateral.rs` resolves slices as `Path::new("mens/data")...` relative to the cwd `[VERIFIED: BENCH_ROOT]`. Run from elsewhere, `load_slice` fails. In the new command make `--bench-root` an argument defaulting to `mens/data`; tests use `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data")` like the existing tests. A slice that fails to load on the present branch is `not_scored` (fail closed), not an absence record.

### Pitfall 6: A stale `compared` report surviving a later absent run
**Avoid:** `remove_file` the `--out` path first (ignore `NotFound`), then write, as 315's writer does for `collateral_damage_report.json`. Every outcome rewrites the file, so a stale comparison cannot sit next to a fresh absence claim.

### Pitfall 7: A "green test" read as a score
Tests inject fake completions. Do not put numbers like `0.84` into any committed file, doc, or the report fixture the CLI could emit. Test literals live only inside test functions.

## Code Examples

### Verified: the existing model-free seam (reuse as is)
```rust
// Source: crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs (score_loaded)
fn score_loaded(
    bench: &str,
    items: &[ReasoningItem],
    max_tokens: usize,
    infer: &mut dyn FnMut(&str) -> Result<String>,
) -> (BenchOutcome, Vec<Row>)
```
Existing tests build `infer` closures from `boxed_response(&gold_from_prompt(p))`; reuse that style in the new tests.

### Verified: the base-only load path
```rust
// Source: crates/vox-plugin-mens-candle-core/src/weight_resolution.rs (resolve_weights)
(false, false) => Ok(ResolvedWeights {
    base_shards: glob_model_shards(model_dir)?,
    lora_factors: HashMap::new(),
}),
```
A directory with neither adapter file loads as a base snapshot; pass the pinned snapshot dir to `load_model`.

### Recommended: the detection result
```rust
pub(crate) enum Detected {
    Ready { snapshot: PathBuf, adapter: PathBuf },
    NotCompared(AbsenceReason),   // carries the strings that go in the record
}
pub(crate) fn detect_inputs(hub_root: &Path, pin: &str, adapter: Option<&Path>) -> Detected;
```
`detect_inputs` takes `hub_root` so tests create `models--Qwen--Qwen3-8B/snapshots/<sha>/` in a tempdir; the CLI resolves `hub_root` from env once.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Serve trusted `status` alone | Serve recomputes slice drops and re-reads per-slice reports | Phase 315 | The new file must never be placed where serve reads |
| Scorer wrote nothing on failure | `not_scored` report, `n: 0`, no score keys, `Err` | Phase 314 | Reused as the per-entry shape |

**Deprecated/outdated:** none relevant.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | hf-hub 1.0 resolves its cache root with the same `HF_HUB_CACHE` > `HF_HOME/hub` > `~/.cache/huggingface/hub` order (XDG not consulted) | Pattern 1 | A user with a nonstandard cache layout gets a false "absent". Mitigated by a `--hub-root` override and by the record printing the path searched. |
| A2 | The pinned Qwen3-8B snapshot ships `model.safetensors.index.json` with a `weight_map` | Pattern 1 | If it ships a single `model.safetensors`, the fallback branch handles it; no plan impact. Not verifiable here (no 8B files, no download allowed). |
| A3 | Exit code 0 for the absent branch is the right operator behavior | Open Questions | A script that treats exit 0 as "compared" would misread it; mitigated by `status: "not_compared"` in the file and a stderr line. |
| A4 | A new `vox mens` tail subcommand needs no `contracts/operations/catalog.v1.yaml` row | Open Questions | `vox ci command-compliance` could fail. Precedent: `eval-collateral-damage` is in the same enum and has no registry row (rg found none). |

## Open Questions

1. **Does `vox ci command-compliance` require a catalog/registry row for a new `vox mens` tail subcommand?**
   - What we know: `mens eval-gate` and `mens eval-local` have rows in `contracts/cli/command-registry.yaml` and `contracts/operations/catalog.v1.yaml`; `mens eval-collateral-damage`, in the same `PopuliMensTail` enum, has none (rg over `contracts/`). The cli-command-registry rule says to add the catalog projection and run `vox ci operations-sync`.
   - What's unclear: whether compliance enforces it for this enum. Not run here (needs a `vox-cli` build).
   - Recommendation: after adding the variant run `cargo run -p vox-cli -- ci command-compliance`; if it fails, add the projection per `.cursor/rules/cli-command-registry.mdc`. Regenerated docs are handled by the `ssot-autoregen` bot; do not hand-edit generated files.
2. **Exit code on the absent branch.** Recommendation: exit 0 and print `NOT COMPARED: <reason>` to stderr, because absence is a correctly recorded expected state on this checkout, while `incomplete` (a scorer failure) returns `Err`. If the user prefers fail-closed everywhere, flip it to `Err`; the record is written either way.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| Rust/cargo (via broker) | tests | ✓ | workspace toolchain | — |
| Pinned `Qwen/Qwen3-8B` snapshot | present branch (real run) | ✗ | — | Absent record (the intended result here) |
| Any adapter directory | present branch (real run) | not looked for | — | `--adapter` omitted gives `adapter_absent` |
| `~/.cache/huggingface/hub` | detection | ✓ | has `models--Qwen--Qwen3-0.6B`, no Qwen3-8B | — |
| MlBackend plugin (candle metal/cuda) | real scoring | not needed for tests | — | Tests inject the scorer |

**Missing dependencies with no fallback:** none for this phase's deliverables.
**Missing dependencies with fallback:** the 8B snapshot and an adapter; the absent branch is the fallback and the specified outcome.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Rust built-in test harness (`#[test]`), `tempfile` for FS fixtures |
| Config file | none |
| Quick run command (pure shapes) | `cargo test -p vox-eval --lib reasoning_bench` (verified this session: 17 passed) |
| Quick run command (new module) | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` |
| Regression for reused scorer | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` (verified this session: 12 passed; cold compile took 3m12s) |
| Compile check | `cargo check -p vox-ml-cli --features gpu` |

A filter that prints `0 passed` is a failed run. Do not run workspace `cargo test`; do not treat the three pre-existing clippy errors (`qwen_reasoning.rs:178`, `trial_flywheel.rs:208`, `trial_flywheel.rs:784`) as phase work. Scope any clippy to the touched crate and read only new findings.

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-01 (SC2) | Absence record names `Qwen/Qwen3-8B`, `snapshot_on_disk` false, status `not_compared`, and a recursive walk finds no `accuracy` / `think_initiation_rate` key and no `status: scored` | unit | `cargo test -p vox-eval --lib reasoning_bench` (shape) | ❌ Wave 0 |
| BENCH-01 (SC2) | Empty hub root and a hub root holding only `models--Qwen--Qwen3-0.6B` both give `base_snapshot_absent`; the 0.6B tree is never scored | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_compare` | ❌ Wave 0 |
| BENCH-01 (pin) | A tree with `snapshots/<other 40-hex sha>/` complete does **not** count as present; only the pinned sha does | unit | same | ❌ Wave 0 |
| BENCH-01 (pin) | Pin without `@sha`, or sha not 40 hex, is `pin_unverifiable` | unit | same | ❌ Wave 0 |
| BENCH-01 | Snapshot dir with a dangling shard link or missing `tokenizer.json` is `base_snapshot_incomplete`, not present | unit | same | ❌ Wave 0 |
| BENCH-01 | Adapter: only one of the two files, or none, gives `adapter_incomplete` / `adapter_absent`; `--adapter` omitted gives `adapter_absent` | unit | same | ❌ Wave 0 |
| BENCH-01 | Adapter manifest `base_model` that is another dir, a Hub id, or missing gives `adapter_base_not_pinned_snapshot`; equal canonical path is accepted | unit | same | ❌ Wave 0 |
| BENCH-01 (SC1) | With a fake scorer: report has all of `gsm8k_think`, `math500_think`, `vox_think` × `base`/`adapter` with separate `accuracy` and `think_initiation_rate` keys, `slice_size` 32, the scope note | unit | same | ❌ Wave 0 |
| BENCH-01 (SC3) | Fake scorer records the items it receives per side; both sides get identical `ReasoningItem` lists, equal to `load_slice` of the checked-in file | unit | same | ❌ Wave 0 |
| BENCH-01 | Fake scorer returns one `NotScored`: status is `incomplete`, that entry has no score keys, function returns `Err` | unit | same | ❌ Wave 0 |
| BENCH-01 | Absent branch never calls the scorer (a scorer that panics proves it) | unit | same | ❌ Wave 0 |
| BENCH-01 | Neither branch creates `collateral_damage_report.json` or `collateral_eval/`, or any file but `--out`, in the adapter, snapshot, or out dir | unit | same | ❌ Wave 0 |
| BENCH-01 | `--out` named `collateral_damage_report.json`, `reasoning_report.json`, or under `collateral_eval` is refused and writes nothing | unit | same | ❌ Wave 0 |
| BENCH-01 | A stale `compared` report at `--out` is replaced by an absence record | unit | same | ❌ Wave 0 |
| BENCH-01 (real, this checkout) | Real run takes the absent branch and writes the record | smoke | `cargo run -p vox-ml-cli --features gpu -- mens eval-reasoning-compare` then `rg -c 'accuracy\|think_initiation' mens/runs/v1/reasoning_before_after.json` must find nothing (exit 1) | ❌ Wave 0 |

Sampling and mutation checks (AGENTS.md: verify guards by mutation, with a before/after presence check around each edit):
- Replace the pinned-sha directory match with "any entry under `snapshots/`": the other-sha test must fail.
- Remove the canonical-path equality in the adapter check: the mismatch test must fail.
- Make the absent branch call the scorer: the panicking-scorer test must fail.
- Make the absence record builder insert `"accuracy"`: the recursive key walk must fail.
- Drop the `--out` file-name refusal: the refusal test must fail.
- Drop the pre-write `remove_file`: make the stale-report test pre-seed a file and fail when the write is also skipped.

### Sampling Rate
- **Per task commit:** the quick command for the crate touched, plus `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` after any `reasoning_eval.rs` edit.
- **Per wave merge:** both quick commands and `cargo check -p vox-ml-cli --features gpu`.
- **Phase gate:** all of the above green with non-zero pass counts, plus the real absent-branch run and the key grep.

### Wave 0 Gaps
- [ ] pure shape builders and tests in `crates/vox-eval/src/reasoning_bench.rs`
- [ ] `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` with its test module (tests first, then implementation)
- [ ] none for framework install (harness and `tempfile` already present)

## Security Domain

`security_enforcement` is not disabled in `.planning/config.json`, so this section applies. Level matches phase 314/315 (ASVS L1).

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no (local CLI) | — |
| V5 Input Validation | yes | `--adapter`, `--hub-root`, `--out`, adapter `base_model`, `adapter_manifest.json` are untrusted text/paths; `load_slice` size cap and validation; canonicalize before compare; `serde_json::Value` with `get().and_then()`, no `unwrap` |
| V6 Cryptography | no | none hashed or signed here; if ever needed use `vox-crypto` |
| V12 Files and Resources | yes | Write only to the refused-name-checked `--out`; never write inside the HF cache or into `collateral_eval/`; never follow `base_model` to read anything beyond canonicalizing the path |
| V7 Error handling | yes | Fail closed: any scorer failure is `not_scored` and `Err`; never a numeric stand-in |

### Known Threat Patterns for this stack

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Absence record or comparison file read by serve as a pass | Spoofing / Elevation | Distinct file name and location; `--out` refusal list; no `status: "pass"`, no `reports`/`reasoning_slices`/`benchmarks` keys; serve code unchanged; test that no gate file is created |
| Writing through HF cache symlinks corrupts blobs | Tampering | No-write scorer entry; Pitfall 1 test |
| Adapter from another base compared against the pinned base (false claim of "before/after") | Spoofing | Canonical-path base check, absence record on mismatch |
| Different revision accepted as "pinned" | Spoofing | Exact `snapshots/<sha>` directory; unverifiable pin is absent |
| Hostile `base_model` path (traversal, symlink loop) | Tampering / DoS | Only `is_dir()` and `canonicalize()` (errors become the mismatch reason); no reads through it |
| Hostile adapter `adapter_manifest.json` (huge or malformed) | DoS | Cap the read (use `vox_bounded_fs::read_utf8_path_capped`, already used by `eval_local.rs`), parse failure is `adapter_incomplete` |
| Partial download counted as present | Spoofing | Index-driven shard completeness with `is_file()` |
| Stale comparison surviving | Repudiation | Remove-then-write; all outcomes rewrite the file |
| Secret leakage | Information disclosure | No secrets involved; no `HF_TOKEN` read; detection makes no network call. Do not set `VOX_MENS_NO_DOWNLOAD` semantics aside: this command never calls `download_model` |

## Sources

### Primary (HIGH confidence, read this session)
- `.planning/phases/316-before-after-comparison-bench-01/316-CONTEXT.md`, `.planning/ROADMAP.md`, `.planning/REQUIREMENTS.md`
- `.planning/phases/315-serve-gate-gate-01-gate-02/315-VERIFICATION.md`, `315-RESEARCH.md`; `.planning/phases/314-.../314-02-SUMMARY.md`, `314-SECURITY.md`
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs`, `eval_collateral.rs`, `eval_local.rs`, `populi/dispatch.rs` (serve gate, EvalCollateralDamage arm), `populi/mens_tail_subcommands.rs`, `populi/action_populi_enum.rs`
- `crates/vox-eval/src/reasoning_bench.rs`, `reasoning_slice.rs`
- `crates/vox-populi/src/mens/mod.rs`, `mens/hub.rs`
- `crates/vox-plugin-mens-candle-metal/src/{backend,inference,model}.rs`, `candle_qlora_train/finalize.rs`; `crates/vox-plugin-mens-candle-core/src/weight_resolution.rs`; `crates/vox-plugin-api/src/extensions/ml_backend.rs`
- `crates/vox-orchestrator/src/catalog.rs`, `crates/vox-ml-cli/src/commands/oratio_cmd.rs` (hub root precedent), `crates/vox-code-audit/src/detectors/unregistered_llm_env.rs`
- Local run: `cargo test -p vox-eval --lib reasoning_bench` (17 passed); `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` (12 passed)
- `ls ~/.cache/huggingface/hub` (no `models--Qwen--Qwen3-8B`)

### Secondary / Tertiary
- None. No web sources were used.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, no new dependencies, all reused items read in source.
- Architecture: HIGH for seams and write-location hazards; MEDIUM for hub-root parity with hf-hub (A1).
- Pitfalls: HIGH, each cites the line it comes from.

**Project Constraints (from .cursor/rules/):**
- LF line endings; run `vox ci line-endings` on the diff. No `.ps1`/`.sh`/`.py` glue; the one real run is a `cargo run` of a Rust CLI, not a script.
- No `env::var` for secrets (none used). `HF_HUB_CACHE`/`HF_HOME` are layout vars allowed by the audit; no new `VOX_*` env var is introduced, so `contracts/config/env-vars.v1.yaml` is untouched.
- Do not open a `turso::Connection`, write event/log data with `fs::write`, or hard-code `.vox/` paths. The report is a one-shot run artifact under the git-ignored `mens/runs/v1/`.
- Test-first for every new `pub fn`/`pub(crate)` surface; `skeleton/untested-pub-api` applies to files over 30 non-blank lines.
- Route cargo through the broker (plain `cargo` on PATH). Format only dirty `.rs` files via `vox run scripts/fmt.vox`; never `cargo fmt --all`.
- Do not edit `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` or `scripts/mens-macos-metal-e2e.vox`. Any docs addition belongs in `docs/src/how-to/how-to-train-mens-macos-metal.md` (existing page, already has frontmatter) and must not state numbers.
- Retired surfaces: none touched. Use `vox-orchestrator` not `vox-dei`; do not mention `recall()`.
- Working tree is dirty with unrelated changes (`eval_local_prompt.rs`, `train_arm.rs`, `trial_flywheel.rs`); stage only this phase's files.

**Research date:** 2026-10-26
**Valid until:** 2026-11-25 (stable code; re-check only if `reasoning_eval.rs` or `weight_resolution.rs` change)
