---
phase: 20-deploy-unblock-public-surface-honesty
reviewed: 2026-10-09T11:55:00Z
depth: standard
diff_range: 99eabe584..83bf3e5bd (excluding .planning)
files_reviewed: 34
files_reviewed_list:
  - .github/workflows/docs-deploy.yml
  - .github/workflows/docs-quality.yml
  - .github/workflows/ci-liveness.yml
  - .github/workflows/link_checker.yml
  - .git-blame-ignore-revs
  - contracts/documentation/site-mounted-repo-docs.v1.json
  - contracts/documentation/tutorial-verification.v1.json
  - docs-astro/astro.config.mjs
  - docs-astro/package.json
  - docs-astro/pnpm-workspace.yaml
  - docs-astro/patches/starlight-llms-txt@0.10.0.patch
  - docs-astro/playwright.config.ts
  - docs-astro/public/_redirects
  - docs-astro/scripts/setup-content.mjs
  - docs-astro/src/content.config.ts
  - docs-astro/src/pages/feed.xml.ts
  - docs-astro/src/pages/retired.astro
  - docs-astro/src/plugins/remark-doc-links.mjs
  - docs-astro/src/routeData.ts
  - docs-astro/src/utils/doc-slug.mjs
  - docs-astro/src/utils/git-dates.mjs
  - docs-astro/src/utils/page-index.mjs
  - docs-astro/src/utils/page-status.mjs
  - docs-astro/src/utils/repo-mounts.mjs
  - docs-astro/src/utils/sidebar.mjs
  - docs-astro/tests/smoke.spec.ts
  - docs-astro/tests/lib/dist.ts
  - docs-astro/tests/lib/llms-links.mjs
  - docs-astro/tests/build-output/honesty.spec.ts
  - docs-astro/tests/build-output/links.spec.ts
  - docs-astro/tests/build-output/dates.spec.ts
  - docs-astro/tests/unit/setup-content.test.mjs
  - docs-astro/tests/unit/tutorial-record.test.mjs
  - scripts/docs/tutorial-verify.vox
findings:
  critical: 0
  warning: 6
  info: 9
  total: 15
status: issues_found
---

# Phase 20: Code Review Report

**Reviewed:** 2026-10-09T11:55:00Z
**Depth:** standard
**Files Reviewed:** 34 (plus spot-checks of `docs/src/reference/installation.md`, `docs/src/tutorials/tut-getting-started.md`, `docs/src/robots.txt`, `docs/src/.well-known/llms.txt`, `docs/src/index.mdx`)
**Status:** issues_found

## Summary

Scope was the phase diff only (`99eabe584..83bf3e5bd`); the merged-in `origin/main` commits were not reviewed. The reviewed files are byte-identical between `83bf3e5bd` and the worktree HEAD `7e261d0e3`.

The workflow changes hold up on the security points that were in focus. Permissions are now scoped per job: only `deploy-pages` gets `pages: write` and `id-token: write`, and only the two notify jobs get `issues: write`. Every `${{ }}` value that reaches a `run:` block goes through `env:`, so there is no expression injection. The lychee cache is saved only from `main`, and concurrency blocks are present. The issue edit-in-place logic is correct for the empty, CRLF, legacy-issue, and marker-replacement cases.

In docs-astro, the mirror deletion guard refuses to delete a directory without the marker and unlinks links rather than following them. Mount routes are collision-checked, and generated frontmatter is JSON-quoted. The git-date parser follows renames correctly, and all 742 docs unit tests pass locally (`node --test tests/unit/*.test.mjs`).

There are no blockers. The six warnings are:

- **Internals pages in the RSS feed.** The feed still publishes research/roadmap pages, even though they are kept out of the sitemap and the llms files.
- **Branch dispatch deploys to production.** A `workflow_dispatch` run from any branch deploys that branch to production on Cloudflare.
- **Warm-cache dead-link bypass.** On a local build with a warm content cache, the dead-link gate passes when it should fail.
- **Tutorial verifier fails open.** The flag check passes when `cargo` cannot run.
- **Path filters can drift.** The trigger path lists are hand-copied and can fall behind the dynamic mount set and the sidebar contract.
- **Research pages labelled `current`.** 64 research-named architecture pages ship with that status.

## Warnings

### WR-01: RSS feed publishes Internals (research/roadmap) pages the phase removes from every other discovery surface

**Status:** Fixed in `b619027f4`. `feed.xml.ts` selects entries through `src/utils/feed-entries.mjs`, which drops `statusPolicy(status).noindex` pages (the sitemap rule). `tests/unit/feed-entries.test.mjs` and a `honesty.spec.ts` feed assertion cover it; the spec failed on the pre-fix `dist/feed.xml` (7 noindex links).

