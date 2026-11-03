---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 09
subsystem: retrieval
tags: [trust-tier, retrieval, preamble, sanitizer, red-team, vox-search, vox-db, mcp]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "search_documents.trust_tier + backfill (03-03), tiered writers (03-05), kb_text sanitizer (03-02), chat-turn leak closure (03-07)"
provides:
  - "VoxDb::trust_tiers_for_chunk_ids / trust_tiers_for_knowledge_nodes (fail closed to Untrusted)"
  - "knowledge-node search never returns research_cache nodes (FTS and LIKE)"
  - "SearchPolicy::exclude_untrusted_hits; [tier:<db_str>] prefix on chunk and KG lines; SearchExecution::min_trust_tier"
  - "RetrievalEvidenceEnvelope::min_trust_tier and a derived, fail-closed envelope trust tier (blanket Trusted stamp removed)"
  - "AutoChatPreamble drops Untrusted hits; ExplicitToolQuery keeps them labelled"
  - "SanitizedText at run_retrieval_bundle (all legs), sources_from_hits, and every vox_kb_* tool result"
  - "crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs (kb_tiers_*, kb_injection_*)"
affects: [03-10, 03-13, phase-05, phase-07]
tech-stack:
  added: []
  patterns: ["tier label as a line prefix, rrf dedup key strips it", "sanitize at ingest and again at injection", "pre-filter rank kept so dropping a hit never reorders survivors"]
key-files:
  created:
    - crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
  modified:
    - crates/vox-db/src/store/ops_memory/search.rs
    - crates/vox-db/src/store/ops_memory/knowledge.rs
    - crates/vox-search/src/policy.rs
    - crates/vox-search/src/execution.rs
    - crates/vox-search/src/rrf.rs
    - crates/vox-search/src/a2a_contract.rs
    - crates/vox-search/tests/execute_search_plan_smoke.rs
    - crates/vox-orchestrator/src/orchestrator/task_dispatch/submit/goal.rs
    - crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/mod.rs
    - crates/vox-orchestrator-mcp/src/chat_tools/chat/research_turn.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - crates/vox-orchestrator-mcp/src/task_tools/tests.rs
key-decisions:
  - "Tier label is a prefix, so crates/vox-search/src/rrf.rs rrf_dedup_key strips a leading [tier:..] label (otherwise fused lines would stop deduping)"
  - "Qdrant sidecar lines carry no stored tier: counted Untrusted and dropped under exclude_untrusted_hits"
  - "KB tool output is sanitized by one recursive string-leaf pass (ok_clean) rather than per-field calls, so a new field cannot be forgotten"
  - "Sources are sanitized before the 600-char cap so the cap cannot cut a construct in half"
patterns-established:
  - "Seed a row the filter must drop AND a row it must keep, so a tier test cannot pass vacuously"
requirements-completed: [TRUST-01, TRUST-05, RET-02]
status: complete
commits: 2
plan_head_before: 741663db9e48973d6a36f4f94038730f45f28a2c
actuals:
  tokens: 9000
  tasks: 2
  commits: 2
duration: long (shared build host; one rate-limit interruption, one nextest stall)
completed: 2026-10-08
---

# Phase 3 Plan 09: Tier-aware retrieval and injection-boundary sanitization Summary

Stored trust tiers now decide what the chat preamble may inject: legacy web rows and external_research nodes are dropped from the automatic preamble but stay queryable (labelled `[tier:untrusted]`) by explicit tools, the envelope tier is the minimum of the kept hits (Untrusted when none), and every line a model sees through retrieval, numbered research sources or the vox_kb_* tools passes the sanitizer.

## Commits

- `eeded4f4a` feat(03-09): tier-aware retrieval, preamble drops Untrusted hits, derived envelope tier (Task 1, tracer)
- `219003a18` feat(03-09): sanitize retrieval bundle, research sources and KB tool output at the injection boundary (Task 2)

## What was built

