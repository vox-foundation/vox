---
phase: 01-retrieval-eval-baseline
fixed_at: 2026-10-05T07:16:10Z
review_path: .planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-REVIEW.md
iteration: 1
findings_in_scope: 13
fixed: 13
skipped: 0
status: all_fixed
---

# Phase 01: Code Review Fix Report

**Fixed at:** 2026-10-05T07:16:10Z
**Source review:** .planning/workstreams/kb/phases/01-retrieval-eval-baseline/01-REVIEW.md
**Iteration:** 1

**Scope:** all Warnings (WR-01..WR-08) plus IN-01, IN-03, IN-04, IN-06 and IN-07, as the orchestrator directed. IN-02, IN-05 and IN-08..IN-11 are out of scope and stay documented (see Deferred Info Findings).

**Summary:**
- Findings in scope: 13
- Fixed: 13
- Skipped: 0

Two refactor commits precede the fixes. They are pure test moves that keep files within the 500 non-blank-line budget:
- `bb5e16932` moves the gate tests to `crates/vox-cli/src/commands/kb/gate/tests.rs`.
- `283929895` moves the baseline tests to `crates/vox-search/src/kb_eval/baseline/tests.rs`.

## Fixed Issues

### WR-01: tighten-only treats any `git show` failure as "absent at base"

**Files modified:** `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`, `crates/vox-git/src/read_cmd.rs`
**Commit:** 3b48d0d02
**Applied fix:**
- The new `baseline_at(git, base)` returns `Ok(None)` (bootstrap) only when `git ls-tree --name-only <base> -- <baseline>` succeeds and lists nothing.
- Every other outcome is a `[tighten-only]` failure: an ls-tree failure, a `git show` failure after the path is listed, or a parse error.
- `ls-tree` was added to the `vox_git::read_cmd` read-only allowlist.
- Tests use an injected fake git and cover all five cases, plus a real-git check (`HEAD` is present, the tree-ish `HEAD:docs` is absent, a missing ref is an error).
- **Mutation-proven:** turning a `git show` failure back into `Ok(None)` makes `baseline_at_bootstraps_only_on_proven_absence` fail. The code was restored and verified afterwards.

### WR-02: Canary GUID matching is case-sensitive (and misses UTF-16)

**Files modified:** `crates/vox-search/src/kb_eval/canary.rs`, `crates/vox-cli/src/commands/kb/gate.rs`
**Commits:** 3d2ebe9db, bb5381ca8 (clippy `chunks_exact_to_as_chunks` follow-up, no behavior change)
**Applied fix:**
- The new `decode_for_scan(bytes)` returns lossy UTF-8. When the bytes contain a NUL, it also appends the UTF-16 LE and BE decodes. Decoding both byte orders from offset 0 also covers an odd-aligned stream. The result is ASCII-lowercased.
- `scan_paths_for_guid` and the gate's mens-jsonl scan use `decode_for_scan`.
- `texts_containing_guid` and `assert_corpus_clean` match case-insensitively. Both sides are ASCII-lowercased.
- The test plants these copies using the placeholder GUID only: uppercase, mixed case, UTF-16LE with a BOM, UTF-16BE without a BOM, and an odd-aligned UTF-16LE stream. It also checks a clean UTF-16 file.

### WR-03: `git ls-files` output is C-quoted

**Files modified:** `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/eval.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`
**Commit:** 6eb2d42d2
**Applied fix:**
- Both call sites now use `ls-files -z` and the new `split_nul`. The sites are the gate's tracked-file list and `--snapshot-corpus`, which uses `ls-files -z -- docs/src`.
- Kb tests may not create a git repo, so the NUL parser is unit-tested instead, with non-ASCII, quote and backslash paths. A real-git `-z` round-trip on `contracts/kb` is also tested.
- No tracked path is quoted today, so the committed corpus snapshot is unaffected.

### WR-04: `--gate` never checks split disjointness or the paraphrase rule

**Files modified:** `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/eval.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`
**Commit:** e8e14eff2
**Applied fix:**
- The gate runs `validate_splits`, which reads only the query-id column of `test.tsv`. Violations are reported as `[golden] split rule: ...`.
- The body of `run_gate` was extracted into `gate_failures(root, base)` so the wiring can be tested.
- The test covers three cases: a valid disjoint split, a held-out id planted into `dev.tsv`, and a dev paraphrase of a held-out seed.
- **Mutation-proven:** removing the wiring makes the test fail.

