---
phase: 03-provenance-trust-tiers-and-leak-closure
reviewed: 2026-10-09T00:00:00Z
depth: standard
scope: gap-closure diff 88fc5e996..HEAD (plans 03-14, 03-15, 03-16)
files_reviewed: 19
files_reviewed_list:
  - CHANGELOG.md
  - crates/vox-db/src/local_tests.rs
  - crates/vox-db/src/store/open.rs
  - crates/vox-db/src/store/ops_memory/library.rs
  - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
  - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
  - crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs
  - crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
  - crates/vox-orchestrator-mcp/tests/stdio_daemon_routing_tests.rs
  - crates/vox-orchestrator-types/src/trust.rs
  - crates/vox-orchestrator/src/memory/manager.rs
  - crates/vox-search/src/bundle.rs
  - crates/vox-search/src/kb_eval/library.rs
  - crates/vox-search/src/library/mod.rs
  - crates/vox-search/tests/execute_search_plan_smoke.rs
  - crates/vox-search/tests/kb_library_chunking.rs
  - crates/vox-search/tests/kb_library_index.rs
  - crates/vox-search/tests/web_capture_test.rs
  - docs/src/architecture/search-retrieval-ssot-2026.md
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 3: Code Review Report (gap-closure re-review)

**Reviewed:** 2026-10-09
**Depth:** standard (changed hunks read in full; callers, writers and the previous 03-REVIEW.md cross-checked)
**Files Reviewed:** 19
**Status:** issues_found

## Summary

The four gap closures hold on the paths they target. I found no remaining bypass of the headline invariant ("an unprompted preamble never carries an Untrusted hit") and no Critical issue.

- **CR-01 / WR-08 (Tavily, trigger-derived exclusion): closed.** `run_search_with_verification` now derives `exclude_untrusted_hits = true` whenever `trigger == AutoChatPreamble`, before `execute_search_plan`, the verification second pass and the Tavily leg all read `policy` (the shadowed `policy` binding covers all three). Under exclusion the Tavily block never builds a client, spends a credit or captures, so a preamble neither fetches nor leaks it. Outside the preamble the lines are labelled `[tier:untrusted]` and `min_trust_tier` is set to `Untrusted`. I checked every other `TavilyClient` user (`probe.rs`, `web_dispatcher.rs`, the web leg inside `execute_search_plan`) and every caller of `run_search_with_verification` (`message.rs`, `retrieval.rs`, `goal.rs`, `web_gather.rs`, GUI and CLI search): the unprompted callers all use `AutoChatPreamble`. The orchestrator `goal.rs` path, which passes a plain policy, is covered by `auto_preamble_trigger_drops_untrusted_chunks_without_the_flag`.
- **WR-05 (research-source sanitization): closed for the prompt path.** `sanitized_source` sanitizes `url`, `title`, `engine` and `snippet` (sanitize first, cap second) for `sources_from_hits` and `pipeline_sources` (deep and failed-run). The new red-team tests check url, engine and block.
- **WR-10 / D-07 / D-07a (agent-written graph rows): closed for graph rows.** `persist_fact` writes `agent_fact` / `agent_concept`. Both map to `Untrusted` through the default arm of `tier_for_knowledge_node_type`, and the tier is computed from `node_type` at read time, so no stored tier can go stale. I grepped every `knowledge_nodes` writer (`upsert_knowledge_node`, the two raw INSERTs in `research.rs` and `research_pipeline.rs`): none writes `fact` or `concept`, so no other model-reachable path lands Advisory. The memory leg (MEMORY.md and daily logs) is deliberately unchanged per D-07a. See IN-03.
- **WR-09 / D-08 (git-gated Library tier): closed on the reindex path.** The tier is decided for every walked file on every run (changed, unchanged and unreadable). `put_kb_index_state` no longer re-stamps rows. `IndexDeps` has exactly one non-test production constructor (`reindex_at`), and any git failure yields an empty set, so the whole corpus is Untrusted (fail-closed). `-z` output avoids path quoting, and `--no-renames` plus `ls-tree` minus `diff HEAD` handles staged, modified, untracked and untracked-after-`rm --cached` files. The remaining gaps are on the upgrade path (WR-01, WR-02) and are not reindex bugs.

New problems are limited to the upgrade path: both one-time re-tier steps run only when the database is below baseline 96, so a database already at 96 never gets them.

## Status of findings from the previous 03-REVIEW.md

