---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 06
subsystem: research
tags: [kb_findings, trust-tier, research-turn, vox-research-shim, vox-cli-research, docs-src-leak, finding_capture]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "WebFindingCapture / capture_finding / web_capture_for_state (03-04), tiered corpus writers (03-05)"
provides:
  - "capture_research_sources / capture_research_synthesis (Untrusted pending findings, no confidence)"
  - "run_quick_with_policy seam; trace stage inbox replaces knowledge_base"
  - "ResearchConfig.finding_capture sink; web_gather hands hits (full text) to it instead of ingesting research documents"
  - "pipeline branch (j) and write_research_doc deleted; persist_to_docs deprecation-marked"
  - "vox research publish writes .vox/cache/research-drafts/<slug>-2026.md, never docs/src"
  - "VoxDb::search_corpus_row_counts() typed count accessor"
affects: [03-07, 03-08, 03-09, 03-11, phase-05]
tech-stack:
  added: []
  patterns: ["research output reaches the KB only as pending Untrusted kb_findings", "sink-injection (no sink => no write, fail-closed)"]
key-files:
  created: []
  modified:
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
    - crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs
    - crates/vox-gui/src/commands/knowledge.rs
    - crates/vox-gui/src/commands/research.rs
    - crates/vox-research-shim/src/research/orchestrator/config.rs
    - crates/vox-research-shim/src/research/orchestrator/web_gather.rs
    - crates/vox-research-shim/src/research/orchestrator/pipeline.rs
    - crates/vox-research-shim/src/research/persistence.rs
    - crates/vox-research-shim/src/research/types.rs
    - crates/vox-research-shim/tests/scientia_phase_0a_persistence.rs
    - crates/vox-cli-research/src/lib.rs
    - docs/src/reference/cli.md
    - crates/vox-db/src/store/ops_memory/search.rs
key-decisions:
  - "D-06 taint rule applied: deep synthesis is one Untrusted pending finding (dedupe scope = query, source URLs in provenance, no confidence key)"
  - "create_research_source session bookkeeping kept as the plan says; it writes an empty-content research_source knowledge node (label+URL only), which tier_for_knowledge_node_type already maps to Untrusted. Tests assert 'only bookkeeping nodes, never page text' rather than 'zero nodes' (the plan's two statements conflict)"
  - "vox research publish writes a local draft (no new crate edge); GUI paths file findings (D-05 interim asymmetry)"
patterns-established:
  - "extract the capture/bookkeeping step into a small fn (file_gathered_hits) so the gather leak is unit-testable without a provider mock"
requirements-completed: [TRUST-02, TRUST-03, TRUST-04, GROW-06]
duration: long (build host intermittently hung; see Issues)
completed: 2026-10-06
status: complete
commits: 4
plan_head_before: 7f5c41a7c89c5b129da62beaec2d43e3ba0a6b54
actuals:
  tokens: 12000
  tasks: 3
  commits: 4
---

# Phase 3 Plan 06: Research paths into the findings inbox Summary

Every research path (quick turn, deep turn, deep-research gather, GUI "Save to KB") now files pending `Untrusted` `kb_findings` and nothing else, and the two latent LLM-to-`docs/src` writers (shim `persist_to_docs` branch, `vox research publish`) are closed.

## Commits

- `604a35abd` fix(03-04): typed count accessors for capture tests (sql-surface-guard) (pre-task fix)
- `270f86234` feat(03-06): quick research turn and GUI save-session file pending findings (Task 1, tracer)
- `c019eb633` feat(03-06): deep research gather hits and synthesis go to the findings inbox (Task 2)
- `d5722e350` fix(03-06): close research writers into docs/src (Task 3)

## What was built

- **Task 1 (tracer).** `capture_research_sources` / `capture_research_synthesis` replace the knowledge-graph persist fn (deleted). `run_quick` split into `run_quick` + `run_quick_with_policy`; trace stage `knowledge_base` renamed `inbox` ("filed N pending findings for review", detail `{pending_findings, query}`). GUI `save_research_session_to_kb` files sources + synthesis (origin `GuiSaveResearchSession`); `research_repo_root` is `pub(crate)`. Test `inbox_quick_turn_writes_only_findings` runs against a wiremock SearXNG: corpus rows (0,0), no knowledge nodes, 2 pending Untrusted `research_quick` findings with url/fetched_at/hash, 2 snapshot files, trace stage `inbox` = 2.
- **Task 2.** `ResearchConfig.finding_capture` (None default, bool in Debug). `web_gather` keeps `create_research_source` but replaces the per-hit research-document ingest with one sink call (origin `ResearchGather`, `full_text` = raw body else snippet); no sink means no write. `run_deep` builds its config with `web_capture_for_state`, attaches the same sink to its `SearchRuntimeContext`, and files the synthesis (origin `ResearchDeep`). Both `handlers_memory.rs` research configs carry the sink.
- **Task 3.** Pipeline branch (j) and `write_research_doc` deleted (+ its test); `slug_from_query` kept (in-file test added for tdd-guard). `persist_to_docs` carries the `vox-deprecated-since` marker. `publish_draft_path` -> `<root>/.vox/cache/research-drafts/<filename>` (gitignored by `**/.vox/cache/`); `Publish` doc comment and `docs/src/reference/cli.md` updated.

