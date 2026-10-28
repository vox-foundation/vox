# Plan 20-12 Summary: Archive Exclusion & Retirement

## What was built
- **Content Mirror (`docs-astro/scripts/setup-content.mjs`)**:
  - Replaced the single symlink at `docs-astro/src/content/docs` with a real directory mirror containing per-entry symlinks to `docs/src`.
  - Excluded `archive`, `.well-known`, and `SUMMARY.md` from being loaded into Starlight's content collection.
  - Implemented `prepareMirrorDir(mirrorPath, sourceDir)` with safe directory guard (`.vox-docs-mirror` marker file) so real untracked files are never deleted.
  - Logged entry count: mirrored 20 top-level entries; excluded `archive`, `.well-known`.
  - Win32 copy fallback: prepared for Windows `EPERM`/symlink privilege failure with recursive entry copy fallback.
- **Cleaned Docs Loader (`docs-astro/src/content.config.ts`)**:
  - Removed dead `exclude` option from `docsLoader()` since docsLoader base base path is `src/content/docs` and exclusion is enforced by the mirror.
- **Retired Notice Page (`docs-astro/src/pages/retired.astro`)**:
  - Custom `StarlightPage` component with `noindex` robots meta, `pagefind: false`, and links to `/` and GitHub archive (`https://github.com/vox-foundation/vox/tree/main/docs/src/archive`).
- **Archive Redirect (`docs-astro/public/_redirects`)**:
  - Appended `/archive/* /retired/ 301` under P20-D8 to cleanly route all former archive URLs to the retired notice page.
- **Verification Tests**:
  - `tests/unit/setup-content.test.mjs`: 4/4 passing unit tests covering all guard conditions.
  - `tests/build-output/archive.spec.ts`: 5/5 passing playwright build-output tests verifying `dist/archive/` absence, sitemap clean, retired page meta and redirects.

## Metrics
- Archive pages removed from `dist/`: 297 pages (reduced total built pages from 971 to 674).
- Mirrored entries: 20 directories and root docs files.
- Tests passing:
  - `node --test tests/unit/setup-content.test.mjs`: 4/4 pass.
  - `npx playwright test tests/build-output/archive.spec.ts tests/build-output/dates.spec.ts`: 7/7 pass (1 skipped).
