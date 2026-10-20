# Plan 20-10 Summary: Blocking Broken Internal Links & Fragment Checker

## What was built
- **Link Checker Failure Fixture (`docs-astro/tests/fixtures/broken-link/index.html`)**:
  - Contains deliberate broken route (`/this-route-does-not-exist/`), broken anchor (`/reference/cli/#no-such-anchor-p20`), and a valid link (`/reference/cli/`).
  - Proved locally and in CI that `lychee` exits with code `2` on link breakage and reports both broken targets.
- **Workflow Link Checker Self-Test (`.github/workflows/docs-quality.yml`)**:
  - Added `lychee_selftest` step running `lycheeverse/lychee-action@v2` with `lycheeVersion: v0.24.2`, `fail: false`, and output to `lychee/selftest.md`.
  - Added verification step that asserts exit code is `2` and both failure targets appear in the report before allowing any link check gate to pass.
- **Fixed Six Broken Anchors (and Heading Slugs)**:
  - `docs/src/reference/cli-vox-deploy.md`: corrected `#init` -> `#vox-init` and `#doctor` -> `#vox-doctor`.
  - `docs/src/reference/cli.md`: removed empty `(#)` link, leaving clean text reference.
  - `docs/src/architecture/deep-research-prior-art-and-vox-roadmap-2026.md`: corrected `#6-target-architecture-four-phases` -> `#6-target-architecture--four-phases` (preserving em-dash double hyphen from `github-slugger`).
  - `docs/src/how-to/how-to-mcp-vox-validate.md`: re-pointed `#vox_validate_source` and `#vox_validate_file` to `#when-to-use-which-tool` section.
  - `docs/src/how-to/how-to-voxdb-canonical-store.md`: corrected `#training-telemetry-sidecar` to `#historical-vox_training_telemetrydb`.
  - `docs/src/reference/scientia-ssot-handbook.md`: corrected `#6-one-page-arxiv-operator-assist-t016` to `#arxiv-operator-assist-t016`.
- **Sidebar & Asset Link Corrections**:
  - `docs-astro/src/utils/sidebar.mjs`: integrated `docSlug` to derive exact Starlight document slugs from file paths, resolving 12 sidebar target mismatches for dotted filenames, `.generated.md` files, and `index.md`.
  - `docs-astro/scripts/setup-content.mjs`: added static image asset copying (`docs/src/assets` -> `docs-astro/public/assets`) so root-relative `/assets/*` links resolve.
  - `docs/src/index.mdx` & `docs/src/explanation/expl-architecture.md`: updated raw `<img>` tags to root-relative `/assets/*` paths.
  - `docs/src/how-to/index.md`: created section landing page so `/how-to/` resolves to an index document.
- **Blocking Full-Site Offline Gate (`.github/workflows/docs-quality.yml`)**:
  - Added blocking `Offline link check` step running `lycheeverse/lychee-action@v2` with `lycheeVersion: v0.24.2` and `fail: true` over all `docs-astro/dist/**/*.html` with `--offline --include-fragments --index-files index.html`.
- **Cached Nightly Link Checker (`.github/workflows/link_checker.yml`)**:
  - Added `actions/cache@v5` step for `.lycheecache` with key `lychee-${{ github.run_id }}` and restore-keys `lychee-`.
  - Added `--cache --max-cache-age 3d` to lychee args.
- **`.lycheeignore` Review**:
  - Reviewed all 223 lines. Existing `file://` rules match only out-of-tree internal references or raw docs source paths, none of which mask `dist/` HTML output. File left unchanged.

## Verification & Metrics
- **Local Full-Dist Lychee Totals**:
  - Total links checked: **512,765**
  - Unique targets: **14,838**
  - OK: **506,690**
  - Excluded: **6,075**
  - Errors: **0**
- **Mutation Tests**:
  - Deliberately added `<a href="/does-not-exist-at-all-12345/">Broken</a>` to `docs-astro/dist/scratch-mutation-test.html`. Full-dist lychee caught the error and exited with code `2`. Removed scratch file; re-verified clean exit 0.
- **Workflow Guards**:
  - `actionlint .github/workflows/docs-quality.yml .github/workflows/link_checker.yml`: passed.
  - `vox ci workflow-concurrency-guard`: passed.
  - `vox ci cache-key-lint`: passed (`cache-key-lint OK`).
  - `vox ci yaml-parse-check`: passed for both workflows.
- **Playwright Suite**:
  - 20 passed, 1 skipped (0 failures).