**File:** `docs-astro/src/pages/feed.xml.ts:18-27`
**Issue:** The feed takes all of `getCollection('docs')` with no status filter. The sitemap filters out `noindexRoutes`, and the llms files exclude `internalsDocIds` through the patch. The feed is the one discovery surface that still advertises Internals pages as "Official documentation updates". The built feed in this worktree (`dist/feed.xml`) lists `/architecture/deep-research-prior-art-and-vox-roadmap-2026/`, which has `status: "research"`, as its second item. Most of the other top 30 items are `*-research-*` or `*-plan-*` architecture pages.
**Fix:** Apply the same policy the sitemap uses, and add a build-output assertion:
```ts
import { statusPolicy } from '../utils/page-status.mjs';
// ...
const items = docs
  .filter((doc) => !statusPolicy(doc.data.status).noindex)
  .map(/* ... */)
```
In `honesty.spec.ts`, assert that no `<link>` in `dist/feed.xml` matches a route in `noindexRoutes(listDocPages(DOCS_SRC))`.

### WR-02: `workflow_dispatch` from a non-main branch deploys that branch to production on Cloudflare (and attempts GitHub Pages)

**Status:** Fixed in `6c5da2ce6`. `deploy-pages`, `deploy-cloudflare` and `smoke-test` run only when `github.ref == 'refs/heads/main'`; `build-docs` still runs on any ref.

**File:** `.github/workflows/docs-deploy.yml:34-39, 130-198, 201-227`
**Issue:** The notify jobs are gated on `github.ref == 'refs/heads/main'`. `deploy-pages`, `deploy-cloudflare`, and `smoke-test` have no ref gate. `deploy-cloudflare` always runs `wrangler pages deploy ... --branch=main` and has no environment protection. So anyone with write access who dispatches the workflow on a feature branch publishes unmerged docs to voxlang.org, and then the smoke test runs against them. This phase adds a `simulate_failure` input that encourages manual dispatches, which makes an accidental dispatch from the wrong branch more likely. The trigger existed before this phase, but the phase rewrote these jobs without closing the gap.
**Fix:** Gate the deploy jobs:
```yaml
  deploy-pages:
    if: github.ref == 'refs/heads/main'
  deploy-cloudflare:
    if: github.ref == 'refs/heads/main'
```
Leave `smoke-test` inheriting the skip. Alternatively, put `deploy-cloudflare` behind a protected `environment:` that is restricted to `main`.

### WR-03: The dead-link gate is bypassed on a warm content-layer cache (a local build can go green on a dead link)

**Status:** Deferred — see deferred-items.md.

**File:** `docs-astro/src/plugins/remark-doc-links.mjs:38, 209-228`
**Issue:** `docLinksGate` only sees dead links that remark records during the current process. Astro's glob loader (`node_modules/astro/dist/content/loaders/glob.js:90-98, 129-140`) skips re-rendering any entry whose content digest matches the persisted data store. It also stores an entry even when its render threw. That leaves two false-green cases:
- After a build that failed the gate, an unchanged rebuild re-uses the cached entry. The remark plugin never runs, `deadLinks` stays empty, the gate passes, and the page ships with an empty body.
- Deleting or renaming a link **target** leaves the linking page's digest unchanged, so the new dead link is never detected on a warm cache.

CI is not affected today, because every job does a fresh `pnpm install` with no `node_modules/.astro` cache. Local `pnpm build` and any future CI cache are affected.
**Fix:** Fold the link-target state into what invalidates the cache. Options:
- In `prebuild`, delete the content data store (`node_modules/.astro/data-store.json`) or run `astro build --force`.
- Make the gate independent of rendering: at `astro:build:done`, re-scan `listDocPages()` sources with `relativeLinkTargets()` plus `resolveDocLink(..., { strict: true })`.

At minimum, document `--force` next to `docLinksGate`.

### WR-04: The tutorial verifier's flag check fails open when `cargo` cannot run or help text mentions `vox-ml-cli`

**Status:** Fixed in `81458ca5b`. A help run that cannot start or exits non-zero is now a `fail:` note naming the unchecked flags. Delegation is detected from the command path (as `main.rs` does) and checked against `vox-ml-cli --help`. With `cargo` off `PATH`, the old script passed all 7 tutorials and the new one fails 3.

