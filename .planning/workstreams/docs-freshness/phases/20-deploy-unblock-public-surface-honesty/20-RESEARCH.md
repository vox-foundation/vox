# Phase 20: Deploy Unblock & Public-Surface Honesty - Research

**Researched:** 2026-10-08
**Domain:** GitHub Actions deploy/alerting, Astro Starlight 0.38 route data + sidebar, starlight-llms-txt 0.10, git-history dating
**Confidence:** HIGH (almost every claim was read from installed source, the repo, `gh`, or the live site this session)

<user_constraints>
## User Constraints (from workstream CONTEXT.md / REQUIREMENTS.md — plus phase 20-CONTEXT.md decisions P20-D1..D7, applied in the next section)

### Locked Decisions (REQUIREMENTS.md D1–D10, verbatim)
- D1: Separate GSD workstream `docs-freshness`, phases 20+.
- D2: All five outcomes in scope across v1.3–v1.4: drift detection, generated reference, agent-drafted fixes, case-by-case pruning, reader freshness signals.
- D3: LLM fixes arrive only as **draft PRs for human review** — never auto-merged (v1.4).
- D4: Build, mostly in-house (hybrid). Keep Starlight. Vendor bots that call LLM hosts directly are design references only (they violate the LLM-facade and secrets policy).
- D5: Pruning is **case by case** from a generated audit report — no blanket or time-based archiving.
- D6: v1.3 = phases 20–24 (no new crate edges). LLM verification + draft-PR bot = v1.4.
- D7: Internal architecture/research/roadmap notes move to a separate **Internals** section: out of the main sidebar, `noindex`, excluded from llms.txt.
- D8: Drift gate **blocks new drift only** (tighten-only baseline ratchet) on reader-facing `status: current` pages; existing drift is baselined.
- D9: No hosted-bot trial. Instead, gather published accuracy and acceptance data for hosted docs bots and set a pivot rule: if they are far better than what we can build, switch to buying.
- D10: No LLM runs on pull requests. PR-time checks are deterministic only.
- D11 (2026-10-08, user): Links on the site must work, repo Markdown must render as doc pages, and tutorials especially must all be current. This promotes docs-as-tests for tutorials from out of scope into v1.3.

### Hard constraints (CONTEXT.md "Hard constraints (repo policy)", verbatim)
- Automation is `.vox` scripts via `vox run` — no new `.sh`/`.py`/`.ps1`.
- GitHub-hosted runners only; jobs ≤30 min on PR, ≤180 min scheduled; every workflow needs `concurrency:`.
- No vendor LLM hostnames/SDKs in workspace code; secrets via `vox_secrets::resolve_secret`.
- `docs/src/archive/` is tombstoned — never read for planning.
- Generated docs are fixed at the generator, never hand-edited.
- New crate deps need layer compliance (`docs/src/architecture/layers.toml`) and may need user-authorized exceptions.

### Claude's Discretion
Not formally recorded. Everything below marked "Recommendation" is discretionary; the Internals membership rule and the alert assignee are flagged in Open Questions for user confirmation.

### Deferred Ideas (OUT OF SCOPE for Phase 20)
- Ledger-driven "Verified against `<sha>`" banners, sidebar verification badges, curated `current` llms.txt set, retiring hand-written `llms-full.txt` → Phase 24 (TRUST-01..03).
- Mention extractor / prune pass → Phase 21. Vale prose linting → out of scope. Upgrading `starlight-llms-txt` → out (0.12 needs astro 7, see Pitfalls).
</user_constraints>

## Decisions Applied (20-CONTEXT.md, user 2026-10-08) — these SUPERSEDE recommendations/open questions below

| # | Decision (verbatim summary) | Effect on this research |
|---|---|---|
| P20-D1 | Internals = every `status: research` or `status: roadmap` page (131), regardless of category, via one shared status module | Locks **Rule A**. No category or frontmatter rewrites; `VALID_CATEGORIES` and the sidebar JSON stay unchanged |
| P20-D2 | Internals stay in Pagefind, visibly labelled Internals; `noindex` for search engines; excluded from every llms.txt variant | Do **not** set `entry.data.pagefind = false` for Internals. Keep the banner/label |
| P20-D3 | `ci-liveness` alert after **14 days** without a successful `docs-deploy.yml` run | N = 14 in the explicit success-age list in `ci-liveness.yml` |
| P20-D4 | One issue edited in place, auto-closed on the next green deploy, label `nightly-failure`, assignee **@brbrainerd** | `gh issue create --assignee brbrainerd --label docs-deploy-broken,nightly-failure`; edit body, never comment on repeats; success job closes |
| P20-D5 | Drop llms.txt links to `front-facing-honesty-audit-2026` and `mcp-tool-reference`; fix `/AGENTS.md`, `/CLAUDE.md`, `/GEMINI.md` by rendering those files as pages (LINKS-02) | Hand `.well-known/llms*.txt` edits + mount AGENTS/CLAUDE/GEMINI (all three carry `title:` frontmatter, see Addendum 2) |
| P20-D6 | Reality Audit page → `status: roadmap` + note "phase 21 of docs-freshness reactivates it"; 2 specs + plan → `status: deprecated` + "Superseded by docs-freshness" | HONEST-03 fully specified. Under P20-D1 the audit page (Contributors) becomes Internals |
| P20-D7 | DEPLOY-01 needs a green **push** run touching `docs/src/`; close #462 only after, recording both root causes | As researched |

Defaults (planner may adjust):
- Pin `wrangler` at the current latest stable. Keep the SUS `too-new` checkpoint on the exact version.
- Pages/OIDC permissions on the Pages job only.
- Fix robots.txt sitemap, the `routeData.ts` comment, and the dead `llmsFullTxt` key.
- `pnpm patch` starlight-llms-txt.
- Date map: `.git-blame-ignore-revs` + ssot-autoregen commits, follow renames, keyed by file path.

Still undecided (from the LINKS/TUT addenda):
- (6) Drop `archive/` from the build?
- (7) Route prefix for mounted repo docs (`/repo/…` suggested)?
- (8) Add a `verified_against` frontmatter key for tutorials?

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DEPLOY-01 | Site deploys from `main`; #462 closed with root cause | Run 37869058499 green but it was `workflow_dispatch`; no green *push* run yet. #462 still OPEN, 57 comments, no assignee. §DEPLOY-01 |
| DEPLOY-02 | Failure escalates once; N-day liveness via `ci-liveness` | Existing notify job comments every failure (the 57-comment cause); `ci-liveness.yml` only knows scheduled workflows. §DEPLOY-02 |
| DEPLOY-03 | Live smoke: home, `/voxup`, every llms.txt URL, no retired syntax | `tests/smoke.spec.ts` has 9 tests, none for `/voxup` or llms URLs; 3 llms-full.txt URLs 404 **today**. §DEPLOY-03 |
| DEPLOY-04 | Correct per-page git dates; mechanical commits excluded | Symlink defeats Starlight's git lookup: 0 of 933 local pages render a date. Use existing `git-dates.mjs` + route middleware. §DEPLOY-04 |
| HONEST-01 | Banner + `noindex` from `status` frontmatter | Route middleware can set `entry.data.banner` + push robots meta; 140 pages qualify. §HONEST-01/02 |
| HONEST-02 | Internals section; out of nav, indexing, every llms.txt variant | Derive from `status` in `sidebar.mjs`; llms-full has no exclude hook → `pnpm patch`. 131 pages by status rule. §HONEST-01/02 |
| HONEST-03 | Overlapping designs merged or marked superseded | 3 files + audit narrative located; repo convention = `status: deprecated` + `> **Superseded (date).**` blockquote. §HONEST-03 |
| LINKS-01 | Relative `.md` links render as working routes (build-time remark rewrite) | 2,498 in-docs links on 421 pages break live. Astro slug = github-slugger per segment, strip `/index`. Custom `remark-doc-links.mjs` with realpath. §Addendum LINKS-01 |
| LINKS-02 | Repo Markdown rendered from a single source; non-md → GitHub blob | `docs/src/AGENTS.md` is a different scoped file (not a copy). Mirror-dir symlinks verified with tinyglobby. 81 escaping `.md` targets classified. §Addendum LINKS-02 |
| LINKS-03 | Blocking internal-link check over `dist/` + broken fixture; external nightly, cached | Existing lychee runs are non-blocking (dist) or source-only (nightly). Use lychee `--offline` in docs-quality + fixture mutation step. §Addendum LINKS-03 |
| TUT-01 | Tutorials audited against code, verified-against commit | Punch list: scaffold drift (getting-started), wrong include (workflow-durability), Node version mismatch, stale `script-execution` caveat. §Addendum TUT-01 |
</phase_requirements>

## Summary

The deploy itself is unblocked: after the user rotated `CF_API_TOKEN`, run 37869058499 passed all four real jobs [VERIFIED: `gh run view 37869058499`]. Two things stop DEPLOY-01 from being *done*: that run was a `workflow_dispatch`, while every push run up to 2026-10-08T21:43Z failed, so success criterion 1 ("a push to `main` … produces a green run") hasn't been observed yet. And #462 is still open [VERIFIED: `gh issue view 462` → `state: OPEN`, 57 comments, `assignees: []`]. The noise came from the in-workflow `notify-on-failure` job, which runs `gh issue comment` on **every** failed run and has no close-on-success path [VERIFIED: `.github/workflows/docs-deploy.yml:204-225`].

The most important technical finding is DEPLOY-04. Starlight 0.38.3 computes `lastUpdated` at build time with `git log -- <srcDir>/content/docs`. In this repo that path is a gitignored symlink to `docs/src`, so git returns nothing, Starlight's catch swallows the error, and no page gets a date. This is independent of `fetch-depth`. The local `docs-astro/dist` (built 2026-09-07 with `lastUpdated: true`) has **zero** `<time datetime>` elements across 933 pages [VERIFIED: grep of dist]. The fix is to stop using Starlight's git lookup and set `starlightRoute.lastUpdated` in the existing `routeData.ts` middleware. The value comes from the existing `src/utils/git-dates.mjs` map, which already queries `docs/src` correctly from the repo root. Extend that map with a mechanical-commit ignore list.

HONEST-01/02 fit Starlight's route-data middleware and the existing frontmatter-driven `sidebar.mjs` without touching 131 files or the `VALID_CATEGORIES` SSOT. The one hard part is llms.txt: `starlight-llms-txt@0.10.0` applies `exclude` only to `llms-small.txt`, and `llms-full.txt` filters only `draft`. The minimal, lockfile-safe fix is a `pnpm patch` that passes `exclude` to the full route and to custom sets. Upgrading is not an option (0.12 requires astro 7 / Starlight ≥0.41).

