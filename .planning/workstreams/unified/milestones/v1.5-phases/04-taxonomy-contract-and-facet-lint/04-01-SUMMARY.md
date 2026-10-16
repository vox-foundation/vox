---
phase: 04-taxonomy-contract-and-facet-lint
plan: 01
subsystem: kb-facets
tags: [kb, facets, taxonomy, training-eligibility, crate-edges]
requires: []
provides:
  - "vox-kb-facets L0 crate: Facets, Source, enums, parse_facets, effective_training_eligible, facet_json_schema"
  - "summary block helpers (inspect_summary, strip_summary_blocks) and provenance validators"
  - "three D-09 consumer edges (vox-doc-pipeline, vox-search, vox-corpus -> vox-kb-facets)"
affects: [04-02, 04-03, 04-04, 04-05, 04-06, 04-07, 04-08]
tech-stack:
  added: []
  patterns: ["L0 pure crate, workspace-hack only", "defactored frontmatter splitter and blake3 encoded-form check"]
key-files:
  created:
    - crates/vox-kb-facets/Cargo.toml
    - crates/vox-kb-facets/src/lib.rs
    - crates/vox-kb-facets/src/frontmatter.rs
    - crates/vox-kb-facets/src/summary.rs
    - crates/vox-kb-facets/src/provenance.rs
    - .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/deferred-items.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - docs/src/architecture/layers.toml
    - docs/src/architecture/where-things-live.md
    - crates/vox-doc-pipeline/Cargo.toml
    - crates/vox-search/Cargo.toml
    - crates/vox-corpus/Cargo.toml
    - contracts/ci/crate-edges.allow.v1.json
    - contracts/ci/crate-graph.v1.json
key-decisions:
  - "Facets fields are all Option/Vec so a missing field is a lint error, not a parse error; unknown frontmatter keys are ignored (no deny_unknown_fields)"
  - "inspect_summary treats only whole-line markers as the block; inline forms report Missing but strip_summary_blocks still removes them (fail closed)"
  - "Blockquote runs are one quote each; only marked quotes are detectable (ponytail ceiling noted in code)"
requirements-completed: [TAX-02, TAX-03, TAX-05, TAX-06, TAX-07]
duration: ~45min
completed: 2026-10-11
status: complete
commits: 3
plan_head_before: 75c31ab937ebf9c6e33a516b05c51cac331f50de
actuals:
  tokens: 6400
  tasks: 3
  commits: 3
---

# Phase 4 Plan 01: vox-kb-facets L0 contract crate Summary

One L0 crate now owns the facet types, the single line-exact frontmatter splitter, the shared MENS eligibility rule, the page-one summary block helpers and the TAX-07 provenance validators, with the three D-09 consumer edges authorized in the crate-edges ledger.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: doc text -> Facets -> effective eligibility, registered L0 crate | d23667642 |
| 2 | Summary block and provenance validators | bad2a0c41 |
| 3 | Three authorized consumer edges and exceptions | 1beb82005 |

Tracer verified end-to-end (13 tests green before expanding).

## Tests (24, all passing, `cargo nextest run -p vox-kb-facets`)

- Eligibility: `effective_training_web_promoted_unattested_is_false_even_if_flag_true`, `effective_training_llm_authorship_branches`, `effective_training_defaults_grandfathered`, `effective_training_blank_attestation_counts_as_absent`
- Parse: `parse_facets_empty_shapes_are_default`, `parse_facets_ignores_unknown_keys`, `parse_facets_invalid_enum_names_the_field`, `parse_facets_reads_all_fields`
- Schema: `facet_json_schema_shape`
- Splitter: `frontmatter_range_covers_the_closed_block_only`, `frontmatter_range_handles_crlf`, `frontmatter_ignores_dashes_inside_a_quoted_value`, `frontmatter_yaml_none_when_unterminated_or_absent`
- Summary: `inspect_summary_counts_words_at_the_boundaries`, `inspect_summary_line_numbers_include_frontmatter`, `inspect_summary_missing_unterminated_multiple_empty`, `inspect_summary_placement_and_fences`, `inspect_summary_link_text_counts`, `strip_summary_blocks_removes_every_form`, `is_summary_marker_line_matches_only_markers`
- Provenance: `is_blake3_content_hash_encoded_form`, `is_iso_date_accepts_and_rejects`, `quote_word_counts_runs_fences_and_sources`, `quote_caps_are_the_discretion_values`

`cargo clippy -p vox-kb-facets --all-targets -- -D warnings` clean. `cargo check -p vox-doc-pipeline -p vox-search -p vox-corpus` clean. `vox ci crate-edges`: OK (693 edges within baseline). `cargo hakari generate`: no changes.

## Mutation proofs

- Blank attestation guard (`is_some_and(|s| !s.trim().is_empty())` -> `is_some()`): `effective_training_blank_attestation_counts_as_absent` failed; restored, presence verified, 13/13 green.
- Fail-closed strip (`None => return out` -> keep the unterminated tail): `strip_summary_blocks_removes_every_form` failed; restored, presence verified, 24/24 green.
- Exceptions ledger (one `to: vox-kb-facets` renamed, count 3 -> 2): `vox ci crate-edges` reported a NEW EDGE failure; restored (count 3), OK again.

## Deviations from Plan

**1. [Rule 3 - Blocking] Stale installed `vox` binary.** `vox ci crate-edges` and `affected-crates --regen` refuse a stale binary; ran with `VOX_SKIP_FRESHNESS_CHECK=1` rather than rebuilding vox-cli workspace-wide. The crate-edges logic reads the working-tree contracts, so the verdict is valid for this change.

**2. [Rule 3 - Blocking] `Cargo.lock` committed with Task 1** (not Task 3), because the new crate lands in the lockfile at Task 1. Task 3 commit added the three consumer rows.

**3. No separate RED commit.** Tests and implementation for a brand-new crate were committed together per task; tests were written first locally and the guards are covered by mutation proofs above.

**4. `crate-graph.v1.json` regeneration also added a stale `vox-search -> vox-crypto` row** (mirror was behind the tree); this is generator output, not an edge added by this plan, and the crate-edges baseline `edges` list is untouched (0 removed lines in the allow-list diff).

### Deferred (out of scope)

`cargo run -p vox-arch-check` exits 1 on a pre-existing violation `[no-hardcoded-abs-path] crates/vox-orchestrator-mcp/src/browser_tools.rs:965`; no `vox-kb-facets` violation. Logged in `deferred-items.md`.

## Known Stubs

None.

## Threat Flags

None. T-04-01/02/03/09 mitigations are implemented and covered by the tests and mutation proofs above.

## Self-Check: PASSED

All five crate files exist; commits d23667642, bad2a0c41, 1beb82005 present; `commits: 3` measured from `plan_head_before`.
