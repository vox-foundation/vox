# Summary 41-01: KB Staleness Detection, Hash-Jittered `review_by` & Mechanical Re-Verification (`STALE-01..04`)

- **Status:** Complete (2026-10-15)
- **Requirements:** `STALE-01`, `STALE-02`, `STALE-03`, `STALE-04`, `DOC-02`
- **Changes:**
  - Implemented [`crates/vox-search/src/kb_staleness.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/kb_staleness.rs):
    - `compute_review_by_date` (hash-jittered per-facet horizon), `days_overdue`, and `staleness_rank_multiplier` (`STALE-01`).
    - `build_verification_queue` ordered by `usage * days_overdue` within page and token budgets (`STALE-02`).
    - `verify_doc_mechanically` (`STALE-03`) advancing `verified_at` only when mechanical evidence resolves; LLM advisory verdicts never advance `verified_at` alone.
    - `demote_to_legacy_markdown` (`STALE-04`) demoting failed/superseded pages to `status: "legacy"` without deleting content.
  - Verified in `cargo --config 'build.rustc-wrapper=""' test -p vox-search --lib` (`267 passed, 0 failed`).
