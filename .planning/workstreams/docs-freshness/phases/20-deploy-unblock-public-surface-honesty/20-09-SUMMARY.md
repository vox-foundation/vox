---
phase: 20-deploy-unblock-public-surface-honesty
plan: 09
subsystem: docs-site
tags: [docs, astro, starlight, remark, links, workflows, llms]
status: complete

requires:
  - 20-02 (git-date map, routeData middleware)
  - 20-07 (llms.spec, .well-known publishing)
  - 20-08 (remark-doc-links + docLinksGate, links.spec)
  - 20-12 (marker-guarded content mirror in setup-content.mjs)
provides:
  - "contracts/documentation/site-mounted-repo-docs.v1.json: always_mount, never_mount_prefixes, titles"
  - "docs-astro/src/utils/repo-mounts.mjs: mountRoute, discoverMounts, wrapperSource, relativeLinkTargets (shared with the 20-05 link guard)"
  - "Repo Markdown linked from docs (plus AGENTS/CLAUDE/GEMINI) renders at /repo/<route>/ from the single repo file"
  - "Every page's edit link and Last-updated date come from its true repo file (fixes 20-08's malformed edit links)"
  - "docs-deploy / docs-quality path filters cover every mounted source, the mount contract, .git-blame-ignore-revs, .lycheeignore"
  - ".well-known llms links to /repo/{agents,claude,gemini}-md/ (no 404s); llms.spec asserts every voxlang.org URL is built"
affects: [20-10]

actuals:
  tokens: 13200
  tasks: 3
  commits: 4
plan_head_before: f03e5498dadca26222ece2f28a2b4ecd17166820

tech-stack:
  added: []
  patterns:
    - "Mounts live in a real repo/ dir inside the marked mirror: titled files are symlinked, untitled ones get a generated page with mounted_from"
    - "A generated page's links resolve from mounted_from (file.data.astro.frontmatter), a symlinked one's from realpath"
    - "routeData derives the repo source once per page (mounted_from or realpath behind the mirror) for both edit URL and git date"

key-files:
  created:
    - contracts/documentation/site-mounted-repo-docs.v1.json
    - docs-astro/src/utils/repo-mounts.mjs
    - docs-astro/tests/unit/repo-mounts.test.mjs
    - docs-astro/tests/build-output/repo-docs.spec.ts
  modified:
    - docs-astro/scripts/setup-content.mjs
    - docs-astro/src/content.config.ts
    - docs-astro/src/plugins/remark-doc-links.mjs
    - docs-astro/src/routeData.ts
    - docs-astro/astro.config.mjs
    - docs-astro/tests/unit/remark-doc-links.test.mjs
    - docs-astro/tests/unit/source-link-targets.test.mjs
    - docs-astro/tests/build-output/llms.spec.ts
    - docs/src/.well-known/llms.txt
    - docs/src/.well-known/llms-full.txt
    - .github/workflows/docs-deploy.yml
    - .github/workflows/docs-quality.yml
    - docs/ci/build-timings/README.md
    - tree-sitter-vox/README.md
    - apps/editor/vox-vscode/README.md

key-decisions:
  - "Edit URLs are set in routeData for every page (not just mounts): baseUrl alone cannot map the mirror path to docs/src/<path>"
  - "Mounted pages' git dates come from a separate git log over the mounted paths, so docs/src dates (20-02) are byte-identical"
  - "Workflow filters list each mounted file by exact path (crate READMEs via crates/**/README.md); no **/*.md"
  - "Relative images (Markdown and raw-HTML src) in mounted pages point at raw/main on GitHub"
  - "README.md gets the contract title 'Vox repository README' (it has no # heading)"

metrics:
  duration: "~25 min"
  completed: 2026-10-09
---

# Phase 20 Plan 09: Repo Markdown mounted under /repo/ Summary

Repo Markdown that the docs link to now renders as site pages under `/repo/<route>/`, built from the one repo file through the content mirror. Docs links to those files, their edit links and their dates all point at the real source. The three 404ing agent-file links in `.well-known` now resolve, and editing a mounted file triggers both docs workflows.

## Mount count

26 files mounted: 13 symlinked (frontmatter title) and 13 generated wrappers (`mounted_from`). They include root `AGENTS.md`, `CLAUDE.md`, `GEMINI.md`, `LANGUAGE_DESIGN_PRIORITIES.md`, `README.md`, `CHANGELOG.md`, `.github/copilot-instructions.md`, `docs/agents/*`, and READMEs under `contracts/`, `crates/vox-tauri-stt/`, `examples/`, `scripts/`, `tree-sitter-vox/`, `apps/editor/vox-vscode/`, `docs-astro/`. Nothing under `docs/superpowers/` or `docs/src/` is mounted.

**win32 fallback:** if `symlinkSync` hits `EPERM`, a titled file is written as a generated page instead of copied. That keeps `mounted_from`, so links, edit URL and date still resolve from the real source. Not exercised on this macOS run.

