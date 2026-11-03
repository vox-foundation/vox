# Summary 40-01: KB Eval-Prioritized Page-One Rewrites & Rewrite Guard (`REWR-01..05`)

- **Status:** Complete (2026-10-15)
- **Requirements:** `REWR-01`, `REWR-02`, `REWR-03`, `REWR-04`, `REWR-05`
- **Changes:**
  - Implemented [`crates/vox-search/src/kb_rewrite_guard.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/kb_rewrite_guard.rs):
    - `verify_page_one_rewrite` (`REWR-01`): verifies code fences and non-summary body are untouched, blocks `DEFAULT_RETIRED_SURFACES`, preserves numeric literals and negation tokens, and checks backticked references.
    - `validate_rewrite_batch` (`REWR-02`): enforces `<= 25` pages per batch, single category grouping, and `training_eligible: false` for LLM authorship unless `human_attested_by` is present.
    - `prioritize_rewrite_candidates` (`REWR-03`, `REWR-04`): orders landing/hub pages first, then top-N by retrieval usage, then long tail.
    - `evaluate_distinctiveness_and_source_bias` (`REWR-05`): tracks summary distinctiveness and retrieval share stratified by authorship (`llm_crowding_alert`).
  - Verified in `cargo --config 'build.rustc-wrapper=""' test -p vox-search --lib` (`267 passed, 0 failed`).
