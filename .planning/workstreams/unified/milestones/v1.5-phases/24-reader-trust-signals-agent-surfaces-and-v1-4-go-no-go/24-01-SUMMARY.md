# Phase 24 Plan 01: Trust Signals & Sidebar Badging Summary

**Status:** Completed
**Requirements Addressed:** TRUST-01, TRUST-04

## Deliverables & Achievements

1. **Freshness Query Module (`docs-astro/src/utils/docs-freshness.mjs`)**:
   - Reads `contracts/reports/docs-freshness/ledger.v1.json` dynamically using `DEFAULT_REPO_ROOT`.
   - Exports `getFreshnessForPath(sourcePath, filePath)` and `loadFreshnessLedger()`.
   - Returns `{ state, shortCommit, commitUrl, title, anchorsCount, brokenMentions }`.

2. **Custom Starlight Banner (`docs-astro/src/components/Banner.astro`)**:
   - Replaced default banner in `docs-astro/astro.config.mjs` via `components: { Banner: './src/components/Banner.astro' }`.
   - Preserves default status banners (research, roadmap, deprecated, legacy).
   - Renders `.vox-trust-banner` with `data-pagefind-ignore` for documentation articles:
     - `verified`: "Verified: Accurate against commit `<short_sha>`" with commit link.
     - `drifting`: "May be out of date: Code changed since verification (at `<short_sha>`)."
     - `broken` / `unanchored`: "Unverified: This page has not yet been anchored to automated code tests."
     - Excludes marketing landing splash page (`/`).

3. **Sidebar Freshness Badges (`docs-astro/src/utils/sidebar.mjs`)**:
   - Integrated freshness state into `collectPages` and `makeItem`.
   - Non-verified items in main sections receive badges:
     - `drifting`: `badge: { text: 'Drifting', variant: 'caution' }`
     - `broken` / `unanchored`: `badge: { text: 'Unverified', variant: 'note' }`
   - Verified items remain unbadged for a clean baseline.
   - Internals section is preserved without per-item duplicate badges.

4. **Styles (`docs-astro/src/styles/reader-layout.css`)**:
   - High-contrast, theme-aware CSS custom properties for `.vox-trust-banner` across light and dark modes.

5. **Playwright Visual Stepper Spec (`docs-astro/tests/trust-signals.spec.ts`)**:
   - 24 tests covering verified, drifting, unverified, and landing pages across desktop, tablet, and mobile in light and dark modes.
   - 100% passing (24/24 in 3.8s).
   - Review bundle screenshots stored in `docs-astro/test-results/review-bundle/`.
