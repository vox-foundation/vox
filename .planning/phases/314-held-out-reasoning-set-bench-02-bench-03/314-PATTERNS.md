# Phase 314: Held-out Reasoning Set - Pattern Map

**Mapped:** 2026-10-26
**Files analyzed:** 13 new/modified
**Analogs found:** 12 / 13 (one file has no code analog; it is data)

Scope: BENCH-02 (three checked-in 32-item slices, Vox items checkable and not the Plan / Syntax / Implementation template) and BENCH-03 (a missing model writes no score; the report records `n`). Phases 315 and 316 are not planned here. No score is produced or invented by anything below; a green test is not a model score.

All analog paths below were verified present in git or on disk this session. `reasoning_eval.rs` is the one analog that is **untracked** (`??`), see Commit Hygiene.

## Commit Hygiene (read before planning any task that stages files)

`git status --short` shows a large dirty tree unrelated to this phase (`.planning/`, `contracts/`, `.github/`, many `crates/`). Every task must `git add <explicit path>` and check `git diff --cached --stat` before commit. Never `git add -A` / `git add .` / `git commit -a`.

| Path | State now | What this phase may stage |
|------|-----------|---------------------------|
| `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` | untracked (`??`) | the whole file (it is the scorer) |
| `crates/vox-ml-cli/src/commands/mens/mod.rs` | modified, `+2` | only the `#[cfg(feature = "gpu")] pub(crate) mod reasoning_eval;` hunk (diff is exactly those 2 lines, at line 26-27) |
| `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` | modified, `+36`, 2 hunks (`@@ -111` and `@@ -118`) | both hunks: `cached_reasoning_report` and the `score_bench` reasoning branch. Nothing else in the file is dirty |
| `crates/vox-ml-cli/src/commands/mens/eval_local_prompt.rs` | modified | **do not stage** (the `prepare_bench_item` edit is explicitly not this phase's contract) |
| `crates/vox-ml-cli/src/commands/mens/populi/train_arm.rs` | modified | **do not stage** |
| other dirty files (`train.rs`, `merge_qlora.rs`, ...) | modified | **do not stage** |
| `.gitignore` | tracked, must be edited | one negation line, see below |
| `mens/data/{gsm8k,math500,vox}_think/reasoning.jsonl` | on disk but gitignored (`.gitignore:189 *.jsonl`); `vox_think` does not exist yet | add negation, then `git add` all three by path |

Expected `git diff --cached --stat` for the "commit the scorer" task: exactly three files (`reasoning_eval.rs`, `mens/mod.rs`, `eval_collateral.rs`). If `mod.rs` or `eval_collateral.rs` ever shows more than the hunks above, use `git add -p` and stage only those hunks.

Rustfmt note: `mod.rs` currently has `pub(crate) mod reasoning_eval;` ahead of `pub(crate) mod eval_gate;`, which is not alphabetical. Do not reorder it in this phase (it would widen the hunk); run `vox run scripts/fmt.vox` only if the formatter touches the staged files.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|-------------------|------|-----------|----------------|---------------|
| `.gitignore` | config | n/a | `.gitignore:189-197` (existing `!contracts/kb/eval/*.jsonl` negation) | exact |
| `mens/data/gsm8k_think/reasoning.jsonl` | data fixture | file-I/O (read by scorer) | itself (64 rows on disk, keep first 32) | exact |
| `mens/data/math500_think/reasoning.jsonl` | data fixture | file-I/O | itself (already 32 rows, unchanged content) | exact |
| `mens/data/vox_think/reasoning.jsonl` | data fixture | file-I/O | `mens/data/gsm8k_think/reasoning.jsonl` row shape; `examples/golden/*.vox` for Vox style | role-match |
| `crates/vox-eval/src/reasoning_bench.rs` (new) | utility (loader + validator + outcome type) | transform / file-I/O | `crates/vox-eval/src/qwen_reasoning.rs` (types, tests-in-file) and `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs::load_items` (logic to lift) | role-match |
| `crates/vox-eval/src/lib.rs` | config (module registry) | n/a | its own `pub mod qwen_reasoning;` line (lib.rs:14) | exact |
| `crates/vox-compiler/tests/vox_think_oracle_test.rs` (new) | test | request-response (run program, compare value) | `crates/vox-compiler/tests/eval_typeck_parity_test.rs` (`run_probe`) | exact |
| `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` | service (model-driving scorer) | batch / request-response | itself (untracked, to be committed then edited) | exact |
| `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` | service (gate wiring) | request-response | itself (`score_bench`, `cached_reasoning_report`, its own `#[cfg(test)]` block) | exact |
| `crates/vox-ml-cli/src/commands/mens/mod.rs` | config (module registry) | n/a | neighbouring `#[cfg(feature = "gpu")] pub mod eval_collateral;` | exact |
| `contracts/config/env-vars.v1.yaml` (only if the `VOX_REASONING_EVAL_*` vars are kept) | config / contract | n/a | any existing `- name: "VOX_..."` entry, lines 3-9 | exact |
| `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` | doc | n/a | its own "Update (2026-10-26)" section | exact |
| tests inside `reasoning_eval.rs` (none exist today) | test | file-I/O | `eval_collateral.rs` `mod tests` (tempfile + `assert!(!path.exists())` pattern) | role-match |

## Pattern Assignments

### `.gitignore` (config)

**Analog:** `.gitignore:186-197`, the existing negations directly under `*.jsonl`.

```gitignore
*.jsonl
# Exception: test fixtures must be committable without `git add -f`.
# Scoped to **/tests/fixtures/** so target/ JSONL/log output stays ignored.
!**/tests/fixtures/**/*.jsonl
!**/tests/fixtures/**/*.log
# Exception: the KB retrieval-eval golden set (corpus snapshot + queries) is a tracked contract.
!contracts/kb/eval/*.jsonl
```

**Copy:** add, after line 189 and following the same comment-then-negation style:

```gitignore
# Exception: the held-out reasoning slices (gsm8k_think, math500_think, vox_think) are tracked bench inputs.
!mens/data/*_think/reasoning.jsonl
```

Scope it to `reasoning.jsonl` under `*_think`. Do not negate `mens/data/*.jsonl` (that would un-ignore `train.jsonl` / `synthetic.jsonl`, which are large and must stay ignored). Verify with `git check-ignore -v mens/data/vox_think/reasoning.jsonl` (exit 1 = not ignored) and that `git check-ignore -q mens/data/train.jsonl` still exits 0. Do not use `git add -f`.

---

### `mens/data/{gsm8k,math500,vox}_think/reasoning.jsonl` (data fixtures)

**Analog:** the two existing files. Row shape (verified):

```json
{"id": "gsm8k-test-0000", "question": "...", "answer": "18", "source": "openai/gsm8k test (MIT), first 64 rows"}
{"id": "test/precalculus/807.json", "question": "...", "answer": "\\left( 3, \\frac{\\pi}{2} \\right)", "source": "HuggingFaceH4/MATH-500, first 32 rows"}
```

Facts to carry into the plan:
- `gsm8k_think` has **64** rows on disk (`wc -l`); lock is the **first 32** (deterministic prefix, same items, not a re-sample). Cut with a one-off `head -n 32` into a temp file then move; this is a command, not a committed script. The `source` string on every row says "first 64 rows". Decide explicitly: either rewrite it to "first 32 rows" (changes 32 lines) or leave it. If left, the pin test (first/last id + count) is what documents the cut. Either way, do not change `id`, `question`, `answer`.
- `math500_think` has **32** rows; content unchanged. Its licence is unverified (RESEARCH A2); do not add a licence claim to `source` unless confirmed.
- `vox_think` does not exist. 32 hand-authored rows. Keys: `id`, `question`, `answer`, `source` (`"hand-authored"`), plus `program` and `kind` (extras are ignored by `load_items`, which reads only `id`/`question`/`answer`).
- LF endings, trailing newline, UTF-8 (`vox ci line-endings` on the diff).
- Item design (from RESEARCH Pattern 3): kinds `trace` / `debug` / `plan-check`; integer golds; the question must end by asking for the final value alone on the last line; no Plan / Syntax / Implementation markers; no "generate a `X` construct in Vox" shape. Programs pure (no `fs`, `http`, `process`, `env`, `net`, `random`, `time`), bare-keyword Vox syntax, `@pure fn` as decorator form. Not byte-identical to any `examples/golden/*.vox`.
- Do not hand-compute golds. Author the program, run it through the oracle test, then write the printed value into `answer`.

---

### `crates/vox-eval/src/reasoning_bench.rs` (new; utility, transform + file-I/O)

**Analog 1 (structure and test placement):** `crates/vox-eval/src/qwen_reasoning.rs`. Module doc comment, `#[must_use]` on pure fns, `Debug/Clone/PartialEq` derives, tests in the same file under `#[cfg(test)] mod tests { use super::*; ... }` (qwen_reasoning.rs:328 onward). Test-First policy requires a failing `#[test]` per new `pub fn` in the same file.

**Analog 2 (error type):** `crates/vox-eval/src/compare.rs:153-159` uses `Result<T, String>` with `format!` messages, no `anyhow` (vox-eval has no `anyhow` dependency; deps are serde, serde_json, regex, tempfile, log, xxhash-rust). Mirror that:

```rust
pub fn compare(arms: &[ArmInput], baseline: &str, opts: &CompareOptions) -> Result<Comparison, String> {
    ...
    return Err(format!("arm label {:?} supplied twice", dup.label));
```

**Analog 3 (logic to lift, do not re-invent):** `reasoning_eval.rs:128-169`, `load_items`. Reuse its parsing rules (skip blank lines, `serde_json::from_str` with `path:lineno` context, reject empty `question`/`answer`, default id `item-{lineno}`), but tighten for a pinned slice: **reject a missing/empty id rather than defaulting**, reject duplicate ids, reject a count other than `expected_n`.

```rust
// reasoning_eval.rs:137-155 (parse + non-empty check to mirror)
let v: serde_json::Value = serde_json::from_str(line)
    .with_context(|| format!("{}:{}", path.display(), lineno + 1))?;
let question = v.get("question").and_then(|x| x.as_str()).unwrap_or("").to_string();
let answer   = v.get("answer").and_then(|x| x.as_str()).unwrap_or("").to_string();
if question.is_empty() || answer.is_empty() { anyhow::bail!("{}:{} needs non-empty question and answer", ...); }
```

In `vox-eval` translate `with_context`/`bail!` to `map_err(|e| format!("{}:{}: {e}", path.display(), lineno + 1))` / `return Err(format!(...))`.

**Types to define** (names are a recommendation from RESEARCH; the planner may rename):
- `pub const SLICE_SIZE: usize = 32;` and `pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];`
- `pub struct ReasoningItem { pub id: String, pub question: String, pub answer: String }`
- `pub fn load_slice(path: &Path, expected_n: usize) -> Result<Vec<ReasoningItem>, String>`
- `pub enum BenchOutcome { Scored(ReasoningScore), NotScored { reason: String, n_requested: usize } }` with `to_json()`.

**Reuse, do not duplicate** (locked): `vox_eval::qwen_reasoning::{ReasoningScore, score_completions, grade_final_answer}`.

```rust
// qwen_reasoning.rs:187-196 - the type to wrap, never to fork
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReasoningScore { pub accuracy: f64, pub think_initiation_rate: f64, pub n: usize }
```

**The one genuinely new rule** (qwen_reasoning.rs:199-221): `score_completions` returns `{0.0, 0.0, n: 0}` for an empty batch. That tuple must not be storable as a result. Give `BenchOutcome::Scored` a constructor that returns `NotScored` (or `Err`) when `n == 0`, and keep the `Scored` variant unreachable from an empty batch. Do not change `score_completions` itself (phase 313 owns it; its 19 tests are the regression net).

**Report JSON shape (BENCH-03, locked):**
- Scored: `{"status":"scored","n":N,"accuracy":..,"think_initiation_rate":..}` with `N >= 1`.
- NotScored: `{"status":"not_scored","n":0,"n_requested":R,"reason":".."}`. The keys `accuracy` and `think_initiation_rate` are **absent** (not `null`, not `0`). Build with `serde_json::json!` per variant; do not use one struct with `Option` fields and `skip_serializing_if`, because that leaves a code path that could set them.

**Test shape to copy** (from RESEARCH Code Examples; this is the required BENCH-03 key-absence test):

```rust
#[test]
fn not_scored_json_has_no_score_keys() {
    let v = BenchOutcome::NotScored { reason: "load failed".into(), n_requested: 32 }.to_json();
    assert_eq!(v["status"], "not_scored");
    assert_eq!(v["n"], 0);
    assert!(v.get("accuracy").is_none());
    assert!(v.get("think_initiation_rate").is_none());
}
```

Other tests required here (VALIDATION rows 314-01-01/02/04/06, 314-02-01/02/03): 32 rows + unique ids + non-empty fields for each of the three files; template-marker guard (`Plan:`, `Syntax considerations`, `Implementation:`, and the "generate a ... construct in Vox" shape) over every `vox_think` question; GSM8K/MATH-500 prefix pin (count, first id, last id); 31- and 33-row slices refused; `Scored` unbuildable from an empty batch; and the gold round trip over all 96 items:

```rust
let ok = format!("<think>\nwork\n</think>\n\n\\boxed{{{}}}", item.answer);
assert!(grade_final_answer(&ok, &item.answer));
assert!(!grade_final_answer("<think>\nwork\n</think>\n\n\\boxed{999999}", &item.answer));
```

If a pre-existing MATH-500 gold fails the round trip, report it; do not edit the dataset row (RESEARCH A3).

Path to the data in tests: `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../mens/data").join(name).join("reasoning.jsonl")`. Pattern already used in `crates/vox-compiler/tests/emission_ladder_test.rs:23` (`PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")`). Never use a bare `"mens/data"` in a test (cwd is the crate dir under `cargo test`).

Size cap on the read (ASVS V5): `vox-eval` has no `vox_bounded_fs` dependency, and the dependency-discipline rule forbids adding a workspace crate edge without a user-authorized exception. Use an explicit `metadata().len()` cap before `read_to_string` inside `vox-eval`; do not add the edge.

---

### `crates/vox-eval/src/lib.rs` (config)

**Analog:** its own module list.

```rust
// lib.rs:9-16
pub mod compare;
pub mod corpus_score;
pub mod corpus_stats;
pub mod mens;
pub mod provenance;
pub mod qwen_reasoning;
pub mod report;
pub mod trial_flywheel;
```

**Copy:** insert `pub mod reasoning_bench;` between `qwen_reasoning` and `report` (alphabetical). One line, no other edit. No new `Cargo.toml` entry is needed.

---

### `crates/vox-compiler/tests/vox_think_oracle_test.rs` (new; test, run-program-and-compare)

**Analog:** `crates/vox-compiler/tests/eval_typeck_parity_test.rs` (top-level integration test in `tests/`, not under `tests/it/`).

**Imports pattern** (lines 15-16):

```rust
use vox_compiler::eval::value::VoxValue;
use vox_compiler::{eval, hir, lexer, parser};
```

**Core pattern to copy verbatim** (`run_probe`, lines 18-31). This is the single sanctioned call sequence; do not invent another:

```rust
fn run_probe(source: &str) -> Result<VoxValue, String> {
    let tokens = lexer::lex(source);
    let module = parser::descent::parse(tokens).map_err(|e| format!("parse: {e:?}"))?;
    let lowered = hir::lower::lower_module(&module);
    let mut interp = eval::Interpreter::new(1_000_000);
    interp.run_module(&lowered).map_err(|e| format!("run: {e:?}"))?;
    interp.call("main", vec![]).map_err(|e| format!("call: {e:?}"))
}
```

**Value comparison:** `VoxValue` (`crates/vox-compiler/src/eval/value.rs:5`) is `#[derive(Debug, Clone)]` with `Int(i64)`, `Str(Rc<str>)`, `Bool(bool)`, and others; RESEARCH reports no `Display` impl, so match on the variant. `assert_eq!(res, VoxValue::Bool(true))` in the analog shows `PartialEq` exists. For an integer gold: `match v { VoxValue::Int(n) => assert_eq!(n.to_string(), item.answer), other => panic!(...) }`. Include `item.id` in every failure message so a bad row is findable.

**Data access:** read `mens/data/vox_think/reasoning.jsonl` via `serde_json::from_str` per line. `vox-compiler` already has `serde_json` (Cargo.toml:20), and `vox-eval` is already a regular dependency (Cargo.toml:15), so `vox_eval::reasoning_bench::load_slice` may be used here as well with **no new crate edge**. Prefer reusing `load_slice` so the oracle and the scorer parse identically; fall back to raw `serde_json` only if that creates friction. Each row's `program` is the source passed to `run_probe`.

**Purity guard** (RESEARCH security table): before running, reject any `program` containing `fs`, `http`, `process`, `env`, `net`, `random`, `time` identifiers, so a data file cannot make the interpreter do side-effectful work. Add a dedicated `#[test]` for the guard so it is not dead code.

**Mutation check (manual, once):** change one `answer`, run `cargo test -p vox-compiler --test vox_think_oracle_test`, expect FAIL, restore. Guard the edit with a before/after presence check (AGENTS.md mutation rule).

---

### `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (service; commit first, then edit)

**Analog:** itself. Phase 313 wrote it and it has never been committed. Step 1 is to commit it unchanged (with the two wiring hunks); step 2 edits it. The edits mirror the existing structure.

**Imports pattern** (lines 10-13): `anyhow::{Context, Result}`, `std::path::Path`, `vox_eval::qwen_reasoning::{self, ReasoningScore}`. Keep `anyhow` here (this crate uses it); add `vox_eval::reasoning_bench::{...}` alongside.

**Replace the private `Item` + `load_items`** (lines 28-32, 128-169) with `vox_eval::reasoning_bench::load_slice`. The path for the scorer already arrives as `items_path`; pass `SLICE_SIZE` as `expected_n` **unless** `VOX_REASONING_EVAL_LIMIT` is set (see env var note).

**Load-failure pattern to change** (lines 57-72). Today every failure in this block exits via `?` before `write_report`, so no file records `n`:

```rust
let loaded = vox_plugin_host::cached_code_plugin(plugin_id).with_context(|| { ... })?;
let backend = loaded.plugin.as_ml_backend().into_option().ok_or_else(|| anyhow::anyhow!("..."))?;
let handle = backend.load_model(adapter.to_string_lossy().as_ref().into())
    .into_result()
    .map_err(|e| anyhow::anyhow!("load_model({}): {e}", adapter.display()))?;
```

**Copy the shape:** isolate the plugin lookup + `load_model` into a helper returning `Result<..>`; on `Err`, build `BenchOutcome::NotScored { reason: e.to_string(), n_requested: items.len() }`, write it (see below), then `return Err(...)` with a message containing `scored 0 of N items`. **Still return `Err`.** `run_collateral_damage_with` propagates scorer errors and `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` (eval_collateral.rs tests) asserts no `collateral_damage_report.json` on `Err`; returning `Ok` with a score of 0 would break that.

**Per-item inference error pattern to change** (lines 84-100). Today `Err(e)` and a JSON-parse miss both become an empty completion that is then graded and counted:

```rust
Err(e) => { eprintln!("  {bench} {} inference error: {e}", item.id); String::new() }
```

Recommended (RESEARCH Open Question 2, assumption A6, medium risk): any inference error or unparseable response makes the whole run `NotScored`, `reason` naming the first failing item id, with `n_requested` and `n_inference_errors` recorded. Never push an errored item into `rows`. The planner must pick strict vs. partial explicitly and record the choice in the plan; strict is the safer default for a gate.

**Report writer to mirror** (lines 171-210, `write_report`). It does `create_dir_all(adapter/collateral_eval/<bench>)` then writes `reasoning_report.json`. For the `NotScored` path:
- Write to the **same** file name so a stale scored report is overwritten (BENCH-03 test: write scored, then not-scored, assert `status == "not_scored"` and no `accuracy` key).
- Write only when `adapter.is_dir()`. For a nonexistent adapter path, `create_dir_all` would fabricate an adapter-looking directory that `serve` could mistake for one; skip the write, return the `Err`, and assert in a test that the path still does not exist.
- Scored path: keep the existing `items` array and keys, route the top-level keys through `BenchOutcome::to_json()` so scored and not-scored share one writer. Keep `benchmark`, `temperature`, `prompt` keys.

**`ReasoningReport` struct** (lines 15-26; `#[derive(Debug, Clone, Copy)]`, `n` marked `#[allow(dead_code)]`): `eval_collateral.rs::cached_reasoning_report` returns this by `Copy`. Keep that signature so the gate wiring stays stable. A `ReasoningReport` should only be constructed from `BenchOutcome::Scored`.

**Env vars** (lines 40, 51): `VOX_REASONING_EVAL_LIMIT` and `VOX_REASONING_EVAL_MAX_TOKENS` are read with `std::env::var` and appear in no contract. Policy: a new `VOX_*` var requires `contracts/config/env-vars.v1.yaml`. Either (a) delete the LIMIT override (preferred: a truncated run is not comparable to a 32-item baseline, and it conflicts with `expected_n = 32`), or (b) register both. If kept, `n` must reflect the truncation and phase 316 must not set it. Do not add any new env var. Do not use `std::env::var("API_KEY")`-style secret reads (none here).

**Tests to add** (the file has none; `#[cfg(test)] mod tests` at the bottom, run with `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval`, because the module is `#[cfg(feature = "gpu")]` and a filter without the feature matches nothing and exits 0 with `0 passed`). Mirror the `tempfile::tempdir()` style of `eval_collateral.rs` tests:

```rust
let dir = tempfile::tempdir().unwrap();
...
assert!(!dir.path().join("collateral_damage_report.json").exists(), "...");
```

- Not-scored write replaces a stale scored `reasoning_report.json` in the same dir.
- A nonexistent adapter path returns `Err` and the path still does not exist afterward.
- Build item paths from `env!("CARGO_MANIFEST_DIR")`, joined with `../../mens/data/...`, never `"mens/data"` (Pitfall 8).

**Mutation check (manual, once):** replace the `NotScored` branch with a pass-through of `ReasoningScore { accuracy: 0.0, think_initiation_rate: 0.0, n: 0 }`; the key-absence test must fail; restore.

---

### `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (service; wiring)

**Analog:** itself. Two uncommitted hunks (`cached_reasoning_report`, lines 114-135; and the reasoning branch in `score_bench`, lines 143-155).

**Existing pattern** (lines 143-155):

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

Planning consequences:
- The identifier the gate loads is `mens/data/<dir_name>/reasoning.jsonl`. The slices must be named exactly `gsm8k_think`, `math500_think`, `vox_think` (and `<name>_init` reuses the same file and one cached generation). A different file name silently falls through to the manifest branch and bails.
- `BENCH_ROOT` is the relative `"mens/data"` (line 9), resolved against the process cwd. No change needed in this phase; just do not write tests that go through `score_bench` with a relative root.
- The cache (lines 114-135) stores only `Ok` reports. With the new `Err`-on-`NotScored` behaviour, a failed run is never cached, so a later retry re-attempts the load. Do not "fix" this by caching the error.
- The only change this phase may need here is none beyond committing the existing hunks. If the planner wants a `scored 0 of N items` assertion at the gate level, extend the existing test pattern below rather than changing `run_collateral_damage_with`.

**Existing test to keep green** (BENCH-03 row 314-02-06, already passing, 4 tests in the module): `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` (`let mut scorer = |_: &str| anyhow::bail!("adapter not found");` then `assert!(run_collateral_damage_with(...).is_err());` and `!dir.path().join("collateral_damage_report.json").exists()`).

---

### `crates/vox-ml-cli/src/commands/mens/mod.rs` (config)

**Analog:** the adjacent gated module declarations (lines 21-27). The 2-line hunk is already in the working tree:

```rust
#[cfg(feature = "gpu")]
pub mod eval_collateral;
#[cfg(feature = "gpu")]
pub(crate) mod reasoning_eval;
pub(crate) mod eval_gate;
```

No edit needed; stage exactly this hunk. Keep the `gpu` gate: removing it would pull the plugin host into default builds.

---

### `contracts/config/env-vars.v1.yaml` (only if the planner keeps the env vars)

**Analog:** any entry, e.g. lines 3-9:

```yaml
  - name: "VOX_ROUTE_POLICY_PROFILE"
    owner_crate: "vox-actor-runtime"
    kind: "string"
    required: false
    introduced_in: "0.3.0"
    description: "Routing capability policy profile selector (for example balanced or restricted)."
```

Copy the same six keys for `VOX_REASONING_EVAL_MAX_TOKENS` (`owner_crate: "vox-ml-cli"`, `kind: "string"` or the integer kind used elsewhere in the file; check an existing integer entry before choosing). Then `vox ci command-sync` / the `ssot-autoregen` bot owns the generated docs; do not hand-edit `docs/src/reference/env-vars.generated.md`. Preferred plan: delete the LIMIT override and register only `MAX_TOKENS`, or delete both reads.

---

### `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` (doc)

**Analog:** its own "Update (2026-10-26)" section. Its statements that the slices are "Checked in" with "first 64 rows" of GSM8K are stale against the locked 32; update that wording only. Keep the existing frontmatter untouched; do not hand-add `last_updated`. Mark non-compiling snippets with ` ```text `. Scoped lint: `cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`.

## Shared Patterns

### Fail closed, count what was scored
**Source:** new `BenchOutcome` in `crates/vox-eval/src/reasoning_bench.rs`; consumed by `reasoning_eval.rs`.
**Apply to:** the load-failure branch, the per-item inference-error branch, and the zero-items case.
`NotScored` has no score fields at all, `n` is `0`, `n_requested` records the intent, and the function still returns `Err` so `collateral_damage_report.json` is never written.

### Reuse the phase 313 grader, never fork it
**Source:** `crates/vox-eval/src/qwen_reasoning.rs` (`thinking_mode_prompt` :145, `answer_region` :153, `grade_final_answer` :169, `score_completions` :199, `ReasoningScore` :187).
**Apply to:** `reasoning_bench.rs` tests, `reasoning_eval.rs`, the oracle test (indirectly through the gold round trip).
Grading rule to design golds around: numeric gold matches the **last number** in the answer region (after `</think>`); non-numeric gold matches a `\boxed{}` body or the last non-empty line. Hence integer golds for `vox_think`, and a question that asks for the final value alone on the last line.

### Tests live in the file; `0 passed` is a failed run
**Source:** `qwen_reasoning.rs:328` (`#[cfg(test)] mod tests { use super::*; }`), `eval_collateral.rs` `mod tests`.
**Apply to:** every new `pub fn`.
Commands, with the expected "N >= 1" rule:
- `cargo test -p vox-eval --lib reasoning_bench`
- `cargo test -p vox-compiler --test vox_think_oracle_test`
- `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` and `-- eval_collateral` (the `gpu` feature is mandatory)
- `cargo check -p vox-ml-cli --features gpu --tests`
No workspace-wide cargo, no `cargo clippy --workspace` locally (AGENTS.md). Plain `cargo` on PATH.

### Absolute test paths from `CARGO_MANIFEST_DIR`
**Source:** `crates/vox-compiler/tests/emission_ladder_test.rs:23`.
**Apply to:** every test that reads `mens/data/*`.
`PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")`.

### Mutation-verify the guards
**Apply to:** the oracle test (change one gold, expect FAIL) and the `NotScored` key-absence test (substitute a zero score, expect FAIL). Check the edit is present before running, restore afterwards.

### LF endings, no stray staging
**Apply to:** all three JSONL files and every Rust file touched. Run `vox ci line-endings` on the diff. Stage by explicit path only.

## No Analog Found

| File / Concern | Role | Data Flow | Reason |
|----------------|------|-----------|--------|
| `vox_think` item content (the 32 programs, questions, golds) | data | n/a | No existing held-out Vox reasoning set. The only near neighbours are `examples/golden/*.vox` (style reference, and a byte-identity exclusion list) and the Plan / Syntax / Implementation corpus from `generate_cot_organic_corpus` (`crates/vox-corpus/src/codegen_vox/part_03.rs`), which is the shape to avoid. Authoring work, verified by the oracle test. |

## Out of Scope (do not plan)

- Serve gate / refusal when the collateral report is missing or worse (phase 315). `populi/dispatch.rs:350-372` reads the report; leave it.
- Live base-versus-adapter run, any numeric baseline or before/after score (phase 316). The pinned Qwen3-8B snapshot and an adapter run directory are not on this machine; the 0.6B models in the HF cache must not be used to produce a number.
- `prepare_bench_item` / `eval_local_prompt.rs` edits, `train.rs`, `merge_qlora.rs`, `train_arm.rs` and the rest of the dirty tree.
- Changing `score_completions` / `grade_final_answer` behaviour.
- Corpus-wide n-gram leakage screening (`eval_gate::leakage::assert_no_text_leakage`); optional and local-only because `train.jsonl` is gitignored. The in-repo required checks are the template-marker guard and non-identity with `examples/golden/`.

## Metadata

**Analog search scope:** `crates/vox-eval/src`, `crates/vox-ml-cli/src/commands/mens`, `crates/vox-compiler/tests` and `src/eval`, `mens/data`, `.gitignore`, `contracts/config`, `examples/golden`.
**Files read:** `reasoning_eval.rs` (full), `eval_collateral.rs` (lines 1-260), `qwen_reasoning.rs` (key ranges), `vox-eval/src/lib.rs`, `vox-eval/Cargo.toml`, `compare.rs` (error style), `eval_typeck_parity_test.rs` (1-60), `eval/value.rs` (1-30), `mens/mod.rs` diff, `.gitignore` 185-197, both existing slice files (head), `env-vars.v1.yaml` (head).
**Verified this session:** `git status` (scorer untracked, 3 files carry the wiring), `git diff --stat` for `eval_collateral.rs` (+36, two hunks), `wc -l` of the slices (64 and 32), `vox_think` absent, `git ls-files mens/data` (only `heldout_bench/manifest.json` and `metadata.json` tracked), `vox-compiler` already depends on `vox-eval` and `serde_json`.
**Pattern extraction date:** 2026-10-26
