---
phase: 1
phase_name: "retrieval-eval-baseline"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 54
  lessons: 14
  patterns: 19
  surprises: 15
missing_artifacts:
  - "UAT.md"
  - "STATE.md"
---

# Phase 1 Learnings: retrieval-eval-baseline

## Decisions

### heading_path = title > H2 > H3 joined by ' › ' (U+203A with spaces); H1 ...
heading_path = title > H2 > H3 joined by ' › ' (U+203A with spaces); H1 never a segment (AP-06)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-01-SUMMARY.md

---

### run_eval takes the retriever's doc->chunk-id map as an explicit argument...
run_eval takes the retriever's doc->chunk-id map as an explicit argument so it stays retriever-agnostic

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-01-SUMMARY.md

---

### golden validation tests live in golden/validation_tests.rs to keep golde...
golden validation tests live in golden/validation_tests.rs to keep golden.rs under the 500-line limit

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-01-SUMMARY.md

---

### Task 1 (tracer, 33a7aab0e)
**Task 1 (tracer, `33a7aab0e`)**: fixture golden set through chunking, BM25 and metrics to a deterministic report. The hand-computed 3-query fixture yields exactly R@5=0.333333, R@10=0.666667, MRR@10=0.380952, nDCG@10=0.364507.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Task 2 (436afcfe2)
**Task 2 (`436afcfe2`)**: answer-in-budget (B=1500, `ceil(bytes/4)`, prefix semantics), ancestor-or-equal chunk-qrel resolution, run dedupe, `dev/all` + `dev/<origin>` keys, `to_json_string`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Task 3 (400b13833)
**Task 3 (`400b13833`)**: strict validation (deny_unknown_fields, closed origin/intent, scores 0..=2, duplicate/unknown-id rejection, canary match, heading_path existence), `validate_splits`, `snapshot_corpus_jsonl`, `load_corpus`, `load_query_texts`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-01-SUMMARY.md

---

### Helper bodies moved verbatim (bit-identical values); only names and docs...
Helper bodies moved verbatim (bit-identical values); only names and docs changed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-02-SUMMARY.md

---

### recall_at_5 sample field and DB column left alone (needs a migration, AP...
recall_at_5 sample field and DB column left alone (needs a migration, AP-04)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-02-SUMMARY.md

---

### Task 1 (2f8f90585)
**Task 1 (`2f8f90585`)**: new `answer_overlap.rs` with `answer_token_overlap(model, gold)` and `answer_keyword_hit_rate(model, evidence)` plus 6 tests (5 relocated edge cases and one overlap test, now asserting the exact value 0.8). `eval.rs` calls them directly and lost the `compute_token_recall` wrapper and its test. `evaluation.rs` keeps only the planner-intent fixtures; `semcov_wave21_tests.rs` lost the import and the "Evaluation helpers" block.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Task 2 (0b671697f)
**Task 2 (`0b671697f`)**: `citation_recall` renamed to `answer_token_overlap` in the eval report, `results.v1.schema.json` (required + properties) and `baseline.v1.json`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-02-SUMMARY.md

---

### Error text asserted on Display + Debug combined, so a substring match su...
Error text asserted on Display + Debug combined, so a substring match survives either rendering

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-03-SUMMARY.md

---

### No scratch code committed for S2; reproduction recipe is prose only
No scratch code committed for S2; reproduction recipe is prose only

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-03-SUMMARY.md

---

### snapshot_decision
snapshot_decision: frozen-snapshot (user, Task 1, D-01)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-04-SUMMARY.md

---

### Seed queries are agent-authored and labelled origin agent-seed, never hu...
Seed queries are agent-authored and labelled origin agent-seed, never human (D-02)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-04-SUMMARY.md

---

### Task 1 (decision, no commit)
**Task 1 (decision, no commit):** the user chose `frozen-snapshot` (01-CONTEXT D-01). 01-06 Task 2 may build the ~10 MB `corpus.jsonl` snapshot.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-04-SUMMARY.md

