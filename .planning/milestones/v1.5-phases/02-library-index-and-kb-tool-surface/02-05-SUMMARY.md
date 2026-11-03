---
phase: 02-library-index-and-kb-tool-surface
plan: 05
subsystem: search
tags: [vox-search, tantivy, dead-code-removal, retrieval, guard-test]
requires:
  - phase: 02-library-index-and-kb-tool-surface
    provides: "02-01 Library index (vox_search::library) as the single lexical leg"
provides:
  - "Writer-less Tantivy docs mirror removed across five crates"
  - "tantivy_mirror_removed static guard test"
affects: [02-08, phase-03]
actuals:
  tokens: 6000
  tasks: 2
  commits: 3
plan_head_before: 195e79000dc50518ce66328d76ce3bd1b190d62f
tech-stack:
  added: []
  patterns:
    - "Runtime source-scan guard test (CARGO_MANIFEST_DIR + read_to_string) listing banned symbols per file"
key-files:
  created:
    - crates/vox-search/tests/tantivy_mirror_removed.rs
  modified:
    - crates/vox-search/src/{lib,execution,bundle,a2a_contract,rrf,policy,semcov_wave21_tests}.rs
    - crates/vox-research-shim/src/research/orchestrator/web_gather.rs
    - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs
    - crates/vox-orchestrator/Cargo.toml
    - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
    - crates/vox-cli/Cargo.toml
  deleted:
    - crates/vox-search/src/lexical_tantivy.rs
key-decisions:
  - "Kept SecretId::VoxSearchTantivyRoot (AP-14): library/lexical.rs still resolves it for the Library index root"
  - "Left SEARCH_CORPUS_BUCKET_COUNT untouched: changing it would shift citation_coverage scores, out of scope for a pure dead-path removal"
requirements-completed: [IDX-07]
status: complete
---

# Phase 2 Plan 05: Remove the Tantivy docs mirror Summary

The writer-less Tantivy docs mirror (module, `TantivyDocsIndex`, `SearchPolicy::tantivy_index_root`, `SearchExecution::tantivy_doc_lines`, the `[tantivy:` RRF dedup branch and every consumer) is deleted, and a static guard test keeps it out.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 (RED) | `tantivy_mirror_removed` guard test, failing on the still-present module | 66cd5b7ba |
| 1 | Remove mirror from vox-search (tracer) | ef6af5915 |
| 2 | Remove from research-shim, orchestrator, MCP; refresh feature comments | eff0438af |

## Verification

- `cargo nextest run -p vox-search --features tantivy-lexical -E 'test(/tantivy_mirror_removed|execute_search_plan|a2a|rrf/)'`: 18 passed. Tracer verify (`execute_search_plan|a2a|rrf|semcov`): 34 passed.
- `cargo check -p vox-research-shim -p vox-orchestrator -p vox-orchestrator-mcp -p vox-cli`: clean.
- `cargo clippy --all-targets -D warnings` clean for vox-search, vox-research-shim, vox-orchestrator, vox-orchestrator-mcp.
- `grep TantivyDocsIndex|tantivy_doc_lines|tantivy_index_root` over `crates/**/*.rs` lists only the guard test.
- Guard went RED first (module file still existed), GREEN after Task 2.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] a2a_contract test expected the removed field's line**
- **Found during:** Task 1
- **Issue:** `from_search_pass_mirrors_mcp_chunk_merging` asserted 3 chunk excerpts (chunk + tantivy + qdrant); with the mirror gone the count is 2.
- **Fix:** Updated the assertion to 2.
- **Files modified:** crates/vox-search/src/a2a_contract.rs
- **Commit:** ef6af5915

**2. [Rule 3 - Blocking] tdd-guard blocked the Task 2 commit on a pre-existing untested `pub fn`**
- **Found during:** Task 2 commit
- **Issue:** goal.rs (touched for the hit-count term) has an untested `pub async fn submit_goal`; the `skeleton/untested-pub-api` pre-commit hook flags the whole staged file.
- **Fix:** Added `chunk_hits_count_chunk_and_qdrant_lines_only` in goal.rs, which exercises the edited `socrates_task_from_search_pass` hit count. No suppression entry written.
- **Files modified:** crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs
- **Commit:** eff0438af

**3. [Plan verify variance] `cargo check -p vox-search --no-default-features` fails, pre-existing**
- `policy.rs` references `crate::tavily_research` without a `tavily` cfg guard, so the no-default-features build already fails at HEAD, independent of this plan. To still prove no leftover tantivy cfg branch, ran `cargo check -p vox-search --no-default-features --features tavily` (clean). Logged to deferred-items.md; not fixed (out of scope).

## Deferred Issues

- `vox-search` does not compile with `--no-default-features` (unguarded `tavily_research` use in `policy.rs`, ~line 493). Pre-existing.
- `SEARCH_CORPUS_BUCKET_COUNT` in execution.rs may still count the removed mirror as a bucket; deliberately left (scoring change). Worth a look when 02-0x retunes `citation_coverage`.

## Known Stubs

None.

## Threat Flags

None. T-02-13 mitigated by the guard test; T-02-14 mitigated by the four-crate `cargo check`.

## Notes

- Public API change: `vox_search::SearchExecution` loses `tantivy_doc_lines`, and `SearchPolicy` loses `tantivy_index_root`. CHANGELOG entry is owned by 02-08.
- `vox run scripts/fmt.vox -- <crate>` reports "formatted N target roots" but touched only the files edited here.

## Self-Check: PASSED

Guard test file exists; `lexical_tantivy.rs` absent; commits 66cd5b7ba, ef6af5915, eff0438af present.
