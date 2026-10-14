---
phase: 04-taxonomy-contract-and-facet-lint
plan: 06
subsystem: kb-search
tags: [kb, taxonomy, altlabel, lexical-search, chunker, retrieval-baseline]
requires: ["04-01", "04-02"]
provides:
  - "vox_search::library::{search_with_taxonomy, load_taxonomy, Taxonomy}"
  - "CHUNKER_VERSION kb_ids-sections-v2+card-facets+est4b+md512"
  - "card chunk lines `topic: <prefLabel>` / `domain: <prefLabel>`"
  - "vox_kb_search taxonomy expansion + taxonomy_warning response field"
affects: [04-08]
tech-stack:
  added: []
  patterns: ["query-time synonym expansion (no re-index when the vocabulary changes)", "availability over synonyms: load failure degrades and is reported, never silent"]
key-files:
  created:
    - crates/vox-search/tests/kb_library_facets.rs
  modified:
    - crates/vox-search/src/kb_ids.rs
    - crates/vox-search/src/library/chunk.rs
    - crates/vox-search/src/library/mod.rs
    - crates/vox-orchestrator-mcp/src/kb_tools/library.rs
    - contracts/kb/retrieval-baseline.v1.json
    - docs/src/architecture/search-retrieval-ssot-2026.md
key-decisions:
  - "search() stays and delegates to search_with_taxonomy(None); the eval retriever keeps calling search (frozen snapshot has no faceted docs)"
  - "MCP reaches the taxonomy only through the vox_search::library::Taxonomy re-export: no new crate edge"
requirements-completed: [TAX-01, TAX-02]
duration: ~35min
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: 4936893962d4f3b6e4ab09476fb3ad933e081be5
actuals:
  tokens: 9000
  tasks: 2
  commits: 2
---

# Phase 4 Plan 06: altLabel search and facet cards Summary

A search for a taxonomy altLabel or deprecated alias now returns the doc tagged with the matching prefLabel (ROADMAP success criterion 3), because the Library card carries the facet lines and `vox_kb_search` expands the query against the committed taxonomy.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: prefLabel-tagged doc -> card facet line -> altLabel expansion -> hit; baseline re-epoch | 98d5d6607 |
| 2 | vox_kb_search applies the repository taxonomy; SSOT note | 1e31e744e |

Tracer verified end to end (5 `kb_library_facets` tests, `kb_eval_library`, `kb eval --gate`) before Task 2.

## Tests run

- `cargo nextest run -p vox-search --test kb_library_facets`: 5/5 (altLabel, deprecated alias, no-taxonomy miss, unchanged plain query, card lines + no summary markers, real taxonomy, trust-tier parity).
- `cargo nextest run -p vox-search kb_ids library`: 33/33. `--test kb_eval_library`: 11/11 (after re-epoch).
- `cargo run -p vox-cli -- kb eval --gate`: OK.
- `cargo nextest run -p vox-orchestrator-mcp kb_search`: 5/5 (incl. `kb_search_taxonomy_altlabel_finds_tagged_doc`, `kb_search_without_taxonomy_warns_but_answers`).
- `vox ci crate-edges`: OK (693 edges, no new edge).

## Baseline re-epoch

Epoch 2 -> 3. `diff` of the `metrics` block (from `"metrics"` to EOF) between the pre-plan copy and the new file is empty; only `epoch`, `epoch_note` and `chunker` lines changed.

## Mutation proofs (presence checked before and after each)

- Drop the `facet_card_lines` append in `card_body`: 3 of 5 `kb_library_facets` tests failed; restored (1 occurrence).
- Disable the `is_summary_marker_line` skip in `first_paragraph`: `card_carries_facet_lines_and_no_summary_markers` failed; restored (1 occurrence, 0 leftover `false &&`).
- Replace `taxonomy.ok()` with `None` in `search_in`: `kb_search_taxonomy_altlabel_finds_tagged_doc` failed; restored.
- A first attempt at the first two mutations silently did not apply (BSD sed rejected the expression); the presence check caught it (count 0) and the proofs were redone with exact-match replacement.

## Deviations from Plan

**1. [Process] Task 1 had no separate RED commit.** Tests and implementation were committed together (as in 04-01/04-02); the guards are covered by the mutation proofs.

**2. [Process] Pre-commit agent-branch allow-list skipped.** Sequential run on the `claude/` feature branch per dispatch instructions; root-pin and protected-branch checks were applied.

## Deferred Issues

- `vox-orchestrator-mcp` test `kb_leak_chat_turn_writes_no_kb_entry_or_finding` fails (finding count 2, expected 0). The chat-turn path does not use `vox_search::library`, so it is not caused by this plan; logged in `deferred-items.md`, not bisected.
- `vox-doc-pipeline --lint-only` reports `facet-contract-unreadable` (no facet backlog yet); intentionally red until 04-08.

## Known Stubs

None.

## Threat Flags

None. T-04-05, T-04-10 and T-04-19 mitigations are covered by tests and the metrics-identical diff; T-04-20 accepted.

## Self-Check: PASSED

kb_library_facets.rs exists; commits 98d5d6607 and 1e31e744e present; `commits: 2` measured from `plan_head_before`.