**File:** `scripts/docs/tutorial-verify.vox:298-311`
**Issue:** `run_text("cargo", ...)` returns `""` when the process cannot start. `help.len() is 0` then records the non-failing note "delegated to vox-ml-cli; flags not checked", so the tutorial is recorded as `commands_in_registry: pass` with no flags verified. The `help.contains("vox-ml-cli")` heuristic has the same effect for any command whose help text merely mentions `vox-ml-cli`. Because CI only trusts the committed record (see IN-05), an all-pass record generated on a machine without `cargo` is indistinguishable from a real verification.
**Fix:** Treat "could not run" as a failure, and detect delegation explicitly:
```vox
// vox:skip — excerpt of check_commands
if help.len() is 0 {
    notes = notes.push("fail: could not run `cargo run -p vox-cli -- " + rpath + " --help`")
    continue
}
if help.starts_with("vox-ml-cli") or help.contains("delegates to vox-ml-cli") { ... }
```
Also use `VOX_BIN` (when set) for `--help`, so the flag check and the snippet check run the same binary.

### WR-05: Deploy and quality path filters are hand-copied lists that drift from the dynamic mount set and the sidebar contract

**Status:** Deferred — see deferred-items.md.

**File:** `.github/workflows/docs-deploy.yml:5-33`, `.github/workflows/docs-quality.yml:4-63`
**Issue:** `discoverMounts()` mounts every repo `.md` that any docs page links to. The `on.push.paths` lists are a static snapshot of today's 26 mounts. They match today (I checked against `dist/repo/`), but nothing enforces that. A new docs link to, say, `apps/foo/README.md` mounts that file, and edits to it never trigger a redeploy, so `/repo/apps-foo-readme-md/` goes stale silently. `docs-deploy.yml` also does not trigger on `contracts/documentation/docs-sidebar-section-order.v1.json`, which `sidebar.mjs` reads at build time, so a sidebar-order change never deploys.
**Fix:** Add a unit test in `tests/unit/repo-mounts.test.mjs` that parses both workflow files and asserts every `discoverMounts()` `repoPath` matches some `paths:` glob. Then add `'contracts/documentation/**'` to the `docs-deploy.yml` paths. Simpler alternative: trigger docs-deploy on `'**/*.md'` plus the existing non-Markdown paths.

### WR-06: 64 of 102 `*research*` / `*findings*` architecture pages ship as `status: current`, with no banner and listed in the sitemap and llms

**Status:** Deferred — see deferred-items.md.

**File:** `docs/src/architecture/*-research-*.md`, for example `agent-chat-ux-and-noise-research-2026-07-30.md`, `agent-harness-testing-and-regression-gating-research-2026-07-30.md`, `mesh-dashboard-and-distributed-compute-research-2026.md` (all `status: "current"`)
**Issue:** The honesty treatment (banner, noindex, Internals group, exclusion from sitemap and llms) is driven only by the author-set `status`. The built sitemap (`dist/sitemap-0.xml`) contains `/architecture/agent-chat-ux-and-noise-research-2026-07-30/` and similar research notes as regular current docs. The code does what the rule says. The gap is in content classification, but it undermines the phase goal of public-surface honesty on about 60 pages, and nothing flags it.
**Fix:** Add a build-output or doc-lint check: a page whose filename matches `-(research|findings)-` must carry `status: research|roadmap`, or appear in an explicit allowlist with a reason, for example a promoted SSOT. Then triage the 64 pages.

## Info

### IN-01: `mounted_from` in any docs/src page silently disables the strict dead-link gate and rewrites its edit link and date

**File:** `docs-astro/src/plugins/remark-doc-links.mjs:168-176`, `docs-astro/src/content.config.ts` (schema allows `mounted_from` on every entry), `docs-astro/src/routeData.ts:57`
**Issue:** `fromFile = resolve(repoRoot, mounted_from)` makes `strict` false for a docs/src page that sets the key. Its dead links then only warn, and its edit URL and git date point at the named path.
**Fix:** Honour `mounted_from` only when the entry id starts with `repo/`. Make the doc lint reject `mounted_from` in `docs/src/**` frontmatter.

### IN-02: The mount deny-list covers only `docs/superpowers/`

**File:** `contracts/documentation/site-mounted-repo-docs.v1.json:4`
**Issue:** Any linked repo `.md` is auto-published under `/repo/`. Tracked internal trees such as `.planning/`, `.agents/`, and `.claude/` are not excluded. Today no docs page links into them, and the repo is public, so the risk is presentation rather than disclosure.
**Fix:** Add `".planning/"`, `".agents/"`, `".claude/"` and `"archive/"` to `never_mount_prefixes`.

### IN-03: `page-index` excludes any nested directory named `archive`, while the mirror excludes only the top-level one

