# Phase 316: Before/After Comparison - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 6 (2 new-surface files, 4 edit sites; plus 1 optional docs line)
**Analogs found:** 6 / 6 (every excerpt below was read from the working tree this session; every analog path is git-tracked)

Scope guard (from CONTEXT.md / RESEARCH.md): no grader, gold, or token-cap change; no model is loaded in tests; the real run on this checkout takes the absent branch; the report must never land where `vox mens serve` reads (`collateral_damage_report.json`, `collateral_eval/<slice>/reasoning_report.json`). Do not modify `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` or `scripts/mens-macos-metal-e2e.vox`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `crates/vox-eval/src/reasoning_bench.rs` (add before/after + absence JSON builders and tests) | model / utility (pure report shape) | transform | same file: `BenchOutcome::to_json` (L135-154) and its tests (L157+) | exact (sibling of the shape it wraps) |
| `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` (NEW: detection, `run_before_after_with`, `--out` guard, CLI body, tests) | service (orchestrator) + utility (detection) | batch + file-I/O, injectable scorer seam | `run_collateral_damage_with` in `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (L48-174) | exact (role + data flow) |
| `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (add no-write, load-once scorer entry + unload) | service | request-response over an `infer` closure | `run_backend` (L131-169) + `score_loaded` (L180-234) in same file | exact (extend, do not fork) |
| `crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs` (new `EvalReasoningCompare` variant) | config (clap enum variant) | request-response | `EvalCollateralDamage` variant (L48-57) | exact |
| `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs` (new match arm) | controller (dispatch) | request-response | `EvalCollateralDamage` arm (L718-731) | exact |
| `crates/vox-ml-cli/src/commands/mens/mod.rs` (mod registration) | config | n/a | `eval_collateral` registration (L24-25) and `reasoning_eval` (L39-40) | exact |
| `docs/src/how-to/how-to-train-mens-macos-metal.md` (optional one line, no numbers) | docs | n/a | same page, collateral section (L59-82) | role-match |

Do NOT touch: `assert_collateral_gate` (`dispatch.rs` L801+), `eval_collateral.rs` logic, grader (`qwen_reasoning`), slice files, `reasoning_slice.rs` constants.

## Pattern Assignments

### 1. `crates/vox-eval/src/reasoning_bench.rs` (pure shape builders)

**Analog:** same file, `BenchOutcome` (L100-154). Wrap `to_json`; never re-derive its keys.

**Existing shape to reuse verbatim** (L135-154):
```rust
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            BenchOutcome::Scored(s) => serde_json::json!({
                "status": "scored",
                "n": s.n,
                "accuracy": s.accuracy,
                "think_initiation_rate": s.think_initiation_rate,
            }),
            BenchOutcome::NotScored {
                reason,
                n_requested,
            } => serde_json::json!({
                "status": "not_scored",
                "n": 0,
                "n_requested": n_requested,
                "reason": reason,
            }),
        }
    }
```
New builders (suggested names, e.g. `before_after_json(...)` and `absence_json(...)`) sit after `impl BenchOutcome` and before `#[cfg(test)]` (L156). Use `crate::reasoning_slice::REASONING_SLICE_SCOPE_NOTE` (defined `reasoning_slice.rs` L17) and `SLICE_SIZE` / `SLICE_NAMES` (L15, L18) so the 32-item scope text is not forked. The existing precedent for a wrapper that embeds the note and `slice_size` is `reasoning_slices_json` (`reasoning_slice.rs` ~L170-186):
```rust
    serde_json::json!({
        "scope_note": REASONING_SLICE_SCOPE_NOTE,
        "slice_size": SLICE_SIZE,
        "benches": benches,
    })
```
Do not reuse the keys `benchmarks`, `reports`, `reasoning_slices` or `status: "pass"`: `run_collateral_damage_with` reads a top-level `benchmarks` object from `--pre-score` (`eval_collateral.rs` L67-81), and serve reads the gate envelope.

**Imports pattern** (L8-13, unchanged; nothing new needed in this crate):
```rust
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;

use crate::qwen_reasoning::ReasoningScore;
```

