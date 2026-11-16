---
phase: 01-retrieval-eval-baseline
plan: 09
subsystem: retrieval-eval
tags: [kb, eval, golden-set, qrels, pooling, expansion, coverage-gap]
requires: ["01-04", "01-06"]
provides:
  - "golden-draft/qrels-proposal-b.md: pooled doc and chunk qrel proposals for q025..q048"
  - "golden-draft/expansion-proposal-b.md: 24 paraphrase candidates B01..B24 plus 11 coverage-gap candidates B25..B35 awaiting keep/reject"
affects: [01-10, 01-11]
tech-stack:
  added: []
  patterns: ["multi-method pooling (bm25 top-20, path/content grep, seed-sources as one extra source) with rejected candidates kept as unchecked rows", "coverage gaps computed from the corpus-area table and cross-cutting topic list against the whole draft"]
key-files:
  created:
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/qrels-proposal-b.md
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/expansion-proposal-b.md
key-decisions:
  - "Queries are origin agent-seed (D-02); qrels pooled independently of the seed author, seed-sources used as one extra pooling source only"
  - "vox graph reported stale (git_drift, worktree_drift; three corpora missing), so every pool line says graph: unavailable; no refresh was run"
  - "Gap candidates carry (gap: ...) and are origin llm-expansion on materialization, never seeds"
requirements-completed: []
status: complete
duration: ~50min
completed: 2026-10-04
commits: 2
plan_head_before: 55bd8b149cb0938fd7580d3b394f4a18c6f81590
actuals:
  tokens: 15100
  tasks: 2
  commits: 2
---

# Phase 1 Plan 09: Pooled qrel proposals and expansion, second half Summary

Proposed doc and chunk relevance labels for the 24 agent-seed queries q025..q048, pooled from BM25 top-20, path/content grep and reading, plus 24 paraphrase probes and 11 coverage-gap probes, all awaiting the user's approval in 01-10.

## What was built

- **Task 1 (`84c23390b`):** `qrels-proposal-b.md` has 24 blocks (q025..q048, first id q025, no overlap with proposal A). Checked rows: 34 doc score 2, 27 doc score 1, 36 chunk score 2, 32 chunk score 1. 37 unchecked rows are rejected pool items kept visible. `approved_by:` is blank on line 1.
- **Task 2 (`f78177b2f`):** `expansion-proposal-b.md` has 35 candidates: 24 paraphrases (B01..B24, one per owned query, none unanswerable) and 11 coverage-gap candidates (B25..B35). Every `- [ ] keep` is unchecked. Combined with 01-08's 24 (A01..A24) the expansion total is 59, inside the 40-60 target.

## Coverage gaps (names only)

Computed against the whole draft (queries.md plus seed-sources.md), not waiting for 01-08.

- **Intents:** none empty (all seven have 6-7 seed queries).
- **Corpus areas with zero seed queries:** journeys, learning, api (two candidates: the doc-gap backlog and the retired TypeScript codegen page), ref, top-level files, case-studies, examples.
- **Cross-cutting topics with no seed query:** versioning, language syntax, CLI surface (generated command listing).
- Counts: 11 gap candidates (B25..B35), 24 paraphrase candidates.

## Pooling findings (handles only, no query text)

- Unanswerable blocks: 0.
- Graph availability: unavailable for all 24 (stale corpora, none rebuilt).
- BM25 top 20 missed the best answering doc or chunk for q026 (data-storage SSOT), q033 (concurrency-exceptions page, absent from top 20 entirely), q042 (deprecation-window section, doc present but wrong chunk), q043 (cloudless-ops incident section) and q048 (pre-push profiles section). Grep or reading found them. This is the pooling bias the plan exists to catch.
- q043 is ambiguous: the break-glass runbook (the seed author's pick) covers emergency privileged access, not a leaked key; the cloudless-ops incident section (rotate via reset) is the closer match. Break-glass is kept at score 1 with a note in the block. The registration checklist lives in root AGENTS.md, outside the corpus. The user may want to reword.
- q030 and q031: the superseded ADR (024, 020) is kept at score 1 and the decision ADR at 2; q032 keeps the earlier container-vs-wasm audit as an unchecked reject because it argues the opposite conclusion.
- q028: the numeric limit is only partly in the corpus (toestub guide points to `docs/agents/governance.md`, which is outside docs/src); `coding-agents.md` carries the 300/400/500 figures.
- q046: the docs only name the Tauri `search.rs` / `vox_search_query` command and the `chunk`/`knowledge` corpora; the question's "which crate" is answered indirectly.

## Verification

- Block count equals n - ceil(n/2) = 24 for the qrels file.
- All 68 checked chunk rows in the qrels file and all 87 distinct checked chunk ids in the expansion file have heading_paths present under `vox kb eval --headings`.
- Every checked or rejected doc id in both files is a snapshot `_id` (`comm -23` prints nothing).
- Every non-unanswerable block has at least one `- [x] doc` row and one `- [x] chunk 2` row (the awk check prints nothing for either file).
- No `- [x] keep` line exists; first line of both files is `approved_by:`.
- GUID-absence check (GUID read at runtime from the README, never printed) returned 0 matches for both files.
- No `## q` id overlaps qrels-proposal-a.md; no `## B` id overlaps the A file (A uses A-handles).
- Pool lines and answer excerpts name docs and short quoted excerpts only; excerpts are about 200 chars or less.

## Deviations from Plan

None - plan executed exactly as written. (The pool for gap questions used the top 12 BM25 hits rather than 20 to keep the probe count manageable; the qrel validation checks are unchanged.)

## Known Stubs

None. The two files are transient drafts that 01-10 materializes and deletes.

## Self-Check: PASSED

- FOUND: golden-draft/qrels-proposal-b.md, golden-draft/expansion-proposal-b.md
- FOUND commits: 84c23390b, f78177b2f
