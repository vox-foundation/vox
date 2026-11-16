---
phase: "342"
slug: "docs-authoring-parity-and-starlight-0-42-upgrade"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-11-05"
---

# Phase 342 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution. Source: `342-RESEARCH.md` § Validation Architecture.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | `node --test` (unit), Playwright `@playwright/test` (build-output/e2e), `cargo test` (hub generator) |
| **Config file** | `docs-astro/playwright.config.ts`; unit tests in `docs-astro/tests/unit/*.test.mjs` |
| **Quick run command** | `cd docs-astro && node --test tests/unit/links-validator-exclude.test.mjs tests/unit/remark-vox-include.test.mjs` |
| **Full suite command** | `cd docs-astro && pnpm build && node --test "tests/unit/*.test.mjs" && BASE_URL=http://localhost:4321 npx playwright test tests/build-output/ tests/reader-layout.spec.ts tests/trust-signals.spec.ts tests/smoke.spec.ts tests/baseline.spec.ts` (start `pnpm preview --port 4321 --ignore-lock` first) |
| **Estimated runtime** | ~360 seconds (build ~245 s, backgrounded) |

---

## Sampling Rate

- **After every task commit:** quick run command, or `cargo check -p vox-doc-pipeline` + the new Rust test only
- **After every plan wave:** full suite command (backgrounded), **diffed against baseline** — baseline already fails: build-output 27 pass / 2 fail; reader/trust/smoke/baseline 33 pass / 18 fail / 1 skip; unit 4 pre-existing failures (`tutorial-record`, `workflow-paths` ×2, `doc-slug` dist cross-check)
- **Before `/gsd-verify-work`:** full suite shows no new failures vs baseline + validator mutation probe passes
- **Max feedback latency:** 360 seconds

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 342-01-T1 | 01 | 1 | DOCS-PARITY-04 | T-342-04 | outside-root guard kept | build | build log has no `resolves outside repository root` | ✅ | ⬜ pending |
| 342-01-T2 | 01 | 1 | DOCS-PARITY-04 | T-342-02 | no dead hub links | unit (Rust) | `cargo test -p vox-doc-pipeline --lib openwiki` | ❌ W0 (test written first in-task) | ⬜ pending |
| 342-01-T2 | 01 | 1 | DOCS-PARITY-04 | T-342-03 | hub = generator output | drift | regenerate via `generate_subsystem_hubs(…, true)`; CI `ssot-drift` verifies | ✅ | ⬜ pending |
| 342-01-T3 | 01 | 1 | DOCS-PARITY-04 | — | N/A | baseline | freeze `342-baseline-{unit,playwright}-failed.txt` | ❌ (created in-task) | ⬜ pending |
| 342-02-T1 | 02 | 2 | DOCS-PARITY-02 | T-342-SC | packages human-verified before install | checkpoint | blocking-human legitimacy gate | manual | ⬜ pending |
| 342-02-T2 | 02 | 2 | DOCS-PARITY-02 | T-342-08 | Internals stay out of llms-full.txt | build + e2e | `pnpm --dir docs-astro build` → `page(s) built`; `tests/build-output/llms.spec.ts` | ✅ | ⬜ pending |
| 342-02-T3 | 02 | 2 | DOCS-PARITY-02 | — | N/A | regression | unit + Playwright diff vs baseline files is empty | ✅ | ⬜ pending |
| 342-03-T1/T2 | 03 | 3 | DOCS-PARITY-03 | T-342-11 | warnings not lost | build-output | `tests/build-output/alerts.spec.ts` (3 tests) | ❌ W0 (spec written first in-task) | ⬜ pending |
| 342-04-T1 | 04 | 4 | DOCS-PARITY-04 | T-342-12 | predicate not over-broad | unit | `node --test docs-astro/tests/unit/links-validator-exclude.test.mjs` | ❌ W0 (test written first in-task) | ⬜ pending |
| 342-04-T2 | 04 | 4 | DOCS-PARITY-04 | T-342-14 | probe leaves no residue | mutation | seeded bad link + hash → build exit ≠0 with `invalid link`/`invalid hash`; restore verified by `cmp` | ✅ (automated in-task) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

- [ ] `crates/vox-doc-pipeline/src/pipeline/openwiki.rs` — failing test for linked-vs-plain crate cell (missing API page → plain text)
- [ ] `docs-astro/tests/build-output/alerts.spec.ts` — asides present, no `[!TYPE]` blockquote left
- [ ] `docs-astro/tests/unit/links-validator-exclude.test.mjs` + `docs-astro/src/utils/links-validator-exclude.mjs`

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| New/bumped npm packages are legitimate | DOCS-PARITY-02 | package-legitimacy seam flags "too-new latest publish" | `checkpoint:human-verify` before `pnpm install`: confirm publishers/versions of astro, @astrojs/starlight, @astrojs/markdown-remark, starlight-links-validator, starlight-github-alerts |
| Lockfile from local pnpm reads cleanly under CI pnpm | DOCS-PARITY-02 | CI pnpm version differs from local | first docs-quality run on the PR |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 360s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