**Test pattern** (L157-185): the in-file `mod tests` with `repo_slice`, `rows`, `write_tmp` helpers, and the JSON round-trip style of `scored_json_carries_n_and_exact_accuracy` (L491-517):
```rust
        let outcome = BenchOutcome::from_score(score, 32);
        assert!(matches!(outcome, BenchOutcome::Scored(_)));
        let v = outcome.to_json();
        assert_eq!(v["status"], "scored");
        assert_eq!(v["n"], 32);
        assert_eq!(v["accuracy"].as_f64(), Some(3.0 / 32.0));
        let text = serde_json::to_string(&v).expect("serialize");
        let back: serde_json::Value = serde_json::from_str(&text).expect("parse");
```
New tests to add here (fast, no `gpu` feature): absence record names `Qwen/Qwen3-8B`, `snapshot_on_disk` false, `status` `not_compared`, `gate_input` false, and a **recursive** key walk finds no `accuracy` / `think_initiation_rate` / `status: "scored"`; compared shape has 3 slices x `base`/`adapter` with separate keys; one `NotScored` entry makes status `incomplete` and keeps `to_json`'s no-score-key shape.

**Size flag for the planner:** this file is 540 lines already (God-object limit is 500 non-blank lines per `docs/agents/governance.md`; most of the bulk is tests). If TOESTUB flags it, put the two builders in a new sibling `crates/vox-eval/src/reasoning_compare_json.rs` (L0/L1 same crate, `pub mod` in `lib.rs` next to L15-16) rather than growing this file. Decide at implementation time by running the audit, not before.

---

### 2. `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs` (NEW; role: orchestrator, data flow: batch + file-I/O with injectable scorer)

**Analog:** `run_collateral_damage_with` in `eval_collateral.rs` (L48-174), plus its CLI wrapper `run_collateral_damage` (L176-189).

**Imports pattern** (`eval_collateral.rs` L1-10):
```rust
use anyhow::{Context, Result};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use vox_eval::reasoning_bench::{SLICE_NAMES, SLICE_SIZE};
```
New file adds `vox_eval::reasoning_bench::{BenchOutcome, ReasoningItem, load_slice}` and `vox_populi::mens::DEFAULT_MODEL_ID` (pin SSOT, `vox-populi/src/mens/mod.rs` L46: `pub const DEFAULT_MODEL_ID: &str = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";`). Do not call `default_model_id()` (honors `VOX_MENS_DEFAULT_MODEL`). No new Cargo deps: `dirs` (L69), `vox-bounded-fs` (L83), `vox-scaling-policy` (L92), dev-dep `tempfile` (L114) are already in `crates/vox-ml-cli/Cargo.toml`; `vox-populi` / `vox-plugin-host` come in via the `gpu` feature.

**Injectable seam pattern** (`eval_collateral.rs` L40-54). The scorer is a `&mut dyn FnMut` parameter so tests need no GPU, and the doc says why:
```rust
/// `score_bench` is a parameter so tests can drive a degradation without a GPU —
/// and, more importantly, so `post` has exactly one source. A scorer error is
/// propagated: a model that cannot be loaded must never leave a report behind.
pub(crate) fn run_collateral_damage_with(
    pre_score_path: &Path,
    run_dir: &Path,
    score_bench: &mut dyn FnMut(&str) -> Result<BenchScore>,
) -> Result<i32> {
```
Mirror it with `run_before_after_with(inputs: Detected, bench_root: &Path, out: &Path, score_side: &mut dyn FnMut(Side, &Path, &[(&str, Vec<ReasoningItem>)]) -> Vec<BenchOutcome>) -> Result<RunStatus>`. `load_slice` is called once per slice on the present branch and the same `Vec<ReasoningItem>` goes to both sides (BENCH-01 SC3).