| ID | Status | Note |
|---|---|---|
| CR-01 Tavily bypass | Resolved | See Summary. |
| WR-05 research-source sanitization | Resolved (prompt path) | Third constructor remains, see IN-02. |
| WR-08 exclusion only in MCP caller | Resolved | Derived in `bundle.rs`. |
| WR-09 Library Trusted from working tree | Resolved on reindex | Upgrade-path residual: WR-02 below. |
| WR-10 model memory store elevates to Advisory | Resolved for graph rows; memory leg accepted by D-07a | Residual: IN-03. |
| WR-01, WR-02, WR-03, WR-04, WR-06, WR-07 | Still open | None of their files is in this diff. Recorded as deferred in `deferred-items.md` (WR-01 and WR-07 owned by Phase 9). Not re-counted below. |
| IN-01 .. IN-05 | Still open | Deferred. IN-02 (`legacy_export_covers_all_baseline_tables`) is still red. |

The previous WR-02 (redact before sanitize) is the only open item I would not leave for later: it lets a secret split by a zero-width character reach the inbox in plaintext. It is not part of this diff, so it is not re-counted here.

## Warnings

### WR-01: Legacy `fact` / `concept` re-type runs only below baseline 96, so a database already at 96 keeps Advisory agent rows

**File:** `crates/vox-db/src/store/open.rs:210, 318-337`
**Issue:** `retier_agent_knowledge_nodes` is called inside `if current_version < BASELINE_VERSION`. Baseline 96 was introduced earlier in this phase (plan 03-03) and is unreleased, but any database that a Phase 3 binary has already opened is recorded at 96. For those databases the re-type never runs. Rows written by the old `persist_fact` stay `fact` / `concept`, which map to Advisory and pass `exclude_untrusted_hits`, so the leak that D-07 closes stays open for them. The test only covers a v95 to v96 upgrade. Two further gaps:
- The `id NOT LIKE 'graphify:%'` guard also skips a legacy agent fact whose model-chosen key starts with `graphify:`, and that row stays Advisory.
- Nothing needs the gate: the UPDATE is idempotent and cheap, and I confirmed no non-agent writer produces `fact` or `concept` (see Summary).

**Fix:** Run the re-type on every open, outside the version gate (the same shape as `ensure_latency_columns`), and drop the `graphify:` exclusion in favor of the id prefix plus the node-type check the graphify writer actually uses:
```rust
// open.rs, next to ensure_latency_columns(conn) after the gate
Self::retier_agent_knowledge_nodes(conn).await?;
```
Update `kb_agent_nodes_retyped_once_on_upgrade_never_deleted` to also open a DB recorded at 96 that holds a `fact` row, and fix the test's final assertion ("k-new keeps Advisory across reopen"), which would then need a distinct non-agent type or be deleted.

### WR-02: Baseline-96 backfill still stamps Library rows with the corpus ceiling, contradicting D-08

**File:** `crates/vox-db/src/store/open.rs:281-312` (comment at 278-281), `docs/src/architecture/search-retrieval-ssot-2026.md` ("Where the tier is stored")
**Issue:** 03-16 made `kb_index_state.trust_tier` a ceiling that never re-stamps rows, but `backfill_search_document_trust_tiers` still does `UPDATE search_documents SET trust_tier = <ceiling> WHERE corpus = ?` for every corpus. A database that has an indexed Library at v95, or one at v96 whose rows were stamped by the old `put_kb_index_state`, carries `Trusted` on every Library row, including untracked or modified files, until someone runs `vox_kb_reindex`. Nothing triggers a reindex on upgrade, and the preamble injects `Trusted` hits unlabelled. The function comment and the SSOT ("the single corpus-level source") are now stale.
**Fix:** In the backfill, set Library-corpus rows to `untrusted` (fail-closed) and let the next reindex promote git-clean documents. Also correct the doc comment. If the gate cannot be rerun for databases already at 96, run the same demotion once at MCP startup when `kb_index_state.indexed_commit` is older than the 03-16 change, or surface `docs_untrusted == 0` as a doctor warning.
```sql
UPDATE search_documents SET trust_tier = 'untrusted' WHERE corpus = 'library'
```
(use the `KB_LIBRARY_CORPUS` constant; keep the `vox://publication/%` advisory rule).

## Info

### IN-01: Library git gate fails closed silently and is blind to `assume-unchanged` / `skip-worktree`