**Primary recommendation:** Build one shared `docs-astro/src/utils/page-status.mjs` (status → banner text, noindex, internals?) and have sidebar, route middleware, llms exclude list and tests all consume it. Fix dates in middleware rather than via `fetch-depth` alone. Replace comment-per-failure with an edit-in-place issue that closes on success. Add a "last success age" check to `ci-liveness.yml`.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Deploy + failure issue + close-on-success | CI (`docs-deploy.yml`) | GitHub Issues | Only the workflow knows its own conclusion |
| N-day "no successful deploy" alert | CI (`ci-liveness.yml`, scheduled) | `vox ci status` hooks (reads `nightly-failure` label) | Dead-man's switch must run when the deploy *doesn't* |
| Live smoke | CI job after CF deploy (Playwright, `docs-astro/tests`) | — | Must hit the deployed site, not `dist` |
| Per-page dates | Build (`git-dates.mjs` → `routeData.ts`) | CI checkout depth | Computed once at build from git |
| Banner / noindex / Internals grouping | Build (Starlight route middleware + `sidebar.mjs`) | Frontmatter (`status`) | Frontmatter is the single input |
| llms.txt exclusion | Build (starlight-llms-txt config + pnpm patch) | Hand file `docs/src/.well-known/llms*.txt` | Two publishers today; both must comply |
| Superseded marking | Docs (frontmatter + blockquote) | doc-pipeline lint | Content convention, no code |

## Standard Stack (all already installed — verify, don't add)

| Component | Version | Purpose | Source |
|-----------|---------|---------|--------|
| `@astrojs/starlight` | 0.38.3 | site, `routeMiddleware`, banner, sitemap | [VERIFIED: docs-astro/node_modules/@astrojs/starlight/package.json] |
| `astro` | 6.1.9 | SSG | [VERIFIED: node_modules/astro/package.json] |
| `starlight-llms-txt` | 0.10.0 (exact pin) | `/llms.txt`, `/llms-full.txt`, `/llms-small.txt`, `/_llms-txt/<slug>.txt` | [VERIFIED: package.json `"starlight-llms-txt": "0.10.0"`] |
| `@playwright/test` | ^1.63.0 | smoke + build-output tests | [VERIFIED: package.json devDependencies] |
| `gray-matter` | ^4.0.3 | frontmatter; exposes `matter.engines.yaml.parse` (js-yaml) → parse `retired-symbols.v1.yaml` **without a new dep** | [VERIFIED: node_modules/gray-matter/index.js:137 `matter.engines = engines;`; lib/engines.js `const yaml = require('js-yaml');`] |
| pnpm | CI pins 11 (`pnpm/action-setup@v6 version: 11`); local 12.9.1 | `pnpm patch` / `patchedDependencies` | [VERIFIED: docs-deploy.yml:42-44; `pnpm --version`] |
| `node:test` | built into Node 24 | pure-logic unit tests (date-map parser, link-checker) with zero deps | [ASSUMED: Node built-in; docs-astro has no vitest] |

**One recommended pin (security, see Security Domain):** `wrangler` is not a dependency. `npx wrangler pages deploy` fetches the *latest* wrangler at deploy time, with `CLOUDFLARE_API_TOKEN` in its env [VERIFIED: docs-deploy.yml:166-169; `docs-astro/package.json` has no wrangler; `node_modules/.bin` has 0 wrangler]. Pin it as a devDependency at a version at least 7 days old.

## Package Legitimacy Audit

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| wrangler | npm | mature (latest 4.149.0 published 2026-10-08) | 25.8M/wk | github.com/cloudflare/workers-sdk | [SUS] reason `too-new` (latest *version* only) | Approved as a pin. Choose a ≥7-day-old version; planner adds `checkpoint:human-verify` on the exact version |
| yaml | npm | mature | 176M/wk | github.com/eemeli/yaml | [SUS] `too-new` (latest version) | **Not needed.** Use gray-matter's bundled js-yaml instead |

**Packages removed due to [SLOP]:** none. **Flagged [SUS]:** wrangler (the version-recency signal only). No postinstall script on either [VERIFIED: `gsd-tools package-legitimacy check`, `npm view wrangler scripts.postinstall` empty].

## Findings by Requirement

### DEPLOY-01 — deploy green, close #462

- Recent runs [VERIFIED: `gh run list --workflow docs-deploy.yml`]: 2026-10-09T01:17Z `workflow_dispatch` **success**; 2026-10-08T21:43Z `push` failure; 10-07, 10-04 (×3), 10-01 `push` failure/cancelled.
- Job wall time on the green run [VERIFIED: `gh run view`]: Build Documentation 13m16s (of which Build Docs Pipeline 7m54s, Build Starlight 2m16s); Deploy to Cloudflare Pages 13m42s (it **rebuilds everything** — duplicate Rust + Starlight build); smoke 1m13s. Both jobs are within the 30-min cap but are the largest jobs in the workflow.
- Root cause to record on #462. Two eras:
  - (a) 2026-05-12 → 2026-09-20: build-side breaks (pnpm 11 `ERR_PNPM_IGNORED_BUILDS`, documented in `docs-astro/pnpm-workspace.yaml`; `starlight-llms-txt` 0.11 → astro 7 peer break `e32f81651`) [CITED: research/PITFALLS.md §0, pnpm-workspace.yaml header comment].
  - (b) 2026-09-20 → 2026-10-08: Cloudflare `Authentication error [code: 10000]` on `/accounts/***/pages/projects/vox-docs` → expired or under-scoped `CF_API_TOKEN` [CITED: CONTEXT.md "Confirmed defects"]. Fixed by the user rotating the token (`Pages:Edit` on `vox-docs`).
- **Plan shape:** wait for (or trigger, by merging any Phase 20 docs change) a green **push** run. Then `gh issue close 462 --comment "<root cause + fixing run URL>"`. If DEPLOY-02's close-on-success job lands first, it closes #462 automatically, but only if #462 still carries the `docs-deploy-broken` label (it does [VERIFIED]).

### DEPLOY-02 — escalate once + liveness

**What exists** [VERIFIED: docs-deploy.yml:200-225]: job `notify-on-failure`, `needs: [build-docs, deploy-pages, deploy-cloudflare, smoke-test]`, `if: failure()`, `permissions: issues: write`. It finds an open issue labelled `docs-deploy-broken` and **comments on it** (`gh issue comment "$existing"`), otherwise creates "docs-deploy is failing on main". It has no assignee, no close on success, and never escalates. That explains #462: 57 comments, and it was never closed.

**Shared mechanisms to reuse (no composite action exists):**
- `nightly-report.yml` [VERIFIED: :1-66]: `workflow_run` on scheduled workflows, label `nightly-failure`, title `Nightly failing: <name>`. It closes the issue on success but **comments on every repeat failure** (same anti-pattern). Its job only runs when `github.event.workflow_run.event == 'schedule'`.
- `ci-liveness.yml` [VERIFIED: :1-80]: a daily cron that, for each workflow named in nightly-report's `on.workflow_run.workflows`, requires a **scheduled** run within 2×cadence (2 or 14 days). Otherwise it opens `Nightly stale: <name>` (label `nightly-failure`, no comment spam) and closes it when a run lands. It needs `on.schedule[0].cron` and errors if a listed workflow has no schedule. Permissions: `contents: read, actions: read, issues: write`.
- `vox ci status` (git pre-commit/pre-push and Claude hooks) lists **open `nightly-failure` issues** to every agent session [VERIFIED: crates/vox-cli/src/commands/ci/status.rs:356-366 `"--label", "nightly-failure"`]. That makes the label a real escalation channel.
- `workflow_policy_guard` (inside `ssot-drift`) **forces any `schedule`-triggered workflow to be listed in nightly-report.yml** [VERIFIED: crates/vox-cli-ci/src/workflow_policy_guard.rs:10-12, `missing_from_report`].

**Recommendation (minimal, no new script files; inline workflow bash matches the existing pattern):**
1. Rewrite `notify-on-failure` to **edit in place**: `gh issue edit <n> --body` with the latest run URL, first-failure timestamp and consecutive-failure count. Never `gh issue comment` on repeats. On create, use `--assignee <owner>` (assignment notifies the owner) plus labels `docs-deploy-broken,nightly-failure`. The second label is the one-time escalation into every agent's `vox ci status`. Use a title distinct from `Nightly failing:` / `Nightly stale:`.
2. Add a `notify-on-success` job (`if: success()`, same `needs`) that closes any open `docs-deploy-broken` issue with **one** comment ("Recovered: <run>").
3. Liveness: add a **second, explicit list** to `ci-liveness.yml`, e.g. `Documentation:14` (name:max-days). For each entry check `gh run list --workflow docs-deploy.yml --branch main --status success --limit 1 --json createdAt`. If it's older than N days, open `Deploy stale: Documentation` (label `nightly-failure`); close it when a newer success lands.
   - Do **not** add a `schedule:` to docs-deploy to "join" the existing loop. Rule 3 would force it into nightly-report, and nightly-report would then open a *second*, comment-spamming issue stream for scheduled failures.
   - N = 14 is safe because `docs/**` changes almost daily (255 docs commits in the last 60 days [CITED: CONTEXT.md baseline]).
4. Forced-failure verification: add a `workflow_dispatch` boolean input (e.g. `simulate_failure`) that fails a trivial step *before* deploy. Dispatch it twice, then assert exactly one open issue and no new comments.

### DEPLOY-03 — live smoke

**What exists** [VERIFIED: docs-astro/tests/smoke.spec.ts:1-78, run by job `smoke-test` with `BASE_URL=https://voxlang.org` after `sleep 30`; playwright.config `retries: 2` in CI]:
- home 200 + title
- homepage free of `@endpoint`, `@table type`, `@mcp.tool`
- `/reference/stability/` 200
- `/llms.txt` 200 + content
- `/.well-known/llms.txt` 200 + content
- sidebar labels
- pagefind button
- www / vox-lang.org redirects

**Gaps vs requirement:**
- No `/voxup` test. Live `/voxup` currently returns 200 `application/octet-stream` and the body starts with `#!/bin/sh` [VERIFIED: curl]. Assert 200 **and** that the body starts with `#!`, so an HTML 404 page piped to `sh` fails. Do the same for `/voxup.ps1`.
- No llms URL walk. Root `/llms.txt` (plugin) lists only the set links (`llms-small.txt`, `llms-full.txt`, `_llms-txt/*.txt`) [VERIFIED: node_modules/starlight-llms-txt/llms.txt.ts:16-34]. The hand files `.well-known/llms.txt` + `.well-known/llms-full.txt` hold 23 unique `https://voxlang.org/...` URLs. **3 of them 404 today**: `/AGENTS.md`, `/CLAUDE.md`, `/GEMINI.md` (in llms-full.txt) [VERIFIED: curl loop]. A correct new check fails on day one, so the plan must fix those links in the same change (point them at `github.com/vox-foundation/vox/blob/main/AGENTS.md`, or drop them).
- Retired syntax is checked only on `/` and only for 3 hard-coded strings. Load patterns from `contracts/documentation/retired-symbols.v1.yaml` (22 `pattern:` regexes, all JS-compatible: `\b`, `(?:…)`, no inline flags [VERIFIED: file lines 9-102]) via `matter.engines.yaml.parse`. Keep the 3 `@endpoint`-family literals; they are **not** in the contract.
  - Sample = reader-facing pages linked from `.well-known/llms.txt` + home. Skip pages that legitimately quote retired names (e.g. `reference/stability/`, anything whose `status` is not `current`). The repo-side check `vox-cli-ci/src/retired_symbol_check.rs` already carves out historical/audit docs (`is_historical_or_audit_doc`) and table first-cells [VERIFIED: :75-160].