---

### Task 2 (6cd05af6c)
**Task 2 (`6cd05af6c`):** `contracts/kb/eval/README.md` documents the BEIR layout, id encoding (doc path, `doc_path#title › H2 › H3`, U+203A joiner, H1 never a segment), query metadata, the stored split rule, held-out policy (`--split test` at milestone close only), the re-epoch rule (`retrieval-baseline.v1.json`, `epoch_note`) and the never-paste-queries rule. It holds exactly one canary line with a fresh lowercase UUID. `golden-draft/queries.md` was created as a coverage scaffold: 7 intents, a corpus-area table (670 tracked non-archive docs under `docs/src`), real-question sources and authoring rules, with no example wording.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-04-SUMMARY.md

---

### Task 3 (fc02b1bb6, plus coordinator commits e5f44982f, bbefbe2b0)
**Task 3 (`fc02b1bb6`, plus coordinator commits `e5f44982f`, `bbefbe2b0`):** seeds were written by an agent after the user delegated authoring (see below). The draft holds **48** queries; per intent: where 7, how-to 7, concept 7, policy 7, why 7, troubleshoot 7, multi-hop 6. `author:` reads `agent-seed (Claude Opus, delegated by brbrainerd 2026-10-04; see 01-CONTEXT D-02)`. The sidecar `seed-sources.md` maps each query number to the answering doc paths (paths only, no query text).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-04-SUMMARY.md

---

### LevelMetrics gained Deserialize + deny_unknown_fields so Baseline reuses...
LevelMetrics gained Deserialize + deny_unknown_fields so Baseline reuses it instead of a parallel struct

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-05-SUMMARY.md

---

### tighten-only with golden unchanged also pins epoch (an epoch bump withou...
tighten-only with golden unchanged also pins epoch (an epoch bump without a golden change is an error, so it cannot launder a loosening)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-05-SUMMARY.md

---

### query_fingerprint_leaks and texts_containing_guid take (label/path, text...
query_fingerprint_leaks and texts_containing_guid take (label/path, text) pairs, not CorpusDoc, so the CLI can feed repo files, MENS pairs and jsonl lines

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-05-SUMMARY.md

---

### Task 1 (44a062cea)
**Task 1 (`44a062cea`)**: `baseline.rs`. Schema v1 (`deny_unknown_fields`, byte-identical JSON round trip), `check_regression` (6 dp, drop must exceed tolerance), `check_tighten_only`, `improvements`, `tightened`, `new_epoch`, `ratchet_split() == Split::Dev`, `RATCHET_KEYS`, `INITIAL_TOLERANCE`. 12 tests.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-05-SUMMARY.md

---

### Task 2 (7a2193325)
**Task 2 (`7a2193325`)**: `canary.rs`. 6 tests, placeholder GUID `aaaaaaaa-bbbb-4ccc-8ddd-eeeeeeeeeeee` only.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-05-SUMMARY.md

---

### Task 3 (66dca6a2a)
**Task 3 (`66dca6a2a`)**: `tests/kb_eval_gate.rs`. Real retriever passes its own baseline; reversed ranking and drop-top-hit both fail on `dev/all`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-05-SUMMARY.md

---

### run_report validates splits only for --split test; dev never calls valid...
run_report validates splits only for --split test; dev never calls validate_splits (it would read test qrels)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-06-SUMMARY.md

---

### Task 1 (161919567)
**Task 1 (`161919567`)**: `commands/kb.rs` (`KbCmd::Eval`, `run`) and `commands/kb/eval.rs` (`KbEvalArgs`, `run_report`, `run_query`, `run_headings`, `run_snapshot`). `Cli::Kb` wired at all three dispatch match sites. 7 tests (3 clap, 4 fixture-driven). No raw `git`: the snapshot lists files through `vox_git::read_cmd::read_only`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-06-SUMMARY.md

---

