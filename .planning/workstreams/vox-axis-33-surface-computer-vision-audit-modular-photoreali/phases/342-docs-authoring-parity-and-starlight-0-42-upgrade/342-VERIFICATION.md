---
phase: 342-docs-authoring-parity-and-starlight-0-42-upgrade
verified: 2026-11-06T06:10:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".github/dependabot.yml"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-01-PLAN.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-01-SUMMARY.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-02-PLAN.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-02-SUMMARY.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-03-PLAN.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-03-SUMMARY.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-04-PLAN.md"
  - ".planning/phases/342-docs-authoring-parity-and-starlight-0-42-upgrade/342-04-SUMMARY.md"
  - "crates/vox-doc-pipeline/src/pipeline/openwiki.rs"
  - "docs-astro/astro.config.mjs"
  - "docs-astro/package.json"
  - "docs-astro/pnpm-lock.yaml"
  - "docs-astro/pnpm-workspace.yaml"
  - "docs-astro/src/plugins/remark-vox-include.mjs"
  - "docs-astro/src/utils/links-validator-exclude.mjs"
  - "docs-astro/tests/build-output/alerts.spec.ts"
  - "docs-astro/tests/unit/links-validator-exclude.test.mjs"
  - "docs-astro/tests/unit/remark-vox-include.test.mjs"
  - "docs/src/architecture/docs-site-asciidoc-parity-findings-2026.md"
covered_digest: "v3:sha256:4fd85d38f733249aec734577e1a24eb248aa53702a695867f0329744f83415bb"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 342: Docs Authoring Parity and Starlight 0.42 Upgrade Verification Report

