# Phase 314: Held-out Reasoning Set - Research

**Researched:** 2026-10-26
**Domain:** Rust eval harness (`vox-eval`, `vox-ml-cli`), checked-in JSONL bench slices, a Vox-interpreter answer oracle
**Confidence:** HIGH on repo state and mechanics; MEDIUM on the Vox item design (authoring work, not research)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Check in three slices named `gsm8k_think`, `math500_think`, and `vox_think`.
- Tests and the scorer read those checked-in files. They do not download items at test time.
- Every Vox item has a checkable answer.
- Vox items are not the fixed three-line Plan / Syntax / Implementation template.
- Each slice has 32 items.
- The report stores `n`.
- This phase does not run the full GSM8K or MATH-500 sets.
- A failed load writes no accuracy number and no think-initiation number.
- The report records how many items were scored. A failed load scores 0 items.
- A green unit test is not a passing score.
- Out of scope: no serve gate (phase 315), no base-versus-adapter run (phase 316), no new base model, no new training recipe.

### Claude's Discretion
- Exact directory and file extension, as long as the three names above are the identifiers the scorer loads.
- How a Vox item's answer is checked, as long as it is checkable and is not the Plan / Syntax / Implementation template.

### Deferred Ideas (OUT OF SCOPE)
- Serve refusal when the collateral report is missing or worse (phase 315).
- The live base-versus-adapter comparison, or an honest "snapshot absent" record with no invented numbers (phase 316).
- Swapping the pinned Qwen3-8B base, or trying a new training technique.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| BENCH-02 | A held-out Vox slice with checkable answers is scored the same way. Its items are not the fixed three-line Plan / Syntax / Implementation template. | Slice file format and location (Architecture Patterns), Vox oracle test run through the interpreter (Pattern 3), template-marker guard and gold round-trip test (Validation Architecture) |
| BENCH-03 | A missing model or a failed load writes no passing score. The report records how many items were scored. | Fail-closed outcome type and report shape (Pattern 2), Pitfalls 3-6, the `Err` path that already keeps `collateral_damage_report.json` from being written |
</phase_requirements>

## Summary

The scorer for the thinking benches already exists, but three things the success criteria depend on are false today. **(1) The GSM8K and MATH-500 "checked-in" slices are not checked in.** `mens/data/gsm8k_think/reasoning.jsonl` and `mens/data/math500_think/reasoning.jsonl` exist on this disk, but root `.gitignore` line 189 (`*.jsonl`) ignores them and `git ls-files mens/data` lists only `heldout_bench/manifest.json` and `metadata.json`. **(2) The GSM8K file has 64 rows, not 32.** The locked decision is 32 each, so the file must be cut to its first 32 rows (still the same items, not a new sample). MATH-500 already has 32. **(3) The scorer is not committed.** `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` is untracked (`??`), and the `mod reasoning_eval` line and the `score_bench` branch that reaches it are uncommitted working-tree edits. This phase has to commit them as part of making the slices the thing the scorer loads.

BENCH-03 needs real code, not only tests. Today a failed `load_model` returns `Err` before any report is written, so no passing score leaks, but **nothing records `n = 0`**, and a stale `reasoning_report.json` from an earlier successful run is left in place. Worse, a per-item inference error is turned into an empty completion and graded wrong, so a backend that loads but cannot generate writes `accuracy: 0.0` with `n: 32`. That is a score that looks real. The fix is a small, pure outcome type (`Scored(ReasoningScore)` or `NotScored { reason }`) whose JSON has **no `accuracy` and no `think_initiation_rate` keys** when not scored, plus a loader that rejects any slice that is not exactly 32 valid items.

**Primary recommendation:** Put the slice loader, the 32-item validator, and the fail-closed report type in `vox-eval` (default features, no GPU, no new crate edge) so they are testable without a model. Keep the model-driving loop in `vox-ml-cli::reasoning_eval` and make it build a `NotScored` outcome on load failure, on zero successful generations, and on any inference error. Prove Vox answers by running each item's embedded program through the interpreter in a `vox-compiler` integration test. Never write a baseline file with numbers.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Slice items (JSONL data) | Repo data files (`mens/data/<name>/reasoning.jsonl`) | — | The collateral scorer already resolves `mens/data/<name>/reasoning.jsonl` (`eval_collateral.rs:148`) |
| Slice parse and 32-item validation | `vox-eval` (layer 2, pure) | — | No GPU, no model, runs under default features; `vox-ml-cli` already depends on `vox-eval` |
| Grading and score math | `vox-eval::qwen_reasoning` (existing) | — | Locked: reuse `score_completions`, `grade_final_answer`, `ReasoningScore` |
| Fail-closed outcome and report JSON | `vox-eval` (pure type + `to_json`) | `vox-ml-cli::reasoning_eval` (writes the file) | Keeps the "no accuracy key when not scored" rule unit-testable without a model |
| Model load and generation | `vox-ml-cli::reasoning_eval` via the `MlBackend` plugin | — | Already there; `gpu`-gated |
| Vox answer truth | Vox interpreter (`vox-compiler::eval`) | `vox-compiler/tests/` | Gold answers are produced and re-checked by running the program, not by hand |
| Gate verdict | `vox-ml-cli::eval_collateral` (existing) | — | Propagates `Err`; phase 315 owns serve behaviour, not this phase |

