---
phase: 01-retrieval-eval-baseline
plan: 04
subsystem: retrieval-eval
tags: [kb, eval, golden-set, canary, vox-search]
requires: []
provides:
  - "contracts/kb/eval/README.md: golden-set contract and the single runtime-readable canary line"
  - "golden-draft/queries.md: 48 agent-seed queries (GUID-free, deleted in 01-10)"
  - "golden-draft/seed-sources.md: query number to answering doc path map (no query text)"
  - "snapshot_decision recorded for 01-06 Task 2"
affects: [01-08, 01-09, 01-10, 01-11]
tech-stack:
  added: []
  patterns: ["canary written once by a shell pipeline, read at runtime by read_canary_guid"]
key-files:
  created:
    - contracts/kb/eval/README.md
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/queries.md
    - .planning/workstreams/kb/phases/01-retrieval-eval-baseline/golden-draft/seed-sources.md
key-decisions:
  - "snapshot_decision: frozen-snapshot (user, Task 1, D-01)"
  - "Seed queries are agent-authored and labelled origin agent-seed, never human (D-02)"
requirements-completed: []
status: complete
duration: ~1h (including the wait on the checkpoint)
completed: 2026-10-04
commits: 4
plan_head_before: a8e933696a54ef9cb57a35064e4e57c9ddf776e9
actuals:
  tokens: 6000
  tasks: 3
  commits: 4
---

# Phase 1 Plan 04: Golden-set contract README, canary and seed queries Summary

The golden-set contract README now holds the single canary line (written by a shell pipeline, never echoed), and the golden draft holds 48 GUID-free seed queries labelled `agent-seed` because the user delegated authoring.

snapshot_decision: frozen-snapshot

## What was built

- **Task 1 (decision, no commit):** the user chose `frozen-snapshot` (01-CONTEXT D-01). 01-06 Task 2 may build the ~10 MB `corpus.jsonl` snapshot.
- **Task 2 (`6cd05af6c`):** `contracts/kb/eval/README.md` documents the BEIR layout, id encoding (doc path, `doc_path#title › H2 › H3`, U+203A joiner, H1 never a segment), query metadata, the stored split rule, held-out policy (`--split test` at milestone close only), the re-epoch rule (`retrieval-baseline.v1.json`, `epoch_note`) and the never-paste-queries rule. It holds exactly one canary line with a fresh lowercase UUID. `golden-draft/queries.md` was created as a coverage scaffold: 7 intents, a corpus-area table (670 tracked non-archive docs under `docs/src`), real-question sources and authoring rules, with no example wording.
- **Task 3 (`fc02b1bb6`, plus coordinator commits `e5f44982f`, `bbefbe2b0`):** seeds were written by an agent after the user delegated authoring (see below). The draft holds **48** queries; per intent: where 7, how-to 7, concept 7, policy 7, why 7, troubleshoot 7, multi-hop 6. `author:` reads `agent-seed (Claude Opus, delegated by brbrainerd 2026-10-04; see 01-CONTEXT D-02)`. The sidecar `seed-sources.md` maps each query number to the answering doc paths (paths only, no query text).

## D-02 delegation note

The user first chose to author the seed queries, then delegated ("solve these yourself"). Mining the user's past session prompts into this public repo was refused by the safety classifier, so the seeds are agent-authored and labelled origin `agent-seed`, never `human`. EVAL-02 is amended to require at least 40 seed queries, where seed = `human` union `agent-seed`; the ratchet subset `dev/human` became `dev/seed` and `dev/all` is unchanged. Coordinator commits `e5f44982f` (`Origin::AgentSeed` and the `dev/seed` key in vox-search/vox-cli) and `bbefbe2b0` (README, plans 01-10 and 01-11) carry that change. Independence safeguard: the agent that authored the seed wording must not propose qrels in 01-08/01-09 and must not copy doc sentences. Real human-authored queries may be added later as `origin: human` through a reviewed re-epoch. These queries do NOT count as human-tier.

## Verification

- Canary line count in the README matching the lowercase 8-4-4-4-12 pattern: 1. README mentions `retrieval-baseline.v1.json`, `epoch_note`, `--split test` and U+203A.
- Draft: `grep -cE '^- \[(where|how-to|concept|policy|why|troubleshoot|multi-hop)\] .{10,}'` prints 48 (at least 40). `author:` line is non-empty. `## Queries` heading count is 1.
- GUID-absence check (GUID read at runtime from the README, never printed) exits 0 for both `queries.md` and `seed-sources.md`.
- Corpus-area doc counts sum to 670, equal to the tracked non-archive `.md` files under `docs/src`.

## Deviations from Plan

### 1. [Plan change, user-directed] Seed authorship delegated
- **Found during:** Task 3 checkpoint.
- **Issue:** the plan required user-written human-tier queries (AP-02, T-01-10).
- **Fix:** agent-authored seeds labelled `agent-seed` per D-02; requirement and downstream plans amended by the coordinator (`25e16ae56`, `e5f44982f`, `bbefbe2b0`). Task 3's acceptance counts are met, but the human-authored intent of EVAL-02 is not, so EVAL-02 is not marked complete.
- **Commits:** `fc02b1bb6`.

### 2. [Measurement note] commits count
`git rev-list --count plan_head_before..HEAD` is 34 because other sequential plans (01-01 to 01-03, 01-05 to 01-07, 01-12) committed on the same branch while this plan waited at its checkpoint. The 4 commits scoped `(01-04)` are the ones counted above: `6cd05af6c`, `e5f44982f`, `bbefbe2b0`, `fc02b1bb6`.

## Known Stubs

None. The draft files are intentionally transient; 01-10 materializes BEIR files from them and deletes them.

## Threat Flags

None. T-01-02 (canary) mitigated: single pipeline-written line, GUID absent from the draft, sidecar, SUMMARY and commit messages. T-01-10 is partially mitigated: provenance is recorded honestly as `agent-seed` rather than `human`. T-01-11 mitigated: the snapshot decision was a checkpoint before any blob.

## Self-Check: PASSED

- FOUND: contracts/kb/eval/README.md, golden-draft/queries.md, golden-draft/seed-sources.md
- FOUND commits: 6cd05af6c, e5f44982f, bbefbe2b0, fc02b1bb6
