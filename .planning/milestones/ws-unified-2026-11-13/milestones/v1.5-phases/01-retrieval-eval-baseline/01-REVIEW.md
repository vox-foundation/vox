---
phase: 01-retrieval-eval-baseline
reviewed: 2026-10-05T06:42:54Z
depth: standard
files_reviewed: 35
files_reviewed_list:
  - crates/vox-search/src/kb_ids.rs
  - crates/vox-search/src/kb_eval/mod.rs
  - crates/vox-search/src/kb_eval/golden.rs
  - crates/vox-search/src/kb_eval/baseline.rs
  - crates/vox-search/src/kb_eval/bm25.rs
  - crates/vox-search/src/kb_eval/canary.rs
  - crates/vox-search/src/kb_eval/metrics.rs
  - crates/vox-search/src/kb_eval/golden/validation_tests.rs
  - crates/vox-search/src/evaluation.rs
  - crates/vox-search/src/ingest.rs
  - crates/vox-search/src/memory_hybrid.rs
  - crates/vox-search/src/lib.rs
  - crates/vox-search/src/semcov_wave21_tests.rs
  - crates/vox-search/tests/kb_eval_determinism.rs
  - crates/vox-search/tests/kb_eval_gate.rs
  - crates/vox-search/tests/kb_eval_golden_contract.rs
  - crates/vox-cli/src/commands/kb.rs
  - crates/vox-cli/src/commands/kb/eval.rs
  - crates/vox-cli/src/commands/kb/gate.rs
  - crates/vox-cli/src/commands/ci/pre_push.rs
  - crates/vox-cli/src/lib.rs
  - crates/vox-cli/src/cli_dispatch/mod.rs
  - crates/vox-cli/src/commands/mod.rs
  - crates/vox-cli-research/src/answer_overlap.rs
  - crates/vox-cli-research/src/eval.rs
  - crates/vox-cli-research/src/lib.rs
  - crates/vox-cli-research/Cargo.toml
  - crates/vox-cli-research/tests/eval_e2e_test.rs
  - crates/vox-db/tests/turso_fts_probe.rs
  - .gitignore
  - .gitattributes
  - .voxignore
  - contracts/kb/eval/README.md
  - contracts/kb/retrieval-baseline.v1.json
  - contracts/operations/catalog.v1.yaml
findings:
  critical: 0
  warning: 8
  info: 11
  total: 19
status: issues_found
---

# Phase 01: Code Review Report

**Reviewed:** 2026-10-05T06:42:54Z
**Depth:** standard
**Files Reviewed:** 35 (catalog: kb.eval row only; README canary line never reproduced)
**Status:** issues_found

## Summary

I reviewed the Phase 1 retrieval-eval engine (`vox_search::kb_eval`, `kb_ids`), the `vox kb eval` CLI with its ratchet gate, the pre-push wiring, the research-eval metric rename, and the turso FTS probe.

The IR metric code matches the BEIR definitions. I recomputed the hand fixture: Recall, MRR, nDCG with linear gain and a log2(rank+1) discount, and IDCG over sorted qrels are all correct. Tie-breaking is fully ordered: quantized score descending, then chunk id ascending, then ordinal. Duplicate chunk ids are deduped before scoring. The tighten-only and regression rules are fail-closed for missing keys, deleted baselines, raised tolerance, and an unchanged-golden epoch bump. NaN cannot enter through JSON.

Policy checks:
- Git is called only through `vox_git::read_cmd::read_only`.
- A `-` ref is refused before any git call.
- No new crate edges and no new crypto were introduced.
- The real canary GUID is absent from every tracked file outside `contracts/kb/eval/`. I checked this case-insensitively without printing the GUID. The fixture uses the all-zero placeholder.
- No golden query of 40 or more characters appears in any tracked text file outside the eval dir (verified).
- The `ingest.rs` and `memory_hybrid.rs` changes only widen visibility to `pub(crate)` on `chunk_markdown_sections_with_size` and `tokenize`. There is no behavior change, so they are safe. They do couple the frozen baseline to those two functions (see IN-05).

Main concerns:
- Several canary scans fail open: case-sensitive matching, quoted `git ls-files` paths, and narrow fingerprint scope.
- One tighten-only path treats any `git show` failure as a bootstrap.
- The gate never validates split disjointness or the paraphrase rule.
- The re-epoch rule has no legal path for a retriever, chunker or budget change.

## Narrative Findings (AI reviewer)

## Warnings

### WR-01: tighten-only treats any `git show` failure as "absent at base" (fail-open bootstrap)

