---
phase: 01-retrieval-eval-baseline
plan: 05
subsystem: retrieval-eval
tags: [vox-search, eval, ratchet, canary, kb]
requires: ["01-01"]
provides:
  - "kb_eval::baseline: Baseline v1 schema, regression check, tighten-only check, tighten and epoch writers"
  - "kb_eval::canary: pure canary checks (repo scan, text scan, root disjointness, corpus-clean, query fingerprints)"
affects: [01-07, 01-11]
tech-stack:
  added: []
  patterns: ["checks return Vec<String> tagged [regression] / [tighten-only]", "GUID only via argument"]
key-files:
  created:
    - crates/vox-search/src/kb_eval/baseline.rs
    - crates/vox-search/src/kb_eval/canary.rs
    - crates/vox-search/tests/kb_eval_gate.rs
  modified:
    - crates/vox-search/src/kb_eval/mod.rs
key-decisions:
  - "LevelMetrics gained Deserialize + deny_unknown_fields so Baseline reuses it instead of a parallel struct"
  - "tighten-only with golden unchanged also pins epoch (an epoch bump without a golden change is an error, so it cannot launder a loosening)"
  - "query_fingerprint_leaks and texts_containing_guid take (label/path, text) pairs, not CorpusDoc, so the CLI can feed repo files, MENS pairs and jsonl lines"
requirements-completed: []
status: complete
duration: ~35min
completed: 2026-10-04
commits: 3
plan_head_before: c7ba29a1f427edafc2deb9c4d1f334355f904503
actuals:
  tokens: 17000
  tasks: 3
  commits: 3
---

# Phase 1 Plan 05: Ratchet and canary decision logic Summary

Pure, mutation-proven `vox-search::kb_eval` halves of EVAL-04 (baseline regression + tighten-only ratchet) and EVAL-03 (canary leak checks), so the 01-07 CLI is only IO glue. EVAL-03 and EVAL-04 stay open: 01-07 and 01-11 also serve them.

## What was built

- **Task 1 (`44a062cea`)**: `baseline.rs`. Schema v1 (`deny_unknown_fields`, byte-identical JSON round trip), `check_regression` (6 dp, drop must exceed tolerance), `check_tighten_only`, `improvements`, `tightened`, `new_epoch`, `ratchet_split() == Split::Dev`, `RATCHET_KEYS`, `INITIAL_TOLERANCE`. 12 tests.
- **Task 2 (`7a2193325`)**: `canary.rs`. 6 tests, placeholder GUID `aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee` only.
- **Task 3 (`66dca6a2a`)**: `tests/kb_eval_gate.rs`. Real retriever passes its own baseline; reversed ranking and drop-top-hit both fail on `dev/all`.

## Public API for 01-07

```rust
// kb_eval::baseline
pub const RATCHET_KEYS: &[&str] = &["dev/all", "dev/human"];
pub const INITIAL_TOLERANCE: f64 = 0.01;
pub fn ratchet_split() -> Split;
pub struct BaselineBlock { pub doc: LevelMetrics, pub chunk: LevelMetrics, pub answer_in_budget: f64 }
pub struct Baseline { schema_version: u32, epoch: u32, epoch_note: String, retriever: String, chunker: String,
    token_estimator: String, budget_tokens: usize, tolerance: f64, metrics: BTreeMap<String, BaselineBlock> } // all pub
impl Baseline { pub fn from_json_str(&str) -> Result<Self>; pub fn to_json_string(&self) -> Result<String> /* trailing \n */ }
pub fn check_regression(report: &Report, baseline: &Baseline) -> Vec<String>;            // "[regression] ..."
pub fn check_tighten_only(base: Option<&Baseline>, head: Option<&Baseline>, golden_changed: bool) -> Vec<String>; // "[tighten-only] ..."
pub fn improvements(report: &Report, baseline: &Baseline) -> Vec<String>;                // informational
pub fn tightened(baseline: &Baseline, report: &Report) -> Baseline;                      // max(baseline, report)
pub fn new_epoch(prev: Option<&Baseline>, report: &Report, note: &str) -> Result<Baseline>;

// kb_eval::canary
pub fn scan_paths_for_guid(root: &Path, rel_paths: &[String], guid: &str, allowed_prefix: &str) -> Vec<String>;
pub fn texts_containing_guid(items: &[(String, String)], guid: &str) -> Vec<String>;   // (label, text) -> labels
pub fn check_roots_disjoint(roots: &[&str], eval_dir: &str) -> Result<()>;
pub fn assert_corpus_clean(corpus: &[CorpusDoc], guid: &str) -> Vec<String>;           // doc ids
pub fn query_fingerprint_leaks(queries: &[(String, String)], docs: &[(String, String)], min_chars: usize) -> Vec<String>; // "qid in path"
```

Semantics 01-07 must respect: bootstrap = `check_tighten_only(None, _, _)` is empty. Golden unchanged: epoch, budget, estimator fixed, no value lowered, no base key dropped. Golden changed: epoch must rise, note must be new and non-empty; metrics may move; tolerance may never rise. Ratchet keys must exist at head in both cases. `check_regression` also errors when report and baseline budget/estimator differ. An empty GUID flags everything (fails closed). `scan_paths_for_guid` takes the allowed prefix as a plain string prefix (pass `contracts/kb/eval/` with the trailing slash).

## Verification

- `cargo nextest run -p vox-search --lib kb_`: 43 passed (12 baseline, 6 canary, 25 from 01-01). `--test kb_eval_gate`: 3 passed.
- `cargo clippy -p vox-search --all-targets -- -D warnings`: clean. `cargo fmt -p vox-search` applied. baseline.rs 445 and canary.rs 235 non-blank lines (limit 500).
- **Mutation proofs (each with before/after grep presence counts):**
  1. `check_tighten_only` comparison `round6(now) < round6(was)` flipped to `>`: `tighten_only_rejects_lowered_metric` and `tightened_never_lowers` failed; restored, 12/12 green.
  2. `scan_paths_for_guid` filter replaced by `.filter(|_| false)` (always empty): `planted_guid_in_docs_is_detected_then_cleared` and `scan_tolerates_non_utf8_and_unreadable_files` failed; restored, 6/6 green.
  3. `check_regression` threshold made unreachable (`> 1e9 + tolerance`): `reversed_ranking_fails_regression` and `drop_top_hit_fails_regression` failed; restored, 3/3 green.
- The planted-GUID test asserts the planted file exists before scan 1 and is absent before scan 2.
- Threat mitigations: T-01-12 (tighten-only rules, mutation-proven), T-01-02 (all five canary checks), T-01-03 (`ratchet_split() == Split::Dev`, tested), T-01-13 (lossy decode, unreadable files skipped, tested).

## Deviations from Plan

None - plan executed exactly as written. Tests and implementation were written together per task and made meaningful by the mutation proofs above (as in 01-01; the plan is `type: execute`).

Notes: `improvements` is called with a 0.005 gain in a test to confirm sub-tolerance gains stay silent. `check_tighten_only` additionally rejects a changed epoch when the golden set is unchanged (stricter than the plan's bullets, no conflict).

## Known Stubs

None.

## Threat Flags

None. Pure functions; the only IO is a read-only file scan in `scan_paths_for_guid`.

## Self-Check: PASSED

- FOUND: crates/vox-search/src/kb_eval/baseline.rs, kb_eval/canary.rs, tests/kb_eval_gate.rs
- FOUND commits: 44a062cea, 7a2193325, 66dca6a2a