### Task 2 (1151d3aa6)
**Task 2 (`1151d3aa6`)**: `contracts/kb/eval/corpus.jsonl`, `linguist-generated=true -diff` in `.gitattributes`, `.voxignore` entry with `.cursorignore`/`.aiignore`/`.aiexclude` regenerated via `vox ci sync-ignore-files`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-06-SUMMARY.md

---

### Typed CorpusHasCanary error (downcast in run_gate) instead of string mat...
Typed CorpusHasCanary error (downcast in run_gate) instead of string matching, so a GUID in the corpus is reported as [canary]

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-07-SUMMARY.md

---

### Local jsonl scan reads one file at a time and passes it to texts_contain...
Local jsonl scan reads one file at a time and passes it to texts_containing_guid (memory bounded to one file)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-07-SUMMARY.md

---

### Base ref starting with '-' is rejected before any git call (option-injec...
Base ref starting with '-' is rejected before any git call (option-injection guard on top of the read_only allowlist)

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-07-SUMMARY.md

---

### Task 1 (bc160d852)
**Task 1 (`bc160d852`)**: new flags `--gate`, `--base REF` (default `origin/main`, requires `--gate`), `--tighten`, `--write-baseline`, `--epoch-note` (mutually required with write-baseline). Gate/tighten/write-baseline are mutually exclusive and conflict with `--split/--query/--headings/--snapshot-corpus`; all use `ratchet_split()`. `gate.rs` holds `run_tighten` (never lowers, writes only on change, errors without a baseline), `run_write_baseline` (needs a non-empty note, epoch 1 or +1) and the shared `dev_report`. `ensure_clean` (index-time canary assert) is called by `dev_report`, `run_report` and `run_query`; it names doc ids, never the GUID.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-07-SUMMARY.md

---

### Task 2 (c0c476ff1)
**Task 2 (`c0c476ff1`)**: `run_gate(root, base)` collects every failure: report load/regression vs the working-tree baseline; base ref resolved fail-closed; base baseline via `git show BASE:...` (absent = bootstrap); golden-changed via `git diff --name-only BASE...HEAD` on queries/qrels/corpus (AP-15) feeding `check_tighten_only`; canary sweep (repo scan over `git ls-files`, root disjointness, real MENS docs extractor via `walk_and_extract_docs` on `root/docs`, local `mens/data` + `target/dogfood` jsonl, 40-char query fingerprints in tracked `docs/`+`crates/` markdown minus archive and the golden dir). Output sorted `[canary] < [regression] < [tighten-only]`, deduped; hints and elapsed go to stderr. `cli.md` documents the flags, categories and the dev-split rule.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-07-SUMMARY.md

---

### Queries are origin agent-seed (D-02); qrels pooled independently of the ...
Queries are origin agent-seed (D-02); qrels pooled independently of the seed author, seed-sources used as one extra pooling source only

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-08-SUMMARY.md

---

### vox graph reported stale (git_drift, worktree_drift), so every pool line...
vox graph reported stale (git_drift, worktree_drift), so every pool line says graph: unavailable

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-08-SUMMARY.md

---

### Task 1 (330c150dc)
**Task 1 (`330c150dc`):** `qrels-proposal-a.md` has 24 blocks. Counts of checked rows: 25 doc score 2, 20 doc score 1, 34 chunk score 2, 31 chunk score 1. 25 unchecked rows are rejected pool items kept visible for the user. `approved_by:` is blank on line 1.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-08-SUMMARY.md

---

### Task 2 (07cd4d9e8)
**Task 2 (`07cd4d9e8`):** `expansion-proposal-a.md` has 24 candidates (one per owned query, none unanswerable), each with an unchecked `- [ ] keep` line and the source query's checked qrel rows copied in.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-08-SUMMARY.md

---

### vox graph reported stale (git_drift, worktree_drift; three corpora missi...
vox graph reported stale (git_drift, worktree_drift; three corpora missing), so every pool line says graph: unavailable; no refresh was run

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-09-SUMMARY.md

---