**Stale-report removal, before anything can fail** (`eval_collateral.rs` L52-65). Copy for `--out` (Pitfall 6):
```rust
    let report_path = run_dir.join("collateral_damage_report.json");
    match std::fs::remove_file(&report_path) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(e).with_context(|| {
                format!("could not remove stale report {}", report_path.display())
            });
        }
    }
```
Difference: here the removed path is the user-chosen `--out`, and BOTH branches (absent and compared) rewrite it afterward. Add the `--out` refusal guard (file name `collateral_damage_report.json` / `reasoning_report.json`, or any component `collateral_eval`) BEFORE this remove, so a refused path is never deleted.

**Fail-closed propagation** (`eval_collateral.rs` L83-88): `post.insert(bench.as_str(), score_bench(bench)?);` with the comment "fails loudly; never falls back to `pre`". New code: any `NotScored` entry makes status `incomplete` and the function returns `Err` after writing the incomplete file; there is no numeric fallback.

**Pinned snapshot detection - new code, closest precedents (read, do not copy blindly):**

- Hub-root order precedent (`crates/vox-orchestrator/src/catalog.rs` L1085-1100; same order as `oratio_cmd.rs` L816-819):
```rust
        let repo_id = base_str.split('@').next().unwrap_or(base_str);
        let revision_sha = base_str.split('@').nth(1);

        let hub_base = custom_hub_base
            .map(|p| p.to_path_buf())
            .or_else(|| {
                std::env::var("HF_HUB_CACHE")
                    .ok()
                    .map(std::path::PathBuf::from)
            })
            .or_else(|| {
                std::env::var("HF_HOME")
                    .ok()
                    .map(|h| std::path::PathBuf::from(h).join("hub"))
            })
            .or_else(|| {
                dirs::home_dir().map(|home| home.join(".cache").join("huggingface").join("hub"))
            });
```
  Layout: `<hub>/models--<owner>--<name>/snapshots/<sha>/` (`repo_id.replace('/', "--")`, L1104-1107). Take `hub_root: &Path` as a parameter in `detect_inputs` (tests build trees in a tempdir); resolve env once in the CLI body; add a `--hub-root` override. **Anti-copy:** the catalog helper falls back to "any snapshot" when the pinned dir is missing (RESEARCH Pitfall 2); the new detector requires the exact `snapshots/<sha>` directory.
- Revision split rule (`crates/vox-populi/src/mens/hub.rs` L48-51): `repo_id.rsplit_once('@').map(|(_, rev)| rev)`. Require 40 lowercase hex or return `PinUnverifiable`. Do NOT call `hub::is_model_cached` (L475): it strips the revision via `repo_name_without_revision` (L44-46) and answers "any revision cached".
- Capped manifest read: `vox_bounded_fs::read_utf8_path_capped(path)` (`vox-bounded-fs/src/lib.rs` L21; used e.g. `parity_report.rs` L129 as `let content = vox_bounded_fs::read_utf8_path_capped(path)`). Parse `adapter_manifest.json` into `serde_json::Value`; a parse/read failure is `adapter_incomplete`.
- Adapter pair + base-pin semantics come from the plugin and must be matched, not imported (`crates/vox-plugin-mens-candle-core/src/weight_resolution.rs` L85-125):
```rust
            let base_shards = match &meta.base_model {
                Some(base) if Path::new(base).is_dir() => glob_model_shards(Path::new(base))?,
                Some(base) => anyhow::bail!(
                    "base_model '{base}' is not a local directory. Hub download is not \
                     supported in the plugin; ...
                ),
                None => anyhow::bail!(
                    "adapter manifest missing `base_model` reference. Cannot load frozen weights."
                ),
            };
...
        (true, false) => anyhow::bail!(
            "{} has a trained adapter file but no adapter_manifest.json — inconsistent \
```
  So detection requires BOTH `candle_qlora_adapter.safetensors` and `adapter_manifest.json` as files; `base_model` must be an existing dir; `canonicalize()` it and the detected snapshot and require equality (`adapter_base_not_pinned_snapshot` otherwise). Use `serde_json::Value::get(..).and_then(Value::as_str)`, no `unwrap`; do not import the plugin's private manifest type.

