---
phase: 04-taxonomy-contract-and-facet-lint
plan: 04
subsystem: corpus
tags: [kb, mens, training-eligibility, facets, summary-strip]
requires: ["04-01"]
provides:
  - "extract_docs.rs parse_frontmatter uses vox_kb_facets::parse_facets + effective_training_eligible (fail closed)"
  - "extract_from_md_file strips page-one summary blocks before both lanes"
affects: [04-05, 04-06, 04-07, 04-08]
key-files:
  created:
    - crates/vox-corpus/src/corpus/extract_docs_facet_tests.rs
  modified:
    - crates/vox-corpus/src/corpus/extract_docs.rs
    - docs/src/architecture/knowledge-base-practices-research-2026.md
    - docs/src/architecture/adr-NNN-scope-tauri-desktop-only.md
    - docs/src/architecture/crate-classification-2026-05-08.md
key-decisions:
  - "A facet parse error excludes the doc (fail closed); the real-tree oracle proves no doc is affected once three mis-shaped YAML values are fixed"
  - "Files with no frontmatter or an unterminated block are now eligible by default (Facets::default); the old substring-on-flag rule for those shapes is gone per D-06 and no real doc has either shape"
requirements-completed: [TAX-06]
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: d7e481deab0d84927ec5067139f254c05eec40bf
actuals:
  tokens: 5500
  tasks: 2
  commits: 2
---

# Phase 4 Plan 04: MENS extractor shares the facet eligibility rule Summary

The MENS doc extractor now decides eligibility with the same `effective_training_eligible` the facet lint uses, fails closed on malformed facets, and strips page-one summary blocks before both extraction lanes.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: web-promoted page excluded through the shared rule | 230a57928 |
| 2 | Strip summary blocks before both lanes; real-tree oracle | 407e4aa3c |

Tracer verified end-to-end (8 tests green) before expanding.

## Tests (`cargo nextest run -p vox-corpus facet_tests extract_docs`: 17 passed)

- `web_promoted_page_with_flag_true_yields_no_pairs`, `web_promoted_page_with_attestation_yields_pairs`
- `llm_authorship_without_attestation_yields_no_pairs`, `human_authorship_and_absent_flag_yield_pairs`
- `explicit_flag_false_without_authorship_is_excluded`
- `facet_parse_error_excludes_doc`, `deprecated_status_still_short_circuits`
- `staleness_penalty_still_parsed_from_typed_frontmatter`
- `summary_text_never_reaches_any_pair` (whole-line, inline, unterminated)
- `real_docs_eligibility_unchanged_for_facetless_docs` (more than 600 real docs; verbatim legacy rule as oracle)
- 9 pre-existing `extract_docs::tests` still pass.

## Mutation proofs (presence checked before/after each)

- Replaced `strip_summary_blocks(&source)` with a no-op: `summary_text_never_reaches_any_pair` failed; restored (count 1), 17/17 green.
- Replaced `effective_training_eligible(&facets)` with `|| true`: `llm_authorship_without_attestation_yields_no_pairs`, `explicit_flag_false_without_authorship_is_excluded`, `web_promoted_page_with_flag_true_yields_no_pairs` failed; restored (no `|| true` left), 17/17 green.

## Deviations from Plan

**1. [Rule 1 - Bug] Three real docs failed the facet parse** (found by the oracle, as the plan anticipated). Fixed YAML shape only: `supersedes:` scalar to a list in `knowledge-base-practices-research-2026.md` and `adr-NNN-scope-tauri-desktop-only.md`; `superseded_by:` list to a single string in `crate-classification-2026-05-08.md` (second target kept as a YAML comment). All three are `training_eligible: false` or tombstoned, so corpus output is unchanged. Included in commit 230a57928.

**2. Strip step landed in the Task 2 commit, tests in the sibling module split across tasks**, so each commit is green on its own.

### Deferred

None new. Pre-existing items in `deferred-items.md` untouched.

## Known Stubs

None.

## Threat Flags

None. T-04-01, T-04-02, T-04-03 and T-04-16 mitigations are implemented and mutation/oracle proven.

## Self-Check: PASSED

extract_docs_facet_tests.rs exists; commits 230a57928 and 407e4aa3c present; `commits: 2` measured from `plan_head_before`.
