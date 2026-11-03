---
phase: 01-retrieval-eval-baseline
plan: 08
subsystem: retrieval-eval
tags: [kb, eval, golden-set, qrels, pooling, expansion]
requires: ["01-04", "01-06"]
provides:
  - "golden-draft/qrels-proposal-a.md: pooled doc and chunk qrel proposals for q001..q024"
  - "golden-draft/expansion-proposal-a.md: 24 paraphrase candidates A01..A24 awaiting keep/reject"
affects: [01-10, 01-11]
tech-stack:
  added: []
  patterns: ["multi-method pooling (bm25 top-20, path/content grep, seed-sources as one extra source) with rejected candidates kept as unchecked rows"]
key-files:
  created:
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/qrels-proposal-a.md
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/expansion-proposal-a.md
key-decisions:
  - "Queries are origin agent-seed (D-02); qrels pooled independently of the seed author, seed-sources used as one extra pooling source only"
  - "vox graph reported stale (git_drift, worktree_drift), so every pool line says graph: unavailable"
requirements-completed: []
status: complete
duration: ~35min
completed: 2026-10-04
commits: 2
plan_head_before: 81ec50b6db0f8f6f0a1ef3299443f5c87f664bea
actuals:
  tokens: 11000
  tasks: 2
  commits: 2
---

# Phase 1 Plan 08: Pooled qrel proposals and paraphrase expansion, first half Summary

Proposed doc and chunk relevance labels for the 24 agent-seed queries q001..q024 (ceil(48/2)), pooled from BM25 top-20, path/content grep and reading, plus 24 paraphrase probes A01..A24, all awaiting the user's approval in 01-10.

## What was built

- **Task 1 (`330c150dc`):** `qrels-proposal-a.md` has 24 blocks. Counts of checked rows: 25 doc score 2, 20 doc score 1, 34 chunk score 2, 31 chunk score 1. 25 unchecked rows are rejected pool items kept visible for the user. `approved_by:` is blank on line 1.
- **Task 2 (`07cd4d9e8`):** `expansion-proposal-a.md` has 24 candidates (one per owned query, none unanswerable), each with an unchecked `- [ ] keep` line and the source query's checked qrel rows copied in.

## Pooling findings (handles only, no query text)

- Unanswerable blocks: 0.
- Graph availability: unavailable for all 24 (corpus stale).
- BM25 top 20 missed the best answering doc for q010, q014 and q019; grep and reading found them. A fourth, q008, missed one supporting reference doc. This is the pooling bias the plan exists to catch.
- q010: the doc ranked best (`reference/skill_marketplace.md`, discovery roots) differs from the seed author's intended doc (`skill-ecosystem-interop-research`), which is kept at score 1.
- q022 and q013: the snapshot answer is partly ambiguous (the crypto SSOT page itself allows sha2/blake3 via `vox-crypto`; the stricter rule lives in `AGENTS.md`/root policy, outside the `docs/src` doc set; Ollama is documented as a cascade fallback rather than a switch). The user may want to reword these at approval.
- q024: `docs/src/AGENTS.md` is a corpus doc; it is labelled score 1 (required keys), with `documentation-governance` score 2 (category vocabulary).

## Verification

- Block count equals ceil(n/2) = 24 for both files.
- All 65 checked chunk ids (per file) have heading_paths present under `vox kb eval --headings`; every checked or rejected doc id is a snapshot `_id`.
- Every non-unanswerable block has at least one `- [x] doc` row and one `- [x] chunk 2` row.
- Every `keep` line is unchecked (24 of 24); first line of both files is `approved_by:`.
- GUID-absence check (GUID read at runtime from the README, never printed) returned 0 matches for both files.
- Answer excerpts are at most 200 chars.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None. The two files are transient drafts that 01-10 materializes and deletes.

## Self-Check: PASSED

- FOUND: golden-draft/qrels-proposal-a.md, golden-draft/expansion-proposal-a.md
- FOUND commits: 330c150dc, 07cd4d9e8
