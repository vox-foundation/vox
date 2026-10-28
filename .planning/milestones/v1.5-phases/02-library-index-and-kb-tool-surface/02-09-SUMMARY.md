---
phase: 02-library-index-and-kb-tool-surface
plan: 09
subsystem: search
tags: [kb, eval, library-index, tantivy, qrels, baseline, mcp, uat]
requires:
  - phase: 01-retrieval-eval-baseline
    provides: "run_eval engine, golden set, ratchet_split, assert_corpus_clean, baseline re-epoch procedure"
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-03 library::search, 02-04 probe-chosen Tantivy lexical leg, 02-06/02-07 vox_kb_* tools and kb:// resources, 02-08 vox kb status/reindex"
provides:
  - "`vox kb eval` (report, gate, tighten, write-baseline) measures retrieval through vox_search::library::search over a fresh Library index of the frozen corpus.jsonl snapshot"
  - "kb_eval::library retriever keyed by kb_ids::chunk_id, with a doc to chunk-id map"
  - "Epoch-2 retrieval baseline naming the Library index, with all dev/all doc-level metrics above epoch 1"
  - "Library lexical leg title/heading boosts retuned 2.0/1.5 to 0.25/0.25 (dev split only)"
  - "External-host UAT evidence: vox_kb_* tools and kb:// resources work from Claude Code on the real corpus"
affects: [phase-03, phase-11]
actuals:
  tokens: 6560
  tasks: 3
  commits: 3
plan_head_before: 445fb033a967c49f8ac1ee99a50057eac96652c7
tech-stack:
  added: []
  patterns:
    - "Eval builds a throwaway Library index from the frozen corpus snapshot; no dependency on the live repo tree or on a running daemon"
    - "Qrel keys and Library chunk ids come from the one kb_ids walk, so resolution is exact by construction and mutation-proven"
    - "Retrieval-quality tuning is swept on the dev split only; the held-out test split is never scored during tuning"
key-files:
  created:
    - crates/vox-search/src/kb_eval/library.rs
    - crates/vox-search/tests/kb_eval_library.rs
  modified:
    - crates/vox-search/src/kb_eval/mod.rs
    - crates/vox-cli/src/commands/kb/eval.rs
    - crates/vox-cli/src/commands/kb/gate.rs
    - crates/vox-search/src/library/lexical.rs
    - contracts/kb/eval/qrels/dev.chunks.tsv
    - contracts/kb/retrieval-baseline.v1.json
    - docs/src/reference/cli.md
key-decisions:
  - "Retuned the Library lexical title/heading boosts from 2.0/1.5 to 0.25/0.25 (Rule 1) instead of stopping on the first-run regression; swept on the dev split only, held-out test split never scored"
  - "Deepened dev chunk qrels q046 and x041 to `› 2.1 Full Stack Layers` once, under the Phase 1 re-epoch (epoch 2); 19 candidates reviewed, q001/x001 flagged and deliberately left as-is"
  - "Re-epoched rather than tightened: the retriever and chunker fields now name the Library index and a qrel diff requires an epoch bump"
patterns-established:
  - "A first-run quality regression on a new retriever is diagnosed and fixed on the dev split, then re-measured; never loosened by editing the baseline"
requirements-completed: [RET-01, IDX-06, IDX-07, IDX-05]
status: complete
---

# Phase 2 Plan 09: Library-index eval, epoch-2 baseline and external-host UAT Summary

`vox kb eval` now measures retrieval through the Library index over the frozen snapshot, and after a dev-split-only retune of the lexical title/heading boosts every dev/all metric beats the Phase 1 BM25 baseline; the baseline was re-epoched to epoch 2 and the KB tools were confirmed working from Claude Code.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (tracer) | `vox kb eval` through the Library index over the frozen snapshot | 8a5e32b24 |
| 1 (deviation) | Down-weight title/heading boosts in the Library lexical leg (Rule 1) | 6b5cdb5e3 |
| 2 | Re-epoch the retrieval baseline onto the Library index (epoch 2) | 6a9a6b4be |
| 3 | External-host UAT and reindex probe (automated part: no repo changes) | n/a (outputs in scratchpad) |

## Metrics (dev/all)

Epoch 1 is the Phase 1 in-memory BM25 baseline. "First run" is the Library index with the original boosts 2.0/1.5. "Final" is the Library index with boosts 0.25/0.25.

| Metric | Epoch 1 BM25 | Library, first run (2.0/1.5) | Library, final (0.25/0.25) |
|--------|-------------:|-----------------------------:|---------------------------:|
| doc recall@5 | 0.4697 | 0.3421 | 0.5520 |
| doc recall@10 | 0.5853 | 0.4715 | 0.6879 |
| doc mrr@10 | 0.5568 | 0.4049 | 0.6077 |
| doc ndcg@10 | 0.4599 | 0.3579 | 0.5489 |
| chunk ndcg@10 | 0.2739 | 0.2013 | 0.3326 |
| answer_in_budget | 0.3553 | 0.3026 | 0.5395 |

