---
phase: 89-docs-astro-integration
plan: 01
subsystem: docs
tags: [vox-diagram, astro, starlight, remark, hast, vox-doc-pipeline, github-actions]

requires:
  - phase: 87-docs-ratchet
    provides: "fence::extract/normalize scanner, mermaid ratchet, doc-lint diagram issues"
  - phase: 85-svg-renderer
    provides: "deterministic SVG writer with the vd-title/vd-desc/vd-arrow id families"
provides:
  - "cargo run -p vox-doc-pipeline -- --render-diagrams: gitignored docs-astro/.vox-diagram/manifest.json"
  - "docs-astro/scripts/render-diagrams.mjs wrapper (cwd = repo root, --dev keep-going)"
  - "remark-vox-diagram plugin: fence -> figure.vd-figure with re-prefixed inline SVG and source details"
  - "inline vox-diagram Astro integration (plugin registration, manifest watch + digest)"
  - "fence-keys.json parity corpus with its Rust half"
  - "pnpm diagrams/predev/prebuild and docs-deploy/docs-quality pre-render steps + path filters"
affects: [89-02, 89-03, 89-04]

actuals:
  tokens: 9700
  tasks: 3
  commits: 3
plan_head_before: ed1c69bf14265b477ceb47d80bfe6877a46d4442
plan_head_after: 5e9346566afbefa1f0ce16c0100779e59513dc6b
commits: 3

tech-stack:
  added: []
  patterns:
    - "Pre-render in Rust, trust-but-parse in JS: fail-closed SVG-to-hast parser for the writer grammar only"
    - "Retype the mdast node to a custom type (vdFigure) so no <pre> wrapper is emitted"
    - "Manifest keyed by normalized fence source; atomic temp-file + rename write"

key-files:
  created:
    - crates/vox-doc-pipeline/src/pipeline/render_manifest.rs
    - docs-astro/scripts/render-diagrams.mjs
    - docs-astro/src/plugins/remark-vox-diagram.mjs
    - crates/vox-diagram/tests/fixtures/fence-keys.json
  modified:
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - docs-astro/astro.config.mjs
    - docs-astro/.gitignore
    - docs-astro/package.json
    - crates/vox-diagram/tests/fence_scan.rs
    - .github/workflows/docs-deploy.yml
    - .github/workflows/docs-quality.yml
    - docs-astro/tests/unit/workflow-paths.test.mjs

key-decisions:
  - "Diagram dedupe tracks tried keys, so a failing source is attributed to (and reported once from) the first file holding it"
  - "Invalid fences are reported only through fence_issues; a second diagram-render line is emitted only for valid-but-unrenderable fences"
  - "Temp manifest file is a hidden dotfile in the target directory, removed on any write/rename error"

patterns-established:
  - "Rust items in the new module are private/pub(crate); no public API or crate edge changed"

requirements-completed: []  # DOCS-03 is NOT ticked: the hard build-failure guard on a manifest miss is plan 89-03

duration: 35min
completed: 2026-10-17
status: complete
---

# Phase 89 Plan 01: Pre-render to inline figure Summary

**VoxDiagram fences now travel Rust pre-render -> gitignored manifest -> remark plugin -> `<figure class="vd-figure">` with re-prefixed inline SVG and a source block on the built reference page, with the pre-render wired into `pnpm diagrams/predev/prebuild` and both docs workflows.**

## Performance

- **Duration:** about 35 min (dominated by a 6 min cold compile of `vox-doc-pipeline` and the 846-page site builds)
- **Tasks:** 3 (1 tracer, 2 auto/tdd), 3 commits
- **Files:** 4 created, 8 modified (848 insertions, 6 deletions)

## Accomplishments

- `--render-diagrams` walks `docs/src` (archive skipped, `.md` and `.mdx`), dedupes by `fence::normalize`, renders each source once with the default `vd` prefix and writes `{schema_version:1, diagrams:[{source,svg}]}` sorted and byte-identical (sha256 equal across two runs from the repo root). Hard fence findings print `docs/src/<file>:<line>:<col>: <code>: <message>`, render failures print `diagram-render`, exit is 1, and the valid diagrams are still written.
- The built `dist/reference/vox-diagram/index.html` has exactly 2 `class="vd-figure"` figures (the page has 2 fences), ids `vd-reference-vox-diagram-1-*` and `-2-*`, 0 `<pre>`-wrapped figures, and 2 source blocks.
- `crates/vox-diagram/tests/fixtures/fence-keys.json` (23 hand-authored CommonMark cases) passes against the Rust key on the first run; plan 89-03 runs the same corpus through remark.
- `docs-deploy.yml` (push) and `docs-quality.yml` (push + pull_request) trigger on `crates/vox-diagram/**` and run `Pre-render VoxDiagram fences` from the repo root before `Build Starlight`; no concurrency/permissions/cache/timeout line was touched.

## Task Commits