**Phase Goal:** Close the verified remainder of the AsciiDoc-parity blueprint for `voxlang.org`: upgrade `docs-astro` to Starlight >=0.42 / Astro >=7.2.10, render the GitHub-alert docs as real asides, and add build-time internal-link validation, without new contracts, new `.mdx` churn, or a parallel status system.
**Verified:** 2026-11-06
**Status:** passed
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | DOCS-PARITY-01: include plugin (tags, `#` leaders, multi-select, ranges, exact names, dedent) and `llms.txt` version from root `Cargo.toml` are committed (no longer "uncommitted") | VERIFIED | `9829418e9` is an ancestor of HEAD; `git status` clean for `docs-astro/`. `astro.config.mjs` L18-20 reads `Cargo.toml` `[workspace.package]` version, used in the llms description. `node --test remark-vox-include.test.mjs links-validator-exclude.test.mjs` run by me: 16 tests, 16 pass, 0 fail (tag with `#` leader, `foo` does not match `foo-2`, multi-select, range, missing anchor throws). |
| 2 | DOCS-PARITY-02: site runs Astro >=7.2.10 and Starlight >=0.42 | VERIFIED | `package.json`: astro `^7.4.0`, `@astrojs/starlight` `^0.42.0`. Lockfile resolves `astro@7.4.5`; plugin peers `@astrojs/starlight >=0.42.0` satisfied. `pnpm-workspace.yaml` has no `minimumReleaseAgeExclude` (grep count 0). Config uses `unified()` processor and an `import.meta.resolve` components alias. |
| 3 | DOCS-PARITY-02: overrides (`Banner`, `Sidebar`, `PageSidebar`, `TableOfContents`, `SiteTitle`), `routeData.ts` and alias still work; build and test suites do not regress | VERIFIED | My own `pnpm build`: exit 0, `907 page(s) built`, no deprecation or error noise. All five component overrides still registered in `astro.config.mjs`. Unit suite rerun by me: 913 tests, 910 pass, 3 fail, and the 3 failing names are exactly the three lines in `342-baseline-unit-failed.txt`. Playwright not rerun (per instructions); SUMMARY diff vs the 20-line baseline is empty, and 20 = 2 build-output + 18 trust-signals matches the VALIDATION-recorded pre-upgrade split, so plausible. |
| 4 | DOCS-PARITY-03: GitHub-alert docs render as asides on the site, with no raw `[!TYPE]` blockquote left | VERIFIED | `starlightGitHubAlerts()` registered first in `plugins` (exact pin 0.5.0 in `package.json` and lockfile). `alerts.spec.ts` asserts per-page mapped `starlight-aside--*` class, no raw blockquote in dist, and no indented alert in source. Built `how-to/how-to-train-mens-4080` and `reference/mens-training` pages contain `starlight-aside--` and 0 `<blockquote`. Source scan: no indented `> [!TYPE]` remains in `docs/src` outside archive. 26 pages carry alerts (ROADMAP says 24; the plan's count was approximate and is a superset). |
| 5 | DOCS-PARITY-04: `starlight-links-validator` runs in the build on default fatal options | VERIFIED | `starlightLinksValidator({ exclude })` in `astro.config.mjs`, no `failOnError:false`; pinned 0.27.0. My build log ends with `All internal links are valid.` before `907 page(s) built`, exit 0. Exclude predicate (`links-validator-exclude.mjs`) is narrow: `.mdx` source or target routes (3 `.mdx` pages in `docs/src`) plus the documented `127.0.0.1:11434` probe URL. Mutation probe (bad link plus bad hash fails the build) is recorded in the 342-04 SUMMARY; not re-run by me. |
| 6 | DOCS-PARITY-04: the two `remark-doc-links` failures are fixed at source | VERIFIED | t3 page: 0 remaining `](../../../../crates/` links; build log has 0 `resolves outside repository root`. Hub: `l5-binaries.generated.md` row is `` `vox-cargo-shim` `` with no link, and `docs/src/api/vox-cargo-shim.generated.md` does not exist, so nothing dangles. Generator fix is `crate_link_cell` in `openwiki.rs` (L441), with unit test `crate_link_cell_links_only_existing_api_pages` (L482). The generated file was regenerated by the generator, not hand-edited. |
| 7 | Constraints: no new contracts, no new `.mdx` churn, no parallel status system, dependabot not pinning old Astro/Starlight | VERIFIED | Phase diff touches only docs-astro, the generator, three generated hubs, two alert docs, the t3 page, the findings SSOT and `.github/dependabot.yml`. No `.mdx` or contracts files changed. `dependabot.yml` docs-astro block has no `ignore:` list. Findings SSOT updated to "Done (Phase 342)" for alerts and validator. |

**Score:** 7/7 truths verified (0 behavior-unverified)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `docs-astro/src/plugins/remark-vox-include.mjs` | include plugin with region extraction | VERIFIED | 16/16 unit tests pass; used in the `unified()` remark chain. |
| `docs-astro/src/utils/links-validator-exclude.mjs` | pure exclude predicate | VERIFIED | Imported and passed to the validator in `astro.config.mjs`; unit-tested. |
| `docs-astro/tests/build-output/alerts.spec.ts` | alert guard spec | VERIFIED | Substantive (3 tests with a vacuous-pass guard); runs against dist. |
| `crates/vox-doc-pipeline/src/pipeline/openwiki.rs` | `crate_link_cell` + test | VERIFIED | Called in `generate_subsystem_hubs` (L416). |
| `docs-astro/patches/starlight-llms-txt@0.12.0.patch` | re-keyed patch | VERIFIED | `pnpm-workspace.yaml` `patchedDependencies` points at it; build applies it. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `astro.config.mjs` | `starlight-github-alerts` | plugins array | WIRED | first entry |
| `astro.config.mjs` | `links-validator-exclude.mjs` | `exclude: linksValidatorExclude(docPages)` | WIRED | `docPages` from `listDocPages` |
| `astro.config.mjs` | root `Cargo.toml` | `readFileSync` version regex | WIRED | feeds llms description |
| `openwiki.rs` generator | `l5-binaries.generated.md` | regeneration | WIRED | row matches generator output |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Include plugin and exclude predicate | `node --test` on the two unit files | 16/16 pass | PASS |
| Unit suite vs baseline | `node --test tests/unit/` | 910 pass, 3 fail (= baseline names) | PASS |
| Full docs build with link validation | `pnpm build` | exit 0, "All internal links are valid.", 907 pages | PASS |
| Alerts rendered | grep dist for `starlight-aside--` / `<blockquote` | asides present, 0 blockquotes | PASS |

### Probe Execution

None declared by the phase. Step 7c: SKIPPED.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| DOCS-PARITY-01 | pre-phase WIP `9829418e9` | include plugin + version SSOT | SATISFIED | Truth 1 |
| DOCS-PARITY-02 | 342-02 | Starlight 0.42 / Astro 7 upgrade | SATISFIED | Truths 2, 3 |
| DOCS-PARITY-03 | 342-03 | GitHub alerts as asides | SATISFIED | Truth 4 |
| DOCS-PARITY-04 | 342-01, 342-04 | link validator plus the two failing links | SATISFIED | Truths 5, 6 |

No orphaned requirement IDs.

### Anti-Patterns Found

None blocking. TBD/FIXME/XXX scan of the phase's new source files found no unreferenced debt markers.

### Warnings and Observations (non-blocking)

1. **"Playwright reader-layout/trust-signals stay green" is not literally true.** 18 `trust-signals.spec.ts` TRUST-01/04 cases (`.vox-trust-banner` not visible) plus 2 `build-output/links.spec.ts` cases fail. They fail identically before the upgrade (frozen baseline in `342-baseline-playwright-failed.txt`), so the phase introduced no regression. The ROADMAP wording overstates; the failures belong to other phases.
2. **3 unit failures are pre-existing** (`docs-deploy` / `docs-quality` workflow path coverage, tutorial verification record). Same names before and after. Worth a follow-up because the new build inputs (`Cargo.toml` read at config time, the new util) may widen the path-coverage gap those tests check.
3. **Validator blind spot:** links from and to the 3 `.mdx` pages are excluded because their routes cannot be derived. This is documented in the predicate and the SSOT, and is narrow, but those pages are not link-checked.
4. **Non-fatal** `[remark-doc-links] Target not found: 'milestones/v4.3-ROADMAP.md'` (mounted `.planning/ROADMAP.md`) is still logged; validator does not flag it.
5. Floors in `package.json` (`^7.4.0`, `^0.42.0`) are lower than the plan's, per the human's "mature-only" checkpoint answer. Lockfile gives astro 7.4.5 / starlight 0.42.9; this meets the roadmap floor (>=7.2.10 / >=0.42).
6. Not independently verified by me: the Playwright runs, the Rust `cargo test` (test exists in the committed file), the mutation probe, and the feed into nightly Docs Link Checker (#578).

### Human Verification Required

None. (Manual-only item from VALIDATION, the package-legitimacy checkpoint, is recorded as approved in the 342-02 SUMMARY; lockfile CI-pnpm compatibility is observable only on the first PR docs-quality run.)

### Gaps Summary

No gaps. The committed code delivers every roadmap success criterion: Astro 7.4.5 / Starlight 0.42.9 builds 907 pages, alerts render as asides with a guard spec, the links validator is fatal by default and passes with zero findings, and both original `remark-doc-links` failures are fixed at their sources.

---

_Verified: 2026-11-06_
_Verifier: Claude (gsd-verifier)_