## Standard Stack

No new external packages. Everything is in-tree.

### Core
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| `vox_eval::qwen_reasoning` | workspace (0.x) | `score_completions`, `grade_final_answer`, `opened_nonempty_think`, `thinking_mode_prompt`, `ReasoningScore` | Locked decision; 19 unit tests passed at phase 313 verification |
| `serde_json` | workspace | Parse JSONL, write report | Already a `vox-eval` dependency (`crates/vox-eval/Cargo.toml`) |
| `vox_compiler::{lexer, parser, hir, eval}` | workspace | Run a Vox program and read `VoxValue` | Same probe pattern as `crates/vox-compiler/tests/eval_typeck_parity_test.rs:19-30` |

### Supporting
| Library | Version | Purpose | When to Use |
|---------|---------|---------|-------------|
| `tempfile` | workspace | Temp dirs in tests | Already a `vox-eval` dependency (and dev dep elsewhere) |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| Loader in `vox-eval` | Keep `load_items` private in `vox-ml-cli::reasoning_eval` | The `gpu` feature gates that module, so tests need `--features gpu --lib` and a heavy build; pure logic belongs where default `cargo test` reaches it |
| Interpreter oracle in `vox-compiler/tests/` | Oracle in `vox-integration-tests` | `vox-integration-tests` links the whole orchestrator/MCP graph. `vox-compiler` has `serde_json` and an existing `run_probe` pattern, and needs no new edge |
| Interpreter oracle | Hand-computed golds only | Hand golds can be wrong and nothing checks them; the interpreter makes "checkable" true by construction |

**Installation:** none.

## Package Legitimacy Audit

No external packages are installed or recommended by this phase. `gsd-tools query package-legitimacy check` was not run because there is nothing to check.

**Packages removed due to [SLOP] verdict:** none
**Packages flagged as suspicious [SUS]:** none

## Architecture Patterns

### System Architecture Diagram

```text
mens/data/<slice>/reasoning.jsonl            (checked in, 32 rows each)
        |
        v
vox_eval::reasoning_bench::load_slice(path, expected_n = 32)
        |  reject: wrong n, empty question/answer, duplicate id,
        |          Vox template markers, unreadable file
        v
Vec<ReasoningItem { id, question, answer }>
        |
        v
reasoning_eval::score_reasoning_file(bench, items_path, adapter)
        |
        |-- load_model(adapter) fails ----------------------+
        |                                                   |
        |-- for each item: thinking_mode_prompt(question)   |
        |       run_inference -> completion                 |
        |       inference error -> counted, NOT scored -----+
        v                                                   |
 score_completions(scored pairs only)                       |
        v                                                   v
 BenchOutcome::Scored(ReasoningScore{accuracy,think,n})   BenchOutcome::NotScored{reason, n: 0}
        |                                                   |
        v                                                   v
 reasoning_report.json                            reasoning_report.json
 {status:"scored", n, accuracy,                   {status:"not_scored", n:0, n_requested,
  think_initiation_rate, ...}                      reason}   <- no accuracy, no think keys
        |                                                   |
        v                                                   v
 score_bench -> Ok(f64)                            score_bench -> Err (gate writes no
                                                   collateral_damage_report.json)
```

### Recommended Project Structure

```text
mens/data/
├── gsm8k_think/reasoning.jsonl     # first 32 rows of the existing 64-row file
├── math500_think/reasoning.jsonl   # existing 32 rows, unchanged
└── vox_think/reasoning.jsonl       # new, 32 hand-authored rows
crates/vox-eval/src/reasoning_bench.rs        # new: loader, validator, BenchOutcome
crates/vox-compiler/tests/vox_think_oracle_test.rs   # new: run each Vox program, compare gold
```

**Slice file format (verified on the two existing files, not invented).** One JSON object per line, LF endings, trailing newline, UTF-8. Keys on the existing rows: `id`, `question`, `answer`, `source`. Existing ids are stable per-dataset strings (GSM8K: `gsm8k-test-0000` onward; MATH-500: `test/<subject>/<n>.json`). `load_items` in `reasoning_eval.rs` reads only `id`, `question`, `answer`, so extra keys such as `source` are ignored and safe. For `vox_think`, add `source: "hand-authored"` and, per item, a `program` (the Vox source shown in the question) and a `kind` label so the oracle test can run it. Extra keys do not change the scorer.