- **vox-db.** `trust_tiers_for_chunk_ids` joins `search_document_chunks` to `search_documents.trust_tier` by chunk rowid; a missing or non-numeric id reads Untrusted. `trust_tiers_for_knowledge_nodes` maps `node_type` through `tier_for_knowledge_node_type` (missing id Untrusted). Both knowledge-search queries exclude `node_type = 'research_cache'`.
- **vox-search.** `SearchPolicy::exclude_untrusted_hits` (serde default false). `execute_search_plan` looks up tiers for chunk and KG hits, prefixes lines `[tier:<db_str>]`, and under the flag drops Untrusted chunk, KG, web and qdrant lines together with their parallel `unified_hits` (KG score keeps its pre-filter rank, so survivor order is unchanged). `SearchExecution::min_trust_tier` folds memory (Advisory), repo/symbol/graphify (Trusted), web/qdrant (Untrusted) and looked-up chunk/KG tiers; `None` when nothing is kept. Web hits are still filed in the findings inbox before the flag drops them.
- **mcp.** `run_retrieval_bundle` sets the flag for `AutoChatPreamble` only; `RetrievalEvidenceEnvelope.min_trust_tier` carries the execution minimum and `to_context_envelope` uses `unwrap_or(Untrusted)`. All five bundle legs, `sources_from_hits` title and snippet, and all `vox_kb_*` JSON string leaves go through `SanitizedText`.

## Evidence

**Red before implementation.** `cargo nextest run -p vox-orchestrator-mcp --test kb_retrieval_tiers` on the unmodified retrieval path: all behavioural tests failed (web-ingest chunk reached the preamble; research_cache node leaked: `kgcacheword leaked: [node:n-cache] ...`; empty bundle `left: Trusted, right: Untrusted`; red-team tag-block code point survived in both the bundle line and the Source). `kb_tiers_findings_never_retrieved` passed already (no retrieval path reads `kb_findings`); it is a regression guard, not a red test.

**Green.** `kb_retrieval_tiers`: 9 passed (`kb_tiers_*` x6, `kb_injection_*` x2, plus the hermetic-home check). `-p vox-orchestrator-mcp --lib -E 'test(/kb_injection_|sources|context_block|kb_tool|sanitized_neutralizes|kb_status|kb_get|kb_cite|kb_search/)'`: 36 passed including `kb_injection_kb_tools_neutralize_exfil_and_tag_block`. `--lib retrieval_bundle|context_envelope|socrates_*`: 7 passed; `retrieval_evidence_context_envelope_contract` passed (JSON-schema validation still green). vox-search `cargo test`: `execute_search_plan_smoke` 2 passed (new `execute_search_plan_labels_tiers_and_excludes_untrusted_on_request`), lib `dedup_key|exclude_untrusted` 6 passed; vox-db lib `trust_tiers_for|knowledge_search_skips` 2 passed. `cargo check -p vox-db -p vox-search -p vox-orchestrator -p vox-orchestrator-mcp --tests`: clean, 0 warnings.

**Red-team coverage (13 cases).** Every case is asserted at the bundle boundary (13 chunks seeded Trusted, each line checked against `must_not_contain` and `must_contain`), at `sources_from_hits` (title and snippet), and the exfil + tag-block cases at the KB-tool boundary (`vox_kb_search`, `vox_kb_get` x2, `vox_kb_cite`; the stored chunk is asserted to still hold the payload, so the check is non-vacuous).

**Mutation proofs (`grep -c` presence counts: before / mutated / restored).**

| Guard | Mutation | Counts | Result |
|-------|----------|--------|--------|
| T-03-15 preamble filter | `policy.exclude_untrusted_hits = false;` in `run_retrieval_bundle` | `exclude_untrusted_hits = trigger == RetrievalTriggerMode::AutoChatPreamble` in retrieval.rs: 1 / 0 / 1 | `kb_tiers_preamble_drops_untrusted_and_labels_the_rest` and `kb_tiers_preamble_preserves_rank_order_of_survivors` FAILED; other 4 passed |
| T-03-15 envelope tier | blanket `vox_orchestrator::ContextTrustTier::Trusted` restored in `to_context_envelope` | `ContextTrustTier::Trusted` in retrieval.rs: 0 / 1 / 0 | `kb_tiers_empty_bundle_is_untrusted` (`left: Trusted, right: Untrusted`), `kb_tiers_explicit_query_keeps_untrusted_labelled`, `kb_tiers_preamble_drops_untrusted_and_labels_the_rest` FAILED |
| T-03-16 bundle mapping | `sanitized()` body replaced with a pass-through | `SanitizedText::new(&l)` in retrieval.rs: 1 / 0 / 1 | `kb_injection_bundle_lines_are_sanitized_for_every_redteam_case` FAILED |
| T-03-16 sources | title and snippet sanitization replaced with raw values | `SanitizedText::new(&h` in research_turn.rs: 2 / 0 / 2 | `kb_injection_research_sources_are_sanitized_for_every_redteam_case` FAILED (`"\u{e0069}" survived`) |