**File:** `crates/vox-cli/src/commands/kb/gate.rs:190-201`
**Issue:** `Err(GitReadError::NonZero { .. }) => None` maps every non-zero exit of `git show <base>:contracts/kb/retrieval-baseline.v1.json` to "no baseline at base". `check_tighten_only(None, ..)` then returns no errors (baseline.rs:220). `resolve_base` only proves the ref is a commit. It does not prove the blob is readable. Other causes of a non-zero exit therefore disable the whole tighten-only check, so any loosening of the baseline passes: a promisor/partial-clone fetch failure (blobless clone offline), a corrupt object, or a lock or permission error. Hosted CI uses `fetch-depth: 0`, but the pre-push hook runs on arbitrary local clones.
**Fix:** Distinguish "path absent" from "git failed". Probe with `cat-file -e` first and treat only a missing-path result as a bootstrap:
```rust
match read_only(root, &["cat-file", "-e", &format!("{base}:{BASELINE_REL}")]) {
    Err(GitReadError::NonZero { stderr, .. }) if stderr.is_empty() || stderr.contains("does not exist") || stderr.contains("Not a valid object name") => None,
    Err(e) => return vec![format!("[tighten-only] cannot read baseline at {base}: {e}")],
    Ok(_) => /* git show, and any failure here is an error */,
}
```

### WR-02: Canary GUID matching is case-sensitive, so an upper- or mixed-case copy evades every scan