1. **Task 1 (tracer): pre-render, wrapper, plugin, integration** - `1fe8e7fd8`
2. **Task 2: production pre-render + key corpus** - `82fbe6dbb`
3. **Task 3: build entry points and workflows** - `5e9346566`

## TDD Evidence

- **Task 1 RED:** `pipeline::render_manifest::tests::one_fence_renders_into_the_manifest` failed with the stub (`left: 2, right: 0`: the `--render-diagrams` flag dispatched to a stub returning 2). GREEN after the implementation (1 passed).
- **Task 2 RED:** `fence_scan::manifest_keys_match_the_shared_corpus` failed with the fixture absent (`fence-keys.json: No such file or directory`); the eight `render_manifest` tests failed to compile (`cannot find function render_diagrams`, a compile-level RED, not a behavioral one). GREEN: 9/9 `render_manifest` tests and 12/12 `fence_scan` tests.
- **Task 3 RED:** `workflow-paths.test.mjs` 5 pass / 3 fail on the three new tests. GREEN: 8/8.
- **Mutation proofs (break -> named test fails -> restore, with presence checks):**
  - Corpus key `["x\n y"]` changed to `["x\ny"]` (presence 4 after mutation): `manifest_keys_match_the_shared_corpus` FAILED naming the case "three-space indented opener strips up to three spaces per line"; restored (presence 1, `cmp` identical to backup).
  - Archive skip `if rel.starts_with("docs/src/archive/")` -> `if false && ...` (presence 1): `render_manifest::tests::archive_is_skipped` FAILED (8 passed, 1 failed); restored (presence 0, `cmp` identical).

## Deviations from Plan

**1. [Rule 1 - Plan error] Built-HTML `language-vox-diagram` acceptance cannot be met literally**
- **Found during:** Task 1 verification.
- **Issue:** Expressive Code re-processes the `<pre><code class="language-vox-diagram">` inside the figure into a framed block carrying `data-language="vox-diagram"`; the `language-vox-diagram` class is restored only by starlight-llms-txt's `improveExpressiveCodeHandling` when building `llms-full.txt` (89-RESEARCH Pitfall 5/6), never in the site HTML.
- **Fix:** Verified the equivalent facts on the built page: `data-language="vox-diagram"` count 2 and `class="vd-source"` count 2. The `llms-full.txt` selector work belongs to plan 89-03.
- **Files modified:** none (verification only).

**2. [Rule 3 - Blocking] `astro build` alone cannot verify Task 1**
- **Found during:** Task 1 verification. `src/content/docs/` (gitignored mirror) is created only by `setup-content.mjs`, so a bare `astro build` rendered an empty collection. Ran the real `pnpm build` (prebuild = wrapper + setup-content) instead; this also exercised Task 3's `prebuild`.

**3. [Ordering] `docs-astro/.gitignore` entry landed in the Task 1 commit** (the plan lists it under Task 2) so the generated manifest never showed as untracked while Task 1 was verified. `git check-ignore` passes.

**4. [Process] The plan said `pnpm`; the host default is pnpm 12, so all runs used `npx --yes pnpm@11.28.6 --dir docs-astro build`** (CI major) with the frozen lockfile; `pnpm-lock.yaml` and `Cargo.lock` are unchanged.

## Auth Gates

None.

## Known Stubs

None. The plugin throws on a manifest miss; the dev placeholder, the miss registry and the build-wide hard-failure guard (the rest of DOCS-03) are plan 89-03's scope.

## Verification Run

- `cargo nextest run -p vox-doc-pipeline --lib -E 'test(/^pipeline::render_manifest::/)'`: 9 passed.
- `cargo nextest run -p vox-diagram --test fence_scan`: 12 passed.
- `node --test tests/unit/workflow-paths.test.mjs`: 8 passed.
- `pnpm build` (CI pnpm 11.28.6): exit 0; log contains the pre-render summary line naming `manifest.json`; reference page: 2 figures, 0 `<pre><figure`, ids `vd-reference-vox-diagram-1-title` and `-2-arrow` present.
- Acceptance diffs against `b176936db`: `Cargo.lock`, `crates/vox-doc-pipeline/Cargo.toml`, `docs-astro/pnpm-lock.yaml`, `contracts/ci/crate-edges.allow.v1.json` clean; `mod.rs` gained 4 lines; `render_manifest.rs` is 360 non-blank lines; workflow diff removes no concurrency/permissions/cache/timeout line; `package.json` diff touches only `diagrams`, `predev`, `prebuild`.
- Not run (CI is the gate): `cargo clippy` on the touched crates, workspace tests.

## Self-Check: PASSED

Created files found: `render_manifest.rs`, `render-diagrams.mjs`, `remark-vox-diagram.mjs`, `fence-keys.json`; commits `1fe8e7fd8`, `82fbe6dbb`, `5e9346566` are ancestors of HEAD.