**Where the 32-item GSM8K slice comes from.** Cut `gsm8k_think/reasoning.jsonl` to its first 32 rows (ids `gsm8k-test-0000` to `gsm8k-test-0031`). That is the "checked-in file", reduced, not a fresh sample. Update each retained row's `source` string from "first 64 rows" to "first 32 rows" so the provenance stays true. [VERIFIED: file read this session, 64 lines, ids unique, `source` is `openai/gsm8k test (MIT), first 64 rows`]

**Un-ignoring the files.** `.gitignore:189` is `*.jsonl`. Existing precedent for tracked JSONL is a negation immediately below it (`!contracts/kb/eval/*.jsonl`, `!**/tests/fixtures/**/*.jsonl`). Add one negation, `!mens/data/*_think/reasoning.jsonl`, placed after line 189. Do not rely on `git add -f`: the repo's own comment says fixtures "must be committable without `git add -f`". Verify with `git check-ignore -v mens/data/vox_think/reasoning.jsonl` (exit 1 means not ignored).

### Pattern 1: Slice loader that refuses a wrong-size slice
**What:** `load_slice(path, expected_n)` returns `Err` unless exactly `expected_n` valid, unique-id items parse.
**When to use:** every consumer of the three names. The scorer passes 32; tests pass 32.
**Example:**
```rust
// Sketch only; names are a recommendation, not existing API.
pub const SLICE_SIZE: usize = 32;
pub const SLICE_NAMES: [&str; 3] = ["gsm8k_think", "math500_think", "vox_think"];

pub struct ReasoningItem { pub id: String, pub question: String, pub answer: String }

pub fn load_slice(path: &Path, expected_n: usize) -> Result<Vec<ReasoningItem>, String>;
```
`VOX_REASONING_EVAL_LIMIT` truncation (see Pitfall 7) is applied after validation and is reflected in the report's `n`, not hidden.

### Pattern 2: Outcome type whose JSON cannot carry a score when nothing was scored
**What:** one enum, two shapes; `NotScored` has no accuracy field at all, so no code path can serialize one.
**Example:**
```rust
pub enum BenchOutcome {
    Scored(ReasoningScore),               // existing type, n >= 1
    NotScored { reason: String, n_requested: usize },
}
// to_json(): Scored   -> {"status":"scored","n":..,"accuracy":..,"think_initiation_rate":..}
//            NotScored-> {"status":"not_scored","n":0,"n_requested":..,"reason":".."}   (keys absent, not null, not 0)
```
`Scored` must be built only when `n >= 1`. The existing `score_completions` returns `accuracy: 0.0, think_initiation_rate: 0.0, n: 0` for an empty batch (`qwen_reasoning.rs:197-221`); the new type is what stops that tuple from being stored as a result.

### Pattern 3: Vox answers proven by the interpreter
**What:** each `vox_think` row carries the Vox `program` it asks about. The oracle test lexes, parses, lowers, runs `main`, and compares the returned `VoxValue` to `answer`.
**Mechanics:** the call sequence is the one in `crates/vox-compiler/tests/eval_typeck_parity_test.rs:19-30` (`lexer::lex`, `parser::descent::parse`, `hir::lower::lower_module`, `eval::Interpreter::new(1_000_000)`, `run_module`, `call("main", vec![])`). `VoxValue` has `Int(i64)`, `Str(Rc<str>)`, `Bool(bool)` and others (`crates/vox-compiler/src/eval/value.rs:5-24`) and has no `Display` impl found in `eval/`, so compare by matching variants, not by formatting.
**Item kinds that fit the existing grader (planner chooses the mix):**
- *trace*: "What does `main()` return?" where gold is an integer. Graded by last-number rule.
- *debug*: a short program with one planted defect, question asks for the value the buggy code returns, or the corrected result. Gold is an integer.
- *plan-check*: pick the count, ordering, or result a small state/workflow/actor sketch produces. Gold is an integer or one token.
Prefer integer golds. Non-numeric golds only grade by `\boxed{}` body or exact last non-empty line (`qwen_reasoning.rs:169-184`), which is brittle for free text.
**Required question wording:** end each Vox question by asking for the final value alone on the last line, because the GSM8K and MATH-500 questions carry no format instruction (checked: zero rows mention "boxed" or "final answer") and the numeric grader takes the last number in the answer region.

### Anti-Patterns to Avoid
- **Storing `ReasoningScore { 0.0, 0.0, 0 }` for a failed run.** That is the empty-batch tuple; it reads as a measured zero.
- **Counting an inference error as a graded-wrong item.** It inflates `n` and invents `accuracy: 0.0`.
- **Authoring Vox rows as "write a construct X" prompts with a think body.** That is the Plan / Syntax / Implementation shape the research doc says to delete (`generate_cot_organic_corpus`, `crates/vox-corpus/src/codegen_vox/part_03.rs`).
- **A baseline file with numbers.** Locked: no invented baselines.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Answer grading | A second grader for Vox | `grade_final_answer` | Locked reuse; it already handles thousands separators, hyphens, unterminated `<think>`, boxed bodies |
| Score aggregation | New accuracy math | `score_completions` and `ReasoningScore` | Locked reuse |
| Thinking prompt | A new ChatML string | `qwen_reasoning::thinking_mode_prompt` | The bench must not go through the `/no_think` wrapper (milestone Pitfall 1) |
| Vox answer truth | Hand arithmetic | The interpreter, via `run_probe`-style test | Makes "checkable" mechanical |
| Text-leakage screening | A new n-gram scorer | `eval_gate::leakage::assert_no_text_leakage(bench_path, corpus_dirs)` as an optional follow-up | Exists and is already a hard precondition for `heldout_bench`; it reads a different manifest shape, so adapting it is optional, not required here |