### Gap candidates carry (gap
Gap candidates carry (gap: ...) and are origin llm-expansion on materialization, never seeds

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-09-SUMMARY.md

---

### Task 1 (84c23390b)
**Task 1 (`84c23390b`):** `qrels-proposal-b.md` has 24 blocks (q025..q048, first id q025, no overlap with proposal A). Checked rows: 34 doc score 2, 27 doc score 1, 36 chunk score 2, 32 chunk score 1. 37 unchecked rows are rejected pool items kept visible. `approved_by:` is blank on line 1.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-09-SUMMARY.md

---

### Task 2 (f78177b2f)
**Task 2 (`f78177b2f`):** `expansion-proposal-b.md` has 35 candidates: 24 paraphrases (B01..B24, one per owned query, none unanswerable) and 11 coverage-gap candidates (B25..B35). Every `- [ ] keep` is unchecked. Combined with 01-08's 24 (A01..A24) the expansion total is 59, inside the 40-60 target.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-09-SUMMARY.md

---

### Task 1 resolved by delegated agent review (01-CONTEXT D-02); every seed ...
Task 1 resolved by delegated agent review (01-CONTEXT D-02); every seed is origin agent-seed, kept expansion is llm-expansion

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-10-SUMMARY.md

---

### Split offset changed from the plan's literal 'every 4th' (positions 4, 8...
Split offset changed from the plan's literal 'every 4th' (positions 4, 8, ...) to positions 3, 7, ... because the literal rule yields 16.7% test, below the 20-35% contract bound

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-10-SUMMARY.md

---

### Test written first and run to RED (queries.jsonl absent, 5 of 5 failed),...
Test written first and run to RED (queries.jsonl absent, 5 of 5 failed), then GREEN after materialization: 5 of 5 pass.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-10-SUMMARY.md

---

### queries.jsonl
`queries.jsonl`: 102 lines sorted by `_id`; seeds keep qNNN ids (no gaps), kept expansion is x001..x054 (A blocks then B blocks). Metadata: origin, author (draft `author:` line for seeds, `llm-expansion (gsd-executor)` for expansion), `approved_by` from the owning proposal file, created 2026-10-04, intent from the block heading, canary read from the README at runtime.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-10-SUMMARY.md

---

### Qrels TSVs: dev 180 doc rows / 214 chunk rows; test 47 doc rows / 67 chu...
Qrels TSVs: dev 180 doc rows / 214 chunk rows; test 47 doc rows / 67 chunk rows. Header tab-separated, sorted by (query-id, corpus-id).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-10-SUMMARY.md

---

### Conversion was an in-session script kept outside the repo (scratchpad); ...
Conversion was an in-session script kept outside the repo (scratchpad); none committed. Nocanary temp file removed.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-10-SUMMARY.md

---

### golden-draft/ (all six tracked files, plus the untracked review-log, alr...
`golden-draft/` (all six tracked files, plus the untracked review-log, already folded into this summary) deleted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-10-SUMMARY.md

---

### Gate fits the fast tier
Gate fits the fast tier: median 5.06 s wall (debug) against the 10 s budget, so no AP-05/AP-12 fallback was needed

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-11-SUMMARY.md

---

### Mutation (a) was proven on two files
Mutation (a) was proven on two files: the plan's named research doc is training_eligible:false so the MENS extractor correctly skips it; an eligible doc proves the extractor path

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-11-SUMMARY.md

---

### Task 1 (64e8af6fd)
**Task 1 (`64e8af6fd`)**: three real-data tests added first and run RED (3 of 8 failed: baseline file absent), then GREEN after `vox kb eval --write-baseline`: 8 of 8 pass. `contracts/index.yaml` gained `kb-retrieval-baseline` and `kb-eval-golden-set`; `vox ci contracts-index` OK (193 entries).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-11-SUMMARY.md

---

