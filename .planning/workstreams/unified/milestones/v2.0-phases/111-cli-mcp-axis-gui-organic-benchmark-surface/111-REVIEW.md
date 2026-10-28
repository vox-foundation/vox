# Phase 111 / Milestone `v2.0` Code Review (`111-REVIEW.md`)

**Reviewed:** 2026-10-19
**Scope:** Phases `108–111` (`crates/vox-corpus/src/training/spoke_roundtrip.rs`, `crates/vox-corpus/src/training/mod.rs`, `crates/vox-audit/src/subcommands/regression_budget.rs`, `crates/vox-cli/src/commands/model/eval.rs`, `crates/vox-cli/src/commands/model/discover.rs`, `crates/vox-gui/src/commands/mens_run_reports.rs`, `crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.tsx`, `crates/vox-gui/ui/src/components/surfaces/Models/MensTrainingView.test.tsx`, `contracts/reports/mens-organic-search-spoke-scorecard.v1.json`, `contracts/eval/spoke-regression-ratchet.v1.json`)
**Mode:** `--fix` (auto-remediated)

## Findings & Fixes Applied

1. **[Warning — Fixed] `crates/vox-gui/src/commands/mens_run_reports.rs` (`parse_organic_scorecard`):**
   - **Issue:** Per-spoke row fields (`search_recall_at_5`, `mean_reciprocal_rank`, `mean_grounding_ratio`, `syntax_validity_rate`, `semantic_validity_rate`, `compilation_pass_rate`, `end_to_end_pass_at_1`) defaulted to `1.0` when omitted in JSON, risking a false-green display on partial/corrupted files.
   - **Fix:** Changed all missing per-spoke numeric defaults to `0.0` (matching `parse_spoke_scorecard`).
2. **[Warning — Fixed] `crates/vox-audit/src/subcommands/regression_budget.rs` (`verify_organic_search_ratchet`):**
   - **Issue:** Checked `overall_search_recall_at_5`, `overall_mrr`, and `overall_compilation_pass_rate`, but did not explicitly check `overall_end_to_end_pass_at_1` against `organic_compilation_pass_rate_floor`.
   - **Fix:** Added explicit `overall_end_to_end_pass_at_1` ratchet check in `verify_organic_search_ratchet`.

## Verdict

- **Critical:** `0`
- **Warning:** `0` remaining (`2` fixed inline)
- **Status:** **PASSED**
