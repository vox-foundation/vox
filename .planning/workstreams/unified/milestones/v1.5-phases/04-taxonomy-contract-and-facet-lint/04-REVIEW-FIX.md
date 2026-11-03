---
phase: 04-taxonomy-contract-and-facet-lint
fixed_at: 2026-10-11T00:00:00Z
review_path: .planning/workstreams/kb/phases/04-taxonomy-contract-and-facet-lint/04-REVIEW.md
iteration: 1
findings_in_scope: 6
fixed: 6
skipped: 0
status: all_fixed
---

# Phase 4: Code Review Fix Report

**Source review:** `04-REVIEW.md` (CR-01, WR-01..WR-05; Info items IN-01..IN-06 out of scope).
**Verification ran in:** the main checkout of this worktree (`workflow.use_worktrees=false`); no temp worktree.
**Not touched:** `deferred-items.md` entries (indented-fence limitation, arch-check path, kb_leak test isolation).

| Finding | Status | Commit | Tests that prove it |
| --- | --- | --- | --- |
| CR-01 unrecognised / unterminated frontmatter fails open | fixed | `3deb14380` | `vox-kb-facets`: `tests::parse_facets_unrecognised_fence_shapes_fail_closed`, `frontmatter::tests::frontmatter_tolerates_a_bom_and_cr_only_line_endings`, `frontmatter::tests::fence_intended_sees_through_bom_blanks_and_indent`. `vox-corpus`: `unrecognised_or_shifted_frontmatter_never_trains_an_opted_out_page`, `bom_and_cr_only_frontmatter_without_opt_out_still_trains`, `fenceless_file_keeps_the_legacy_substring_opt_out`, `real_docs_eligibility_unchanged_for_facetless_docs` (still green). `vox-doc-pipeline`: `unparseable_frontmatter_is_a_hard_parse_error` |
| WR-01 re-epoch admits rows for docs absent from the base tree | fixed | `ba6ba0039` | `vox-kb-facets`: `backlog::tests::re_epoch_rejects_rows_for_docs_absent_from_the_base_tree`, `gate_rejects_same_epoch_addition`. `vox-doc-pipeline`: `facet_backlog_gate_text_cases`. `vox-cli`: `commands::kb::facet_backlog::tests::gate_rejects_same_epoch_addition` (real git repo), `docs_at_splits_nul_and_fails_closed` |
| WR-02 `human_attested_by` accepts any non-blank scalar | fixed | `6a5013438` | `vox-kb-facets`: `tests::effective_training_placeholder_attestation_counts_as_absent`. `vox-doc-pipeline`: `invalid_attestation_is_a_hard_lint_error`. `vox-corpus`: `placeholder_attestation_does_not_clear_the_forced_exclusion` |
| WR-03 misspelled facet keys silently ignored | fixed | `c29b65712` | `vox-kb-facets`: `keys::tests::*` (4), `facet_json_schema_shape` (FACET_KEYS tracks the struct). `vox-doc-pipeline`: `near_miss_facet_key_is_a_hard_lint_error`, `rule_ids_are_stable`. `vox-corpus`: `near_miss_of_an_eligibility_key_excludes_the_page` |
| WR-04 `expand_query` odd-quote guard counts escaped quotes | fixed | `24f043ebc` | `vox-kb-facets`: `taxonomy::tests::expand_query_odd_quote_guard_ignores_escaped_quotes` |
| WR-05 fence tracking ignores fence char and length | fixed | `f42657978` | `vox-kb-facets`: `frontmatter::tests::fence_tracker_follows_commonmark_open_and_close_rules`, `summary::tests::markers_inside_nested_or_mixed_fences_are_ignored`, `provenance::tests::quotes_inside_nested_or_mixed_fences_are_not_counted` |

## What changed, by finding