**Key insight:** the only genuinely new logic is "refuse to call anything a score unless at least one item was scored". Everything else is reuse.

## Runtime State Inventory

Not a rename or migration phase. Omitted. One state fact the planner must carry: a prior run may have left `<adapter>/collateral_eval/<bench>/reasoning_report.json` and `<run_dir>/collateral_damage_report.json` on disk. Pitfall 4 covers the first; the second is a phase 315 concern.

## Common Pitfalls

### Pitfall 1: "Checked in" is currently false
**What goes wrong:** the planner trusts the 2026-09-21 research doc ("Checked in: `mens/data/gsm8k_think/reasoning.jsonl`") and the phase brief, adds no commit step, and a fresh clone has no slices, so `score_bench` falls through to the manifest branch and bails with a missing-manifest error.
**Why it happens:** `.gitignore:189` ignores `*.jsonl`; the files were written locally on 2026-10-26 10:38 and never staged. [VERIFIED: `git check-ignore -v` printed `.gitignore:189:*.jsonl	mens/data/gsm8k_think/reasoning.jsonl`; `git ls-files mens/data` lists two files]
**How to avoid:** a task that adds the negation, cuts GSM8K, adds `vox_think`, and `git add`s all three; a test that reads each file from `CARGO_MANIFEST_DIR/../../mens/data/<name>/reasoning.jsonl`.
**Warning signs:** `git status --short mens/data` shows nothing for the files while they exist on disk.

### Pitfall 2: The scorer is uncommitted
**What goes wrong:** `reasoning_eval.rs` is `??`; `mod.rs` and `eval_collateral.rs` carry uncommitted hunks that wire it. If phase 314 only edits these files, its diff will hide the fact that the whole scorer is new to git, and a clean checkout will not compile `score_bench`'s call into `super::reasoning_eval`.
**How to avoid:** the first task stages exactly: `reasoning_eval.rs`, the 2-line `pub(crate) mod reasoning_eval;` addition in `mens/mod.rs`, and the `cached_reasoning_report` plus `score_bench` hunks in `eval_collateral.rs`. Do **not** stage the other modified files in the tree (`eval_local_prompt.rs`, `train.rs`, `merge_qlora.rs`, etc.): CONTEXT.md says the `prepare_bench_item` edit is not this phase's contract. [VERIFIED: `git status --short` and `git diff` this session]

### Pitfall 3: A load failure records nothing, and `n` is nowhere
**What goes wrong:** `load_model(...)` is followed by `?` at `reasoning_eval.rs:70-72`, so the function returns before `write_report` (line 120). No file says "0 items scored". BENCH-03 requires the report to record it.
**How to avoid:** wrap load in a match that builds `BenchOutcome::NotScored`, writes it, then returns `Err`. Keep returning `Err`: `run_collateral_damage_with` propagates scorer errors and the existing test `an_unloadable_adapter_fails_loudly_and_writes_no_passing_report` already asserts no `collateral_damage_report.json` appears on `Err`.

### Pitfall 4: A stale passing report survives a later failed load
**What goes wrong:** run A scores and writes `reasoning_report.json`. Run B cannot load the model and writes nothing. The directory still says `accuracy: 0.9`.
**How to avoid:** the `NotScored` path overwrites the same `reasoning_report.json`. Test it: write a scored report, run the not-scored writer into the same dir, assert the file now has `"status":"not_scored"` and no `accuracy` key.

### Pitfall 5: Inference errors become "scored, wrong"
**What goes wrong:** `reasoning_eval.rs:95-98` maps both a JSON-parse miss (`unwrap_or_default()`) and an `Err` from `run_inference` (`String::new()`) to an empty completion; the item is then graded and counted. A backend that loads but fails every call gives `accuracy: 0.0`, `think_initiation_rate: 0.0`, `n: 32`.
**How to avoid:** keep a separate `n_inference_errors`. Score only items that produced a completion. If `n_scored == 0` the outcome is `NotScored`. Recommended stricter rule (see Open Questions 2): any inference error makes the run `NotScored`, because the gate compares a 32-item accuracy to a 32-item baseline and a partial run is not comparable.