The two sanitization mutations were applied in the same build (each test depends on only one of the two guards, and each failed). All four were restored with the Edit tool; restored counts re-checked, `rustfmt --check` clean.

**Acceptance greps.** `rg "ContextTrustTier::Trusted" retrieval.rs`: empty. `exclude_untrusted_hits` present in both retrieval.rs and execution.rs. `research_cache` present in knowledge.rs. `rg "kb_findings" crates/vox-search/src/execution.rs crates/vox-orchestrator-mcp/src/memory_tools/retrieval.rs`: empty. `SanitizedText` present in all three mcp files.

## Deviations from Plan

**1. [Rule 3 - Blocking] `crates/vox-search/src/rrf.rs` modified (not in files_modified).** The `[tier:..]` prefix hid the `[chunk:`/`[node:` markers `rrf_dedup_key` keys on, so fused lines would have stopped deduplicating. The key now strips a leading tier label; unit test `dedup_key_ignores_the_tier_label`.

**2. [Rule 2 - Missing critical] Qdrant sidecar and web lines under the flag.** Plan named chunk, KG and web lines; qdrant lines carry no stored tier, so they are treated as Untrusted and dropped under `exclude_untrusted_hits` (else a sidecar hit would bypass the preamble filter).

**3. Tests added outside the named file.** `execute_search_plan_labels_tiers_and_excludes_untrusted_on_request` in `crates/vox-search/tests/execute_search_plan_smoke.rs` (vox-search level label/filter/`unified_hits` parallelism/min tier) and `kb_injection_kb_tools_neutralize_exfil_and_tag_block` in `kb_tools/library.rs` (it needs that module's `#[serial]` index harness; `kb_tool_surface.rs` is only a surface-list test).

**4. Process.** `nextest` stalled after a successful build for `-p vox-search -p vox-db` (killed after the 3300 s timeout); those new tests were run with `cargo test` instead, per the lean-verification fallback. A rate-limit interruption stopped the run once; resumed from disk state.

## Known gaps

- `crates/vox-search/src/bundle.rs` appends Tavily web lines to `execution.web_lines` after `execute_search_plan`, outside both the flag and `min_trust_tier`. The MCP bundle does not consume `web_lines`, so nothing reaches the preamble today, but a future consumer of `web_lines` must treat them Untrusted. Not changed (file not in this plan; tavily feature not compiled here).
- Sanitizing a whole line fail-closes to `""` on adversarial nesting, which leaves an empty string in the leg rather than removing the entry (counts stay consistent with the envelope).

## Verify lines deferred to CI

Per the lean policy these plan lines were not run locally and are left to hosted CI: the full `cargo nextest run -p vox-orchestrator-mcp -E 'test(/kb_tiers_|retrieval_bundle|context_envelope/)'` as a single filter (equivalent subsets run above), `cargo nextest run -p vox-search -p vox-db -E 'test(/execute_search_plan|knowledge|search_document|trust_tiers_for|capture_/)'` as written (nextest stalled locally; `cargo test` subsets of the new tests passed, but the broader `knowledge|search_document|capture_` matches in vox-search and vox-db were not run), and the full vox-orchestrator-mcp and vox-orchestrator test suites. `cargo check -p vox-orchestrator -p vox-search --tests` was run locally and passed.

## Known Stubs

None.

## Threat Flags

None. T-03-15, T-03-16, T-03-33, T-03-34 and T-03-44 are mitigated as specified (T-03-15 and T-03-16 mutation-proven; T-03-44 by the `kb_tiers_findings_never_retrieved` test plus the no-`kb_findings` acceptance grep).

## Self-Check: PASSED

- FOUND: crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
- FOUND commits: eeded4f4a, 219003a18
- `git rev-list --count 741663db9..HEAD` = 2 at write time