- **CR-01.** The shared splitter strips a UTF-8 BOM and splits on lone CR. `parse_facets` returns `Err` when the first non-blank line is a `---` fence the splitter cannot read (leading blank or space, unterminated); `Err` is already "not eligible" in `extract_docs` and a hard `facet-parse-error` in the lint, including for backlogged docs. `extract_docs::parse_frontmatter` keeps the legacy `training_eligible: false` substring guard for fence-less files.
- **WR-01.** `Backlog::tighten_only_violations` and `facet_backlog_gate_failures` take the set of `docs/src` paths in the base tree. A re-epoch (valid new note) now rejects every head row whose path is not in it. `run_facet_backlog_gate` lists the tree with `git ls-tree -r --name-only -z <base> -- docs/src` and fails closed on a git error. Bootstrap (no base backlog) is unchanged. The governance doc states the rule.
- **WR-02.** `is_valid_attestation` (non-blank, not a placeholder or AI name such as `false`, `none`, `n/a`, `tbd`, `todo`, `unknown`, `claude`, `llm`) feeds `forced_training_ineligible`, so a placeholder counts as absent. New hard lint rule `facet-invalid-attestation`.
- **WR-03.** `facet_key_near_misses` (case and `-`/`_` variants, or within 1 edit, 2 for keys of 10+ characters) backs the new hard lint rule `facet-unknown-key`. The MENS builder excludes a doc whose near-miss targets `training_eligible`, `human_attested_by`, `origin` or `authorship`. Both new rule ids are registered in `types.rs`, `rule_id`, `message`, `workflow`, `BACKLOGGABLE_RULE_IDS` and the governance table. The key set is a const checked against the generated schema; the schema and TS type are unchanged, so no regeneration was needed (a `Facets` doc comment feeds the schema description, so the note was kept as a code comment).
- **WR-04.** Only unescaped quotes count toward the odd-quote guard.
- **WR-05.** One `FenceTracker` (same character, closer at least as long, nothing after it, backtick info strings cannot contain a backtick) is shared by `summary.rs` and `provenance.rs`. Indented fences remain the accepted limitation in `deferred-items.md`.

## Mutation proofs (guard broken, test red, guard restored; `grep -c` before/after confirmed 1/0 on every mutation)

| Guard | Mutation | Red tests |
| --- | --- | --- |
| CR-01 `None if fence_intended(...) => Err` | `None if false && ...` | `parse_facets_unrecognised_fence_shapes_fail_closed`, `unparseable_frontmatter_is_a_hard_parse_error`, `unrecognised_or_shifted_frontmatter_never_trains_an_opted_out_page` |
| CR-01 substring opt-out in `extract_docs.rs` | `if false && ...` | `fenceless_file_keeps_the_legacy_substring_opt_out` |
| CR-01 BOM strip (`bom_len`) | `if false && text.starts_with('\u{feff}')` | `frontmatter_tolerates_a_bom_and_cr_only_line_endings`, `fence_intended_sees_through_bom_blanks_and_indent`, `parse_facets_unrecognised_fence_shapes_fail_closed` |
| WR-01 `.filter(\|row\| !base_docs.contains(&row.path))` | `&& false` | `re_epoch_rejects_rows_for_docs_absent_from_the_base_tree`, `facet_backlog_gate_text_cases` |
| WR-02 `is_some_and(is_valid_attestation)` in `forced_training_ineligible` | old `!s.trim().is_empty()` | `effective_training_placeholder_attestation_counts_as_absent` |
| WR-02 lint `.filter(\|v\| !is_valid_attestation(v))` | `.filter(\|_\| false)` | `invalid_attestation_is_a_hard_lint_error` |

WR-03..WR-05 tests were written before their fixes where the code compiled (WR-04, WR-05 red first: 3 failing tests before the change, green after).

## Verification run

- `cargo test -p vox-kb-facets` (58 unit + schema golden): pass.
- `cargo test -p vox-doc-pipeline --lib` (52): pass.
- `cargo test -p vox-corpus --lib facet_tests` (15, includes the real-tree oracle `real_docs_eligibility_unchanged_for_facetless_docs`): pass.
- `cargo test -p vox-cli --lib facet_backlog` (3): pass.
- `cargo test -p vox-search --test kb_library_facets` (5) and `--lib kb_` (58): pass.
- Real-tree lint `VOX_SKIP_FRESHNESS_CHECK=1 cargo run -p vox-doc-pipeline -- --lint-only`: exit 0, "no hard errors", with the committed backlog (no row added, no rule id added).

## Not done / notes

- No workspace-wide clippy or tests (lean verification per project policy). No new crate edges, no backlog rows, no generated-artifact regeneration.
- Info items IN-01..IN-06 were out of scope and are untouched.
- CR-only documents now parse for facets, but the MENS section scanner still reads `\n` lines only, so a CR-only body yields no pairs; eligibility is correct and that scanner limitation is unchanged.

---

_Fixed: 2026-10-11_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