### Pitfall 6: Writing the report can create a phantom adapter directory
**What goes wrong:** `write_report` calls `create_dir_all(adapter/collateral_eval/<bench>)` (`reasoning_eval.rs:177-178`). For a missing adapter path that creates the adapter directory, and `serve` treats a directory with `training_manifest.json` or `candle_qlora_adapter.safetensors` as an adapter, so an empty shell directory is a confusing artifact.
**How to avoid:** write the `NotScored` file only when `adapter.is_dir()`; otherwise return the `Err` (message includes `scored 0 of N items`) and write nothing. Test with a nonexistent temp path and assert the path still does not exist afterward.

### Pitfall 7: `VOX_REASONING_EVAL_LIMIT` / `_MAX_TOKENS` are unregistered env vars
**What goes wrong:** both are read with `std::env::var` in `reasoning_eval.rs:40,51` and neither appears in `contracts/config/env-vars.v1.yaml` (searched `contracts`, `docs`, `crates`; only the research doc and `reasoning_eval.rs` mention them). Repo policy: "Do NOT introduce a new `VOX_*` env var without updating `contracts/config/env-vars.v1.yaml`." Committing this file introduces them to git.
**How to avoid:** register both in the YAML (and let `vox ci command-sync` / the `ssot-autoregen` bot handle the generated docs), or delete the LIMIT override. If kept, the report's `n` must reflect the truncation, and phase 316 must not set it.

### Pitfall 8: Relative `BENCH_ROOT`
`eval_collateral.rs:9` is `const BENCH_ROOT: &str = "mens/data"`, resolved against the process cwd. `cargo test` runs with cwd = crate dir, so any test that goes through `score_bench` finds no file. New tests must build paths from `env!("CARGO_MANIFEST_DIR")` joined with `../../mens/data`.

### Pitfall 9: Gate keys map to one directory
`score_bench` strips a `_init` suffix and loads `<dir>/reasoning.jsonl` (`eval_collateral.rs:144-149`), caching one report per adapter and bench. So `vox_think` and `vox_think_init` reuse one generation pass. The new slice must therefore be named exactly `vox_think` and live at `mens/data/vox_think/reasoning.jsonl`. A different file name would silently route to the manifest branch.

### Pitfall 10: Grader brittleness on the Vox golds
`last_number` takes the last number in the answer region, and hyphen-inside-token is not a sign (`qwen_reasoning.rs` tests `last_number_hyphen_inside_a_token_is_not_a_sign`). A completion that ends "... after 3 steps" after the real answer grades wrong. Keep Vox golds integer, require the final value on its own last line in the question, and avoid golds that are dates or ranges.

## Code Examples

### Not-scored report must omit the score keys
```rust
// Test shape, not implementation.
#[test]
fn not_scored_json_has_no_score_keys() {
    let v = BenchOutcome::NotScored { reason: "load failed".into(), n_requested: 32 }.to_json();
    assert_eq!(v["status"], "not_scored");
    assert_eq!(v["n"], 0);
    assert!(v.get("accuracy").is_none());
    assert!(v.get("think_initiation_rate").is_none());
}
```

### Gold round trip over every item of every slice
```rust
// For each item: a completion that ends with the gold must grade true,
// and the same completion with the gold changed must grade false.
let ok = format!("<think>\nwork\n</think>\n\n\\boxed{{{}}}", item.answer);
assert!(grade_final_answer(&ok, &item.answer));
assert!(!grade_final_answer("<think>\nwork\n</think>\n\n\\boxed{999999}", &item.answer));
```
This catches golds the grader cannot match (for example a gold the normalizer strips to empty, which `grade_final_answer` rejects at `gold_norm.is_empty()`). It is cheap and covers all 96 items. Whether every existing MATH-500 gold passes the round trip is [ASSUMED] until the test runs; if one fails, report it rather than editing the dataset row.

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Bench through `assemble_prompt` (forces `/no_think` and an empty think block) | Bare `<|im_start|>assistant\n` via `thinking_mode_prompt` | 2026-10-26 (phase 313 / `reasoning_eval.rs`) | Think-initiation is measurable; empty block no longer counts |
| Single boilerplate "Plan / Syntax / Implementation" think rows (104 rows, one template) | Held-out Vox items with executable answers | this phase | Held-out set cannot be satisfied by template text |

