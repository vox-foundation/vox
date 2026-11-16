---
phase: 04-taxonomy-contract-and-facet-lint
plan: 05
subsystem: doc-pipeline
tags: [kb, facets, lint, backlog, taxonomy]
requires: ["04-01", "04-02"]
provides:
  - "vox_kb_facets::{BACKLOG_REL, Backlog, BacklogRow, Backlog::from_yaml_str, Backlog::rules_for}"
  - "vox-doc-pipeline LintKind::Facet(FacetIssue) with 19 stable facet-* rule ids (backlog keys)"
  - "facets::{FacetCtx::load, facet_lint_rel_path, facet_issues, lint_facets, BACKLOGGABLE_RULE_IDS}"
affects: [04-06, 04-07, 04-08]
key-files:
  created:
    - crates/vox-kb-facets/src/backlog.rs
    - crates/vox-doc-pipeline/src/pipeline/facets.rs
    - crates/vox-doc-pipeline/src/pipeline/facets_tests.rs
  modified:
    - crates/vox-kb-facets/src/lib.rs
    - crates/vox-doc-pipeline/src/pipeline/types.rs
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - crates/vox-doc-pipeline/src/pipeline/lint.rs
key-decisions:
  - "A missing or invalid taxonomy/backlog is one hard facet-contract-unreadable error (fail closed); the contracts load once in lint_in before the rayon pass"
  - "Backlog rows excuse exact rule ids per exact docs/src path; an unknown rule id in the backlog makes the contract unreadable"
  - "hard_errors counts Facet issues via is_hard() (only facet-backlog-stale is soft)"
requirements-completed: [TAX-01, TAX-03, TAX-04, TAX-05, TAX-06, TAX-07]
completed: 2026-10-11
status: complete
commits: 2
plan_head_before: a18b863d15061ef15e33236219d73c92d9455e7a
actuals:
  tokens: 14000
  tasks: 2
  commits: 2
---

# Phase 4 Plan 05: Facet lint in vox-doc-pipeline Summary

The doc lint now holds every authored `docs/src/**/*.md` to the facet rules (topic/domain prefLabels, summary block, authorship, provenance, quote caps), excuses a backlog doc for exactly its listed rule ids, warns on stale excuses, and fails closed when its contracts are missing.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: contracts loaded once; unlisted doc fails, listed doc passes | 9d2e7e879 |
| 2 | Full facet rule set for ROADMAP success criterion 1 | ee2b27272 |

Tracer verified end-to-end (31 doc-pipeline tests green) before expanding.

## Tests (`cargo nextest run -p vox-doc-pipeline -p vox-kb-facets`: 89 passed; clippy `-D warnings` clean on both crates)

- Backlog: `backlog_accepts_valid_and_looks_up_by_exact_path`, `backlog_rejects_every_malformed_shape`
- Scope: `facet_lint_rel_path_scopes_to_authored_md_under_docs_src`
- Base rules: `facet_issues_unfaceted_doc_reports_the_four_required_ids`, `facet_issues_invalid_value_is_parse_error_only`, `clean_page_has_no_issues`
- Labels (D-07): `topic_and_domain_must_be_pref_labels` (unknown + did-you-mean, altLabel names prefLabel, deprecated, domain mismatch)
- Summary: `summary_word_count_boundaries` (39/40/120/121), `summary_placement_and_missing`
- Authorship/eligibility: `authorship_model_required_for_llm_authorship`, `training_eligible_contradiction`
- Provenance: `web_promoted_sources_are_required_and_complete`, `bad_source_date_or_hash_fails_on_any_page`, `quote_over_cap_per_quote_and_per_page_on_promoted_pages_only`, `invalid_review_date_is_reported`
- Ids: `rule_ids_are_stable`
- Backlog semantics: `backlog_excuses_only_its_listed_rules` (also renamed doc, archive, generated), `stale_backlog_rule_is_a_soft_warning`, `stale_backlog_entry_warns_but_exits_zero` (lint_in)
- Concurrency: `facet_errors_do_not_depend_on_thread_order`
- lint_in: `lint_in_facet_backlog_tracer`, `missing_contract_is_one_hard_error` (taxonomy and backlog), `empty_backlog_holds_every_doc`; `lint_in_reads_readme_sync_under_root_not_cwd` still passes

## Mutation proofs (presence checked before/after each)

- Summary range `..=` to `..` (120 words rejected): `summary_word_count_boundaries` failed; restored, presence verified.
- Backlog filter replaced by `if true` (nothing excused): `backlog_excuses_only_its_listed_rules` and `lint_in_facet_backlog_tracer` failed; restored (no `if true` left), 89/89 green.

## Deviations from Plan

**1. [Process] Task 1 has no separate RED commit.** Tests and implementation committed together per task (same as 04-01/04-02/04-04); guards covered by the mutation proofs.

**2. [Rule 1] `lint_facets` falls back to anchoring on `docs/src/`** when `strip_prefix(repo_root)` fails (drive casing), mirroring existing `lint.rs` behavior, so a path-shape mismatch cannot silently skip the facet rules.

**3. `hard_errors` uses a guard arm** (`LintKind::Facet(i) if i.is_hard()`) rather than a plain `matches!` list entry, since one wrapper variant has one soft id.

### Deferred

None new. The real-tree lint is intentionally red (fail-closed `facet-contract-unreadable`) until 04-08 commits the seeded backlog; not pushed, per the plan's Push rule. `deferred-items.md` untouched.

## Known Stubs

None.

## Threat Flags

None. T-04-01, T-04-04, T-04-11, T-04-17, T-04-18 mitigations are implemented and covered by named tests; the tighten-only base-diff gate for T-04-04 lands in 04-07.

## Self-Check: PASSED

backlog.rs, facets.rs, facets_tests.rs exist; commits 9d2e7e879 and ee2b27272 present; `commits: 2` measured from `plan_head_before`.