**Absence/compared output and default path:** default `--out` is `Path::new(vox_scaling_policy::DEFAULT_MENS_RUNS_V1).join("reasoning_before_after.json")` (`DEFAULT_MENS_RUNS_V1 = "mens/runs/v1"`, `vox-scaling-policy/src/lib.rs` L70; same constant used as a clap default at `mens_tail_subcommands.rs` L17). `create_dir_all(parent)` before write. Absent branch: exit 0, stderr `NOT COMPARED: <reason>`; never call the scorer.

**CLI entry pattern** (`eval_collateral.rs` L176-189): a thin `pub fn` builds the real scorer closure and owns process exit so the core stays testable:
```rust
pub fn run_collateral_damage(pre_score_path: PathBuf, post_adapter_path: PathBuf) -> Result<()> {
    ...
    let mut scorer = |bench: &str| score_bench(bench, &post_adapter_path);
    let code = run_collateral_damage_with(&pre_score_path, &post_adapter_path, &mut scorer)?;
    if code != 0 {
        std::process::exit(code);
    }
    Ok(())
}
```
New CLI body: `run_reasoning_compare(adapter: Option<PathBuf>, hub_root: Option<PathBuf>, bench_root: PathBuf, out: PathBuf) -> Result<()>`; `--bench-root` defaults to `mens/data` (do not hard-wire cwd-relative like `BENCH_ROOT` at `eval_collateral.rs` L14; tests use the `CARGO_MANIFEST_DIR` join below). The real scorer closure calls the new no-write entry in `reasoning_eval.rs` (section 3).

**Test patterns to copy** (`eval_collateral.rs` tests):
- Exact call-sequence recorder, not `>= 1` (L306-L351 `a_degraded_adapter_writes_a_failing_report`): `let mut asked: Vec<String> = Vec::new(); let mut scorer = |bench: &str| { asked.push(bench.to_string()); Ok(...) };` then `assert_eq!(asked, [...])`. Use for "both sides receive identical `ReasoningItem` lists equal to `load_slice` of the checked-in file".
- Failing scorer leaves no gate file (L357-L371):
```rust
        let mut scorer = |_: &str| anyhow::bail!("adapter not found");
        assert!(run_collateral_damage_with(&pre, dir.path(), &mut scorer).is_err());
        assert!(
            !dir.path().join("collateral_damage_report.json").exists(),
            "a load failure must never leave a report the serve gate would accept"
        );
```
  Use for "absent branch never calls the scorer (a panicking scorer)" and "no `collateral_damage_report.json`, no `collateral_eval/`, no file but `--out` in snapshot/adapter/out dirs".
- Helper `no_report(dir)` (L508) and `scorer_over` (L490) show the helper-fn style; stale-report tests (L617-L628) seed `{"status":"pass"}` at the target path, run, and assert it is gone. Mirror with a stale `compared` file at `--out` replaced by an absence record.
- Test fixtures: `tempfile::tempdir()` hub trees `models--Qwen--Qwen3-8B/snapshots/<sha>/{config.json,tokenizer.json,model.safetensors.index.json,<shards>}`; dangling-symlink shard for `base_snapshot_incomplete`; a `models--Qwen--Qwen3-0.6B`-only hub for `base_snapshot_absent`. Slice path: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data/gsm8k_think/reasoning.jsonl")` (precedent `reasoning_eval.rs` tests ~L296-L298).
- Test literals (e.g. accuracies) live only inside `#[test]` fns; never in a committed fixture the CLI could emit (Pitfall 7).

**Module gating:** compile only under the `gpu` feature (see section 6), consistent with `eval_collateral` and `reasoning_eval`.

---

### 3. `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (add no-write, load-once scorer)

**Analog:** `run_backend` (L131-169) and `score_loaded` (L180-234) in the same file. Keep the edit tiny: the file is 552 lines, already over the 500-line God-object limit; put orchestration in `reasoning_compare.rs`.