### Task 2 (6a56512d1)
**Task 2 (`6a56512d1`)**: `base_ref_or_default` (pure) + `prepush_base_ref()` (the only reader of the override; `VOX_PREPUSH_BASE` appears once in the file) used by `git_diff_name_only_for_prepush` and the new `step_kb_eval_gate`, appended to the fast list after canonical-map-verify. Test `prepush_base_ref_defaults_to_origin_main` passes.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-11-SUMMARY.md

---

### Docs (72e69fe06)
**Docs (`72e69fe06`)**: fast-tier row in `docs/src/contributors/local-ci-pre-push.md` now lists the gate (not in the plan's file list; keeps the tier SSOT truthful).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-11-SUMMARY.md

---

### kb.eval row sets ref_cli_required
kb.eval row sets ref_cli_required: true (like drift-check) so compliance enforces the cli.md mention

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 01-12-SUMMARY.md

---

### Task 1 (c49e32a22)
**Task 1 (`c49e32a22`)**: `kb.eval` catalog row (handler `commands::kb`); `operations-sync --target cli --write` projected the command registry; `cli.md` gained a `vox kb eval` section (only the flags shipped in 01-06: `--split`, `--json`, `--query`/`--top`, `--headings`, `--snapshot-corpus`; `--split test` noted as milestone-close only) and a `kb` reachability row.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-12-SUMMARY.md

---

### Task 2 (c22b99bbe)
**Task 2 (`c22b99bbe`)**: where-things-live "Common tasks" row covering the CLI, `kb_eval` engine, `kb_ids.rs`, the golden set (never index/paste) and the tighten-only baseline path.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 01-12-SUMMARY.md

## Lessons

### None. The plan executed as written.
None. The plan executed as written.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-03-SUMMARY.md

---

### Notes: improvements is called with a 0.005 gain in a test to confirm sub...
Notes: `improvements` is called with a 0.005 gain in a test to confirm sub-tolerance gains stay silent. `check_tighten_only` additionally rejects a changed epoch when the golden set is unchanged (stricter than the plan's bullets, no conflict).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-05-SUMMARY.md

---

### 1. [Rule 1 - Bug] Split rule gave a held-out share below the contract bo...
**1. [Rule 1 - Bug] Split rule gave a held-out share below the contract bound**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-10-SUMMARY.md

---

### Found during
**Found during:** Task 2 materialization.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-10-SUMMARY.md

---

### Issue: the literal AP-14 rule (positions 4, 8, ... in each group) gives ...
**Issue:** the literal AP-14 rule (positions 4, 8, ... in each group) gives groups of 6-10 only 1 test query: 17 of 102 = 16.7%, which fails the plan's own `test_fraction_in_range` (0.20..=0.35).

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-10-SUMMARY.md

---

### Fix: same stored every-4th stride, started at position 3 (3, 7, ...). Re...
**Fix:** same stored every-4th stride, started at position 3 (3, 7, ...). Result 13 + 13 test, 25.5%, both origins and all seven intents represented in test. Data-side only; the loader and test bound untouched.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-10-SUMMARY.md

---

### 1. [Rule 1 - Plan defect] Mutation (a) target is excluded from MENS by d...
**1. [Rule 1 - Plan defect] Mutation (a) target is excluded from MENS by design**

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-11-SUMMARY.md

---

### Issue: knowledge-base-practices-research-2026.md carries training_eligib...
**Issue:** `knowledge-base-practices-research-2026.md` carries `training_eligible: false` (LLM-synthesized research), so `walk_and_extract_docs` correctly skips it; the plan's expectation that the MENS extractor names this file cannot hold. The repo scan did name it. Nothing in the gate is wrong.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-11-SUMMARY.md

---

### Fix: ran the same mutation (a2) on a tracked, eligible doc (where-things...
**Fix:** ran the same mutation (a2) on a tracked, eligible doc (`where-things-live.md`); both repo-scan and MENS extractor named it. Restored and verified.

**Context:** Recorded during phase execution and deviation tracking.
**Source:** 01-11-SUMMARY.md

---

### IN-02 (silently ignored mode flags)
**IN-02 (silently ignored mode flags):** A usability issue with no correctness or leak impact. Adding `conflicts_with_all` for `--json`/`--split` is small, but it changes the accepted CLI surface, so it is left for a deliberate CLI pass.

**Context:** Surfaced in structured phase code review.
**Source:** 01-REVIEW-FIX.md

---

### IN-05 (frozen baseline depends on shared chunker/tokenizer)
**IN-05 (frozen baseline depends on shared chunker/tokenizer):** This needs a design choice: either pin golden outputs or defactor about 25 lines out of `ingest.rs`/`memory_hybrid.rs`. WR-05 removes the dead end the review cited, because a deliberate chunker change can now re-epoch. A silent drift is still caught only by a metric move.

**Context:** Surfaced in structured phase code review.
**Source:** 01-REVIEW-FIX.md

---

### IN-08 (recall_at_5 sample field in vox-cli-research)
**IN-08 (`recall_at_5` sample field in vox-cli-research):** A rename in a different crate, and it may affect persisted samples (a serde alias decision). Not part of the kb gate.

**Context:** Surfaced in structured phase code review.
**Source:** 01-REVIEW-FIX.md

---

### None outstanding
None outstanding. The three items from the first pass are resolved under user delegation (see "Resolution of human items").

**Context:** Verified in phase acceptance checks.
**Source:** 01-VERIFICATION.md

---

### No gaps. All five success criteria hold when exercised, including three ...
No gaps. All five success criteria hold when exercised, including three live mutations that I reverted. The labelling items (SC2 wording, README "human-pruned") are fixed in the tree.

**Context:** Verified in phase acceptance checks.
**Source:** 01-VERIFICATION.md

## Patterns

### retriever as a closure Fn(&str)->Vec<RankedChunk>
retriever as a closure Fn(&str)->Vec<RankedChunk>

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-01-SUMMARY.md

---

### ordered collections only
ordered collections only

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-01-SUMMARY.md

---

### quantized BM25 ranking (score*1e9 as i64)
quantized BM25 ranking (score*1e9 as i64)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-01-SUMMARY.md

---

### honestly named, crate-private helpers instead of IR-named public fns
honestly named, crate-private helpers instead of IR-named public fns

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-02-SUMMARY.md

---

### characterization test that fails on dependency drift and names the decis...
characterization test that fails on dependency drift and names the decision it guards

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-03-SUMMARY.md

---

### canary written once by a shell pipeline, read at runtime by read_canary_...
canary written once by a shell pipeline, read at runtime by read_canary_guid

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-04-SUMMARY.md

---

### checks return Vec<String> tagged [regression] / [tighten-only]
checks return Vec<String> tagged [regression] / [tighten-only]

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-05-SUMMARY.md

---

### GUID only via argument
GUID only via argument

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-05-SUMMARY.md

---

### mode fns take explicit paths + a writer; stdout deterministic, diagnosti...
mode fns take explicit paths + a writer; stdout deterministic, diagnostics on stderr

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-06-SUMMARY.md

---

### CLI gathers inputs, decisions stay in 01-05 pure functions
CLI gathers inputs, decisions stay in 01-05 pure functions

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-07-SUMMARY.md

---

### all failures collected, sorted, deduped
all failures collected, sorted, deduped

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-07-SUMMARY.md

---

### typed CorpusHasCanary error so the gate tags it [canary] not [regression...
typed CorpusHasCanary error so the gate tags it [canary] not [regression]

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-07-SUMMARY.md

---

### multi-method pooling (bm25 top-20, path/content grep, seed-sources as on...
multi-method pooling (bm25 top-20, path/content grep, seed-sources as one extra source) with rejected candidates kept as unchecked rows

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-08-SUMMARY.md

---

### coverage gaps computed from the corpus-area table and cross-cutting topi...
coverage gaps computed from the corpus-area table and cross-cutting topic list against the whole draft

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-09-SUMMARY.md

---

### stored split by (origin, intent) group, every 4th query starting at the ...
stored split by (origin, intent) group, every 4th query starting at the 3rd, so the held-out share is 25%

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-10-SUMMARY.md

---

### canary injected via sed + jq --arg only; no GUID literal anywhere
canary injected via sed + jq --arg only; no GUID literal anywhere

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-10-SUMMARY.md

---

### single reader of the diff-base override (prepush_base_ref)
single reader of the diff-base override (prepush_base_ref)

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-11-SUMMARY.md

---

### mutation proofs with before/after presence checks and git diff --quiet r...
mutation proofs with before/after presence checks and git diff --quiet restore evidence

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-11-SUMMARY.md

---

### catalog -> operations-sync projection; generated.md left to ssot-autoreg...
catalog -> operations-sync projection; generated.md left to ssot-autoregen

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 01-12-SUMMARY.md

## Surprises

### Estimate variance on 01-01
Estimate variance on 01-01: planned 95000 tokens vs 17000 actual (0.18x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-01-PLAN.md, 01-01-SUMMARY.md

---

### Estimate variance on 01-02
Estimate variance on 01-02: planned 35000 tokens vs 6000 actual (0.17x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-02-PLAN.md, 01-02-SUMMARY.md

---

### Estimate variance on 01-03
Estimate variance on 01-03: planned 30000 tokens vs 6500 actual (0.22x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-03-PLAN.md, 01-03-SUMMARY.md

---

### Estimate variance on 01-04
Estimate variance on 01-04: planned 25000 tokens vs 6000 actual (0.24x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-04-PLAN.md, 01-04-SUMMARY.md

---

### Estimate variance on 01-05
Estimate variance on 01-05: planned 70000 tokens vs 17000 actual (0.24x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-05-PLAN.md, 01-05-SUMMARY.md

---

### Estimate variance on 01-06
Estimate variance on 01-06: planned 60000 tokens vs 2800 actual (0.05x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-06-PLAN.md, 01-06-SUMMARY.md

---

### Estimate variance on 01-07
Estimate variance on 01-07: planned 75000 tokens vs 7300 actual (0.10x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-07-PLAN.md, 01-07-SUMMARY.md

---

### Estimate variance on 01-08
Estimate variance on 01-08: planned 85000 tokens vs 11000 actual (0.13x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-08-PLAN.md, 01-08-SUMMARY.md

---

### Estimate variance on 01-09
Estimate variance on 01-09: planned 95000 tokens vs 15100 actual (0.16x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-09-PLAN.md, 01-09-SUMMARY.md

---

### Estimate variance on 01-10
Estimate variance on 01-10: planned 55000 tokens vs 22400 actual (0.41x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-10-PLAN.md, 01-10-SUMMARY.md

---

### 1. [Rule 1 - Bug] Split rule gave a held-out share below the contract bo...
**1. [Rule 1 - Bug] Split rule gave a held-out share below the contract bound**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 01-10-SUMMARY.md

---

### Issue: the literal AP-14 rule (positions 4, 8, ... in each group) gives ...
**Issue:** the literal AP-14 rule (positions 4, 8, ... in each group) gives groups of 6-10 only 1 test query: 17 of 102 = 16.7%, which fails the plan's own `test_fraction_in_range` (0.20..=0.35).

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 01-10-SUMMARY.md

---

### Estimate variance on 01-11
Estimate variance on 01-11: planned 65000 tokens vs 9500 actual (0.15x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-11-PLAN.md, 01-11-SUMMARY.md

---

### 1. [Rule 1 - Plan defect] Mutation (a) target is excluded from MENS by d...
**1. [Rule 1 - Plan defect] Mutation (a) target is excluded from MENS by design**

**Impact:** Required in-flight adjustment beyond the initial plan.
**Source:** 01-11-SUMMARY.md

---

### Estimate variance on 01-12
Estimate variance on 01-12: planned 30000 tokens vs 2500 actual (0.08x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 01-12-PLAN.md, 01-12-SUMMARY.md

