---
phase: "89"
slug: "docs-astro-integration"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-17"
---

# Phase 89 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | Rust: cargo-nextest. `vox-doc-pipeline` lib unit tests in the new `pipeline::render_manifest` module (temp-dir roots, no new dev-dependency); `vox-diagram` lib unit tests (`layout::text::tests`, `layout::seq::tests`, `layout::input` tests module) and integration binaries (`fence_scan`, `svg_render`, `svg_corpus`, `sequence_corpus`, `svg_sequence_geometry`, `svg_sequence`, `svg_determinism`, `svg_geometry_props`, `grammar_card`). docs-astro: `node:test` unit files (`tests/unit/*.test.mjs`; Node 24 in CI, 26 locally) and `@playwright/test` 1.63 (fs-only `tests/build-output/*.spec.ts`; browser `tests/vox-diagram.spec.ts` on Chromium against `astro preview` of `dist/`). No new npm or cargo dependency |
| **Config file** | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`); `docs-astro/playwright.config.ts` (`testDir ./tests`, ignores `unit/**`, `lib/**`, `fixtures/**`, webServer `pnpm preview --port 4321` unless `BASE_URL`) |
| **Quick run command** | `cargo nextest run -p vox-doc-pipeline --retries 0 --lib -E 'test(/^pipeline::render_manifest::/)'` ; `pnpm --dir docs-astro exec node --test tests/unit/remark-vox-diagram.test.mjs tests/unit/render-diagrams.test.mjs tests/unit/workflow-paths.test.mjs` |
| **Full suite command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` ; `cargo nextest run -p vox-doc-pipeline --retries 0 --lib -E 'test(/^pipeline::render_manifest::/) \| test(/^pipeline::diagrams::/)'` ; `pnpm --dir docs-astro exec node --test "tests/unit/*.test.mjs"` ; `pnpm --dir docs-astro build && pnpm --dir docs-astro exec playwright test tests/build-output/ tests/vox-diagram.spec.ts --reporter=line --output=test-results/vd-run` |
| **Estimated runtime** | Rust filters seconds once built (cold builds take minutes: background them); node unit tests ~5 s; `pnpm build` 25-45 s plus the pre-render; Playwright build-output ~10 s, diagram spec ~1 min |

Command rules: every node and Playwright command runs with cwd `docs-astro` (`pnpm --dir docs-astro exec ...` sets it). nextest exits non-zero when a filter selects no test, which is the failing signal for a mistyped filter; `node --test` prints `# tests N` and `# fail N`; Playwright prints a passed/failed summary and `Error: No tests found` for a bad path. Before a local Playwright run, stop any `astro preview` already on port 4321 (`reuseExistingServer` would serve a stale `dist/`). `target/debug/vox` goes stale at every commit: rebuild with `cargo build -p vox-cli` (background) before the closing `vox ci` gates.

---

## Sampling Rate

- **After every task commit:** `cargo check -p <touched crate>` plus the task's own nextest filter, or the touched `node --test` file(s); a task that changes docs-astro build behavior also runs `pnpm --dir docs-astro build` (background).
- **After every plan wave:** Wave 1: `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` and the `render_manifest` filter. Wave 2: all docs unit tests, `pnpm build`, all build-output specs. Wave 3: the full suite command.
- **Before `/gsd-verify-work`:** 89-04 Task 3 closing gates: `target/debug/vox ci ssot-drift`, `crate-edges`, `check-links`, `pre-push` (fast tier); `cargo fmt -p vox-diagram -p vox-doc-pipeline --check`; `cargo clippy -p vox-diagram --all-targets -- -D warnings`; the full suite; `git diff --quiet b176936db -- Cargo.lock docs-astro/pnpm-lock.yaml contracts/ci/crate-edges.allow.v1.json`. Hosted CI on the milestone PR is authoritative (docs-quality now runs the browser spec; docs-deploy builds the site).
- **Max feedback latency:** 60 seconds for unit filters once built; about 2 minutes for build plus browser spec.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| DOCS-03 | SC1: pre-render writes a deterministic, deduped, sorted manifest; bad fences reported at file:line:col with exit 1; atomic write | `cargo nextest run -p vox-doc-pipeline --retries 0 --lib -E 'test(/^pipeline::render_manifest::/)'` | non-zero exit, "no tests to run", any FAIL |
| DOCS-03 | SC1: Rust and remark agree on the manifest key | `cargo nextest run -p vox-diagram --retries 0 --test fence_scan` ; `pnpm --dir docs-astro exec node --test tests/unit/remark-vox-diagram.test.mjs` | non-zero exit, any FAIL / `# fail` above 0 |
| DOCS-03 | SC1: md and mdx inline as hast figures, no `<pre>` wrapper, page-prefixed ids, llms-full keeps source, no Mermaid-selector match | `pnpm --dir docs-astro build && pnpm --dir docs-astro exec playwright test tests/build-output/ --reporter=line` | non-zero exit, `No tests found`, any failed test |
| DOCS-03 | SC1: a fence with no render fails the build naming the file and `pnpm diagrams` (md) | `rm -f docs-astro/.vox-diagram/manifest.json && ! pnpm --dir docs-astro exec astro build > target/vd-neg-md.log 2>&1 && grep -q 'pnpm diagrams' target/vd-neg-md.log && grep -q 'reference/vox-diagram.md' target/vd-neg-md.log` (then `pnpm --dir docs-astro diagrams`) | non-zero exit of the chain (build exited 0, or the message lacks file or command) |
| DOCS-03 | SC3: deploy and quality workflows pre-render before `pnpm build` and trigger on `crates/vox-diagram/**`; quality installs Chromium before the browser specs | `pnpm --dir docs-astro exec node --test tests/unit/workflow-paths.test.mjs` | non-zero exit, `# fail` above 0, `# tests` below 9 |
| DOCS-04 | SC2 (Rust half): every padded text box clears a font ~21% wider than the table; goldens re-blessed; oracles and determinism green | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` | non-zero exit, any FAIL, a pending `.snap.new`, a golden mismatch |
| DOCS-04 | SC2: no duplicate ids, labels fit under three stacks (one measurably wide), contrast in both themes with a failing-palette self-test, forced colors, wide policy | `pnpm --dir docs-astro exec playwright test tests/vox-diagram.spec.ts --reporter=line --output=test-results/vd-run` | non-zero exit, `No tests found`, any failed or flaky test, passed count below 13 |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 89-01-01 | 01 | 1 | DOCS-03 | T-89-SC | No dependency or lockfile change | unit + e2e build | `cargo nextest run -p vox-doc-pipeline --retries 0 --lib -E 'test(/^pipeline::render_manifest::/)'` ; `pnpm --dir docs-astro exec node scripts/render-diagrams.mjs && grep -c '"schema_version":1' docs-astro/.vox-diagram/manifest.json` ; `pnpm --dir docs-astro build && grep -o 'class="vd-figure"' docs-astro/dist/reference/vox-diagram/index.html \| wc -l` | ❌ W0 (created test-first in the task) | ⬜ pending |
| 89-01-02 | 01 | 1 | DOCS-03 | T-89-01, T-89-02 | Atomic manifest write; every bad fence reported, exit 1 | unit | `cargo nextest run -p vox-doc-pipeline --retries 0 --lib -E 'test(/^pipeline::render_manifest::/)'` ; `cargo nextest run -p vox-diagram --retries 0 --test fence_scan` ; `git check-ignore -q docs-astro/.vox-diagram/manifest.json` | ❌ W0 (`fence-keys.json` new; `fence_scan.rs` exists) | ⬜ pending |
| 89-01-03 | 01 | 1 | DOCS-03 | T-89-03 | permissions/concurrency/cache lines untouched | node unit | `pnpm --dir docs-astro exec node --test tests/unit/workflow-paths.test.mjs` | ✅ (extended) | ⬜ pending |
| 89-02-01 | 02 | 1 | DOCS-04 | T-89-05 | Blessing tests only; number-only golden diffs | unit + golden | `cargo nextest run -p vox-diagram --retries 0 --lib -E 'test(/^layout::text::tests::/) \| test(/^layout::seq::tests::/)'` ; `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 --test svg_render --test sequence_corpus --test svg_sequence_geometry --test svg_sequence --test svg_determinism` | ✅ (extended) | ⬜ pending |
| 89-02-02 | 02 | 1 | DOCS-04 | T-89-04, T-89-05 | Render caps unchanged; independent oracles re-run | full crate | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0` ; `cargo clippy -p vox-diagram --all-targets -- -D warnings` | ✅ | ⬜ pending |
| 89-03-01 | 03 | 2 | DOCS-03 | T-89-07 | A missing render cannot ship (guard mutation-proven) | node unit + e2e build | `pnpm --dir docs-astro exec node --test tests/unit/remark-vox-diagram.test.mjs` ; md negative-control chain (Requirement map row 4) | ❌ W0 (unit file new) | ⬜ pending |
| 89-03-02 | 03 | 2 | DOCS-03 | T-89-06, T-89-08 | Closed-grammar SVG parser rejects hostile markup; numeric `--vd-w` | node unit | `pnpm --dir docs-astro exec node --test tests/unit/remark-vox-diagram.test.mjs tests/unit/render-diagrams.test.mjs` | ❌ W0 (`render-diagrams.test.mjs` new) | ⬜ pending |
| 89-03-03 | 03 | 2 | DOCS-03 | T-89-07, T-89-09 | No placeholder shipped; no Mermaid-selector match | fs spec | `pnpm --dir docs-astro build > target/vd-build.log 2>&1 && pnpm --dir docs-astro exec playwright test tests/build-output/ --reporter=line` | ❌ W0 (`diagrams.spec.ts` new) | ⬜ pending |
| 89-04-01 | 04 | 3 | DOCS-04 | T-89-10, T-89-11 | Discovery cannot pass empty; CI installs only the browser and fonts | node unit + browser | `pnpm --dir docs-astro exec node --test tests/unit/workflow-paths.test.mjs` ; `cargo run -p vox-doc-pipeline -- --lint-only --paths reference/vox-diagram.md` ; `pnpm --dir docs-astro exec playwright test tests/vox-diagram.spec.ts --reporter=line --output=test-results/vd-run` | ❌ W0 (spec and lib new) | ⬜ pending |
| 89-04-02 | 04 | 3 | DOCS-04 | T-89-10 | Font-collapse probe and contrast self-test prevent vacuous passes | browser | `pnpm --dir docs-astro exec playwright test tests/vox-diagram.spec.ts --reporter=line --output=test-results/vd-run` ; `ls docs-astro/test-results/review-bundle/vox-diagram/ \| grep -c 'png$'` | ❌ W0 | ⬜ pending |
| 89-04-03 | 04 | 3 | DOCS-03, DOCS-04 | T-89-12, T-89-SC | No new edge, no lockfile change, CI cap respected | gates | `target/debug/vox ci ssot-drift && target/debug/vox ci crate-edges && target/debug/vox ci check-links` ; `target/debug/vox ci pre-push` | ✅ | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Test-first inside each task (no separate scaffold plan): every new test file lands, failing, in the task that implements it.

- [ ] `crates/vox-doc-pipeline/src/pipeline/render_manifest.rs` `#[cfg(test)] mod tests` (89-01 Tasks 1-2)
- [ ] `crates/vox-diagram/tests/fixtures/fence-keys.json` + `manifest_keys_match_the_shared_corpus` in `tests/fence_scan.rs` (89-01 Task 2)
- [ ] floor tests in `crates/vox-diagram/src/layout/text/tests.rs`, `layout/seq/tests.rs`, `layout/input.rs` tests module (89-02)
- [ ] `docs-astro/tests/unit/remark-vox-diagram.test.mjs`, `docs-astro/tests/unit/render-diagrams.test.mjs` (89-03)
- [ ] `docs-astro/tests/build-output/diagrams.spec.ts` (89-03 Task 3)
- [ ] `docs-astro/tests/vox-diagram.spec.ts`, `docs-astro/tests/lib/diagrams.ts` (89-04)
- [ ] extended `docs-astro/tests/unit/workflow-paths.test.mjs` (89-01 Task 3, 89-04 Task 1)
- [ ] No framework install (Chromium is a browser download in CI, already cached locally)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| mdx negative control: a fence missing from the manifest in an `.mdx` page fails `astro build` naming the file | DOCS-03 | No committed mdx page has a vox-diagram fence until Phase 90; needs a temporary edit to `docs/src/index.mdx` | 89-03 Task 1 step 4(c): restore the manifest, append a one-node fence to `index.mdx`, `pnpm --dir docs-astro exec astro build` must exit non-zero naming `index.mdx`, then `git checkout -- docs/src/index.mdx`; paste the lines into the SUMMARY |
| Guard mutation: removing the integration's build hooks lets an md miss ship | DOCS-03 | A deliberate temporary break of the guard | 89-03 Task 1 step 4(b): remove both hooks (grep confirms), the md negative control must exit 0, restore (grep confirms) |
| Dev loop: an edited fence shows a placeholder until `pnpm diagrams`, then the figure | DOCS-03 | Needs a running `astro dev` server | 89-03 Task 2 step 7; non-gating, recorded in the SUMMARY |
| By-eye review of light, dark and forced-colors screenshots (Phase 86 carried item, incl. multi-hop labels) and font-policy confirmation | DOCS-04 | Visual judgement | 89-04 Task 2 step 7: Read every PNG in `docs-astro/test-results/review-bundle/vox-diagram/`, record per-figure findings |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