**File:** `crates/vox-search/src/kb_eval/canary.rs:31,45,82`
**Issue:** `scan_paths_for_guid`, `texts_containing_guid` and `assert_corpus_clean` all use `str::contains(guid)` on the lowercase GUID. `read_canary_guid` only accepts lowercase (golden.rs:241). An uppercased copy evades the repo scan, the MENS extractor scan, the jsonl scan and the index-time assert. Many tools emit GUIDs in uppercase (Windows/.NET `ToString("B")`, some editors' "uppercase" transforms). Benchmark-contamination canaries are normally matched case-insensitively for exactly this reason. UTF-16-encoded files are also missed, because the NUL bytes break the lossy-UTF-8 match.
**Fix:** Lowercase the haystack before matching. The needle is already lowercase.
```rust
String::from_utf8_lossy(&bytes).to_ascii_lowercase().contains(guid)
// and in texts_containing_guid / assert_corpus_clean:
text.to_ascii_lowercase().contains(guid)
```
Add one test that plants `guid.to_ascii_uppercase()`.

### WR-03: `git ls-files` output is C-quoted, so non-ASCII or special-character paths are silently skipped by the canary scans and dropped from the corpus snapshot

**File:** `crates/vox-cli/src/commands/kb/gate.rs:253-254`, `crates/vox-cli/src/commands/kb/eval.rs:136-138`
**Issue:** `read_only` does not set `core.quotePath=false` or `-z`. Neither the repo nor the global config sets it either. Git therefore prints paths with non-ASCII bytes, `"`, `\` or control characters as quoted escapes. I verified this: `docs/src/café.md` is listed as `"docs/src/caf\303\251.md"`. Three paths are affected:
- `scan_paths_for_guid` joins the quoted string onto the root, the read fails, and the file is silently skipped ("unreadable files are skipped"). A GUID in such a file is never flagged (fail-open).
- `fingerprint_doc_paths` drops the path the same way.
- `snapshot_corpus_jsonl` rejects the quoted path in `is_kb_corpus_path` (it starts with `"`), so the doc silently disappears from the corpus.
**Fix:** Use `-z` and split on NUL. Both call sites go through `read_only`, so the fix is local:
```rust
read_only(root, &["ls-files", "-z"])?.split('\0').filter(|s| !s.is_empty()).map(str::to_owned).collect()
// eval.rs: read_only(root, &["ls-files", "-z", "--", "docs/src"])
```

### WR-04: `--gate` never checks split disjointness or the paraphrase rule (held-out isolation is not gated pre-push)

**File:** `crates/vox-cli/src/commands/kb/gate.rs:57-62` (with `crates/vox-search/src/kb_eval/golden.rs:383-431`)
**Issue:** `dev_report` calls only `GoldenSet::load(.., Dev)`. That load accepts any query id that exists in `queries.jsonl`, including ids that are also listed in `qrels/test.tsv`. `validate_splits` is the only check for dev/test overlap, coverage, and "a paraphrase shares its seed's split". It runs only for `--split test` (eval.rs:97-98) and in the `vox-search` contract test. Two failures therefore pass `vox kb eval --gate` and the pre-push fast tier: a test query added to `dev.tsv`, and a dev paraphrase of a held-out seed. Both are precisely the leaks EVAL-02/03 exist to stop. `validate_splits` reads only the query-id column of the two doc qrels files, so calling it from the gate exposes no test judgments.
**Fix:** In `run_gate`, add `if let Err(e) = validate_splits(&root.join(EVAL_DIR)) { fails.push(format!("[regression] split rule: {e:#}")) }`, or use a dedicated `[split]` tag. Add a gate test that plants a test id in `dev.tsv`.

### WR-05: No legal re-epoch path for a retriever, chunker, budget or estimator change

**File:** `crates/vox-search/src/kb_eval/baseline.rs:235-261`, `108-118`; `crates/vox-cli/src/commands/kb/gate.rs:203-207`
**Issue:** `golden_changed` is true only when `queries.jsonl`, `qrels/` or `corpus.jsonl` change. When it is false:
- `check_tighten_only` rejects any epoch change and any budget or estimator change.
- `check_regression` rejects a report whose budget or estimator differs from the baseline.

Some planned changes legitimately change the yardstick: AP-11 says Phase 7 replaces `BUDGET_TOKENS` per model, `CHUNKER` may change, and `chunk_markdown_sections_with_size` and `tokenize` are shared code. Each of these can only land by making a cosmetic edit to the golden set to unlock a re-epoch. That defeats the meaning of the epoch rule and invites the same "touch the golden set to make the gate green" behaviour the README forbids. `retriever` and `chunker` are also never compared at all, so a silent retriever swap is undetected unless the metrics drop.
**Fix:** Treat a change in any of `retriever`, `chunker`, `budget_tokens` or `token_estimator` between base and head as epoch-licensing, under the same epoch bump and new-note requirement. When none of them changed, require them to be equal. In `check_regression`, also compare `report.retriever` and `report.chunker` with the baseline.

### WR-06: README contradicts the code on the heading path of pre-heading content

**File:** `contracts/kb/eval/README.md:35` vs `crates/vox-search/src/kb_ids.rs:166-172`, `crates/vox-search/src/kb_eval/bm25.rs:32-34`
**Issue:** The README says "Content before the first heading has the empty heading path (`doc_path#`)". The code assigns the doc title as the root path (`chunk_doc` starts at `heading_path = title`, and `doc_heading_paths` always inserts `title`, never `""`). A qrel authored per the README (`docs/src/x.md#`) is rejected by `GoldenSet::load` with "heading_path \"\" does not exist" (golden.rs:348). This file is the contract that qrel authors follow.
**Fix:** Change the README line to: "Content before the first heading belongs to the root chunk `doc_path#<title>`."

### WR-07: Doc ids may contain `#`, but chunk ids are split at the first `#`, so such docs are silently mis-scored

**File:** `crates/vox-search/src/kb_ids.rs:126-128`, `146-164`
**Issue:** `is_kb_corpus_path` accepts `docs/src/c#-notes.md`. `split_chunk_id("docs/src/c#-notes.md#Title")` then yields the doc `docs/src/c`. In `run_eval` (mod.rs:199-203) that doc never matches its doc qrel, so the doc-level metrics for that query silently drop. Its chunk qrels fail to load with a misleading "doc is not in corpus.jsonl" error. No such path exists today (0 tracked under `docs/src`), but nothing prevents one.
**Fix:** Reject `#` in `is_kb_corpus_path`, which then also excludes it from the snapshot. Add a test case:
```rust
if rel.is_empty() || rel.starts_with('/') || rel.contains(['\\', ':', '#']) || !rel.ends_with(".md") {
```

### WR-08: The query-fingerprint leak scan covers less than the README promises

**File:** `crates/vox-cli/src/commands/kb/gate.rs:82-89`
**Issue:** The README (line 7) forbids pasting golden queries into "docs, research notes, plans, code comments, commit messages". `fingerprint_doc_paths` scans only `.md` files under `docs/` or `crates/`. Tracked `.planning/**/*.md` plans, `.rs`/`.vox` comments, `contracts/**` YAML/JSON and root-level docs are never fingerprinted. I checked: no current leak exists in any tracked text file. However, the gate would not catch a future paste into a plan or a code comment, which are the most likely places for an agent to paste a query.
**Fix:** Fingerprint every tracked UTF-8 text file outside `contracts/kb/eval/` and `archive/`, with the same exclusion list as `scan_paths_for_guid`. Skip binary files by checking for NUL bytes. At minimum, add `.planning/` and `*.rs` comments.

## Info

### IN-01: Pre-existing doc comment now attaches to the new `base_ref_or_default`

**File:** `crates/vox-cli/src/commands/ci/pre_push.rs:862-873`
**Issue:** The orphaned "Returns a `Command` that invokes the current `vox` binary…" doc block now prefixes `base_ref_or_default`. Rustdoc renders it as that function's description.
**Fix:** Move the orphaned block to the function it describes, or delete it.

### IN-02: Mode flags are silently ignored in combinations that clap accepts

**File:** `crates/vox-cli/src/commands/kb/eval.rs:28-35,73-91`
**Issue:** `--json` is accepted, then ignored, alongside `--gate`, `--tighten`, `--write-baseline`, `--query`, `--headings` and `--snapshot-corpus`. `--split test --query x` silently drops the split.
**Fix:** Add `conflicts_with_all` for `json` and `split` against the special modes.

### IN-03: A parse error in the HEAD baseline is also reported as "baseline deleted"

**File:** `crates/vox-cli/src/commands/kb/gate.rs:225-231,252`
**Issue:** When `read_baseline` fails to parse, `head` becomes `None` and `tighten_failures` adds a misleading "baseline deleted at head but present at base" line.
**Fix:** Skip `tighten_failures`' deletion message when the file exists but failed to parse, or pass a tri-state.

### IN-04: Bootstrap and epoch values are never range-checked

**File:** `crates/vox-search/src/kb_eval/baseline.rs:219-224`
**Issue:** At bootstrap (base absent), head content is unchecked: any tolerance (including negative or ≥1) and any metric values pass. Later, tolerance may only fall, so a negative tolerance would permanently redden the gate.
**Fix:** Validate `0.0 <= tolerance <= INITIAL_TOLERANCE`, metrics in `[0,1]`, and `schema_version == 1` on every head baseline.

### IN-05: The frozen baseline silently depends on shared ingest and tokenizer code

**File:** `crates/vox-search/src/ingest.rs:89`, `crates/vox-search/src/memory_hybrid.rs:56`, `crates/vox-search/src/kb_eval/bm25.rs:13-15`
**Issue:** The visibility changes are safe. However, any later edit to `chunk_markdown_sections_with_size` or `tokenize` for the RAG path changes the eval chunker and tokenizer without touching `CHUNKER`/`RETRIEVER`. Gains are invisible, and drops hit WR-05's dead end.
**Fix:** Pin a golden test of `chunk_doc` and `tokenize` output in `kb_eval`, or copy the ~25 lines in under the defactor rule.

### IN-06: `--write-baseline` with an unchanged golden set writes an epoch the gate will reject

**File:** `crates/vox-cli/src/commands/kb/gate.rs:294-301`
**Issue:** `new_epoch` bumps the epoch unconditionally. When the golden set is unchanged versus the base, the next `--gate` fails with "epoch changed without a golden-set change", and nothing warns at write time.
**Fix:** Warn, or refuse, when the golden-set diff against `origin/main` is empty and a baseline already exists.

### IN-07: The `Debug` derive on `GoldenSet` and `QueryMeta` prints the canary GUID

**File:** `crates/vox-search/src/kb_eval/golden.rs:88,115`
**Issue:** `#[derive(Debug)]` includes the `canary` fields. Any future `{:?}`, `dbg!`, or `assert_eq!` failure on these types writes the real GUID to logs or CI output. No such call exists today.
**Fix:** Write a manual `Debug` impl that redacts `canary`, or wrap the GUID in a newtype with a redacting `Debug`.

### IN-08: The research-eval sample field is still named `recall_at_5`

**File:** `crates/vox-cli-research/src/eval.rs:96,215`
**Issue:** The metric was renamed to `answer_token_overlap` in the JSON and schema, but the sample struct field is still `recall_at_5`. This perpetuates the misnomer the rename was fixing.
**Fix:** Rename the field. Add a serde alias if persisted samples must keep parsing.

### IN-09: The turso FTS probe depends on Cargo feature unification

**File:** `crates/vox-db/tests/turso_fts_probe.rs:56-71`
**Issue:** `native_fts_index_needs_turso_fts_feature` asserts that the `fts` cargo feature is absent. Features unify per build set, so `cargo test --workspace` and `cargo test -p vox-db` can disagree once any crate enables `turso/fts`.
**Fix:** Note this in the doc comment, or assert the feature state via `cfg` instead of an error string.

### IN-10: The 10 MB `corpus.jsonl` is committed with `-diff`

**File:** `.gitattributes:82`, `contracts/kb/eval/corpus.jsonl`
**Issue:** Every re-snapshot adds roughly 10 MB to history. Because of `-diff`, reviewers of a re-epoch (which the README requires to be human-reviewed) cannot see which corpus docs changed.
**Fix:** Drop `-diff` (keep `linguist-generated`), or record a per-doc hash manifest that does diff.

### IN-11: The base tip and the merge-base are mixed in `tighten_failures`

**File:** `crates/vox-cli/src/commands/kb/gate.rs:190,203`
**Issue:** The base baseline is read at the tip of `base`, but `golden_changed` uses `base...HEAD`, which is relative to the merge-base. An un-rebased branch, after `main` re-epochs or tightens, gets "epoch changed" or "lowered" errors that blame the branch. This fails closed, but the cause is confusing. Separately, `bm25.rs:150` uses platform `f64::ln`, and the 1e-9 quantization does not fully guarantee cross-platform tie stability.
**Fix:** Read the base baseline at `git merge-base base HEAD`, or say "rebase onto {base}" in the error.

---

_Reviewed: 2026-10-05T06:42:54Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