- No reusable live-URL `vox ci` command exists: no `llms` reference in `crates/vox-cli-ci/src`, `crates/vox-cli/src/commands/ci`, `crates/vox-doc-pipeline/src` [VERIFIED: grep]. `vox ci check-links` covers internal Markdown links only. **Recommendation:** extend the TypeScript Playwright spec. It is test code, not glue automation, so VoxScript-first doesn't apply, and it avoids a 13-min cargo build in the smoke job.
- "A deliberately broken llms.txt link must fail it": put the link walk in a small pure helper (`tests/lib/llms-links.ts`: parse URLs → fetch → return failures). Cover it with a test that feeds a fixture containing `https://voxlang.org/__definitely-missing__/` and expects one failure.

### DEPLOY-04 — correct per-page dates

**How Starlight does it** [VERIFIED: node_modules/@astrojs/starlight]:
- `integrations/virtual-user-config.ts:60-93`: at build, it inlines `getAllNewestCommitDate(rootPath, docsPath)` with `docsPath = resolveCollectionPath('docs', srcDir)` = `docs-astro/src/content/docs`.
- `utils/git.ts:58-121`: runs `git log --format=t:%ct --name-status -- <docsPath>` from the repo root and keys results by `relative(rootPath, repoRoot/file)`.
- `utils/routing/data.ts:78-94`: `getLastUpdated` uses frontmatter `lastUpdated` if it is a Date, else `getNewestCommitDate(entry.filePath)`, inside `try { } catch { return undefined }`.
- `components/LastUpdated.astro`: renders whenever `starlightRoute.lastUpdated` is truthy.

**Why it fails here:**
- `docs-astro/src/content/docs` is a symlink (`→ /…/docs/src`, created by `scripts/setup-content.mjs`) and is gitignored (`.gitignore:20`). `git log -- docs-astro/src/content/docs` returns empty with exit 0 [VERIFIED: ran locally].
- Even if it returned results, the keys (`../docs/src/x.md`) would never match `entry.filePath` (`src/content/docs/x.md`).
- Proof: 0/933 dist pages have `<time datetime` [VERIFIED].
- So `fetch-depth: 0` is **necessary but not sufficient**. Today CI is doubly broken (shallow clone + symlink); locally it's broken by the symlink alone.

**Existing correct building block:** `docs-astro/src/utils/git-dates.mjs` (`getGitDates()`) runs `git log --format=C|%cI --name-only -- docs/src` from `git rev-parse --show-toplevel` and returns `Map<"path/without-ext", ISO date>` (newest-first, first sighting wins) [VERIFIED: :20-46]. Today only `src/pages/feed.xml.ts` uses it. Run cost locally: 0.25 s [VERIFIED: `time git log`].