dev/seed also rose on every metric. `vox kb eval --gate` exits 0 against the epoch-2 baseline, and two eval runs print byte-identical reports.

## Chunk qrels

- All Phase 1 chunk qrel keys resolve exactly to Library chunk ids (`kb_ids::chunk_id`); proven by `chunk_qrels_resolve_exactly_to_library_chunks` and by mutation (breaking the shared id walk makes the test fail).
- 19 candidate rows were reviewed for parent-heading qrels whose answer now lives in a child section. Two were deepened exactly once: q046 and x041, both to `› 2.1 Full Stack Layers`. q001 and x001 were flagged and left as-is (human approved leaving them).
- The change went through the Phase 1 re-epoch (epoch 2 with an epoch_note recording the delta), not a silent loosening. Only dev chunk qrels changed; the held-out test split was never scored.

## Task 3: external-host UAT

Automated part, run against the real corpus with a temporary probe page:

- `vox kb reindex`: docs_seen 671, 13440 chunks, 671 cards, errors `[]`.
- `vox kb status`: consistent true, lexical_backend `tantivy-0.22`, db_chunk_count == lexical_docs == 13440, 0 paths containing `/archive/`.
- Probe page `uat-probe.md` under docs/src/library/ was found by `vox_kb_search` after reindex (docs_changed 1) and pruned after it was deleted and reindexed (docs_pruned 1). The final reindex reported docs_unchanged 671 == docs_seen. The probe page was never committed.
- Stdio script check of the `vox mcp` server: `vox_kb_search` returns `kb://` URIs; `vox_kb_cite` returns `[Where Things Live](docs/src/architecture/where-things-live.md#where-things-live)`.

## Human check

The human-verify checkpoint is resolved. brbrainerd replied "approved" in chat on 2026-10-05, in response to a request covering: the Claude Code external-host UAT (a `vox-kb-uat` MCP server registered by the user, then removed), approval of the lexical boost retune (6b5cdb5e3), and approval of the epoch-2 baseline and qrel changes (6a9a6b4be), including leaving q001/x001 as-is.

## Deviations from Plan

**1. [Rule 1 - Bug] Lexical title/heading boosts retuned after a first-run regression**
- **Found during:** Task 1 (first `vox kb eval` through the Library index)
- **Issue:** with boosts 2.0/1.5 every doc-level metric fell below the epoch-1 baseline (doc recall@5 0.3421 vs 0.4697), which would fail the plan's non-regression truth.
- **Fix:** swept the title/heading boosts on the dev split only and settled on 0.25/0.25. The held-out test split was never scored.
- **Files modified:** crates/vox-search/src/library/lexical.rs
- **Commit:** 6b5cdb5e3

**2. [Rule 3 - Blocking] The plan's build lacks the MCP server**
- **Found during:** Task 3
- **Issue:** the default `vox` build does not include `vox mcp`, so the stdio and Claude Code checks could not run.
- **Fix:** rebuilt with `cargo build -p vox-cli --features mcp-server`.
- **Files modified:** none

**3. [Environment side effect] daemon-ensure replaced the user's installed daemon**
- **Found during:** Task 3
- **Issue:** daemon-ensure staged the `target/debug` daemon into `~/.vox/bin/vox-orchestrator-d`, overwriting the user's Sep-10 install.
- **Fix:** none needed to proceed; backups of the original are at `~/.vox/bin/vox-orchestrator-d.bak-2026-09-10` (and a copy in the scratchpad). The user may want to reinstall.
- **Files modified:** none in the repo

**4. [Plan text discrepancy] `vox_kb_get` takes `path`, not `doc_path`**
- **Found during:** Task 3
- **Issue:** plan item 3 names the argument `doc_path`; the implemented tool schema uses `path`.
- **Fix:** used `path`; no code change.

**5. [Environment] Installed `vox` is stale**
- **Found during:** verification
- **Issue:** the installed `vox` fails the freshness check for `vox ci` gates.
- **Fix:** run gates with `VOX_SKIP_FRESHNESS_CHECK=1` or reinstall `vox`.

## Known Stubs

None.

## Threat Flags

None. The only new surface is a retriever over a local temp index built from the frozen snapshot.

## Self-Check: PASSED

- Commits 8a5e32b24, 6b5cdb5e3, 6a9a6b4be exist in the branch history; `git rev-list --count 445fb033a..HEAD` = 3.
- Created files exist: crates/vox-search/src/kb_eval/library.rs, crates/vox-search/tests/kb_eval_library.rs.