**File:** `docs-astro/src/utils/page-index.mjs:9, 29` vs `docs-astro/scripts/setup-content.mjs:32`
**Issue:** Today only `docs/src/assets/archive` (no Markdown) is affected. A future `docs/src/<section>/archive/*.md` would be rendered by Starlight but missing from the noindex, Internals, sidebar, and mount-discovery sets.
**Fix:** Exclude `archive` only at depth 0 (compare `relative(docsSrc, full)`), matching `MIRROR_EXCLUDED`.

### IN-04: Sidebar badges use the raw status, but the policy normalizes it

**File:** `docs-astro/src/utils/sidebar.mjs:28`
**Issue:** `statusPolicy` trims and lowercases the status; `STATUS_BADGE[p.status]` does not. A page with `status: Research` lands in Internals with no badge. A prototype key such as `constructor` would yield a function as the badge.
**Fix:** `const key = String(p.status).trim().toLowerCase(); const badge = Object.hasOwn(STATUS_BADGE, key) ? STATUS_BADGE[key] : undefined;`

### IN-05: The tutorial record's freshness is keyed only on the tutorial blob, and `verified_at_commit` names the parent of the verifying commit

**File:** `contracts/documentation/tutorial-verification.v1.json:4`, `docs-astro/tests/unit/tutorial-record.test.mjs`
**Issue:** CI never runs `tutorial-verify.vox --check`. It only checks that `blob_sha` matches, so these changes leave an all-pass record in place:
- a command being deprecated in `command-registry.yaml`
- a flag being removed
- the Node.js or pnpm minimum changing in `installation.md`

`verified_at_commit` is `b10e20dfd`, but `tut-actor-basics.md` was last rewritten in `c0b7fc9cc`. The value is the HEAD *before* the record was committed.
**Fix:** Include the hashes of `command-registry.yaml` and `installation.md` in the record and assert them in the unit test. Rename the field to `verified_on_parent`, or document what it means.

### IN-06: The snippet check uses `vox` from PATH by default (possibly stale), while the flag check uses `cargo run` of the tree

**File:** `scripts/docs/tutorial-verify.vox:23-29, 611`
**Issue:** The snippet check can pass against an older installed compiler, while the flag check runs against the current tree.
**Fix:** Default to `cargo run -q -p vox-cli --` when `VOX_BIN` is unset, the same as the flag check.

### IN-07: Git dates use the committer date of branch commits, so merged pages can show pre-merge dates

**File:** `docs-astro/src/utils/git-dates.mjs:125`
**Issue:** `git log` (not `--first-parent`) credits each file with the date of the branch commit, not the merge into `main`. On a merge-based repo this understates freshness. That errs in the safe direction, but the date is not "when it went live". Separately, `.git-blame-ignore-revs` lists `ce22efcf0`, `c2c1b9f2c`, and `3456cc901`. Those are 95- and 60-file commits with more than 1,000 inserted lines each, which is content cleanup rather than purely mechanical change.
**Fix:** Consider `--first-parent -m` (credit merges), or document the semantics. Re-evaluate the three non-mechanical ignore entries.

### IN-08: `_redirects` has dead duplicate rules, and GitHub Pages ignores `_redirects`

**File:** `docs-astro/public/_redirects:28-36, 51`
**Issue:**
- `/architecture/architecture-index{,.html,/}` is mapped twice. Cloudflare uses the first match, so the second block, which targets the retired `/architecture/research-index/`, is dead and misleading.
- The parallel GitHub Pages deploy does not apply `_redirects`, so `/archive/*` returns 404 there rather than `/retired/`.
- `upload-pages-artifact` (not given `include-hidden-files`) may also omit `.well-known/` from the Pages artifact. Verify this.

**Fix:** Delete the second `architecture-index` block. Note the Cloudflare-only behaviour next to the `/archive/*` rule, or drop the GitHub Pages job per the cutover plan.

### IN-09: The lychee cache stores failure statuses for 3 days, and the deploy-staleness check alarms on quiet periods

**File:** `.github/workflows/link_checker.yml:45`, `.github/workflows/ci-liveness.yml:102-125`
**Issue:**
- `--cache-exclude-status '429,500..=599'` still caches 403 and 404 answers. A transient bot-block 403 then keeps failing the nightly for up to `--max-cache-age 3d`, and the run saves the cache under `always()`.
- `docs-deploy.yml` is path-filtered, so 14 days without docs pushes opens a "Deploy stale" issue with nothing to deploy. It self-closes on the next deploy.

**Fix:** Add `403` (or `400..=499` minus `404`) to `--cache-exclude-status`, or save the cache only on `success()`. For staleness, accept the occasional alert, or consider the last-push time of docs paths.

---

_Reviewed: 2026-10-09T11:55:00Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
