---
phase: 01-retrieval-eval-baseline
plan: 10
subsystem: retrieval-eval
tags: [kb, eval, golden-set, beir, held-out, contract-test]
requires: ["01-08", "01-09"]
provides:
  - "contracts/kb/eval/queries.jsonl: 102 BEIR queries (48 agent-seed, 54 llm-expansion), canary on every line"
  - "contracts/kb/eval/qrels/{dev,test}{,.chunks}.tsv: stored held-out split"
  - "crates/vox-search/tests/kb_eval_golden_contract.rs: contract test over the real golden set"
affects: [01-11]
tech-stack:
  added: []
  patterns: ["stored split by (origin, intent) group, every 4th query starting at the 3rd, so the held-out share is 25%", "canary injected via sed + jq --arg only; no GUID literal anywhere"]
key-files:
  created:
    - contracts/kb/eval/queries.jsonl
    - contracts/kb/eval/qrels/dev.tsv
    - contracts/kb/eval/qrels/test.tsv
    - contracts/kb/eval/qrels/dev.chunks.tsv
    - contracts/kb/eval/qrels/test.chunks.tsv
    - crates/vox-search/tests/kb_eval_golden_contract.rs
  modified: []
key-decisions:
  - "Task 1 resolved by delegated agent review (01-CONTEXT D-02); every seed is origin agent-seed, kept expansion is llm-expansion"
  - "Split offset changed from the plan's literal 'every 4th' (positions 4, 8, ...) to positions 3, 7, ... because the literal rule yields 16.7% test, below the 20-35% contract bound"
requirements-completed: [EVAL-02]
status: complete
duration: ~25min
completed: 2026-10-04
commits: 1
plan_head_before: 080fd228267c30f0dd6c150a4e285f4b3ed06f48
actuals:
  tokens: 22400
  tasks: 2
  commits: 1
---

# Phase 1 Plan 10: Materialize the BEIR golden set Summary

`contracts/kb/eval/` now holds the approved BEIR golden set: 48 agent-seed and 54 llm-expansion queries with doc and chunk qrels, a stored dev/test split, the README canary on every query line, and a contract test that passes against the real data. The drafts are deleted.

## Task 1 (resolved before this run): approval

Done under the user's delegation (01-CONTEXT D-02). An independent agent reviewer (not the seed author, not the 01-08/01-09 proposers) set `approved_by: agent-reviewer (Claude Opus, delegated by brbrainerd 2026-10-04; see 01-CONTEXT D-02)` on all four proposal files and wrote `golden-draft/review-log.md`. Its verdicts, counts only:

- Seeds reviewed: 48 (q001..q048). Dropped: 0. Reworded for lexical echo: 12 (intent unchanged, origin stays agent-seed). Approved as proposed: 36, of which 6 carried a flag the reviewer resolved in place (q022, q028, q030, q031, q032, q043).
- Row edits: 2 rows unchecked, 5 scores raised 1 to 2, no rows added; the same edits were mirrored into the paraphrase blocks that copy those seeds.
- Expansion: 59 candidates reviewed, 54 kept (A 19 of 24, B 35 of 35 including all 11 coverage-gap candidates), 5 dropped as near-duplicates of reworded seeds.
- Every checked doc id matched a snapshot `_id`; every checked chunk heading path resolved via `vox kb eval --headings`. The chunker splits only at `#` and `##`, so a `###` qrel path resolves to its `##` section at eval time, as the loader intends.
- Seeds per intent: where 7, how-to 7, concept 7, policy 7, why 7, troubleshoot 7, multi-hop 6.
- Weak spots the reviewer recorded (no query text here): three seeds have thin or negative answers (one internally soft policy source, one thin base-URL answer, one "no such tag" answer). They stay, flagged for 01-11 triage.

## What was built (Task 2, `9378de6dd`)

- Test written first and run to RED (queries.jsonl absent, 5 of 5 failed), then GREEN after materialization: 5 of 5 pass.
- `queries.jsonl`: 102 lines sorted by `_id`; seeds keep qNNN ids (no gaps), kept expansion is x001..x054 (A blocks then B blocks). Metadata: origin, author (draft `author:` line for seeds, `llm-expansion (gsd-executor)` for expansion), `approved_by` from the owning proposal file, created 2026-10-04, intent from the block heading, canary read from the README at runtime.
- Qrels TSVs: dev 180 doc rows / 214 chunk rows; test 47 doc rows / 67 chunk rows. Header tab-separated, sorted by (query-id, corpus-id).
- Conversion was an in-session script kept outside the repo (scratchpad); none committed. Nocanary temp file removed.
- `golden-draft/` (all six tracked files, plus the untracked review-log, already folded into this summary) deleted.

### Counts (query counts)

| split | agent-seed | llm-expansion | total |
|---|---|---|---|
| dev | 35 | 41 | 76 |
| test | 13 | 13 | 26 |

Test fraction 26 of 102 = 25.5%. Seed total 48 (>= 40); llm-expansion 54.

| intent | dev seed | test seed | dev expansion | test expansion |
|---|---|---|---|---|
| where | 5 | 2 | 7 | 2 |
| how-to | 5 | 2 | 8 | 2 |
| concept | 5 | 2 | 5 | 2 |
| policy | 5 | 2 | 5 | 2 |
| why | 5 | 2 | 5 | 2 |
| troubleshoot | 5 | 2 | 6 | 2 |
| multi-hop | 5 | 1 | 5 | 1 |

## Deviations from Plan

**1. [Rule 1 - Bug] Split rule gave a held-out share below the contract bound**
- **Found during:** Task 2 materialization.
- **Issue:** the literal AP-14 rule (positions 4, 8, ... in each group) gives groups of 6-10 only 1 test query: 17 of 102 = 16.7%, which fails the plan's own `test_fraction_in_range` (0.20..=0.35).
- **Fix:** same stored every-4th stride, started at position 3 (3, 7, ...). Result 13 + 13 test, 25.5%, both origins and all seven intents represented in test. Data-side only; the loader and test bound untouched.
- **Files modified:** contracts/kb/eval/qrels/*.tsv
- **Commit:** 9378de6dd

Otherwise the plan ran as amended for D-02.

## Known caveat for 01-11

Several expansion paraphrases are in dev while the seed they paraphrase is in test, so dev tuning can see a rewording of a held-out question. Split assignment was kept to the plan's per-(origin, intent) rule. If the held-out claim must be strict, reassign paraphrases to their source seed's split in a reviewed re-epoch.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- Files exist: queries.jsonl, four qrels TSVs, contract test (verified by test run); golden-draft and target/kb-eval/queries.nocanary.jsonl absent.
- Commit `9378de6dd` exists; `git rev-list --count` from plan_head_before = 1.
- Canary check: zero query lines with a canary differing from the README; no GUID in crates, commits or this summary.
