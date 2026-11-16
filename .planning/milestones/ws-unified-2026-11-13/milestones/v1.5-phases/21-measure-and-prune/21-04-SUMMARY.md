---
phase: "21-measure-and-prune"
plan: "04"
status: completed
date: "2026-10-13"
requirements_met:
  - MEASURE-06
---

# Plan 21-04 Summary: Apply Prune Dispositions & Verification

## Execution Summary
Applied the confirmed user policy for the documentation prune process: research and roadmap pages leave the public sidebar by default and are grouped under the `Internals` group (`status: research` or `status: roadmap`). Verified that existing URLs remain valid, Astro build succeeds cleanly with 0 broken links, and the complete test suite passes.

## Delivered Actions
1. **Frontmatter Updates Applied**:
   - Updated 99 research and roadmap documents in `docs/src/architecture/` with `status: research` or `status: roadmap`.
   - Populated `training_rationale:` on docs with `training_eligible: true` to satisfy doc pipeline governance.
   - Starlight automatically routes these pages out of the public navigation sidebar and into the collapsed `Internals` section with appropriate status banners and `noindex` headers.
   - All permalinks remain unchanged; no 404s or broken internal links were introduced.
2. **Site Verification**:
   - `cargo run -p vox-doc-pipeline -- --lint-only` passed with 0 errors across the entire docs corpus.
   - `pnpm --dir docs-astro build` built all 698 documentation pages and the Pagefind search index in 25.20s without errors.
   - `pnpm --dir docs-astro exec playwright test tests/reader-layout.spec.ts` ran 12/12 tests passing across desktop, tablet, and mobile in light/dark modes.
   - `cargo run -p vox-arch-check` verified architecture constraints with exit code 0.