**File:** `crates/vox-orchestrator-mcp/src/kb_tools/library.rs:213-266`
**Issue:** Any git failure (missing git, `safe.directory` "dubious ownership", no HEAD, repo root not the git top-level) logs at `debug` and returns an empty set, so every doc is Untrusted. That is the right fail-closed result, but the operator only sees `docs_untrusted == docs_seen` with no reason. Separately, `git diff HEAD --name-only` does not report paths whose index entry carries the `assume-unchanged` or `skip-worktree` flag, so a modified doc with that flag set still counts as clean. Reaching that state needs a deliberate `git update-index`.
**Fix:** Log the git failure at `warn` and add a `tier_gate: "git unavailable"` field to `report_json`. Optionally reject any path that `git ls-files -v -- docs/src` reports with a lowercase tag (`h`, `S`).

### IN-02: "The only way to build a numbered `Source`" is not enforced

**File:** `crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs:193-205, 214-238`; `crates/vox-gui/src/commands/knowledge.rs:265-277`
**Issue:** `Source` has public fields and `sources_context_block` sanitizes nothing itself; it relies on constructor discipline. `save_research_session_to_kb` builds `Source` literals directly from stored citations. That path only reaches `capture_research_sources`, which sanitizes through `clean()`, so there is no live prompt leak. A future caller that builds a `Source` and passes it to `sources_context_block` would not be sanitized.
**Fix:** Run `sanitized_source` in the GUI mapping, or make the fields private to the module and expose a constructor, or call `SanitizedText` inside `sources_context_block` (idempotent, cheap). Correct the doc comment either way.

### IN-03: Model-originated text still reaches the preamble through the memory leg (accepted by D-07a)

**File:** `crates/vox-search/src/execution.rs:384-386` (unchanged), `crates/vox-orchestrator-mcp/src/memory_tools/handlers_memory.rs`
**Issue:** WR-10 asked that model output stored through the memory tool never be Advisory. D-07a narrows that to graph rows. The same `memory_store` call still writes the value to MEMORY.md, and that line reaches the automatic preamble as Advisory, including text the model copied from the web in the same turn. The e2e test pins this on purpose. This is an explicit user decision and not a defect in the plans, but the invariant "nothing elevates model or web output" is therefore not yet true end to end. It should stay on the Phase 7 list with an owner.
**Fix:** Keep `deferred-items.md` as the tracker. When Phase 7 lands, tier memory-file lines by origin and drop the pinned assertion.

### IN-04: Tier invariants for `knowledge_nodes` rest on convention

**File:** `crates/vox-orchestrator-types/src/trust.rs:94-111`, `crates/vox-db/src/schema/domains/knowledge.rs:7`
**Issue:** `fact` and `concept` stay Advisory for "non-agent writers", but no writer is non-agent today, and the DDL default `node_type DEFAULT 'concept'` is Advisory. `tiered_writer_invariant.rs` pins `search_documents` and `kb_findings` writers, not `knowledge_nodes` writers, so a future model-reachable call to `upsert_knowledge_node(.., Some("fact"), ..)` would silently re-open D-07. The DDL default is already in `deferred-items.md`.
**Fix:** Add a static test that scans non-test `.rs` for `upsert_knowledge_node` and `INTO knowledge_nodes` callers with an allow-list, mirroring the existing writer invariant. Longer term, drop the `fact` / `concept` Advisory arm until a real curated writer exists.

### IN-05: Tavily tests mutate process env alongside parallel tests

**File:** `crates/vox-search/tests/web_capture_test.rs:137, 224`, `crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs:44-49`
**Issue:** `set_var` runs inside `multi_thread` tests after the mock server's worker threads exist, and `web_capture_test.rs` holds several tests in one binary that run on parallel threads and read env through `SearchPolicy`. Setting env while other threads read it is the unsound pattern that the `unsafe` block exists to flag. It is the same pattern the existing `capture_crag_tavily_reaches_sink_not_corpus` already uses, so flakiness risk is not new, but the second copy doubles it. The new preamble test also depends on the key being present to be meaningful: if `vox_config::snapshot::bump` ever stops reflecting the key, the "no request" assertion passes vacuously.
**Fix:** Mark both Tavily tests `#[serial]` (the crate already uses `serial_test` elsewhere), and add a positive control in the same test (the explicit-trigger assertion in `kb_retrieval_tavily.rs` already does this; add the equivalent to the `web_capture_test.rs` case, for example a second `run_search_with_verification` with `ExplicitToolQuery` that must reach the mock).

---

_Reviewed: 2026-10-09_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
