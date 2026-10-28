# Phase 24 Plan 02: Agent Surfaces & LLMs.txt Consolidation Summary

**Status:** Completed
**Requirements Addressed:** TRUST-02, TRUST-03

## Deliverables & Achievements

1. **Retired Hand-Committed Split-Brain `llms-full.txt` (TRUST-03)**:
   - Deleted `docs/src/.well-known/llms-full.txt` and `docs-astro/public/.well-known/llms-full.txt`.
   - Added permanent 301 redirect in `docs-astro/public/_redirects`:
     `/.well-known/llms-full.txt   /llms-full.txt   301`
   - Preserves single-source truth: agents and downstream scrapers receive the generated, Internals-filtered `/llms-full.txt` produced by `starlight-llms-txt`.

2. **Curated `llms.txt` Verified-Only Invariant (TRUST-02)**:
   - Audited `docs/src/.well-known/llms.txt` against `contracts/reports/docs-freshness/ledger.v1.json`.
   - Pruned and updated all links to point strictly to `state === 'verified'` public documentation.
   - Guaranteed zero references to `Internals` pages (`status: research | roadmap` or category `Internals`).

3. **CI Automated Tests (`docs-astro/tests/build-output/llms.spec.ts` & `smoke.spec.ts`)**:
   - Verified that `dist/.well-known/llms-full.txt` does not exist on disk.
   - Verified that `_redirects` contains the 301 rule.
   - Added `curated .well-known/llms.txt links point only to verified reader-facing pages (TRUST-02)` test, ensuring any future addition to `llms.txt` is verified in `ledger.v1.json`.
   - Updated `smoke.spec.ts` to query canonical `/llms-full.txt`.
   - 100% of Playwright tests pass (3/3 llms build-output, 15/15 smoke tests).