### WR-05: No legal re-epoch path for a retriever, chunker, budget or estimator change

**Files modified:** `crates/vox-search/src/kb_eval/baseline.rs`, `contracts/kb/eval/README.md`
**Commit:** 12fa016fa
**Applied fix:**
- `check_tighten_only` now treats a change in `retriever/chunker/budget_tokens/token_estimator` as licensing a re-epoch, under the same rule as a golden-set change (an epoch bump plus a new non-empty note).
- With no golden or config change, the epoch must stay fixed and metrics may not drop. The old budget/estimator branch is subsumed.
- `check_regression` now compares the whole config, so it also catches a retriever or chunker swap.
- The README re-epoch rule now names the config fields. This is a prose edit; the canary line is untouched.
- The tests cover each of the four config edits. Each is rejected without a bump, rejected with the same note, and accepted with a bump and a new note. `check_regression` is shown to flag a retriever or chunker mismatch.
- **Requires human verification** because this is a ratchet-semantics change.

### WR-06: README contradicts the code on the heading path of pre-heading content

**Files modified:** `contracts/kb/eval/README.md`
**Commit:** 4d31932ab
**Applied fix:**
- The Ids section now states that content before the first heading, and content under an H1, belongs to the root chunk `doc_path#<title>`, and that there is no empty heading path.
- Prose only. The canary line was never printed, copied or altered; the diff was checked with the canary line filtered out.

### WR-07: Doc ids may contain `#`

**Files modified:** `crates/vox-search/src/kb_ids.rs`, `crates/vox-search/src/kb_eval/golden.rs`, `crates/vox-search/src/kb_eval/golden/validation_tests.rs`
**Commit:** 5581d2485
**Applied fix:**
- `is_kb_corpus_path` rejects `#`.
- `load_corpus` fails with `_id ... contains '#' (chunk ids split at the first '#')`.
- `snapshot_corpus_jsonl` fails with an error naming a KB-root path that contains `#`. A `#` path outside the KB roots is not corpus and is not an error.
- Tests cover both the load and the snapshot path.

### WR-08: The query-fingerprint leak scan covers less than the README promises

**Files modified:** `crates/vox-search/src/kb_eval/canary.rs`, `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`
**Commit:** f46064d2c
**Applied fix:**
- `fingerprint_paths` now covers every tracked file except `contracts/kb/eval/` and `archive` components. Files containing a NUL byte are treated as binary and skipped at read time.
- `query_fingerprint_leaks` streams docs through a single case-insensitive `RegexSet`. Each pattern is the escaped query words joined by `\s+`. This keeps the old lowercase and collapsed-whitespace semantics without copying or normalizing each doc. The function now returns `Result` and sorted output; a build failure is reported as a `[canary]` failure.
- The tests plant a paste in a `.rs` comment (uppercased) and in a `.planning/` plan, and add a NUL-bearing binary that must be skipped.
- **Mutation-proven:** restoring the old `.md`-only scope makes the test fail.
- **Runtime:** the naive per-query `contains` took the debug gate from 6.5 s to 8.7 s. With the RegexSet it runs in 5.5 to 6.8 s (see Verification).

### IN-01: Pre-existing doc comment now attaches to `base_ref_or_default`

**Files modified:** `crates/vox-cli/src/commands/ci/pre_push.rs`
**Commit:** d63b94603
**Applied fix:** I deleted the orphaned "Returns a `Command` that invokes the current `vox` binary" block. No function it could describe still exists in the file.

### IN-03: A parse error in the HEAD baseline is also reported as "baseline deleted"

**Files modified:** `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`
**Commit:** cbf6ec7b2
**Applied fix:**
- The head baseline is now tri-state. Unreadable is reported once, and the tighten-only check is skipped. Absent is reported as "no baseline at HEAD".
- This also fixes a WR-04 side effect: the old `None if fails.is_empty()` guard would have hidden the "no baseline" message whenever a `[golden]` failure was present. The test covers both cases.

### IN-04: Bootstrap and epoch values are never range-checked

**Files modified:** `crates/vox-search/src/kb_eval/baseline.rs`, `crates/vox-search/src/kb_eval/baseline/tests.rs`
**Commit:** 218175a7d
**Applied fix:**
- `check_tighten_only` runs `shape_errors` on every head, including at bootstrap. It requires `schema_version == 1`, a tolerance in `[0, 0.01]`, and every metric in `[0, 1]`.
- The tests cover negative and oversized tolerance, the schema version, and out-of-range metric values, both at bootstrap and after.