**Recommended design:**
1. Extend `git-dates.mjs` to `getGitDates({ ignore })`:
   - Switch to `--name-status -M` so renames are tracked. Walking newest → oldest, an `R old new` entry aliases `old → new`, so an ignored bulk-rename commit doesn't orphan a page's older history.
   - Skip commits whose sha is listed in a root **`.git-blame-ignore-revs`** (doesn't exist yet [VERIFIED: `ls`]). That is the standard format, and GitHub's blame UI honours it too.
   - Also skip commits whose subject matches the ssot-autoregen bot message `chore(ssot): auto-regenerate` [VERIFIED: ci.yml:399].
   - Optional threshold heuristic: a commit touching ≥50 `docs/src` files counts as mechanical. Measured bulk commits: `014a41d01` (743 files, CF migration + sidebar category overhaul), `4e98a0f87` (337, strip `last_updated`), `ce22efcf0`/`c2c1b9f2c` (95 each, clavis→secrets rename), `3456cc901` (60, stale crate names) [VERIFIED: git log]. Conventional-commit prefixes are **not** a reliable signal: `fix(docs):`, `docs:`, `chore:` are mixed real/mechanical.
2. In `src/routeData.ts`, memoise the map at module level. Then set `context.locals.starlightRoute.lastUpdated = new Date(map.get(key))`, where `key` is `entry.filePath` minus the `src/content/docs/` prefix and extension. Do **not** use `starlightRoute.id`, because ids are slugified: `architecture/qwen-3.7-profile…` → `architecture/qwen-37-profile…` [VERIFIED: dist dir name].
3. Set `lastUpdated: false` in `astro.config.mjs`. The middleware becomes the only source, and Starlight's broken lookup is skipped. `LastUpdated.astro` still renders whatever the middleware sets.
4. Have `feed.xml.ts` use the same filtered map, so RSS and page footers agree.
5. CI: in **every job that runs `pnpm build`** (`build-docs` and `deploy-cloudflare` in docs-deploy.yml, plus the Starlight build in docs-quality.yml if its output is asserted) add `with: fetch-depth: 0`. Repo pack is 421 MiB, 6,397 commits [VERIFIED: `git count-objects`, `rev-list --count`]. Prefer `fetch-depth: 0` + `filter: blob:none`: `git log --name-status` needs only commits and trees [ASSUMED: actions/checkout `filter` input; verify against the v7 README before use]. Simpler alternative: make `deploy-cloudflare` download the `build-docs` output instead of rebuilding. It saves ~13 min, leaves one job needing full history, and the unused `docs.tar.gz` "Archive docs" step (docs-deploy.yml:93-94, never uploaded) becomes useful.
6. Build guard: fail the build-output test if fewer than ~90% of pages carry a `<time datetime>`. That catches a silent regression to shallow checkout.

### HONEST-01 / HONEST-02 — banners, noindex, Internals, llms exclusion

**Sidebar today** [VERIFIED: docs-astro/src/utils/sidebar.mjs:1-130]:
- It walks `docs/src` (skips `archive`, `.well-known`) and reads frontmatter with gray-matter.
- It groups by `category` in `SECTION_ORDER` from `contracts/documentation/docs-sidebar-section-order.v1.json`. That file has 12 sections; collapsed: ADRs, Architecture SSOTs, Contributors, CI & Quality, Operations [VERIFIED: json:1-24]. Unknown categories are appended collapsed.
- Missing `status` defaults to `current`.
- `STATUS_BADGE` already badges experimental, research, roadmap, deprecated and legacy.

`VALID_CATEGORIES` (lint.rs:19-35) is the same 12 labels plus `"archive"`. `VALID_STATUS` (lint.rs:37-45) = `"approved", "current", "experimental", "legacy", "research", "roadmap", "deprecated"` [VERIFIED].

**Corpus counts** (668 live `.md`, excluding archive/.well-known) [VERIFIED: /tmp/docs-count.cjs over gray-matter frontmatter]:

| status | count |
|---|---|
| current | 335 |
| (none → treated as current) | 189 |
| research | 85 |
| roadmap | 46 |
| deprecated | 7 |
| approved | 2 |
| experimental | 2 |
| legacy | 2 |

- Category "Architecture SSOTs" = 389 pages: 217 current, 81 research, 46 roadmap, 38 none, 3 deprecated, 2 approved, 2 experimental. 387 of them live in `architecture/`.
- research ∪ roadmap = **131**: 127 Architecture SSOTs + 1 each in ADRs, CI & Quality, Contributors, Operations.
- HONEST-01 banner set (research ∪ roadmap ∪ deprecated ∪ legacy) = **140**: 131 in `architecture/`, 4 `adr/`, 1 each in api, ci, contributors, operations, reference.
- (CONTEXT.md's "~383 current / ~200 research" baseline was approximate. Use these numbers.)

**Internals membership rule — options with counts:**

| Rule | Pages | Notes |
|---|---|---|
| **A (recommended):** `status ∈ {research, roadmap}` (any category) | 131 | Frontmatter-only, same input as HONEST-01, zero file edits, no `VALID_CATEGORIES` change |
| B: category "Architecture SSOTs" ∧ status ∈ {research, roadmap} | 127 | Leaves 4 research/roadmap pages in main nav |
| C: whole "Architecture SSOTs" category | 389 | Hides `where-things-live`, `classification-ssot-2026`, `cryptography-ssot-2026`, `telemetry-trust-ssot` — all listed in `.well-known/llms.txt` as `current`; conflicts with "excluded from every llms.txt variant" |

`deprecated` / `legacy` (9 pages): banner + noindex, but they stay in place under their category. They are reader-relevant migration notices.

**Implementation (no component overrides needed):**
- `src/utils/page-status.mjs`: the single map from `status` → `{ banner, noindex, internals }`. Banner wording follows the governance definitions [VERIFIED: docs/src/contributors/documentation-governance.md:68-74, e.g. `research` = "investigation, findings, or synthesis not equivalent to shipped behavior"].
- `sidebar.mjs`: route pages with `internals` into one `Internals` group, `collapsed: true`, appended **last** (after `SECTION_ORDER`). Internals is derived from status, so `category` stays as is, and lint/`VALID_CATEGORIES`/the section-order JSON need no change. If the planner instead makes "Internals" a real `category` value, then `VALID_CATEGORIES`, the JSON and governance §categories must all change together, plus 131 frontmatter edits. Not recommended.
- `routeData.ts`:
  - If `!entry.data.banner`, set `starlightRoute.entry.data.banner = { content }`. Starlight's `Banner.astro` reads exactly `Astro.locals.starlightRoute.entry.data.banner` and renders `set:html` with `data-pagefind-ignore` [VERIFIED: components/Banner.astro:1-5]. 0 pages set `banner:` today [VERIFIED: rg].
  - Push `{tag:'meta', attrs:{name:'robots', content:'noindex'}}` into `starlightRoute.head`. That is the existing archive pattern, and live archive pages show `<meta name="robots" content="noindex"/>` [VERIFIED: dist archive page].
- **Correct a false comment:** `routeData.ts` says the robots meta makes Pagefind skip the page. Starlight never emits robots meta. Pagefind indexing is controlled by `data-pagefind-body`, added only when `entry.data.pagefind !== false` [VERIFIED: components/Page.astro:36-47; no `robots` string anywhere in Starlight source]. The archive page in dist still has `data-pagefind-body` [VERIFIED]. If Internals or archive should leave site search, also set `starlightRoute.entry.data.pagefind = false`. D7 doesn't require it (Open Question 2).
- **Sitemap:** Starlight auto-adds `@astrojs/sitemap` unless one is already configured [VERIFIED: index.ts:101], so noindex pages are listed in `sitemap-index.xml`, which is a mixed signal. Recommended: register `@astrojs/sitemap` yourself with a `filter` that drops noindex pages. It is already a transitive dependency; adding it to `package.json` makes it direct.
- **robots.txt** (source `docs/src/robots.txt`, copied to `public/` by `setup-content.mjs:93-99`) [VERIFIED]:
  - It declares `Sitemap: https://voxlang.org/sitemap.xml`, which **404s**. The real file is `/sitemap-index.xml` (200) [VERIFIED: curl]. Fix it.
  - Its `Disallow:` rules are mdBook-era `.html` paths (dead) plus `/ci/` and `/operations/`.
  - **Do not add `Disallow` for Internals paths.** A disallowed URL is never crawled, so Google never sees its `noindex` and may still index the bare URL [ASSUMED: standard Google guidance].
- **llms.txt variants:**
  - Plugin root `/llms.txt`: lists sets only, no page URLs (fine).
  - `/llms-small.txt`: honours `exclude`.
  - `/llms-full.txt`: `generateLlmsTxt(context, {minify:false, description})` with **no exclude**. The generator only filters `!doc.data.draft` [VERIFIED: llms-full.txt.ts:12-17, generator.ts:30-36].
  - Custom sets: `include` only.
  - `draft: true` is not usable, because it removes the page from the production site entirely.
  - **Fix:** `pnpm patch starlight-llms-txt@0.10.0`, a one-line change in `llms-full.txt.ts` (and `llms-custom.txt.ts`) passing `exclude: starlightLllmsTxtContext.exclude`. Commit it via `patchedDependencies` (pnpm-workspace.yaml or package.json) so `--frozen-lockfile` stays reproducible. The plugin is MIT and small. Upstream 0.11 *changed* exclude semantics (CHANGELOG line 110, `26fa616`), and 0.12 needs `astro ^7`, `@astrojs/starlight >=0.41.0` [VERIFIED: `npm view starlight-llms-txt@latest peerDependencies`], so don't upgrade.
  - `exclude` matches micromatch globs against **`doc.id` (slugified)**. Build the list from the same frontmatter walk with Starlight's slugging, or derive it after collection load. The id≠path case `qwen-3.7` → `qwen-37` exists among the 131 [VERIFIED]. A build-output test must assert that no Internals page title appears in `llms-full.txt` / `llms-small.txt`.
  - `llmsFullTxt: true` in `astro.config.mjs` is **not an option in 0.10.0**: no occurrence in the plugin source, and the plugin always injects `/llms-full.txt` [VERIFIED: index.ts:20-40]. Remove the dead key.
- **Hand-committed agent files must comply too:**
  - `docs/src/.well-known/llms.txt` links `architecture/front-facing-honesty-audit-2026`, which is `status: research`. Under rule A it becomes Internals, so remove the link or re-status the page.
  - It also links `reference/mcp-tool-reference`, which is `legacy` (banner + noindex, but still listed for agents; decide).
  - All other 21 listed pages are current/none [VERIFIED: /tmp/docs-count.cjs].
- **Governance doc must change in the same PR:**
  - `documentation-governance.md:51` says research pages are "sidebar-listed via frontmatter". It should describe Internals instead.
  - `:92` still describes a `last_updated` field "derived from Git" for frontmatter, which is wrong. Dates come from git at build; hand-authoring is a lint error.

### HONEST-03 — one authoritative design

Located [VERIFIED: ls + sed]:
- `docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md`: frontmatter `status: "roadmap"`, `category: "architecture"` (non-canonical label); body `**Status:** approved for planning`.
- `docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md`: `status: "current"`, `category: "Architecture SSOTs"`.
- `docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md` (the companion plan; the roadmap's `*` glob covers it).
- `docs/src/contributors/docs-reality-audit-program.md`: frontmatter `status: "current"`, but the body says `**Status: dormant.**` at line 54. That is a frontmatter/body contradiction, and it is a **public** page (Contributors).

Repo convention for supersession [VERIFIED: e.g. `docs/superpowers/plans/2026-08-22-vox-dashboard-corpus-repair.md` header describes "a `status`/banner flip rather than a line-by-line rewrite"; several files use `status: deprecated` + a top blockquote `> **Superseded (YYYY-MM-DD).** …` / `> **Superseded by:** [link]`]. Minimal marking:
- Set `status: "deprecated"`.
- Add `> **Superseded (2026-10-xx) by the docs-freshness workstream** — see `.planning/workstreams/docs-freshness/REQUIREMENTS.md`; retained as a historical record.` (`.planning/workstreams/docs-freshness/*` is git-tracked [VERIFIED: `git ls-files`]).

For the audit program page, the research recommendation (SUMMARY §Contradictions 6) is to **reactivate** its taxonomy and schemas in Phase 21 (MEASURE-04). So mark it `merged into docs-freshness` with status kept honest: `roadmap`, or keep `current` with the dormant note moved into a banner. Do not deprecate it. `docs/superpowers/**` is not published on the site (content root is `docs/src` only), so for those three files the marking matters for agents and repo readers, not for voxlang.org.

## Common Pitfalls

1. **"fetch-depth: 0 fixes dates."** It doesn't: the symlink defeats Starlight's lookup regardless. Warning sign: dist has no `<time datetime` elements.
2. **A new smoke check that's red on day one.** 3 llms-full URLs 404 now. Fix the links in the same PR, or the deploy goes red and DEPLOY-02 fires immediately.
3. **Two issue streams.** Adding `schedule:` to docs-deploy pulls it into nightly-report (enforced by `workflow_policy_guard`), giving duplicate comment-spamming issues. Put liveness in ci-liveness as a "last success age" list instead.
4. **llms `exclude` keyed on paths instead of slugified ids.** It silently misses pages like `qwen-3.7…`. Assert on the generated output, not the config.
5. **`robots.txt` Disallow + noindex.** Disallowing hides the noindex from crawlers. Use noindex alone.
6. **Upgrading starlight-llms-txt for the exclude feature.** That breaks the build (astro 7 peer), the same failure class as `e32f81651`. Use `pnpm patch`.
7. **Rename commits in the ignore list orphan history.** Track `R old new` with `-M` when walking the log.
8. **Retired-symbol scan on pages that legitimately quote retired names.** Sample only `current` reader pages from llms.txt; mirror `retired_symbol_check.rs` carve-outs.
9. **Middleware ordering.** Set the banner only when frontmatter has none, so an author-specified banner wins.
10. **`vox-gui`-style worktree gotcha does not apply.** But docs-astro needs `pnpm install` + `setup-content.mjs` (prebuild) before any local build/test.

## Code Examples

Route middleware (the documented Starlight route-data pattern; field names verified against installed source):

```ts
// docs-astro/src/routeData.ts (sketch)
import { defineRouteMiddleware } from '@astrojs/starlight/route-data';
import { getGitDates } from './utils/git-dates.mjs';
import { pageStatus } from './utils/page-status.mjs';
let dates: Map<string, string> | undefined;
export const onRequest = defineRouteMiddleware((context) => {
  const route = context.locals.starlightRoute;
  dates ??= getGitDates();
  const key = route.entry.filePath.replace(/^src\/content\/docs\//, '').replace(/\.mdx?$/, '');
  const iso = dates.get(key);
  if (iso) route.lastUpdated = new Date(iso);
  const s = pageStatus(route.entry.data.status);
  if (s.banner && !route.entry.data.banner) route.entry.data.banner = { content: s.banner };
  if (s.noindex || route.id.startsWith('archive/') || route.id === 'summary')
    route.head.push({ tag: 'meta', attrs: { name: 'robots', content: 'noindex' } });
});
```

pnpm patch flow:

```bash
cd docs-astro && pnpm patch starlight-llms-txt@0.10.0   # edit llms-full.txt.ts + llms-custom.txt.ts: pass exclude
pnpm patch-commit <printed-dir>                          # writes patches/ + patchedDependencies; lockfile updated
```

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | `actions/checkout@v7` supports `filter: blob:none` with `fetch-depth: 0` | DEPLOY-04 | Fall back to plain `fetch-depth: 0` (+~420 MiB download) |
| A2 | Google ignores noindex on robots-disallowed URLs | HONEST | Low; recommendation (don't Disallow) is safe either way |
| A3 | `pnpm patch` / `patchedDependencies` behaves identically under CI pnpm 11 and local 12 | HONEST-02 | Verify `pnpm install --frozen-lockfile` with pnpm 11 in CI |
| A4 | Alert owner/assignee is `brbrainerd` | DEPLOY-02 | Needs user confirmation |
| A5 | `node --test` acceptable for docs-astro unit tests (no vitest) | Validation | Alternative: Playwright runner without browser |

## Open Questions

> **Status 2026-10-08:** questions 1–5 below are **resolved** by 20-CONTEXT.md (P20-D1 → rule A; P20-D2 → keep searchable; P20-D3/D4 → 14 days, @brbrainerd; P20-D5 → drop both links; P20-D6 → audit page `roadmap`). They are kept for traceability only. Still open: 6 (drop `archive/` from build), 7 (mounted-docs route prefix), 8 (`verified_against` key).

1. **Internals rule:** status-only (A, 131 pages) vs category-scoped (B, 127)? Recommendation: A. Needs user confirmation because it moves 4 non-architecture pages.
2. **Site search for Internals:** keep in Pagefind (default) or set `pagefind = false`? D7 only names search engines. Recommendation: keep searchable, but confirm.
3. **Alert assignee and N:** who gets assigned, and the liveness window (recommend 14 days)?
4. **`front-facing-honesty-audit-2026` and `mcp-tool-reference` in `.well-known/llms.txt`:** drop the links or re-status the pages?
5. **Reality-audit page status:** `roadmap` vs `current` with a dormant banner, until Phase 21 reactivates it.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| node | astro build, tests | ✓ | 24 in CI | — |
| pnpm | install/patch | ✓ | 12.9.1 local / 11 CI | — |
| gh (authenticated, admin on vox-foundation/vox) | close #462, verify runs/issues | ✓ | — | — |
| git full history | dates | ✓ local (6,397 commits) | — | — |
| Playwright Chromium | smoke | CI installs (`npx playwright install chromium --with-deps`) | ^1.63 | `request`-only tests need no browser |
| Cloudflare token | deploy | ✓ (rotated by user; green run 37869058499) | — | — |

## Addendum (2026-10-08): Working links (LINKS-01..03) & Tutorials current (TUT-01)

Requirements (REQUIREMENTS.md:40-46, verbatim):
- **LINKS-01**: "Relative links between docs pages (`foo.md`, `../x/bar.md#anchor`, `index.md` / `README.md`) render as working site routes. They are rewritten at build time (remark plugin in `docs-astro/src/plugins/`) to Starlight slugs, anchors preserved; source Markdown stays repo-relative so GitHub rendering keeps working."
- **LINKS-02**: "Links that leave `docs/src/` resolve. Repo Markdown files (e.g. `AGENTS.md`, `LANGUAGE_DESIGN_PRIORITIES.md`, `README.md`, `CONTRIBUTING.md`, crate READMEs) are rendered as doc pages on the site from a single source (no hand copies). Non-Markdown repo files (source, contracts, `Cargo.toml`) link to their GitHub blob on `main`."
- **LINKS-03**: "The built site fails CI on any broken internal link (blocking check over `dist/`), including a deliberately broken fixture. External links are checked nightly, cached, and non-blocking."
- **TUT-01**: "Every tutorial (7 today) is audited and corrected against current code. … Each tutorial records the commit it was verified against."

### Measured link census

Method: `/tmp/link-census.mjs` walks the 668 live pages (archive and `.well-known` excluded), strips fenced and inline code, and classifies `](…)` targets. My counts differ slightly from the brief's 2,803 / 438 / 1,909, which probably used a different fence/archive filter. Both measurements agree on the order of magnitude. [VERIFIED: script output this session]

| Class | Count |
|---|---|
| Relative `.md` links that stay inside `docs/src` | **2,498** on **421** pages |
| …target missing on disk (already broken in the repo too) | 9 (e.g. `architecture/vox-as-llm-target-audit-and-plan-2026.md` → `AGENTS.md`, `docs/src/reference/cli-command-surface.generated.md`: wrong relative depth) |
| …slug ≠ path (needs slugification) | 19 |
| …targets `index.md` / `README.md` | 6 |
| …with `#anchor` / anchor not found among the target's github-slugger heading ids | 44 / 2 |
| Relative links to directories inside docs/src | 10 |
| Links that **leave** `docs/src` | **2,003**: `crates/` 1,316, `contracts/` 191, `AGENTS.md` 123, `docs/` (superpowers, agents) 116, `examples/` 43, `.github/` 31, `scripts/` 27, `LANGUAGE_DESIGN_PRIORITIES.md` 23, … |
| …to `.md` files | 294 links → **81 distinct files** (1 missing: `crates/_frozen.md`) |
| …to non-`.md` files | 1,709 (47 targets missing on disk) |

Live confirmation [VERIFIED: curl]:
- `/tutorials/tut-getting-started/` emits `href="../reference/installation.md"`, `href="tut-first-app.md"`, etc. The resolved `/tutorials/reference/installation.md` returns **404**, while `/reference/installation/` returns 200.
- Even a sibling link like `tut-first-app.md` breaks, because the page URL is a directory.

Why nobody noticed:
- `link_checker.yml` (nightly "Docs Link Checker") runs lychee over **source** `'**/*.md'` with `--root-dir .`, where relative paths resolve correctly [VERIFIED: link_checker.yml:28-34].
- The `dist/` lychee step in docs-deploy.yml is `continue-on-error: true` + `fail: false` [VERIFIED: docs-deploy.yml:78-90].
- `vox ci check-links` also checks source Markdown only.

### LINKS-01 — build-time rewrite

**Starlight's slug function** (what the rewrite must reproduce) [VERIFIED]:
- Starlight `docsLoader` is Astro's `glob()` with `base` = `src/content/docs` and `` pattern: `**/[^_]*.{${extensions.join(',')}}` `` (node_modules/@astrojs/starlight/loaders.ts:42-58). It has **no `exclude` option**: its only parameter is `{ generateId }` (loaders.ts:15-23).
- Default id: `if (data.slug) { return data.slug; }`, else `getContentEntryIdAndSlug(...).slug` (node_modules/astro/dist/content/loaders/glob.js:9-28). That function computes `rawSlugSegments.map((segment) => githubSlug(segment)).join("/").replace(/\/index$/, "")` with `import { slug as githubSlug } from "github-slugger"` (astro/dist/content/utils.js:5, 264-278).
- Consequences:
  - Lowercase, punctuation stripped per segment (`qwen-3.7…` → `qwen-37…`, confirmed by the dist dir name).
  - `index` collapses to its directory.
  - **`README` does not collapse**: `adr/README.md` → `/adr/readme/`. `adr/README.md` and `adr/index.md` both exist as distinct pages.
  - 0 pages set `slug:` [VERIFIED: rg].
  - The installed copy is `github-slugger@2.0.0` (node_modules/.pnpm).
- Anchors: GitHub and Astro heading ids are both github-slugger. Fragments written for GitHub should carry over unchanged; only 2 of 44 anchored links miss a heading [VERIFIED: census]. [ASSUMED: Astro's heading-id plugin uses github-slugger with per-document dedupe; the census emulated that.]

**Existing packages** [VERIFIED: `npm view` + `gsd-tools package-legitimacy check`]:

| Package | Version | Peer deps | Legitimacy | Fit |
|---|---|---|---|---|
| `astro-rehype-relative-markdown-links` | 0.19.2 | `{ astro: '>=2 <8' }` (compatible) | OK, 4,670/wk, github.com/vernak2539/astro-rehype-relative-markdown-links | Rewrites in-collection `.md` → slug using github-slugger. It resolves from the vfile path **inside `src/content/docs`** (the symlink path), so links escaping `docs/src` resolve against `docs-astro/` (wrong). No GitHub-blob or repo-mount mapping, so LINKS-02 still needs custom code. [ASSUMED: behaviour inferred from its README/dependency list (`github-slugger`, `gray-matter`, `unist-util-visit`); source not read] |
| `starlight-links-validator` | **0.24.0** is the newest compatible (`{"astro":">=6.0.0","@astrojs/starlight":">=0.38.0"}`); 0.25.0 needs `astro >=7.0.2`, Starlight `>=0.41.0`; latest 0.26.0 needs Starlight `>=0.42.0` | — | OK, 133k/wk, github.com/HiDeoo/starlight-links-validator | A validator, not a rewriter. It fails the build on broken internal links/anchors, but would flag every relative link until a rewrite exists, and its ordering relative to our remark plugin is unverified. [ASSUMED] Optional, not needed if lychee-offline is adopted (below). |

**Recommendation: a single custom remark plugin `docs-astro/src/plugins/remark-doc-links.mjs`** (~120 lines), registered beside `remarkVoxInclude` in `astro.config.mjs` `markdown.remarkPlugins`. Declare `github-slugger` directly in package.json; it is already a transitive dependency, so it's not a new package. Reasons: LINKS-02's repo-mount table and GitHub-blob fallback exist in no package, the symlink forces realpath resolution, and owning ~120 lines beats two plugins plus a patch.

Algorithm:
1. `src = fs.realpathSync(file.path)`. This handles the `src/content/docs → docs/src` symlink and any mounted file symlinks.
2. For each `link` / `definition` node with a relative URL (not `/`, `#`, or a scheme): `target = path.resolve(dirname(src), decodeURI(pathPart))`.
3. If `target` is a `.md`/`.mdx` file under `docs/src`, compute the route: `relative(docs/src, target)` without the extension, `githubSlug` each segment, strip `/index`. Emit `/<slug>/` plus `#anchor`. That's site-absolute with a trailing slash, so it's independent of page depth.
4. Else if `target` is in the mounted-repo table (LINKS-02), emit its route.
5. Else if `target` is inside the repo, emit `https://github.com/vox-foundation/vox/blob/main/<repo-rel>` (`/tree/main/` for directories).
6. If an in-docs `.md` target **doesn't exist**, throw with file:line. This is the strict build-time half of LINKS-03, so the 9 + 1 + 47 already-dead targets must be fixed in the same change.

Put the slug function in a shared `src/utils/doc-slug.mjs`, used by this plugin, the llms `exclude` list (HONEST-02), and tests. One slug implementation, no copies. Source Markdown stays repo-relative, so GitHub rendering is unchanged.

### LINKS-02 — rendering repo Markdown from one source

**`docs/src/AGENTS.md` is NOT a copy of root `AGENTS.md`.** `diff -q` reports that they differ [VERIFIED]. `docs/src/AGENTS.md` has 27 lines and heading "Documentation Rules (docs/src/ scope)"; root `AGENTS.md` has 698 lines. The docs/src file is a nested, directory-scoped agent-rules file, published at `/agents/` (live 200) with the misleading frontmatter `title: "AGENTS.md"`, `category: "Architecture SSOTs"`. Nothing syncs the two, and nothing should. Recommendations:
- Exclude it from the site; it's agent instructions, not reader content.
- Mount root `AGENTS.md` at a distinct route (e.g. `/repo/agents/`) to avoid the `/agents/` collision.
- Add `_redirects` `/agents/ /repo/agents/ 301`.

**Mount mechanism (recommended): `setup-content.mjs` builds `docs-astro/src/content/docs` as a real, gitignored "mirror" directory instead of a single symlink.** Today it is one dir symlink to `docs/src` (setup-content.mjs:18-49). The mirror holds:
- one symlink per top-level `docs/src` entry, except `archive/`, `.well-known/`, `SUMMARY.md` and the scoped `AGENTS.md`;
- a `repo/` subdirectory of **file symlinks** to allow-listed repo Markdown.

Evidence it works:
- Astro's glob loader calls `tinyglobby(globOptions.pattern, { cwd, expandDirectories: false })` (astro/dist/content/loaders/glob.js:7, 176-179).
- With the installed `tinyglobby@0.2.17`, pattern `'**/[^_]*.{md,mdx}'` returned `[ 'sub/linked.md', 'dirlink/a.md' ]` for a file symlink and a dir symlink [VERIFIED: /tmp/tg-test.mjs].

This also fixes a defect found in passing. **`content.config.ts:9-11` passes `exclude: ['archive/**', 'SUMMARY.md', '.well-known/**']` to `docsLoader`, which silently ignores it** (loaders.ts:15-23), so the tombstoned archive is built and published. Live `/archive/research-2026-q1/automated-testing-research-2026/` → 200 [VERIFIED: curl]. Dropping archive from the build changes public URLs, so it needs a user decision (Open Question 6). The alternative is to keep archive and exclude it from link checks.

Rejected alternatives:
- A second collection: Starlight renders only `docs`.
- Two glob loaders feeding one collection: each glob load prunes entries it didn't see [ASSUMED].
- A `generateId` remap over a repo-root glob: it changes every `filePath` and breaks editLink/date keys.

Required companions:
- Route middleware overrides `editUrl` for `repo/*` entries (default editLink base `https://github.com/vox-foundation/vox/edit/main/docs/src/`, astro.config.mjs:21-23).
- The DEPLOY-04 date map is keyed by **realpath relative to the repo root**, and `git log` includes the mounted paths.
- `sidebar.mjs` walks `docs/src` only, so mounted pages are off the sidebar but reachable by link.

**Which files qualify** (81 distinct escaping `.md` targets) [VERIFIED: /tmp/esc-fm.mjs + header reads]:
- **Mount: frontmatter with `title`** (required by `docsSchema`). Confirmed:
  - root `AGENTS.md` (`title: "AGENTS.md"`)
  - `CLAUDE.md` (`title: "Claude Code Overlay"`)
  - `docs/agents/governance.md` (`title: "Governance"`)
  - `GEMINI.md`, `LANGUAGE_DESIGN_PRIORITIES.md` and `CONTRIBUTING.md` begin with `---`, but `title:` wasn't confirmed [ASSUMED].
  - Their `category: "contributor"` is outside `VALID_CATEGORIES`. That's harmless because vox-doc-pipeline lints only docs/src.
- **GitHub blob unless given frontmatter:** `README.md` (starts with `<!--`), `examples/{STYLE,PARSE_STATUS}.md`, `examples/compile-suite/README.md`, `contracts/README.md`, `contracts/speech-to-code/*.md`, `tree-sitter-vox/{GRAMMAR_SSOT,README}.md`, `docs-astro/README.md`, `crates/vox-tauri-stt/README.md`, `apps/editor/vox-vscode/README.md`, `scripts/**/README.md`, `docs/ci/build-timings/README.md`, other `docs/agents/*.md`. Root `README.md` should stay blob-linked, because frontmatter would render as a table on GitHub.
- **Never mount (GitHub blob):** `docs/superpowers/{specs,plans}/**`, 54 files / 81 links. These are internal planning; per D7 they stay off the public site.

Encode the allowlist once, e.g. `contracts/documentation/site-mounted-repo-docs.v1.json` (`x-vox-version: 1`, entries `{ path, route }`), read by both `setup-content.mjs` and the remark plugin.

**llms-full.txt tie-in:** the 404s `https://voxlang.org/AGENTS.md`, `/CLAUDE.md`, `/GEMINI.md` in `docs/src/.well-known/llms-full.txt` (DEPLOY-03) become `/repo/agents/` etc. once mounted. One fix serves both LINKS-02 and DEPLOY-03.

### LINKS-03 — blocking internal, nightly external

- **PR-time, blocking (docs-quality.yml, after "Build Starlight"):** run lychee **offline** over the rendered site: `--offline --root-dir docs-astro/dist --include-fragments 'docs-astro/dist/**/*.html'` with `fail: true`.
  - Reuses `lycheeverse/lychee-action@v2`, already in two workflows. No new dependency.
  - It checks real output, so it also catches rewrite bugs, component links and `_redirects` gaps.
  - Review `.lycheeignore` (223 lines) first. [ASSUMED: flag names/behaviour of `--offline`/`--include-fragments` in the action's bundled lychee; verify `lychee --help` in CI.]
- **Build-time strictness:** the remark plugin throws on a missing in-docs target. This is the earliest and cheapest layer.
- **Deliberately broken fixture**, using either or both:
  - (a) `node --test` unit: feed the plugin a fixture tree containing a link to a missing page and expect a throw;
  - (b) a CI step: copy `docs-astro/tests/fixtures/broken-link/index.html` (`href="/definitely-missing/"`) into a temp copy of dist, run the same lychee command, assert a **non-zero** exit. Guard it with a before/after presence check (AGENTS.md "verify guards by mutation").
- **Nightly external:** reuse `link_checker.yml` (scheduled; already in nightly-report.yml, so failures become one `nightly-failure` issue).
  - Add `--cache --max-cache-age 3d` with `actions/cache` on `.lycheecache`. Scheduled runs are on `main`, so this satisfies the main-only cache rule (`vox ci cache-key-lint`).
  - Optionally switch its input from source `**/*.md` to built HTML.
- **Pre-work before blocking:** fix or relink the 9 broken in-docs targets, the 47 missing non-md repo targets, `crates/_frozen.md`, the 2 bad anchors, and the 10 directory links (→ `/tree/main/` or the section index). If archive stays built, `--exclude-path archive`.

### TUT-01 — tutorial punch list (audit only; nothing fixed)

Tools used: `/tmp/tut-audit.mjs` (registry from `contracts/cli/command-registry.yaml` parsed via gray-matter's YAML engine), manual reads, and the installed `vox 0.6.0+build.6382 (9a2575123)` (15 commits behind HEAD 6397, run with `VOX_SKIP_FRESHNESS_CHECK=1`).

**Doctest coverage:**
- `vox ci doctest-md --strict docs/src/tutorials` → `Doctests OK. Checked 7 files.` in ~0.1 s [VERIFIED].
- A probe with a syntax error failed (`Expected identifier`), so parse errors are caught [VERIFIED]. Type-checking wasn't established [ASSUMED: parse-level].
- Its green result **did not catch** the wrong include in tut-workflow-durability (below), because the included code compiles.

| Tutorial | Lines / vox fences / `vox:skip` | Stale items found |
|---|---|---|
| `tut-getting-started.md` (no `status`) | 138 / 2 / 0 | (1) **Scaffold drift:** the page explains `query get_notes()` + `mutation create_note(...)`, but `vox init my-app` now generates `table Note`, `server add_note(...)`, `server list_notes()`, `component App()`, `routes {` [VERIFIED: ran `vox init` in /tmp]. (2) Prereqs say **Node.js (20+)**; `reference/installation.md:23` says "Node.js >= 18". (3) "pnpm (9+)" vs CI pnpm 11. (4) Rust 1.98.1 matches installation.md:166 ✓. (5) `vox build src/main.vox -o dist` matches the init "Next steps" and `-o, --out-dir` ✓. (6) 7 relative links: present on disk, **all 404 on the site** (LINKS-01). |
| `tut-first-app.md` (current) | 87 / 4 / 0 | `vox init --kind application`: `kind.unwrap_or("application")` (init.rs:11), so the flag is redundant. Confirm the flag spelling with `--help`. Check the code against the current scaffold (same drift risk). 2 links 404 on the site. |
| `tut-actor-basics.md` (no `status`) | 102 / 3 / 0; include `ref_actors.vox:basic_actor` ✓ (anchor at line 13) | Link `../reference/cli.md#vox-run-file----args` must match the slug of heading `` ### `vox run <file> [-- <args>…]` `` (cli.md:135); verify. The "lowers actor constructs directly into … Rust primitives" claim is unverified. 3 links 404 on the site. |
| `tut-workflow-durability.md` (current) | 61 / 1 / 0; 1 include | (1) **Wrong include:** "Defining a Workflow" includes `getting_started.vox:logic`, which contains `query get_notes`, `mutation create_note`, `fn order`, and `@test fn test_order`. It has **no `workflow`/`activity`** and no `with` block, yet the next prose explains `retries` / `timeout` / `initial_backoff` [VERIFIED: getting_started.vox:35-57]. (2) The sentence "Use the bare `activity` and `workflow` keywords…" is duplicated. (3) `vox mens workflow` is in the registry ✓. 2 links 404 on the site. |
| `tut-ui-integration.md` (current) | 169 / 7 / 0 | Doctests pass. Verify `table Task { task_id: Id[Task] …}` against current table syntax. No CLI commands. 4 links 404 on the site. |
| `tut-first-vox-app-checkpoints.md` (no `status`) | 35 / 0 / 0 | "`vox run …` for script mode only when built with **`script-execution`**" is stale: `default = ["keyring-store", "script-execution"]` (crates/vox-cli/Cargo.toml:35). `vox populi serve` active ✓. Verify that `vox check --json` emits `category`. 6 links 404 on the site. |
| `use-a-react-component-from-vox.md` (current) | 114 / 2 / 3 | All 3 `vox:skip` carry reasons ✓ (lines 19, 35, 48). Confirm `examples/golden/react_interop.vox` exists and is tested. 3 links 404 on the site. |

**Registry check:**
- The scanner's "MISSING" hits (`vox table`, `vox query get`, `vox fn`, `vox routes`, `vox cargo install`) are false positives from prose and Vox source.
- Every real invocation found is `status: active`: `init`, `doctor`, `check`, `build`, `run`, `populi serve`, `mens workflow`.
- **The registry records `path` + `status`, not flags.** Flag verification needs `vox <cmd> --help` or `docs/src/reference/cli-command-surface.generated.md`.

**Cross-cutting:**
- 3 of 7 tutorials lack `status:`.
- None records a verified-against commit. The key (e.g. `verified_against: <sha>`) is a new authored frontmatter key, so define it once in `vox-doc-pipeline`, consistent with DRIFT-03, and keep it distinct from the banned hand-written `last_verified`. Planner should reconcile with D-decisions.

**Sizing:** 1 substantive rewrite (getting-started), 1 content fix (workflow include + prose), ~3–4 small edits (Node version, checkpoints caveat, flag checks, status frontmatter). The 27 tutorial links are fixed mechanically by LINKS-01.

### Addendum pitfalls
1. **Resolving from the symlink path:** escaping links resolve against `docs-astro/…`. Always `realpathSync` the source file.
2. **README is not index:** only `/index` collapses; `adr/README.md` → `/adr/readme/`.
3. **Turning lychee blocking before fixing the ~70 already-dead targets** makes every docs PR red.
4. **Mounting a file without `title` frontmatter** fails the Astro schema.
5. **`/agents/` collision** between the scoped `docs/src/AGENTS.md` and mounted root `AGENTS.md`.
6. **Silently ignored options** (`exclude` on `docsLoader`, `llmsFullTxt` on the llms plugin): assert behaviour on `dist/`, never on config.

**Additional open questions:**
- 6. Drop `archive/` from the published site (URL breakage; tombstoned anyway), or keep and exclude from link checks?
- 7. Route prefix for mounted repo docs (`/repo/…`)?
- 8. Accept a new `verified_against` frontmatter key for tutorials (TUT-01)?

## Addendum 2 (2026-10-08, after P20 decisions): LINKS/TUT design details

### Supersedes earlier addendum text
- **Files that qualify for mounting:** the earlier text marked `GEMINI.md` / `LANGUAGE_DESIGN_PRIORITIES.md` as unconfirmed. That was a CRLF false negative in my first check. A CRLF-safe re-check shows **both carry `title:` frontmatter** [VERIFIED: /tmp/esc-list.mjs]. The full list is below.
- **Totals:** the escape census is restated with exact buckets below. 80 distinct `.md` files exist plus 1 missing (`crates/_frozen.md`) = the 81 cited earlier.

### How the plugin learns the current file's path (symlink)
- Astro passes the **symlink path** in `file.path` (or `file.history[0]`).
- Proof: the existing `remark-vox-include.mjs` uses `dirname(file.path)` with no realpath (lines 84-95). `setup-content.mjs` explicitly creates a second symlink `docs-astro/src/examples → examples` so that `{{#include ../../../examples/golden/X.vox}}` resolves *from the symlink location*: "From docs-astro/src/content/docs/<section>/, going up 3 levels reaches docs-astro/src/" [VERIFIED: setup-content.mjs:51-55 comment, remark-vox-include.mjs:81-95].
- Therefore the link plugin must do `realpathSync(file.path.startsWith('file://') ? fileURLToPath(file.path) : file.path)` before resolving any relative link. Copy the include plugin's `file://` handling.
- The mirror-dir change for LINKS-02 keeps the same depth (`src/content/docs/<section>/x.md`), so the include plugin is unaffected.

### Links inside include output and `.mdx`
- **Include output:** `remarkVoxInclude` only rewrites `code` node values (`visit(tree, 'code', …)`, line 101). Included text is code, so it never contains link nodes and needs no handling. Register the link plugin **after** `remarkVoxInclude` anyway.
- **`.mdx`:** exactly one live file, `docs/src/index.mdx`, uses only site-absolute links (`href="/tutorials/tut-getting-started/"`, `](/explanation/expl-architecture/)`) and contains 0 `.md` references [VERIFIED].
  - Starlight auto-adds `@astrojs/mdx` (`integrations.push(mdx({ optimize: true }))`, starlight/index.ts:104-105). MDX inherits `markdown.remarkPlugins` by default [ASSUMED: `extendMarkdownConfig` default true].
  - JSX `<a href>` is not rewritten. That's acceptable today; the plugin should ignore `mdxJsxFlowElement` nodes.

### Slug mapping, exact rules (verified this session)
- Per path segment: `slug()` from `github-slugger@2.0.0`, the same import Astro uses (`import { slug as githubSlug } from "github-slugger"`, astro/dist/content/utils.js:5). Then `.replace(/\/index$/, "")` (utils.js:272).
- Frontmatter `slug:` overrides (glob.js:10-12); 0 pages use it.
- Probe outputs [VERIFIED: /tmp/slug.mjs]:
  - `slug('qwen-3.7-profile-and-mens-4b-feasibility-2026-06-07')` → `qwen-37-profile-and-mens-4b-feasibility-2026-06-07`
  - `slug('README')` → `readme` (README is **not** collapsed)
  - Heading `` `vox run <file> [-- <args>…]` `` → `vox-run-file----args`. So tut-actor-basics' anchor `#vox-run-file----args` is **correct**, which closes that open item.
- Case: github-slugger lowercases, so `AGENTS.md` inside docs/src → `/agents/`.
- `index.mdx` at the root → `/`.

### LINKS-02: full out-of-tree target census [VERIFIED: /tmp/esc-list.mjs over 668 live pages + index.mdx]

| Bucket | Distinct targets | Links | Handling |
|---|---|---|---|
| `.md` files that exist | 80 | 293 | mount or blob (below) |
| Directories | 204 | 391 | `https://github.com/vox-foundation/vox/tree/main/<path>` (top: `crates/vox-skills` 8, `crates/vox-arch-check` 7, `crates/vox-compiler`/`vox-gui`/`vox-cli`/`vox-config` 6 each, `contracts` 5) |
| Other files | 615 | 1,270 | `…/blob/main/<path>` (by extension: `.rs` 902, `.yaml` 126, `.json` 63, `.vox` 48, `.yml` 43, `.toml` 40, `.txt` 9, `.ts` 6, `.mjs` 5, `.mdc` 4, `.tsx` 3, `.snap` 2) |
| Missing on disk | 36 | 48 | fix or remove before LINKS-03 blocks (see below) |

**Line-suffix links.** Several "missing" targets are `path:line` or `path:start-end` (e.g. `crates/vox-scientia/src/lib.rs:12`, `crates/vox-code-audit/src/detectors/victory_claim.rs:23-49`, `crates/vox-cli/src/commands/repair.rs:134`). The plugin should strip a trailing `:N` / `:N-M` and emit GitHub's `#LN` / `#LN-LM` fragment when the base file exists.

**Genuinely dead targets** are mostly retired crates: `crates/vox-dashboard/**` (≥8 links; deleted 2026-05-12 per AGENTS.md), `crates/vox-install-policy`, `vox-primitives`, `vox-protocol`, `vox-plugin-{cloud,grammar-export,oratio,oratio-mic,script-execution}`, `vox-orchestrator-core`, `vox-checksum-manifest`, `crates/_frozen.md`. These are content fixes, and Phase 21 prune input. In Phase 20 they must be relinked or delinked before the blocking gate.

**Distinct out-of-tree `.md` targets with frontmatter `title:` (eligible to mount; `T`), and without (`-` → GitHub blob):**
- **Mount (T, non-planning):** `AGENTS.md` (123), `LANGUAGE_DESIGN_PRIORITIES.md` (23), `docs/agents/governance.md` (14), `CLAUDE.md` (5), `GEMINI.md` (2), `CHANGELOG.md` (2), `apps/editor/vox-vscode/README.md` (3), `tree-sitter-vox/README.md` (1), `scripts/show/README.md` (1), `docs/agents/gui-ia-blueprint.md` (1), `docs/agents/orchestrator.md` (1), `docs/agents/database-nomenclature.md` (1), `.github/copilot-instructions.md` (1).
  - The last is an agent overlay. Mounting is optional; recommend blob.
  - All `docs/agents/*` carry `category: "contributor"` (non-canonical, harmless off-sidebar).
- **Blob (no title):** `docs-astro/README.md` (6), `examples/PARSE_STATUS.md` (6), `README.md` (4), `contracts/README.md` (4), `examples/STYLE.md` (3), `tree-sitter-vox/GRAMMAR_SSOT.md` (2), `scripts/README.md` (2), `contracts/speech-to-code/labeling_rubric.md` (2), `contracts/speech-to-code/README.md` (1), `scripts/coverage-graph/README.md` (1), `docs/ci/build-timings/README.md` (1), `crates/vox-tauri-stt/README.md` (1), `examples/compile-suite/README.md` (1).
- **Blob by decision (D7, internal planning):** 54 `docs/superpowers/{specs,plans}/**` files (~81 links), whether or not they carry a title.
- `CONTRIBUTING.md` is linked **0** times from docs/src. Mount it anyway only if REQUIREMENTS' example list is meant literally.

**`docs/src/AGENTS.md` (`/agents/`) is not a copy.** It's a hand-authored 27-line, directory-scoped rules file; root `AGENTS.md` is 698 lines. There is no generator and no sync. So there is no split-brain today, but there will be a URL collision once root is mounted. With P20-D1 it isn't Internals (`status: current`). Recommendation: exclude it from the collection (agent-only), mount root `AGENTS.md` at the chosen prefix, and add a `_redirects` entry for `/agents/`.

### LINKS-03: lychee vs starlight-links-validator (given installed astro 6.1.9 / Starlight 0.38.3)

| | lychee offline over `dist/` | starlight-links-validator 0.24.0 |
|---|---|---|
| Version fit | action already used (`lycheeverse/lychee-action@v2`) | 0.24.0 peers `astro >=6.0.0`, `@astrojs/starlight >=0.38.0`; **0.25+ requires astro 7** [VERIFIED: npm view] |
| What it checks | rendered HTML incl. rewritten links, components, sidebar, `_redirects` gaps, fragments | Markdown link nodes at build, headings/anchors |
| New dependency | none | one (OK verdict, 133k/wk) |
| Risk | flag-name verification (`--offline`, `--include-fragments`) [ASSUMED] | ordering vs our remark plugin; errors on relative links by default [ASSUMED] |
| **Choice** | **Use this** (blocking, docs-quality.yml) | optional later; not needed |

The broken-link fixture and nightly cached external check are as in the first addendum.

### TUT-01: per-tutorial command/flag inventory (verified with `vox … --help`, build 9a2575123)

| Tutorial | `vox` commands / flags used | Registry | Flags exist? |
|---|---|---|---|
| tut-getting-started | `doctor`, `init my-app`, `check src/main.vox`, `build src/main.vox -o dist`, `run src/main.vox` (+ `cargo install --locked --path crates/vox-cli`) | all `active` | `-o, --out-dir` ✓ |
| tut-first-app | `init --kind application`, `check`, `build`, `run` | active | `--kind <KIND>` ✓ (redundant: default `"application"`, init.rs:11) |
| tut-actor-basics | `build`, `run` (+ anchor link to `vox run <file> [-- <args>…]`) | active | anchor ✓ |
| tut-first-vox-app-checkpoints | `check app.vox`, `check --json`, `build app.vox`, `run …`, `populi serve` | active | `--json` ✓ (alias `--format`). `populi`/`mens` delegate to `vox-ml-cli` (warning seen: "the ML command is running code from a different commit"), so the tutorial must say `vox-ml-cli` is required (installation.md:116 `cargo install --locked --path crates/vox-ml-cli --features populi`) |
| tut-workflow-durability | `mens workflow` (prose) | `mens workflow` in registry | delegated to vox-ml-cli; verify subcommand args |
| tut-ui-integration | none | — | — |
| use-a-react-component-from-vox | none | — | `examples/golden/react_interop.vox` exists (18 non-blank lines) ✓ |

**Doctest coverage:**
- `vox ci doctest-md --strict` over `docs/src` (default path) includes `tutorials/`, and runs in docs-quality.yml as "Doctest extraction and check (SSG-agnostic)" [VERIFIED: docs-quality.yml:65-66].
- It covers 19 inline `vox` fences plus 2 `{{#include}}` fences across the 7 files. It catches parse errors. It is **not** semantic: it passed the wrong-include workflow tutorial.
- 3 `vox:skip`, all with reasons (react tutorial).
- No tutorial is exercised end-to-end; that is TUT-02 (Phase 22).

**Defect list (input to plan):**
1. **tut-getting-started**:
   - (a) The explained code (`query get_notes`, `mutation create_note`) ≠ what `vox init` generates (`server add_note`, `server list_notes`, `routes {}`).
   - (b) Node "20+" vs installation.md "Node.js >= 18".
   - (c) "pnpm (9+)" vs repo pnpm 11.
   - (d) Missing `status:`.
   - (e) 7 relative links 404 on the site.
2. **tut-workflow-durability**:
   - (a) The include `getting_started.vox:logic` has no workflow/activity/`with` block, while the prose describes `retries`/`timeout`/`initial_backoff`. Needs a real workflow golden example (e.g. an ADR-041-subset `workflow` + `activity` golden with `@test`).
   - (b) Duplicated sentence.
   - (c) `vox mens workflow` requires vox-ml-cli; say so.
   - (d) 2 links 404.
3. **tut-first-vox-app-checkpoints**:
   - (a) Stale "only when built with `script-execution`" (default feature).
   - (b) `vox populi serve` needs vox-ml-cli + `populi` feature; link installation.md §ML.
   - (c) Missing `status:`.
   - (d) 6 links 404.
4. **tut-first-app**:
   - (a) `--kind application` is redundant but valid; keep or simplify.
   - (b) Re-check its code against the current scaffold (same drift risk as #1).
   - (c) 2 links 404.
5. **tut-actor-basics**:
   - (a) Missing `status:`.
   - (b) Verify the "lowers actors into Rust primitives" claim against codegen.
   - (c) 3 links 404 (the anchor itself is fine).
6. **tut-ui-integration**: verify `table Task { task_id: Id[Task] … }` and the view-builder calls against the current compiler (doctest passes, so likely OK); 4 links 404.
7. **use-a-react-component-from-vox**: no content defects found; 3 links 404.
8. **All 7:** add a verified-against commit (open question 8), and the LINKS-01 rewrite fixes all 27 tutorial links mechanically.

## Validation Architecture

### Test Framework
| Property | Value |
|----------|-------|
| Framework | Playwright `@playwright/test` ^1.63 (docs-astro), `cargo test -p vox-doc-pipeline`, `vox ci` gates |
| Config file | `docs-astro/playwright.config.ts` (testDir `./tests`, retries 2 in CI) |
| Quick run command | `cd docs-astro && npx playwright test tests/build-output.spec.ts` (after `pnpm build`; fs-only, seconds) |
| Full suite command | `cd docs-astro && pnpm build && npx playwright test tests/build-output.spec.ts && BASE_URL=https://voxlang.org npx playwright test tests/smoke.spec.ts` |
| Build time | Starlight build 2m16s in CI; doc-pipeline build ~8 min cold in CI [VERIFIED: run timings] |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DEPLOY-01 | Latest push run on main green; #462 closed | ops check | `gh run list --repo vox-foundation/vox --workflow docs-deploy.yml --branch main --event push --limit 1 --json conclusion -q '.[0].conclusion'` == `success`; `gh issue view 462 --json state -q .state` == `CLOSED` | n/a |
| DEPLOY-02 | One edit-in-place issue; close on success; liveness | workflow e2e | `gh workflow run docs-deploy.yml -f simulate_failure=true` ×2 → `gh issue list --label docs-deploy-broken --state open --json number,comments` (1 issue, comments unchanged); `gh workflow run ci-liveness.yml` (with a test override of N) → `Deploy stale:` issue appears, then closes; `vox ci ssot-drift` (workflow-policy/concurrency/permissions guards stay green) | ❌ Wave 0 (dispatch input) |
| DEPLOY-03 | home, /voxup (+ `#!`), every llms URL, retired-symbol scan | live e2e + unit | `BASE_URL=https://voxlang.org npx playwright test tests/smoke.spec.ts`; negative: `npx playwright test tests/llms-links.spec.ts` (fixture with a 404 URL must report 1 failure) | smoke ✅ extend; helper+negative ❌ Wave 0 |
| DEPLOY-04 | dates from git, mechanical commits ignored, through symlink | unit + build-output | `node --test docs-astro/tests/unit/git-dates.test.mjs` (synthetic `git log` text: ignored sha skipped, rename aliasing, autoregen subject skipped); build-output: ≥90% of `dist/**/index.html` contain `<time datetime`, and a fixture page's date equals the expected non-mechanical commit | ❌ Wave 0 |
| HONEST-01 | banner + noindex from status | build-output | `tests/build-output.spec.ts`: a research page has `.sl-banner` + `<meta name="robots" content="noindex">`; a current page has neither | ❌ Wave 0 |
| HONEST-02 | Internals group; absent from main groups; excluded from all llms variants | build-output | same spec: sidebar HTML of a current page puts research links only under `Internals`; `dist/llms-full.txt`, `dist/llms-small.txt`, `dist/_llms-txt/*.txt`, `dist/.well-known/llms*.txt` contain no Internals title/URL; `dist/robots.txt` has a valid `Sitemap:` and no Internals `Disallow` | ❌ Wave 0 |
| HONEST-03 | specs marked superseded; one authority | lint + grep | `cargo run -p vox-doc-pipeline -- --lint-only --paths <changed files>`; `rg -l '^status: "?deprecated' docs/superpowers/specs/2026-08-22-docs-corpus-repair-design.md docs/superpowers/specs/2026-09-14-deep-research-documentation-engine-design.md docs/superpowers/plans/2026-09-14-deep-research-documentation-engine.md` returns 3 | n/a |
| LINKS-01 | relative `.md` links → Starlight slugs, anchors kept, symlink-safe | unit + build-output | `node --test docs-astro/tests/unit/remark-doc-links.test.mjs` (fixtures: sibling, `../x/y.md#a`, `index.md`, `README.md`, `qwen-3.7`→`qwen-37`, escaping link from realpath); build-output: `dist/tutorials/tut-getting-started/index.html` contains `href="/reference/installation/"` and no `href="…\.md"` remains in any `dist/**/*.html` (excluding code blocks) | ❌ Wave 0 |
| LINKS-02 | repo Markdown rendered from symlink; non-md → GitHub blob | build-output + smoke | build-output: `dist/repo/agents/index.html` exists and its text matches root `AGENTS.md` heading; a `crates/…rs` link renders as `https://github.com/vox-foundation/vox/blob/main/…`; live smoke: `/repo/agents/` 200, llms-full URLs resolve | ❌ Wave 0 |
| LINKS-03 | blocking internal-link check + broken fixture fails | CI step + unit | docs-quality.yml: `lycheeverse/lychee-action@v2` `--offline --root-dir docs-astro/dist --include-fragments` `fail: true`; mutation check: same command over dist+`tests/fixtures/broken-link/` must exit non-zero; remark plugin throws on missing target (unit). Nightly: `gh workflow run link_checker.yml` with cache | ❌ Wave 0 |
| TUT-01 | each tutorial verified against code | lint + doctest + scripted | `vox ci doctest-md --strict docs/src/tutorials` (~0.1 s); `rg -c '^verified_against:' docs/src/tutorials/*.md` = 7; registry/flag check via `vox <cmd> --help` per command in the punch list; build-output lychee covers tutorial links. (Full step execution is TUT-02, Phase 22.) | ✅ doctest / ❌ verified_against |
| DEPLOY-02 (P20-D4) | failure issue assigned + labelled | ops check | `gh issue list --repo vox-foundation/vox --label docs-deploy-broken --state open --json assignees,labels,comments` → assignee `brbrainerd`, labels include `nightly-failure`, `comments` count unchanged across two forced failures | n/a |
| LINKS-02 (line refs) | `path.rs:N` / `:N-M` → GitHub `#LN` / `#LN-LM` | unit | fixture case in `remark-doc-links.test.mjs` (e.g. `../../crates/vox-cli/src/commands/repair.rs:134` → `…/blob/main/crates/vox-cli/src/commands/repair.rs#L134`) | ❌ Wave 0 |

### Sampling Rate
- **Per task commit:** `vox ci pre-push` (fast: fmt, ssot-drift incl. workflow guards, scoped doc lint) + the relevant unit/build-output spec.
- **Per wave merge:** `pnpm build` + `build-output.spec.ts`; `act`/manual dispatch for workflow changes.
- **Phase gate:** green push run of docs-deploy.yml including the extended smoke job, plus the forced-failure dispatch check.

### Wave 0 Gaps
- [ ] `docs-astro/tests/build-output.spec.ts`: fs assertions over `dist/` (wire into `docs-quality.yml` after "Build Starlight" so PRs gate on it; that job's checkout then needs `fetch-depth: 0` for the date assertion).
- [ ] `docs-astro/tests/unit/git-dates.test.mjs`: requires extracting a pure `parseGitLog(text, ignore)` from `git-dates.mjs`.
- [ ] `docs-astro/tests/lib/llms-links.ts` + negative spec.
- [ ] `simulate_failure` dispatch input in docs-deploy.yml.
- [ ] `docs-astro/tests/unit/remark-doc-links.test.mjs` + fixture tree (LINKS-01/03); shared `src/utils/doc-slug.mjs`.
- [ ] `docs-astro/tests/fixtures/broken-link/index.html` + lychee mutation step in docs-quality.yml (LINKS-03).
- [ ] `contracts/documentation/site-mounted-repo-docs.v1.json` allowlist + mirror-dir rewrite of `setup-content.mjs` (LINKS-02).
- [ ] Pre-fix the ~70 already-dead link targets before lychee goes blocking (LINKS-03).

## Security Domain

| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2/V3 Auth/Session | no | — |
| V4 Access Control | yes (workflow token scopes) | job-level least-privilege `permissions:` |
| V5 Input Validation | low | issue bodies built from `${{ github.* }}` run URLs only; pass via `env:`, never interpolate into `run:` script text |
| V6 Cryptography | no | — |
| V14 Config / supply chain | yes | pin deploy tooling; SHA-pin where practical |

**Concrete issues found:**
1. **Over-broad token.** `permissions: pages: write, id-token: write` is set at **workflow top level** [VERIFIED: docs-deploy.yml:19-21]. So `build-docs`, `deploy-cloudflare` and `smoke-test`, which all run `pnpm install` / cargo builds of third-party code, get a token that can mint OIDC tokens and write Pages. Move those two scopes to `deploy-pages` only, and set top-level `permissions: contents: read`. `workflow_permissions_guard` only requires that a top-level block exists [VERIFIED: workflow_permissions_guard.rs:1-40], so `contents: read` satisfies it.
2. **Unpinned wrangler holding the CF token.** `npx wrangler` resolves the latest npm release at deploy time. Pin `wrangler` in devDependencies (lockfile-verified) or use `cloudflare/wrangler-action` pinned by SHA.
3. **CF token scope (user action).** Keep it to `Account › Cloudflare Pages › Edit` on the single account, with no zone/DNS scopes. Set an expiry reminder: an expiring token is exactly how 2026-09-20 → 10-08 happened, and DEPLOY-02's liveness alert is now the backstop.
4. New issue-writing steps keep `issues: write` at job level only (the current pattern) and use `github.token`. No PAT is needed.

## User Actions Required
- Confirm Open Questions 1–5 (Internals rule, Pagefind, assignee + N, two llms.txt links, audit-page status).
- Optional: review CF token scope/expiry (Security item 3).
- Approve the exact `wrangler` pin version (SUS checkpoint).

## Sources

### Primary (HIGH)
- Installed source: `docs-astro/node_modules/@astrojs/starlight@0.38.3` (`utils/git.ts`, `utils/gitInlined.ts`, `integrations/virtual-user-config.ts`, `utils/routing/data.ts`, `components/{Banner,LastUpdated,Page}.astro`, `index.ts`), `starlight-llms-txt@0.10.0` (`generator.ts`, `llms*.txt.ts`, `index.ts`, `types.ts`, `CHANGELOG.md`).
- Repo: `.github/workflows/{docs-deploy,ci-liveness,nightly-report,docs-quality}.yml`, `docs-astro/{astro.config.mjs,src/routeData.ts,src/utils/*.mjs,scripts/setup-content.mjs,tests/*.ts,package.json,pnpm-workspace.yaml}`, `crates/vox-doc-pipeline/src/pipeline/lint.rs`, `crates/vox-cli-ci/src/{retired_symbol_check,workflow_policy_guard,workflow_permissions_guard}.rs`, `crates/vox-cli/src/commands/ci/status.rs`, `contracts/documentation/{retired-symbols.v1.yaml,docs-sidebar-section-order.v1.json}`, `docs/src/{robots.txt,.well-known/*}`.
- Live: `gh run view/list`, `gh issue view 462`, curl of voxlang.org (`/robots.txt`, `/sitemap*.xml`, `/voxup`, 23 llms URLs).
- Local measurement: dist `<time datetime>` count, frontmatter tallies, bulk-commit file counts.

### Secondary (MEDIUM)
- Workstream research/PITFALLS.md, ARCHITECTURE.md, SUMMARY.md (root-cause history of the deploy outage).

## Metadata
**Confidence breakdown:** Stack HIGH (installed versions read). Architecture HIGH (code paths read, failure reproduced). Pitfalls HIGH (each tied to an observed artifact). Alerting design MEDIUM (design choice; verified only against existing workflow shapes).
**Research date:** 2026-10-08. **Valid until:** ~2026-11-07 (stable; re-check if Starlight/astro or the llms plugin is bumped).