**Backend load/inference pattern to reuse** (L133-169):
```rust
fn run_backend(
    bench: &str,
    items: &[ReasoningItem],
    adapter: &Path,
    max_tokens: usize,
) -> Result<(BenchOutcome, Vec<Row>)> {
    let plugin_id = vox_populi::mens::select_mens_backend(
        vox_populi::mens::DeviceKind::Best,
        &vox_plugin_host::probe(),
    );
    let loaded = vox_plugin_host::cached_code_plugin(plugin_id).with_context(|| {
        format!("{plugin_id} plugin not found — install vox-plugin-{plugin_id}")
    })?;
    let backend = loaded
        .plugin
        .as_ml_backend()
        .into_option()
        .ok_or_else(|| anyhow::anyhow!("{plugin_id} plugin does not provide MlBackend"))?;
    let handle = backend
        .load_model(adapter.to_string_lossy().as_ref().into())
        .into_result()
        .map_err(|e| anyhow::anyhow!("load_model({}): {e}", adapter.display()))?;

    let mut infer = |prompt_json: &str| -> Result<String> {
        backend
            .run_inference(&handle, prompt_json.into())
            .into_result()
            .map(|resp| resp.as_str().to_string())
            .map_err(|e| anyhow::anyhow!("{e}"))
    };
    let done = score_loaded(bench, items, max_tokens, &mut infer);
    // `loaded` is a cache reference and must outlive `handle`; it drops here
    // with the other locals, which is after the last inference.
    let _plugin = loaded;
    Ok(done)
}
```
New entry (e.g. `pub(crate) fn score_dir_no_write(model_dir: &Path, benches: &[(&str, Vec<ReasoningItem>)], max_tokens: usize) -> Vec<BenchOutcome>`): same plugin/handle acquisition, **one** `load_model` per side, call `score_loaded` once per `(bench, items)` inside that load, then `backend.unload_model(&handle)` (signature `fn unload_model(&self, model: &MlModelHandle) -> RResult<(), RBoxError>`, `vox-plugin-api/src/extensions/ml_backend.rs` L43; no host code calls it today) before returning, and keep the `loaded`-outlives-`handle` comment. A load error becomes `BenchOutcome::NotScored { reason, n_requested }` for every slice (fail closed, never a number). Token cap: reuse `parse_max_tokens(max_tokens_env().as_deref())` (L115, L127) once for both sides; do not store it on the report.

**Anti-pattern (do not call):** `score_reasoning_file` (L37) / `write_outcome` (L238-275). `write_outcome` does `adapter.join("collateral_eval").join(bench)` + `create_dir_all` + `fs::write(... "reasoning_report.json")`, i.e. writes into the model dir (an HF cache snapshot for the base side, and the serve-read location for the adapter side).

**Model-free seam, unchanged** (L180-234): `score_loaded(bench, items, max_tokens, infer: &mut dyn FnMut(&str) -> Result<String>) -> (BenchOutcome, Vec<Row>)`; it builds the prompt JSON with `"temperature": 0.0, "seed": 0`, grades with `qwen_reasoning::grade_final_answer` / `opened_nonempty_think`, and returns `NotScored` on the first failing item. Do not change it.

**Tests in this file stay as the regression guard** (helpers at ~L318-L345: `synthetic_items`, `gold_from_prompt`, `boxed_response`); run `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` after the edit (12 pass at research time). Adding the no-write entry needs only a model-free test of the pieces that do not touch a plugin (e.g. a not-loadable dir returns `NotScored` for all benches and creates no `collateral_eval`), modeled on `an_adapter_dir_without_a_loadable_model_writes_not_scored` / `a_missing_adapter_path_is_err_and_not_created` (L295-L317) but asserting the **absence** of any written file.

---

### 4. `populi/mens_tail_subcommands.rs` (new `EvalReasoningCompare` variant)

