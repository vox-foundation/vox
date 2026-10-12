---
phase: 20-deploy-unblock-public-surface-honesty
plan: 10
subsystem: docs-site
tags: [docs, ci, lychee, links, workflows]
status: complete

requires:
  - 20-08 (remark-doc-links + docLinksGate)
  - 20-09 (repo Markdown mounted under /repo/)
provides:
  - "docs-quality.yml: lychee self-test over a broken-link fixture (must exit 2 and report both dead URLs)"
  - "docs-quality.yml: blocking offline lychee gate over every built page (routes + anchors), run from docs-astro/"
  - "link_checker.yml: .lycheecache response cache (3d max age, 429/5xx not cached), saved only from main"
  - "Built site has 0 broken internal links or anchors (was 11)"
affects: [docs-quality, link_checker]

actuals:
  tokens: 21000
  tasks: 3
  commits: 3
plan_head_before: f4bed25355936736cafa9af41fd2dfb255e5b6ed

tech-stack:
  added: []
  patterns:
    - "Prove a checker can fail before trusting a green run: a fixture with one dead route and one dead anchor must make lychee exit 2"
    - "The internal gate runs lychee from docs-astro/ so the repo-root .lycheeignore (scoped to external links) cannot mask site links"
    - "Assets that sharp cannot optimise (animated WebP) live under docs-astro/public/ and are referenced site-absolute"

key-files:
  created:
    - docs-astro/tests/fixtures/broken-link/index.html
  modified:
    - .github/workflows/docs-quality.yml
    - .github/workflows/link_checker.yml
    - docs/src/reference/cli-vox-deploy.md
    - docs/src/reference/cli.md
    - docs/src/reference/scientia-ssot-handbook.md
    - docs/src/architecture/deep-research-prior-art-and-vox-roadmap-2026.md
    - docs/src/how-to/how-to-mcp-vox-validate.md
    - docs/src/how-to/how-to-voxdb-canonical-store.md
    - docs/src/explanation/expl-architecture.md
    - docs/src/index.mdx
    - scripts/render-durable-animation.vox
    - apps/build-tools/render-durable-animation/package.json
  moved:
    - "docs/src/assets/durable_essentialist_loop.webp -> docs-astro/public/media/durable_essentialist_loop.webp"

completed: 2026-10-09
---

# Phase 20 Plan 10: Blocking internal link gate Summary

A broken internal link or anchor on the built docs site now fails `docs-quality.yml`. Before the gate runs, a self-test proves lychee can detect breakage: a fixture with one dead route and one dead anchor must make it exit 2. All 11 broken links on the site are fixed. The nightly external link checker now keeps a response cache.

## Commits

- `6ae0d089f`: link-checker self-test fixture and assert step (Task 1)
- `a2283aad7`: fix the 6 measured broken anchors (Task 2)
- `324912e23`: the blocking built-site gate, the nightly cache, and the remaining 5 broken links (Task 3)

## Link census (offline, full `docs-astro/dist`)

| Point | Links | Errors |
|---|---|---|
| Baseline (f4bed2535) | 510,637 | 11 |
| After Task 2 | 510,637 | 5 |
| After Task 3 | 510,636 (14,827 unique) | **0** |

All 6,072 exclusions are http(s) URLs, skipped because the gate runs offline. External URLs stay with the nightly `link_checker.yml`. No `file://` link is excluded.

## Self-test evidence

`lychee --offline --root-dir <abs dist> --include-fragments --index-files index.html tests/fixtures/broken-link/index.html` (run from docs-astro) exits **2** and reports both `/this-route-does-not-exist/` and `/reference/cli/#no-such-anchor-p20`. The assert step requires exit code 2 and both URLs in `lychee/selftest.md`. Exit code 1 means a runtime or input error, so it does not count as detecting the links.

**Mutation check of the gate.** I added a scratch page to `dist/` with a dead anchor (`../reference/cli/#mutation-p2010`) and a missing file. The site-wide run then exited 2 with exactly those 2 errors. I removed the page, and the clean run exits 0.

## .lycheeignore review

The file is unchanged. The nightly repo-Markdown scan depends on it. Run from the repo root, some of its patterns match built-site paths and masked 35 internal links. For example, unanchored `crates/.*` matches `/contributors/publishing-crates/#…`, and `file://.*vox-vscode/.*` matches `/adr/031-deprecate-vox-vscode/`. Both lychee steps in `docs-quality.yml` now set `workingDirectory: docs-astro`, which has no `.lycheeignore`, so those 35 links are checked. They all pass.