**New mounts need a filter line.** A new docs link to repo Markdown adds a mount on the next build. The Task 3 coverage check (`UNCOVERED <workflow> <event> <path>`) fails until that path is added to docs-deploy and docs-quality.

## Verification

- Task 1: `setup-content.mjs` run twice leaves `git status docs/src` unchanged. The mirror is still ignored and nothing in `docs-astro/src/content` shows in git status. `repo-docs.spec.ts` and `archive.spec.ts` pass.
- Task 2: unit tests pass: repo-mounts plus remark-doc-links, 735 across `tests/unit`. In the mutation check, removing the `mounted_from` base-path branch fails the wrapper test (27 pass, 1 fail), and restoring it gives 28/28. `repo-docs`, `links`, `dates` and `archive` specs pass. `/repo/agents-md/` has edit link `/edit/main/AGENTS.md` and is dated from AGENTS.md history. `ci/runner-contract` links `href="/repo/agents-md/"`.
- Task 3: `llms`, `repo-docs` and `archive` specs pass. The old-URL count is 0 in both `.well-known` files. The coverage script prints `COVERED`. In the mutation check, deleting `'AGENTS.md'` from docs-deploy prints `UNCOVERED docs-deploy.yml push AGENTS.md`, and restoring it prints `COVERED`. actionlint is clean. `workflow-concurrency-guard` and `yaml-parse-check` pass, run with the worktree-built `vox` via `cargo run -p vox-cli`; the installed `vox` is stale and refuses to run guards.
- Final: all 37 `tests/build-output` specs pass on a fresh build. The build logs no remark-doc-links warnings.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Malformed edit links on every page (20-08 deferred)**
- **Found during:** Task 2, as the orchestrator flagged
- **Issue:** `editLink.baseUrl` ended in `docs/src/`, and Starlight appends `src/content/docs/<path>`, giving `…/docs/src/src/content/docs/…`.
- **Fix:** `baseUrl` is now the repo root. `routeData.ts` sets every page's `editUrl` to `edit/main/<true repo path>`, which is `docs/src/<path>` for docs pages and the mounted source for `/repo/` pages. Frontmatter string `editUrl` values are respected. `repo-docs.spec.ts` asserts that docs pages, `/agents/` and every mounted page link to existing repo files. `deferred-items.md` marks the row resolved.
- **Commit:** bba0087c9

**2. [Rule 1 - Bug] Four dead links in now-published repo READMEs**
- **Found during:** Task 2 (`links.spec.ts` failed: relative hrefs above the site root on two mounted pages)
- **Fix:** `tree-sitter-vox/README.md` and `apps/editor/vox-vscode/README.md` now link to the archived docs' actual paths (`docs/src/archive/research-2026-q1/…`, rendered as blob links). In `docs/ci/build-timings/README.md`, the removed `ensure_cuda_path.ps1` row now names `vox doctor --fix-cuda-path`.
- **Commit:** bba0087c9

**3. [Rule 2 - Missing] Raw-HTML `<img src>` in mounted pages**
- **Issue:** README's hero images are raw HTML `<img src="docs/src/assets/…">`, which would 404 under `/repo/readme-md/`.
- **Fix:** mounted pages also rewrite relative raw-HTML `src` attributes to raw GitHub URLs. Unit tested.
- **Commit:** bba0087c9

**4. [Rule 3 - Blocking] `astro check` type error in routeData**
- **Fix:** typed the merged git-date map as `Map<string, string>`.
- **Commit:** 7a1a8a061

**5. [Refactor] `relativeLinkTargets` / `stripCode` moved from `tests/unit/source-link-targets.test.mjs` into `repo-mounts.mjs`**
- The test re-exports it, so the 20-05 guard and mount discovery share one link extractor ("same rules as 20-05's guard").

## Deferred Issues

Logged in `deferred-items.md`:
- The new contract is not registered in `contracts/index.yaml`. This is the same gap as the 20-04 contract.
- `docs-astro/README.md` is still the Starlight starter-kit README and is now published at `/repo/docs-astro-readme-md/`.
- Mounted files are outside the strict dead-link gate. A dead link there warns and then fails `links.spec.ts`.

Pre-existing and out of scope: `astro check` still reports 120 errors in `src/content/docs/architecture/fableforge-impl/*.ts`, `VoxPlayground.astro` and `honesty.spec.ts`.

## Threat Flags

None beyond the plan's threat model. T-09-1 is mitigated: there is a real `repo/` dir inside the marked mirror, wrappers are written with `wx`, and setup throws if `docs/src/repo` exists. T-09-2 is mitigated: `never_mount_prefixes` is enforced, mounts are only linked or allowlisted files that realpath inside the repo and outside docs/src, and the spec asserts no superpowers route. T-09-3 is mitigated: titles are JSON-quoted. T-09-4 is mitigated: workflow edits are append-only and checked by actionlint, the guards and the coverage check.

## Self-Check: PASSED