## Mutation proofs (grep -c presence counts)

| Sink | Mutation | Before / mutated / restored | Result |
|------|----------|-----------------------------|--------|
| S-quick (T-03-11/TRUST-02) | restore `upsert_knowledge_node` for quick sources in `run_quick_with_policy` (research_turn.rs) | 0 / 1 / 0 | `inbox_quick_turn_writes_only_findings` FAILED on `list_knowledge_nodes(..).is_empty()` |
| S3 gather (T-03-07) | restore `ingest_research_document_async` in `file_gathered_hits` (web_gather.rs) | 0 / 1 / 0 | both `finding_capture_*` tests FAILED (`assertion left == right`, corpus counts) |
| docs writer (T-03-12) | point `publish_draft_path` at `docs/src/architecture` (cli-research lib.rs) | 0 / 1 / 0 (`docs/src/architecture` occurrences) | `publish_draft_path_never_in_docs_src` FAILED |

All mutations were reverted; the restored files were byte-identical to the tested copies (counts re-checked at 0).

## Verification

- `cargo nextest run -p vox-orchestrator-mcp inbox_`: 8 passed (3 new inbox tests + 5 pre-existing name matches).
- `cargo nextest run -p vox-research-shim finding_capture`: 2 passed; `-E 'test(/slug_from_query|helpers|finding_capture/)'`: 7 passed + the 2 finding_capture tests passed after the fix below.
- `cargo nextest run -p vox-cli-research publish_draft_`: 2 passed.
- `cargo check -p vox-gui`: green (sidecar autobuilt by build.rs).
- `vox ci crate-edges`: OK (one pre-existing stale-baseline warning for `vox-gui -> vox-db-types`, not touched). `vox ci sql-surface-guard --all`: OK. `vox-doc-pipeline --lint-only --paths reference/cli.md`: no hard errors.
- Acceptance greps: no `persist_research_findings_to_knowledgebase`, `upsert_knowledge_node` (research_turn.rs), `ingest_research_document_async` (shim src), `write_research_doc` (crates) left.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] sql-surface-guard failure left by 03-04 (orchestrator-directed pre-task fix)**
- **Found during:** pre-task
- **Issue:** `kb_findings_capture.rs` and `web_capture_test.rs` counted rows with raw `.connection().query("SELECT COUNT(*) ...")`, so `ci sql-surface-guard --all` exited 1.
- **Fix:** added `VoxDb::search_corpus_row_counts() -> (documents, chunks)` (in-file test `search_corpus_row_counts_track_documents_and_chunks`) and used it in both tests; deleted the raw helpers. Guard now exits 0; both test files pass (`web_capture_test` needed retries on this loaded host: its pre-existing "web leg must have run" assertion flaked on a 10 s deadline, unrelated to the count change).
- **Files modified:** crates/vox-db/src/store/ops_memory/search.rs, crates/vox-orchestrator-mcp/tests/kb_findings_capture.rs, crates/vox-search/tests/web_capture_test.rs
- **Commit:** 604a35abd

**2. [Rule 1 - Bug in plan] "knowledge_nodes stays at 0 rows" conflicts with "keep create_research_source"**
- **Found during:** Task 2 test run
- **Issue:** `create_research_source` inserts an empty-content `research_source` knowledge node, so a literal zero-row assertion fails.
- **Fix:** kept the bookkeeping call (plan: keep) and asserted every node is `research_source` with empty content (no page text), plus corpus counts (0,0). Such nodes are Untrusted by the allowlist in `tier_for_knowledge_node_type`.
- **Commit:** c019eb633

**3. [Rule 3 - blocking] tdd-guard on persistence.rs**
- `slug_from_query` lost its host file's size exemption after the writer was deleted; added an in-file unit test. (First Task 3 commit attempt was rejected by the hook; the retry is `d5722e350`.)

**4. [Scope note] run_deep synthesis capture landed in Task 1's commit**
- Deleting the KG persist fn forced `run_deep` to call `capture_research_synthesis` in Task 1; Task 2 added only the config/sink wiring.

## Known gaps / limits

- No mcp test drives `run_deep` end to end (needs a live LLM); its wiring is covered by `inbox_synthesis_*` (capture fn) plus the acceptance greps. Snippets and embeddings table counts are not asserted (no typed accessor; no code path in these functions writes them).
- `research_cache` knowledge node (sink S14) still written as a cache; 03-09 excludes it from search (per plan).
- Per D-05, CLI research paths have no capture sink; `vox research publish` writes a local draft only.

## Issues Encountered

- Build host: rustc for `vox-corpus`/`vox-publisher` hung in uninterruptible I/O on stale `target/debug/incremental/<crate>-*` dirs; killed and removed those incremental dirs (build artefacts only, gitignored). Fresh test binaries then took ~18 s each to list (Gatekeeper scan), so nextest runs were long.

## Threat Flags

None new. T-03-07, T-03-11, T-03-12 mitigated (mutation-proven); T-03-25 provenance carries query, engine/rank or the source URL list plus content hash and snapshot.

## Self-Check: PASSED

- Commits 604a35abd, 270f86234, c019eb633, d5722e350 present in `git log`.
- `commits: 4` measured from `plan_head_before` ledger (includes the pre-task fix commit).
