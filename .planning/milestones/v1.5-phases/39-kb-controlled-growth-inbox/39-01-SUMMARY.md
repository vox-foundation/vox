# Summary 39-01: KB Web Fallback & Agent Finding Inbox (`GROW-01..06`)

- **Status:** Complete (2026-10-15)
- **Requirements:** `GROW-01`, `GROW-02`, `GROW-03`, `GROW-04`, `GROW-05`, `GROW-06`
- **Changes:**
  - Implemented [`crates/vox-search/src/kb_inbox.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/kb_inbox.rs):
    - `WebFallbackSpendLedger` and `plan_web_fallback` (`GROW-01`) ordering free providers (`Searxng -> DuckDuckGo`) first and paid `PaidTavily` only under the daily spend cap with explicit `spend_cap_notice`.
    - `summarize_web_capture_quarantined` (`GROW-02`) producing sanitized `ContextTrustTier::Untrusted` findings (`quarantined: true`, `injectable_as_curated: false`).
    - `check_similar_docs` and `review_inbox_finding` (`GROW-03`).
    - `build_promotion_draft_pr` (`GROW-04`, `GROW-05`) requiring `>= 2` distinct corroborating domains (or explicit `reviewer_override`) and emitting isolated-worktree draft PR specs with `requires_human_merge: true`.
  - Verified in `cargo --config 'build.rustc-wrapper=""' test -p vox-search --lib` (`267 passed, 0 failed`).