**Deprecated/outdated:** the research doc's statement that the slices are "Checked in" with "first 64 rows" of GSM8K is out of date against this phase's locked 32. Update it in the same PR (`docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`, section "Update (2026-10-26)"). It already has frontmatter.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | The first 32 rows of the existing 64-row GSM8K file are the intended 32-item slice (as opposed to a different 32 of the 64) | Architecture Patterns | Low. Locked text says "checked-in files, not a new sample", and a prefix is deterministic and matches the `source` convention ("first N rows"). User may prefer a different cut. |
| A2 | MATH-500 content licence is permissive enough to commit 32 rows | Package/Data | Low-medium. The file's own `source` is `HuggingFaceH4/MATH-500, first 32 rows` with no licence note; GSM8K's says MIT. Rows are already in the working tree and the user's decision is to check them in. Confirm the licence statement before adding it to `source`. |
| A3 | Every existing MATH-500 gold passes the boxed round trip through `grade_final_answer` | Code Examples | Low. Normalization is symmetric (`normalize_answer` on both sides); not executed this session. |
| A4 | `VoxValue::Int` is what `main` returns for an `int` function and matches on `i64` | Pattern 3 | Low. `value.rs:5-6` shows `Int(i64)`; the exact snippets for 32 programs are authoring work and each must be run. |
| A5 | Registering `VOX_REASONING_EVAL_*` in `env-vars.v1.yaml` is the right policy path versus deleting the LIMIT override | Pitfall 7 | Low. Either satisfies the policy; the planner picks. |
| A6 | Treating any per-item inference error as `NotScored` is the desired strictness | Pitfall 5 | Medium. CONTEXT only locks the load-failure case. Stricter is safer for a gate; looser keeps a partial datapoint. |

## Open Questions (RESOLVED)

RESOLVED: stage the scorer by explicit path and confirm the commit shows only those files. The plans do this.
RESOLVED: any per-item inference error makes the whole run NotScored. The plans do this.
RESOLVED: leakage checks are the in-repo template-marker and golden-copy guards. Corpus-wide screening stays optional and local.
DEFERRED to phase 315: refusing non-finite baseline values in `eval_collateral_damage`. Not required by BENCH-02 or BENCH-03.


1. **How to commit the uncommitted scorer without sweeping unrelated edits.** RESOLVED: stage by path.
   - What we know: `reasoning_eval.rs` is untracked; `mod.rs` and `eval_collateral.rs` have uncommitted hunks that belong to it; other modified files in the tree are not this phase's.
   - What's unclear: whether `eval_collateral.rs` has unrelated uncommitted hunks beyond the 36 added lines. The diff read this session shows only the `cached_reasoning_report` and `score_bench` additions.
   - Recommendation: stage by path, and `git diff --cached --stat` before committing; expect three files.

2. **Strictness on per-item inference errors (A6).** RESOLVED: any error is NotScored.
   - Recommendation: any error makes the whole run `NotScored` with `reason` naming the first failing item id. Report `n_requested` and `n_inference_errors` for diagnosis.

3. **Should the phase add a leakage check of `vox_think` against the training corpus?** RESOLVED: in-repo guards only.
   - What we know: `mens/data/train.jsonl` and `synthetic.jsonl` are gitignored, so CI cannot rely on them. `eval_gate::leakage` already checks `heldout_bench` answers against corpus completions.
   - Recommendation: required in-repo checks are (a) no question contains the three template markers and (b) no `program` is byte-identical to a file under `examples/golden/`. Corpus-wide n-gram screening is optional and local-only.

4. **Does the `eval_collateral_damage` pre-score validator need to refuse non-finite baseline values?** DEFERRED to phase 315.
   - Not required by BENCH-02/03. `run_collateral_damage_with` already refuses a missing or empty `benchmarks` object. Leave for 315 unless the planner wants a one-line guard.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| `cargo` | all tests | ✓ | 1.98.1 | — |
| Qwen3-8B pinned snapshot | live scoring (phase 316 only) | ✗ | — | Not needed in 314. `~/.cache/huggingface/hub` has `Qwen3-0.6B` and `mlx-community/Qwen3-0.6B-4bit`, not 8B. Do not use them to produce a score. |
| An adapter run dir (`mens/runs/...`) | live scoring (phase 316) | ✗ (`mens/runs` holds only `logs`) | — | Not needed in 314 |
| `pnpm`, Playwright | GUI visual rule | n/a | — | This phase has no GUI surface |

**Missing dependencies with no fallback:** none for this phase. All 314 acceptance runs without a model.
**Missing dependencies with fallback:** the model, by design. The failed-load path is exercised with a nonexistent adapter path.

## Validation Architecture

> `.planning/config.json` has no `workflow.nyquist_validation` key, so this section is included.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (plus `cargo nextest` in CI) |
| Config file | none |
| Quick run command | `cargo test -p vox-eval --lib reasoning_bench` |
| Full suite command | every row in the map below, then `cargo check -p vox-ml-cli --features gpu --tests` |

### Feature flags and the "0 passed is a failed run" rule
Phase 313 proved its commands with these exact shapes, and the gating is real: `vox-ml-cli` serve and `reasoning_eval` modules are `#[cfg(feature = "gpu")]` (`mens/mod.rs:24-27`), so a filter without the feature matches nothing, prints `0 passed`, and exits 0. [VERIFIED: `313-VALIDATION.md` note and `313-01-SUMMARY.md` deviation 1]

- `vox-ml-cli`: always `cargo test -p vox-ml-cli --features gpu --lib -- <filter>`
- `vox-populi` (only if a task touches it; 314 does not need to): `cargo test -p vox-populi --features mens-train --lib <filter>`
- `vox-eval` and `vox-compiler` need no feature flag.

