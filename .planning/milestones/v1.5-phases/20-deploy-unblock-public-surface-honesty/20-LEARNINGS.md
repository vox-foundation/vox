---
phase: 20
phase_name: "deploy-unblock-public-surface-honesty"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 38
  lessons: 5
  patterns: 2
  surprises: 1
missing_artifacts:
  - "STATE.md"
---

# Phase 20 Learnings: deploy-unblock-public-surface-honesty

## Decisions

### Approved wrangler@4.146.0 exact devDependency pin per Task 2 human check...
Approved wrangler@4.146.0 exact devDependency pin per Task 2 human checkpoint.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 20-01-SUMMARY.md

---

### workerd postinstall explicitly denied (workerd
workerd postinstall explicitly denied (workerd: false in docs-astro/pnpm-workspace.yaml allowBuilds) since pages deploy does not require local workerd execution.

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 20-01-SUMMARY.md

---

### Zero issue comments in deploy failure path to prevent notification noise...
Zero issue comments in deploy failure path to prevent notification noise on repeated failures (#462 resolution pattern).

**Rationale:** Explicit decision recorded in summary frontmatter.
**Source:** 20-01-SUMMARY.md

---

### Status Policy SSOT (docs-astro/src/utils/page-status.mjs)
**Status Policy SSOT (`docs-astro/src/utils/page-status.mjs`)**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-06-SUMMARY.md

---

### Established single source of truth for page status policy (research, roa...
Established single source of truth for page status policy (`research`, `roadmap`, `deprecated`, `legacy`, `current`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-06-SUMMARY.md

---

### Defined statusPolicy(status), isInternalsStatus(status), bannerFor(statu...
Defined `statusPolicy(status)`, `isInternalsStatus(status)`, `bannerFor(status)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-06-SUMMARY.md

---

### Built unit test suite tests/unit/page-status.test.mjs (all cases, casing...
Built unit test suite `tests/unit/page-status.test.mjs` (all cases, casing, whitespace).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-06-SUMMARY.md

---

### Route Middleware Integration (docs-astro/src/routeData.ts)
**Route Middleware Integration (`docs-astro/src/routeData.ts`)**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-06-SUMMARY.md

---

### Patched starlight-llms-txt@0.10.0
**Patched `starlight-llms-txt@0.10.0`**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-07-SUMMARY.md

---

### Created reproducible patch docs-astro/patches/starlight-llms-txt@0.10.0....
Created reproducible patch `docs-astro/patches/starlight-llms-txt@0.10.0.patch` via `pnpm patch`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-07-SUMMARY.md

---

### Patched llms-full.txt.ts and llms-custom.txt.ts to forward exclude
Patched `llms-full.txt.ts` and `llms-custom.txt.ts` to forward `exclude: starlightLllmsTxtContext.exclude` to `generateLlmsTxt`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-07-SUMMARY.md

---

### Added patchedDependencies to docs-astro/pnpm-workspace.yaml and verified...
Added `patchedDependencies` to `docs-astro/pnpm-workspace.yaml` and verified with `pnpm install --frozen-lockfile`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-07-SUMMARY.md

---

### Configured Internals Exclusion in astro.config.mjs
**Configured Internals Exclusion in `astro.config.mjs`**

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-07-SUMMARY.md

---

### Exported pure resolveDocLink(url, fromFile, ctx) and plugin remarkDocLin...
Exported pure `resolveDocLink(url, fromFile, ctx)` and plugin `remarkDocLinks(options)`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-08-SUMMARY.md

---

### In-docs relative links (foo.md, ../reference/installation.md#anchor) are...
In-docs relative links (`foo.md`, `../reference/installation.md#anchor`) are rewritten to clean Starlight URLs (`/reference/installation/#anchor`), computing routes via `docSlug()`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-08-SUMMARY.md

---

### Out-of-tree links leaving docs/src to repository code or contracts are r...
Out-of-tree links leaving `docs/src` to repository code or contracts are rewritten to GitHub URLs (`https://github.com/vox-foundation/vox/blob/main/<path>` or `/tree/main/<path>` for directories).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-08-SUMMARY.md

---

### Handles line number and range suffixes (:42 -> #L42, :10-20 -> #L10-L20)...
Handles line number and range suffixes (`:42` -> `#L42`, `:10-20` -> `#L10-L20`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-08-SUMMARY.md

---

### Out-of-tree documentation (docs/superpowers/) and retired research (docs...
Out-of-tree documentation (`docs/superpowers/**`) and retired research (`docs/src/archive/**`) are sent to GitHub blob.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-08-SUMMARY.md

---

### Defined x-vox-version
Defined `x-vox-version: 1`, `always_mount: ["AGENTS.md", "CLAUDE.md", "GEMINI.md"]`, and `never_mount_prefixes: ["docs/superpowers/", "docs/ci/", "apps/", "tree-sitter-vox/"]`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-09-SUMMARY.md

---

### Excludes internal planning (docs/superpowers/) and non-crate tooling dir...
Excludes internal planning (`docs/superpowers/**`) and non-crate tooling directories (`docs/ci/**`, `apps/**`, `tree-sitter-vox/**`) which link to unmounted or internal assets.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-09-SUMMARY.md

---

### mountRoute(repoPath)
`mountRoute(repoPath)`: generates clean lower-case slug with `/`, `.`, `_` replaced by hyphens (e.g. `AGENTS.md` -> `agents-md`, `crates/vox-cli/README.md` -> `crates-vox-cli-readme-md`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-09-SUMMARY.md

---

### discoverMounts({repoRoot, docsSrc, contract})
`discoverMounts({repoRoot, docsSrc, contract})`: scans all non-archive docs pages for relative links pointing outside `docs/src` into the repo, filters against `never_mount_prefixes`, unions with `always_mount`, and inspects each file for frontmatter title.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-09-SUMMARY.md

---

### Extended marker-guarded mirror to populate src/content/docs/repo/.
Extended marker-guarded mirror to populate `src/content/docs/repo/`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-09-SUMMARY.md

---

### Contains deliberate broken route (/this-route-does-not-exist/), broken a...
Contains deliberate broken route (`/this-route-does-not-exist/`), broken anchor (`/reference/cli/#no-such-anchor-p20`), and a valid link (`/reference/cli/`).

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-10-SUMMARY.md

---

### Proved locally and in CI that lychee exits with code 2 on link breakage ...
Proved locally and in CI that `lychee` exits with code `2` on link breakage and reports both broken targets.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-10-SUMMARY.md

---

### Added lychee_selftest step running lycheeverse/lychee-action@v2 with lyc...
Added `lychee_selftest` step running `lycheeverse/lychee-action@v2` with `lycheeVersion: v0.24.2`, `fail: false`, and output to `lychee/selftest.md`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-10-SUMMARY.md

---

### Added verification step that asserts exit code is 2 and both failure tar...
Added verification step that asserts exit code is `2` and both failure targets appear in the report before allowing any link check gate to pass.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-10-SUMMARY.md

---

### docs/src/reference/cli-vox-deploy.md
`docs/src/reference/cli-vox-deploy.md`: corrected `#init` -> `#vox-init` and `#doctor` -> `#vox-doctor`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-10-SUMMARY.md

---

### extractLlmsUrls(text, { origin, baseUrl })
`extractLlmsUrls(text, { origin, baseUrl })`: extracts, dedupes, and maps internal URLs in `llms.txt` and `.well-known` manifests, supporting base URL rewriting for testing against local preview servers.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-11-SUMMARY.md

---

### checkUrls(urls, fetcher, concurrency)
`checkUrls(urls, fetcher, concurrency)`: asynchronously checks a batch of URLs with bounded concurrency (default 8), reporting non-2xx responses and connection exceptions.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-11-SUMMARY.md

---

### 4/4 passing unit tests covering normal URL extraction/rewriting, 200 OK ...
4/4 passing unit tests covering normal URL extraction/rewriting, 200 OK batch verification, 404 detection (negative proof), and thrown fetch error handling.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-11-SUMMARY.md

---

### /voxup: verified returns status 200 and begins with #! shebang.
`/voxup`: verified returns status 200 and begins with `#!` shebang.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-11-SUMMARY.md

---

### /voxup.ps1: verified returns status 200.
`/voxup.ps1`: verified returns status 200.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-11-SUMMARY.md

---

### Replaced the single symlink at docs-astro/src/content/docs with a real d...
Replaced the single symlink at `docs-astro/src/content/docs` with a real directory mirror containing per-entry symlinks to `docs/src`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-12-SUMMARY.md

---

### Excluded archive, .well-known, and SUMMARY.md from being loaded into Sta...
Excluded `archive`, `.well-known`, and `SUMMARY.md` from being loaded into Starlight's content collection.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-12-SUMMARY.md

---

### Implemented prepareMirrorDir(mirrorPath, sourceDir) with safe directory ...
Implemented `prepareMirrorDir(mirrorPath, sourceDir)` with safe directory guard (`.vox-docs-mirror` marker file) so real untracked files are never deleted.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-12-SUMMARY.md

---

### Logged entry count
Logged entry count: mirrored 20 top-level entries; excluded `archive`, `.well-known`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-12-SUMMARY.md

---

### Win32 copy fallback
Win32 copy fallback: prepared for Windows `EPERM`/symlink privilege failure with recursive entry copy fallback.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 20-12-SUMMARY.md

## Lessons

### Status: Deferred — see deferred-items.md.
**Status:** Deferred — see deferred-items.md.

**Context:** Surfaced in structured phase code review.
**Source:** 20-REVIEW.md

---

### File: docs/src/architecture/*-research-*.md, for example agent-chat-ux-a...
**File:** `docs/src/architecture/*-research-*.md`, for example `agent-chat-ux-and-noise-research-2026-07-30.md`, `agent-harness-testing-and-regression-gating-research-2026-07-30.md`, `mesh-dashboard-and-distributed-compute-research-2026.md` (all `status: "current"`) **Issue:** The honesty treatment (banner, noindex, Internals group, exclusion from sitemap and llms) is driven only by the author-set `status`. The built sitemap (`dist/sitemap-0.xml`) contains `/architecture/agent-chat-ux-and-noise-research-2026-07-30/` and similar research notes as regular current docs. The code does what the rule says. The gap is in content classification, but it undermines the phase goal of public-surface honesty on about 60 pages, and nothing flags it. **Fix:** Add a build-output or doc-lint check: a page whose filename matches `-(research|findings)-` must carry `status: research|roadmap`, or appear in an explicit allowlist with a reason, for example a promoted SSOT. Then triage the 64 pages.

**Context:** Surfaced in structured phase code review.
**Source:** 20-REVIEW.md

---

### No gaps. All seven success criteria are verified against main, CI runs o...
No gaps. All seven success criteria are verified against main, CI runs on main, and the live site. The escalation half of DEPLOY-02, previously human_needed, passed its live drill (see truth 2 and 20-UAT.md). Review warnings WR-03 and WR-05 were recorded in deferred-items.md; WR-05 was fixed after phase close (PR #640). WR-06 (research notes still marked `current`) and IN-05 (tutorial record freshness) are covered by phases 21 and 22.

**Context:** Verified in phase acceptance checks.
**Source:** 20-VERIFICATION.md

---

### _Verified: 2026-10-09T17:33:23Z; re-verified 2026-10-09T20:10:00Z_ _Veri...
_Verified: 2026-10-09T17:33:23Z; re-verified 2026-10-09T20:10:00Z_ _Verifier: Claude (gsd-verifier)_

**Context:** Verified in phase acceptance checks.
**Source:** 20-VERIFICATION.md

---

### total: 1 passed: 1 issues: 0 pending: 0 skipped: 0 blocked: 0
total: 1 passed: 1 issues: 0 pending: 0 skipped: 0 blocked: 0

**Context:** Observed during user acceptance testing.
**Source:** 20-UAT.md

## Patterns

### Least-privilege workflow permissions with job-scoped write tokens
Least-privilege workflow permissions with job-scoped write tokens

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 20-01-SUMMARY.md

---

### Edit-in-place managed issue status blocks with HTML comments
Edit-in-place managed issue status blocks with HTML comments

**When to use:** Apply when modifying or extending the affected subsystem.
**Source:** 20-01-SUMMARY.md

## Surprises

### Estimate variance on 20-01
Estimate variance on 20-01: planned 70000 tokens vs 15000 actual (0.21x).

**Impact:** Captured in estimation calibration to refine future token budgets.
**Source:** 20-01-PLAN.md, 20-01-SUMMARY.md