### IN-06: `--write-baseline` with an unchanged golden set writes an epoch the gate will reject

**Files modified:** `crates/vox-cli/src/commands/kb/gate.rs`, `crates/vox-cli/src/commands/kb/gate/tests.rs`
**Commit:** 309667d06
**Applied fix:**
- `run_write_baseline` warns on stderr when a baseline already exists and neither the golden set (`git diff --name-only origin/main`, working tree included) nor the retrieval config has changed.
- It also warns when it cannot diff. It still writes the baseline, because the gate stays the enforcement point.
- A shared `golden_changed` helper replaces the inline diff in `tighten_failures`.
- The pure `reepoch_warning` is tested for each case.

### IN-07: The `Debug` derive on `GoldenSet` and `QueryMeta` prints the canary GUID

**Files modified:** `crates/vox-search/src/kb_eval/golden.rs`
**Commit:** d5a05d4ca
**Applied fix:**
- Manual `Debug` impls print `<redacted>` for `canary`. `QueryLine` keeps its derive and goes through `QueryMeta`.
- The test asserts that the fixture's placeholder canary is absent from `{:?}` of both a `GoldenSet` and a `QueryLine`.

## Deferred Info Findings (out of scope, still documented)

- **IN-02 (silently ignored mode flags):** A usability issue with no correctness or leak impact. Adding `conflicts_with_all` for `--json`/`--split` is small, but it changes the accepted CLI surface, so it is left for a deliberate CLI pass.
- **IN-05 (frozen baseline depends on shared chunker/tokenizer):** This needs a design choice: either pin golden outputs or defactor about 25 lines out of `ingest.rs`/`memory_hybrid.rs`. WR-05 removes the dead end the review cited, because a deliberate chunker change can now re-epoch. A silent drift is still caught only by a metric move.
- **IN-08 (`recall_at_5` sample field in vox-cli-research):** A rename in a different crate, and it may affect persisted samples (a serde alias decision). Not part of the kb gate.
- **IN-09 (turso FTS probe vs feature unification):** A vox-db test concern outside the kb workstream's code.
- **IN-10 (`-diff` on the 10 MB corpus.jsonl):** A review-ergonomics and history-size trade-off. It is a `.gitattributes` policy choice for the owner.
- **IN-11 (base tip vs merge-base):** This fails closed today; it only produces confusing messages on un-rebased branches. Switching to `merge-base` changes which baseline the ratchet compares against, so it deserves its own review.

## Verification

All verification ran in the **main checkout** (`workflow.use_worktrees: false`, at `/Users/brbrainerd/dev/vox/.claude/worktrees/research-detection-chat-gui-05d826`). No isolated fixer worktree was created.

- `cargo nextest run -p vox-search`: 301 passed. This includes the `kb_*` unit tests and the `kb_eval_determinism`, `kb_eval_gate` and `kb_eval_golden_contract` integration tests.
- `cargo nextest run -p vox-cli --lib commands::kb`: 24 passed.
- `cargo nextest run -p vox-cli --lib pre_push`: 13 passed.
- `cargo nextest run -p vox-git read_cmd`: 3 passed.
- `cargo clippy -p vox-search -p vox-cli -p vox-git --all-targets -- -D warnings`: clean (after `bb5381ca8`).
- After `cargo build -p vox-cli`, `./target/debug/vox kb eval --gate` on the real repo exits 0. It reports "OK (10844 tracked files scanned)" and took 5.53 s, 5.46 s and 5.53 s on three debug-build runs. Before the fixes the old binary took 6.47 s; mid-work runs measured 6.7 s. All are within the 10 s budget.
- **Baseline metrics unchanged:** every metric in `vox kb eval --json` (dev) equals `contracts/kb/retrieval-baseline.v1.json` at 6 dp (0 diffs). The baseline file was not modified, and there was no re-epoch.
- Every touched `.rs` file is within 500 non-blank lines (`golden.rs` is exactly 500). `pre_push.rs` was already 1638 lines and only lost lines.
- No GUID-shaped literal was added to any diff or commit message. The tests use only the existing placeholder GUIDs. The real canary was never printed.

---

_Fixed: 2026-10-05T07:16:10Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