## #578 (Docs Link Checker nightly)

The cause was already fixed by ad6346ca1 (#624). The manual run after that fix (37859557657) passed. The cache added here only makes nightly runs cheaper and is not the fix. Per instructions, I made no GitHub writes (no comment, no close).

## Verification

- Task 1: the yq structural check passes: the assert step names both fixture URLs, compares the exit code to 2, and the self-test output is `lychee/selftest.md`. No `${{` appears inside any `run:` script; expressions are passed through `env:`. actionlint is clean.
- Task 2: the 6 targeted anchors resolve. Errors went from 11 to 5.
- Task 3:
  - The full-dist gate reports 0 errors.
  - actionlint is clean on both workflows.
  - These checks pass: `workflow-concurrency-guard`, `cache-key-lint` ("no PR-reachable cache writes"), and `yaml-parse-check` on both workflows. All were run with `cargo run -p vox-cli -- ci …`.
  - `ssot-drift` passes in a throwaway detached worktree.
  - `ci check-links` passes (4,631 internal links).
  - `vox ci workflow-scripts` fails as it already did before this plan. Its `docs-quality.yml` hit, `scripts/show/README.md`, also appears in the base commit's file. Not in scope.
- The build produces 698 pages. `dist/index.html` serves `/_astro/old_internet_knot_abstract.*.png`, and `dist/media/durable_essentialist_loop.webp` exists.

## Deviations from Plan

**1. [Scope widened, user-approved: Option A] 4 broken links beyond the plan's measured set.** The census found 11 errors, but the plan had measured 7. The extra 4 were three raw-HTML `<img>` tags whose assets the build never emitted, and the home page's `/how-to/` link, which has no index page. I stopped at the checkpoint, and the user chose to fix them.
- `expl-architecture.md` prism diagram: changed to a Markdown image, so it is fingerprinted, and a missing file fails the build. The alt text is kept.
- `index.mdx` knot image: now an Astro import (`oldInternetKnot.src`). It keeps `width="80%"` and the alt text.
- `index.mdx` How-To link: now points at `/how-to/external-app-bootstrap/`, the first entry in the How-To sidebar group. I did not add a new landing page.
- `durable_essentialist_loop.webp`: kept as raw HTML, because the 600px size and the rounded, shadowed styling matter. The file moved to `docs-astro/public/media/`. As a Markdown image, the 4.7 MB animated WebP fails sharp's `limitInputPixels` (all frames count) and breaks the build. `public/` is copied verbatim, and the new gate catches a missing file. I updated the output path in `scripts/render-durable-animation.vox` and `apps/build-tools/render-durable-animation/package.json` to match. This also resolves the 20-08 row in `deferred-items.md`.

**2. [Rule 2] The gate runs from `docs-astro/`.** See the .lycheeignore review above. As a result, the self-test arguments and the assert step use paths relative to docs-astro.

**3. [Rule 2] Main-only cache save.** `link_checker.yml` restores the cache on any ref with `actions/cache/restore@v5` and saves only when `github.ref == 'refs/heads/main'` with `actions/cache/save@v5`. This follows the repo's cache policy (`cache-key-lint`). I also added `--cache-exclude-status '429,500..=599'` so throttled responses and server errors are re-checked instead of cached.

**4. [Minor]**
- The self-test step sets `jobSummary: false` so the fixture's expected failures do not appear in the job summary as if they were real.
- In `cli.md`, a self-link with an empty anchor (`[docs/src/reference/cli.md](#)`) became plain code text instead of being given an anchor.

**5. [Tooling] Context-tool banner.** One edit injected the context-compression tool's banner as the first line of `docs-quality.yml`. I stripped it before linting, and every staged diff was checked for the banner before each commit.

## Deferred Issues

None new. The 720 absolute `https://voxlang.org/...` links in the docs are skipped in offline mode, so neither the gate nor the nightly scan checks them as internal routes. That is outside LINKS-03 and only noted here.

## Self-Check: PASSED

- The fixture, the workflow steps, and the moved asset exist at the listed paths.
- `git log f4bed2535..HEAD` lists `6ae0d089f`, `a2283aad7`, and `324912e23`.
- The full-site lychee run (from docs-astro) shows 0 errors.