**Analog:** `EvalCollateralDamage` (L48-57):
```rust
    /// Check for catastrophic forgetting against a static benchmark
    #[command(name = "eval-collateral-damage", visible_alias = "eval")]
    EvalCollateralDamage {
        /// Baseline score JSON path
        #[arg(long)]
        pre_score: PathBuf,
        /// Adapter path to evaluate
        #[arg(long, id = "post")]
        post_adapter: PathBuf,
    },
```
New variant: `#[command(name = "eval-reasoning-compare")]` with `#[arg(long)] adapter: Option<PathBuf>`, `hub_root: Option<PathBuf>`, `#[arg(long, default_value = "mens/data")] bench_root: PathBuf`, `out: Option<PathBuf>` (None = `DEFAULT_MENS_RUNS_V1/reasoning_before_after.json`). Note the variant itself is NOT cfg-gated (the enum compiles without `gpu`); only the dispatch body is. Do not reuse `visible_alias = "eval"`. Default-value precedent from the same file (L17-18): `#[arg(long, default_value = vox_scaling_policy::DEFAULT_MENS_RUNS_V1)] run_dir: PathBuf,`. Doc comment must say it writes a non-gate report. File is 113 lines; no size concern.

---

### 5. `populi/dispatch.rs` (new match arm)

**Analog:** the `EvalCollateralDamage` arm (L718-731), which shows the feature-gate idiom for a variant that exists in non-`gpu` builds:
```rust
            super::mens_tail_subcommands::PopuliMensTail::EvalCollateralDamage {
                pre_score,
                post_adapter,
            } => {
                #[cfg(not(feature = "gpu"))]
                let _ = (pre_score, post_adapter);

                #[cfg(feature = "gpu")]
                crate::commands::mens::eval_collateral::run_collateral_damage(
                    pre_score,
                    post_adapter,
                )?;
                Ok(())
            }
```
New arm: same shape, binding all new fields, `let _ = (...)` under `not(feature = "gpu")`, call `crate::commands::mens::reasoning_compare::run_reasoning_compare(...)` under `gpu`. Place after this arm (before `BenchCompletion`, L733). `run` is `pub async fn run(action: PopuliAction, ...)` (L28); the new body is sync like `run_collateral_damage`. Do NOT touch `assert_collateral_gate` (L801+): serve must never read the new file; the existing gate tests (L1031+) must stay green untouched. dispatch.rs is 1268 lines; add only the arm.

---

### 6. `crates/vox-ml-cli/src/commands/mens/mod.rs` (mod registration)

**Analog** (L24-25 and L39-40):
```rust
#[cfg(feature = "gpu")]
pub mod eval_collateral;
...
#[cfg(feature = "gpu")]
pub(crate) mod reasoning_eval;
```
Add `#[cfg(feature = "gpu")] pub(crate) mod reasoning_compare;` next to `reasoning_eval` (keep alphabetical: after `plugin_heal`, before/after `reasoning_eval`). `pub(crate)` suffices because dispatch is in the same crate. If `eval_collateral`-style `pub` is used, rustdoc/`missing_docs` rules apply; `pub(crate)` avoids it.

---

### 7. Optional docs line: `docs/src/how-to/how-to-train-mens-macos-metal.md`