Every command must show `test result: ok. N passed` with **N >= 1**, and the plan should state the expected N per command. `0 passed`, or only `filtered out`, is a failed run, not a pass. A runnable guard: pipe through `tee` and fail on `test result: ok. 0 passed` (for example `... 2>&1 | tee /tmp/out.txt; ! rg -q "test result: ok\. 0 passed" /tmp/out.txt && rg -q "test result: ok\. [1-9]" /tmp/out.txt`). With multiple test binaries in one invocation (lib plus integration targets), judge the lib line, as phase 313 did.

### Phase Requirements to Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| BENCH-02 | `vox_think` has exactly 32 rows, unique ids, non-empty question and answer | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-02 | No Vox question contains the template markers (`Plan:`, `Syntax considerations`, `Implementation:`) or the "generate a `X` construct in Vox" shape | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-02 | Every `vox_think` program runs in the interpreter and its returned value equals `answer` | integration | `cargo test -p vox-compiler --test vox_think_oracle_test` | ❌ Wave 0 |
| BENCH-02 | Mutation check: changing one `answer` makes the oracle test fail (run once by hand, restore) | manual-once | edit one gold, run the line above, expect FAIL, restore | n/a |
| BENCH-02 | `gsm8k_think` and `math500_think` each have exactly 32 rows, are tracked by git, and are the prefixes of the pre-existing files | unit + shell | `cargo test -p vox-eval --lib reasoning_bench` and `git ls-files mens/data/gsm8k_think/reasoning.jsonl mens/data/math500_think/reasoning.jsonl mens/data/vox_think/reasoning.jsonl` (expect 3 lines) | ❌ Wave 0 |
| BENCH-02 | `git check-ignore mens/data/vox_think/reasoning.jsonl` exits 1 | shell | `git check-ignore -q mens/data/vox_think/reasoning.jsonl; test $? -eq 1` | n/a |
| BENCH-02 | Every gold of all 96 items round-trips through `grade_final_answer` (true for gold, false for a wrong answer) | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-03 | `BenchOutcome::NotScored` JSON has `n: 0`, `status: not_scored`, and no `accuracy` or `think_initiation_rate` key | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-03 | `Scored` cannot be built from an empty batch | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-03 | A wrong-size slice (31 or 33 rows) is refused by `load_slice` | unit | `cargo test -p vox-eval --lib reasoning_bench` | ❌ Wave 0 |
| BENCH-03 | A not-scored write overwrites an earlier scored `reasoning_report.json` in the same dir | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ❌ Wave 0 |
| BENCH-03 | A nonexistent adapter path returns `Err` and the path still does not exist afterward | unit | `cargo test -p vox-ml-cli --features gpu --lib -- reasoning_eval` | ❌ Wave 0 |
| BENCH-03 | A scorer error still leaves no `collateral_damage_report.json` | unit | `cargo test -p vox-ml-cli --features gpu --lib -- eval_collateral` | ✅ existing, 4 tests |
| BENCH-03 | Compiles with the new wiring | check | `cargo check -p vox-ml-cli --features gpu --tests` | ✅ |
| BENCH-03 | Mutation check: replace the `NotScored` branch with a pass-through of `ReasoningScore{0,0,0}`, confirm the key-absence test fails, restore | manual-once | as stated | n/a |

Model-free note: a green run of any row above is **not** a score. The phase deliverable is "scorer refuses and counts", and VALIDATION.md should say so.

### Sampling Rate
- **Per task commit:** that task's row(s) only.
- **Per wave merge:** all rows for plans in the wave.
- **Phase gate:** every row green with N >= 1 before `/gsd-verify-work`, plus `vox ci line-endings` on the diff (LF in the three JSONL files) and the scoped doc lint for the edited research doc (`cargo run -p vox-doc-pipeline -- --lint-only --paths docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`).

