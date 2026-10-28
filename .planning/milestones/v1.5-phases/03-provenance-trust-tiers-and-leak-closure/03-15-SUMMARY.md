---
phase: 03-provenance-trust-tiers-and-leak-closure
plan: 15
subsystem: retrieval
tags: [trust-01, trust-03, d-07, d-07a, wr-10, d-03, gap-closure]
requires:
  - phase: 03-provenance-trust-tiers-and-leak-closure
    provides: "tier_for_knowledge_node_type allowlist (03-03), tier-aware KG leg and exclude_untrusted_hits (03-09), baseline-96 one-time gate (03-03)"
provides:
  - "AGENT_FACT_NODE_TYPE ('agent_fact') and AGENT_CONCEPT_NODE_TYPE ('agent_concept') in vox_orchestrator_types::trust; both Untrusted through the existing default arm"
  - "MemoryManager::persist_fact writes its fact node and relation nodes with the agent types (WR-10)"
  - "retier_agent_knowledge_nodes: one-time re-type of legacy fact/concept rows (non-graphify ids) inside the baseline-96 gate"
affects: [phase-07-injection, phase-09-promotion]
requirements-completed: [TRUST-01, TRUST-03]
tech-stack:
  added: []
  patterns: ["shared node-type constants in the L0 trust crate used by writer and migration", "re-type inside an existing unshipped baseline gate: no DDL, no new baseline integer"]
key-files:
  created: []
  modified:
    - crates/vox-orchestrator-types/src/trust.rs
    - crates/vox-orchestrator/src/memory/manager.rs
    - crates/vox-orchestrator-mcp/tests/kb_retrieval_tiers.rs
    - crates/vox-db/src/store/open.rs
    - crates/vox-db/src/local_tests.rs
key-decisions:
  - "D-07a honored: crates/vox-search/src/execution.rs untouched; the memory leg (MEMORY.md, daily logs) still feeds the preamble unlabelled, and the e2e test pins that"
  - "D-06 narrowed, not removed: fact/concept stay Advisory for non-agent writers; the allowlist match is unchanged"
duration: ~1 h wall, mostly cold cargo builds under machine load
completed: 2026-10-09
status: complete
commits: 2
plan_head_before: b23aa0134fb7968d3be8eef66f85d8e5f650ed25
actuals:
  tokens: 6500
  tasks: 2
  commits: 2
---

# Phase 3 Plan 15: Agent-written knowledge nodes are Untrusted Summary

A fact the model stores through `memory_store` now lands in the knowledge graph as an `agent_fact` / `agent_concept` row (Untrusted): it never reaches the automatic chat preamble and is labelled `[tier:untrusted]` on explicit queries; legacy rows are re-typed once on upgrade.

## Commits

- `378a618e0` fix(03-15): persist_fact graph rows are Untrusted agent nodes; memory leg unchanged (D-07a) (Task 1, tracer)
- `0fc550fde` feat(03-15): re-type legacy persist_fact graph rows once in the baseline-96 gate (D-03, D-07) (Task 2)

## What was built

- **Task 1 (tracer).** Two constants in `trust.rs` (doc comments cite D-07/D-07a; `tier_for_knowledge_node_type` doc notes fact/concept are Advisory only for non-agent writers; the match is unchanged). `persist_fact` passes them to `upsert_knowledge_node` for the fact node and relation nodes. The MEMORY.md write, `memories` row, embedding and both `recall_memory(.., Some("fact"), ..)` calls are untouched.
- **Task 2.** `retier_agent_knowledge_nodes(conn)` runs two parameterized UPDATEs (`fact` -> `agent_fact`, `concept` -> `agent_concept`, both `AND id NOT LIKE 'graphify:%'`), called right after `backfill_search_document_trust_tiers` in the `current_version < BASELINE_VERSION` block. No DDL, no baseline integer, no digest change, no delete.

## Tests and evidence (nextest, `CARGO_INCREMENTAL=0`, scratch `CARGO_TARGET_DIR`)

- `cargo nextest run -p vox-orchestrator-mcp --test kb_retrieval_tiers`: 10/10 green (one pre-existing LEAK flag on `kb_tiers_empty_bundle_is_untrusted`, not a failure), including new `kb_tiers_agent_fact_from_memory_store_never_reaches_the_preamble` (waits for the spawned persist_fact writes, asserts both nodes Untrusted, explicit query labelled `[tier:untrusted]` with envelope min Untrusted, preamble has no `[node:kb-agent-fact]`/`[node:zephyrrel]` line in any leg, and the MEMORY.md memory line still present).
- `cargo nextest run -p vox-orchestrator-types -E 'test(/node_type/)'`: 2/2 (`agent_written_node_types_are_untrusted`, `node_type_allowlist_is_fact_and_concept_only` unchanged).
- `cargo nextest run -p vox-db -E 'test(/kb_agent_nodes_|kb_trust_tier_|kb_findings_migration_from_95|baseline_policy_matches_compiled_schema/)'`: 9/9, including new `kb_agent_nodes_retyped_once_on_upgrade_never_deleted`.
- `cargo check -p vox-orchestrator -p vox-orchestrator-types -p vox-db`: clean.
- RED before GREEN for Task 1: the e2e test failed with `left: Advisory, right: Untrusted` on the unmodified `persist_fact` (nodes were present, so the failure is not vacuous).

## Mutation proofs (restored from scratchpad copies; presence counts checked)

1. **Writer type.** `grep -c 'Some(AGENT_FACT_NODE_TYPE)' manager.rs`: before 1, after reverting to `Some("fact")` 0. `kb_tiers_agent_fact_from_memory_store_never_reaches_the_preamble` went red (`Advisory` vs `Untrusted`). Restored: count 1, `git diff` shows the intended change only.
2. **Legacy re-type.** `grep -c 'retier_agent_knowledge_nodes(conn)' open.rs`: before 1, after deleting the call 0. `kb_agent_nodes_retyped_once_on_upgrade_never_deleted` went red (`Advisory` vs `Untrusted` on `k-fact`). Restored: count 1.

## `memory_lines` test inventory (acceptance)

Existing tests asserting on `memory_lines`: `crates/vox-orchestrator-mcp/tests/kb_retrieval_tavily.rs:72`, `crates/vox-search/tests/execute_search_plan_smoke.rs:46-48`, and the `every_line` helper in `kb_retrieval_tiers.rs`. None changed (execution.rs untouched, `git diff c690da24e -- crates/vox-search/src/execution.rs` empty); the kb_retrieval_tiers ones pass in the run above.

## Deviations from Plan

None - plan executed as written. Task 2 test was written before the implementation but the implementation landed before its first run, so its RED is shown by the mutation proof rather than a separate pre-implementation run.

## Threat Flags

None. T-03-53 and T-03-55 mitigated; T-03-54 transferred per D-07a (pinned by the e2e memory-line assertion); T-03-56 mitigated (UPDATE only, test asserts all four rows survive).

## Known Stubs

None.

## Self-Check: PASSED

Files modified exist; commits `378a618e0` and `0fc550fde` present; SUMMARY `commits: 2` measured from the persisted ledger (`b23aa0134..HEAD`).