Existing page with frontmatter (L3 `description:`), collateral section at L59-82 already describes the gate artifacts. Add at most one short paragraph naming `vox mens eval-reasoning-compare`, its default output `mens/runs/v1/reasoning_before_after.json`, that it is **not** a serve-gate input, and that without the pinned snapshot it records absence and writes no scores. State no numbers. No fenced `vox`/`rust` blocks (they compile as doctests); use ```text for command lines.

## Shared Patterns

### Fail closed, no numeric stand-in
**Source:** `reasoning_bench.rs` `BenchOutcome` (L100-154); `reasoning_eval.rs` module doc L1-15; `eval_collateral.rs` L83-88.
**Apply to:** `reasoning_compare.rs`, the no-write scorer, the report builders. A missing model/slice/item is `NotScored` (no `accuracy`, no `think_initiation_rate`, `n: 0`); a partial result is `incomplete` + `Err`; absence is `not_compared` with no score-bearing keys anywhere (test with a recursive walk).

### Never write where serve reads
**Source:** `eval_collateral.rs` L52-65 (stale-report removal) and `dispatch.rs` `assert_collateral_gate` (L801+; reads `collateral_damage_report.json` and `collateral_eval/<slice>/reasoning_report.json`).
**Apply to:** the `--out` guard (reject names `collateral_damage_report.json`, `reasoning_report.json`, any `collateral_eval` component), the no-write scorer, and a test asserting no stray files in the adapter/snapshot/out tempdirs after either branch.

### Injectable scorer seam + exact-sequence test
**Source:** `eval_collateral.rs` L48-54 and L306-351.
**Apply to:** `run_before_after_with`. Tests record the `(Side, items)` received and assert exact equality, not `>= 1`.

### Pin, not "any revision"
**Source:** `vox-populi/src/mens/mod.rs` L46 (`DEFAULT_MODEL_ID`), `hub.rs` L48-51 (`rsplit_once('@')`). Counter-examples (do not copy): `hub::is_model_cached` L475, catalog any-snapshot fallback.
**Apply to:** `detect_inputs`.

### Paths and safety
**Source:** `vox-scaling-policy` `DEFAULT_MENS_RUNS_V1` (L70); `vox_bounded_fs::read_utf8_path_capped` (`lib.rs` L21); repo rules (no hard-coded `.vox/`, `Path`/`join` not string concat, LF line endings, no `env::var` for secrets - `HF_HUB_CACHE`/`HF_HOME` are layout vars and are allowed).
**Apply to:** all new Rust. No new `VOX_*` env var, so `contracts/config/env-vars.v1.yaml` is untouched.

### Test-first and mutation checks
**Source:** `AGENTS.md` Test-First Policy; RESEARCH.md Validation Architecture (mutation list: any-snapshot match, drop canonical-path equality, absent branch calls scorer, absence builder inserts `"accuracy"`, drop `--out` refusal, drop pre-write `remove_file`).
**Apply to:** every new `pub`/`pub(crate)` fn. Guard each mutation with a before/after presence check.

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|----------------|------|-----------|--------|
| Pinned-revision HF cache detection (`detect_inputs`, snapshot completeness from `model.safetensors.index.json` `weight_map`, dangling-symlink check) | utility | file-I/O | Existing helpers either ignore the revision (`is_model_cached`) or fall back to any snapshot (catalog); only the hub-root env order and layout string are reusable. Planner uses RESEARCH.md Pattern 1. |
| Adapter base-pin check (canonicalize `base_model` vs snapshot) | utility | file-I/O | No host-side code reads `adapter_manifest.json`; only the plugin does, with different semantics. RESEARCH.md Pattern 2. |
| Host-side `unload_model` call | service | request-response | No host code calls it today (RESEARCH Pitfall 4); the trait method exists at `ml_backend.rs` L43. |

## Metadata

**Analog search scope:** `crates/vox-ml-cli/src/commands/mens/` (`eval_collateral.rs`, `reasoning_eval.rs`, `mod.rs`, `populi/{dispatch,mens_tail_subcommands}.rs`), `crates/vox-eval/src/{reasoning_bench,reasoning_slice}.rs`, `crates/vox-populi/src/mens/{mod,hub}.rs`, `crates/vox-orchestrator/src/catalog.rs`, `crates/vox-ml-cli/src/commands/oratio_cmd.rs`, `crates/vox-plugin-mens-candle-core/src/weight_resolution.rs`, `crates/vox-plugin-api/src/extensions/ml_backend.rs`, `crates/vox-scaling-policy/src/lib.rs`, `crates/vox-bounded-fs/src/lib.rs`.
**Tracked-source gate:** `git ls-files` confirmed tracked for `reasoning_eval.rs`, `eval_collateral.rs`, `reasoning_bench.rs`, `dispatch.rs`, `mens_tail_subcommands.rs`, `weight_resolution.rs`, `oratio_cmd.rs`, `catalog.rs`, `hub.rs`. No `.gsd/capabilities` mirror paths are used.
**Working-tree note:** the tree has unrelated dirty files (`eval_local_prompt.rs`, `train_arm.rs`, `trial_flywheel.rs`, ...); none of the analogs above are among them. Line numbers are from the working tree on 2026-10-26.
**Pattern extraction date:** 2026-10-26