### Wave 0 Gaps
- [ ] `crates/vox-eval/src/reasoning_bench.rs` plus `pub mod reasoning_bench;` in `crates/vox-eval/src/lib.rs` (tests live in the file, per the Test-First policy)
- [ ] `crates/vox-compiler/tests/vox_think_oracle_test.rs`
- [ ] `#[cfg(test)]` tests in `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (the file currently has none)
- [ ] The negation line in `.gitignore`
- [ ] No framework install needed

## Security Domain

`security_enforcement` is not set to `false`, so included.

### Applicable ASVS Categories

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | — |
| V3 Session Management | no | — |
| V4 Access Control | no | — |
| V5 Input Validation | yes | `load_slice` rejects malformed JSON, empty fields, duplicate ids, wrong count; read with the bounded reader (`vox_bounded_fs::read_utf8_path_capped`) where the crate already depends on it, or an explicit size cap in `vox-eval` |
| V6 Cryptography | no | If a content pin is wanted, use `vox-crypto` per the cryptography policy. Do not import `blake3` or `sha2` directly. Prefer pinning first and last ids plus count in a test and skip hashing. |

### Known Threat Patterns

| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| A failed run reads as a passing score | Repudiation / Tampering | `NotScored` has no score fields; `Err` keeps `collateral_damage_report.json` unwritten |
| Stale passing report after a failed run | Tampering | Overwrite on every outcome; Pitfall 4 test |
| Interpreter running committed Vox from a data file | Elevation of privilege | The oracle runs only repo-committed programs inside the interpreter test harness; keep programs pure (no `fs`, `http`, `process`, `env`) and assert that in the oracle test by rejecting those identifiers |
| Slice mutated after baseline | Tampering | Test pins count and first/last id; the baseline-before-adapter comparison in 316 depends on identical files |

## Project Constraints (from .cursor/rules/)

- **Test-first:** a new `pub fn` in `crates/*/src/**` needs a `#[test]` in the same file (`skeleton/untested-pub-api`). Write the failing test first.
- **Build broker:** call plain `cargo` on PATH. Never an explicit `~/.cargo/bin/cargo`. Per `AGENTS.md` do not run workspace-wide clippy or tests locally; use `-p <crate>`.
- **Dependency discipline:** do not add a workspace crate edge without an `exceptions` ledger entry (USER-AUTHORIZED-ONLY). This plan adds none: `vox-ml-cli` already depends on `vox-eval`, and the oracle test is in `vox-compiler` which already has `serde_json`.
- **Env vars:** a new `VOX_*` var requires `contracts/config/env-vars.v1.yaml`. See Pitfall 7.
- **Data policy:** the slices are static checked-in fixtures, not event or log data. Report output is a run artifact under the adapter dir, as it already is. Do not open a `turso::Connection`.
- **Line endings:** LF for the JSONL files; run `vox ci line-endings` on the diff.
- **Secrets:** none touched.
- **VoxLang-first:** if any helper script is wanted (for example to cut the GSM8K file), write it as `.vox`, not `.py` or `.sh`. A one-off `head -n 32` by the executor is a command, not a project script, and does not need to be committed.
- **LLM facade:** the scorer calls the model through the `MlBackend` plugin, not a vendor host; do not add hostnames.
- **Retired surfaces:** none of the retired symbols are involved. Do not cite `vox-dei`, `recall()`, `TURSO_*`.
- **Vox syntax in items:** use bare keyword forms (`fn`, `workflow`, `actor`, `activity`); no `@endpoint`, `@component fn`, `@py.import`. `@pure fn` is the decorator form for purity.
- **Docs:** edits to `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md` must keep its frontmatter intact. Fenced `vox` blocks in docs compile as doctests; use ` ```text ` for anything that should not.
- **Not this phase:** the serve gate (315), the live comparison (316), and any numeric baseline.

## Sources

### Primary (HIGH confidence; read this session)
- `crates/vox-eval/src/qwen_reasoning.rs` (full file): `score_completions`, `grade_final_answer`, `ReasoningScore`, empty-batch zeros
- `crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs` (full file): load, inference, error handling, `write_report`
- `crates/vox-ml-cli/src/commands/mens/eval_collateral.rs` (full file): `BENCH_ROOT`, `score_bench`, `_init` suffix, cache, existing tests
- `crates/vox-ml-cli/src/commands/mens/mod.rs` diff, `git status`, `git check-ignore`, `git ls-files`
- `mens/data/gsm8k_think/reasoning.jsonl`, `mens/data/math500_think/reasoning.jsonl`: counts, keys, ids, source strings (structure only; items not reproduced here)
- `.planning/phases/314-.../314-CONTEXT.md`, `.planning/REQUIREMENTS.md`, `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/research/SUMMARY.md`, `.planning/research/FEATURES.md`
- `.planning/phases/313-lane-prefix-lane-01-lane-02/313-VALIDATION.md`, `313-VERIFICATION.md`, `313-01-SUMMARY.md`: feature-flag lessons and passing counts
- `docs/src/architecture/mens-reasoning-preservation-research-2026-09-21.md`: template finding and 2026-10-26 update
- `crates/vox-compiler/tests/eval_typeck_parity_test.rs:16-30`, `crates/vox-compiler/src/eval/value.rs:5-24`, `crates/vox-integration-tests/tests/golden_vox_test_runner.rs`: interpreter call pattern
- `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:350-372`: how serve reads the report (context only)
- `AGENTS.md` and `.cursor/rules/*`

### Secondary / Tertiary
- None. No web search was needed; the phase is repo-internal. Dataset licence for MATH-500 is not verified (A2).

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH, all in-tree and read this session.
- Architecture: HIGH for the fail-closed shape and file layout; MEDIUM for the Vox item mix, which is authoring.
- Pitfalls: HIGH for 1-6, 8-9 (each shown by a file or git command this session); MEDIUM for 7 and 10 (policy and grader-edge reasoning).

**Research date:** 2026-10-26
**Valid until:** 2026-11-02 for the uncommitted-tree facts (Pitfalls 1-2 will change as soon as anyone stages these files); 30 days for the rest.
