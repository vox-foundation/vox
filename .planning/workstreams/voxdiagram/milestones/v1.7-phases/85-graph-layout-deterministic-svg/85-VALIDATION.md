---
phase: "85"
slug: "graph-layout-deterministic-svg"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-17"
---

# Phase 85 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest over `cargo test` binaries; `proptest` (workspace, existing dev-dep); byte-golden helper `assert_golden` (`tests/support/roundtrip.rs`, `UPDATE_GOLDEN=1`); `quick-xml` 0.41 (new dev-dep from 85-01, workspace version, hakari no-op) for the SVG allowlist checker. No `insta` for SVG: goldens are real `.svg` files the contact sheet and Linux check reuse |
| **Config file** | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`; no per-crate config) |
| **Quick run command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(<test file stem>) \| test(/^svg::/) \| test(/^layout::/)'` |
| **Full suite command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` (340 pass / 1 skipped before this phase) |
| **Estimated runtime** | ~120 s for the full crate suite after the first build (dagre layouts are 0.2-5 ms per corpus diagram; the property suites dominate) |

Filter rules: integration-test files are selected with `binary(<file stem>)`: `svg_render`, `svg_errors`, `svg_hostile`, `svg_determinism`, `svg_geometry_props`, `svg_theme`, `svg_corpus`, `determinism_guard`. Library unit tests in the new private modules are selected by name: `test(/^svg::/)` (`svg::writer::tests::…`, `svg::paths::…`, `svg::tests::…`) and `test(/^layout::/)` (`layout::tests::…` engine doubles, `layout::text::…`, `layout::verify::…`, `layout::input::…`, `layout::finish::…`). The research draft named an integration binary `layout_failclosed`; that cannot reach the crate-internal `Engine` seam, so the doubles are unit tests in `src/layout/tests.rs` instead. The contact-sheet generator is `#[ignore]` and runs only with `--run-ignored only -E 'binary(svg_corpus) & test(contact_sheet)'` and `CONTACT_SHEET_OUT` set. Goldens are blessed with `UPDATE_GOLDEN=1` and must pass without it. nextest exits non-zero when a filter selects no test, which is the failing signal for a mistyped filter.

---

## Sampling Rate

- **After every task commit:** `cargo check -p vox-diagram --all-targets` + the task's own `binary(...)` / `test(...)` filter (backgrounded; never block the foreground on cargo).
- **After every plan wave:** `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(/^svg_/) | test(/^svg::/) | test(/^layout::/) | binary(determinism_guard)'`.
- **Before `/gsd-verify-work`:** full crate suite, `cargo clippy -p vox-diagram --all-targets -- -D warnings`, `cargo fmt -p vox-diagram --check`, `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`, `vox ci crate-api-sync`, `vox ci crate-edges`, `cargo run -p vox-arch-check`, `cargo hakari generate --diff`, `cargo deny check licenses bans sources`, scoped doc lint (85-05 Task 3). Hosted CI on the milestone PR (full workspace suite, because `Cargo.toml`/`Cargo.lock` are CI sentinels, on `ubuntu-latest` against macOS-blessed goldens) is authoritative.
- **Max feedback latency:** 120 seconds.

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 85-01-01 | 01 | 1 | REND-01, REND-02, REND-03, REND-04 | T-85-01, T-85-02, T-85-03, T-85-04, T-85-SC | single escaper; caps before layout; panic and NaN never reach the writer; validated id prefix; dagre pinned, deny- and hakari-clean | tracer (golden + unit) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) \| test(/^svg::/) \| test(/^layout::/) \| binary(determinism_guard)'` ; `cargo hakari generate --diff && cargo deny check licenses bans sources` | ❌ W0 | ⬜ pending |
| 85-01-02 | 01 | 1 | REND-01 | — | labels sized to stay inside every shape | property + golden | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) \| test(/^layout::/) \| test(/^svg::/)'` | ❌ W0 | ⬜ pending |
| 85-02-01 | 02 | 2 | REND-01, REND-02 | T-85-07 | edge labels only in escaped text; integer-only routing | unit + golden | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) \| test(/^svg::/) \| test(/^layout::/)'` | ❌ W0 | ⬜ pending |
| 85-02-02 | 02 | 2 | REND-01 | T-85-05, T-85-06 | panic/NaN/huge/missing to `RenderError::Layout`; unfixable overlap to `Geometry`; bounded ladder | unit (Engine doubles) + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'test(/^layout::/) \| test(/^svg::/) \| binary(svg_render)'` | ❌ W0 | ⬜ pending |
| 85-03-01 | 03 | 3 | REND-01 | T-85-08 | parent walk bounded; IR depth never trusted | unit + golden | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_render) \| test(/^layout::/) \| test(/^svg::/)'` | ❌ W0 | ⬜ pending |
| 85-03-02 | 03 | 3 | REND-01 | T-85-08, T-85-09, T-85-10 | containment fails closed; hostile hand-built IR refused or rendered, never panics | unit + integration | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_errors) \| test(/^layout::/) \| binary(svg_render)'` | ❌ W0 | ⬜ pending |
| 85-04-01 | 04 | 4 | REND-03 | T-85-11, T-85-12, T-85-13 | allowlisted well-formed SVG for hostile input; no user text in attributes; prefix-scoped unique ids; each escape arm mutation-proven | fixture + property + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_hostile) \| test(/^svg::writer::/)'` ; `git diff --quiet -- crates/vox-diagram/src/svg/writer.rs` | ❌ W0 | ⬜ pending |
| 85-04-02 | 04 | 4 | REND-02, REND-01 | T-85-14 | byte determinism; drawn geometry independently checked; float/clock/seam guards | property + guard + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_determinism) \| binary(svg_geometry_props) \| binary(determinism_guard)'` ; `git diff --quiet -- crates/vox-diagram/src` | ❌ W0 | ⬜ pending |
| 85-04-03 | 04 | 4 | REND-04 | — | var-only paints; contrast in dark, light and fallback | unit | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_theme)'` | ❌ W0 | ⬜ pending |
| 85-05-01 | 05 | 5 | REND-01, REND-02, REND-04 | T-85-15, T-85-16 | live-corpus goldens; reproducible two-theme sheet from one markup | integration + golden | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_corpus)'` ; `CONTACT_SHEET_OUT=<phase-dir>/85-CONTACT-SHEET.html cargo nextest run -p vox-diagram --run-ignored only -E 'binary(svg_corpus) & test(contact_sheet)'` | ❌ W0 | ⬜ pending |
| 85-05-02 | 05 | 5 | REND-01, REND-02 | T-85-15 | decision recorded with evidence | doc lint + review | `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/voxdiagram-layout-go-no-go-2026.md` | ❌ W0 | ⬜ pending |
| 85-05-03 | 05 | 5 | REND-01, REND-02, REND-03, REND-04 | T-85-17 | no baseline edits; API page via sync only | gates | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` ; `cargo clippy -p vox-diagram --all-targets -- -D warnings && cargo fmt -p vox-diagram --check` ; `target/debug/vox ci crate-api-sync && target/debug/vox ci crate-edges` | ✅ (existing gates) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is folded into 85-01 (its tracer creates the harness every later task uses):

- [ ] `Cargo.toml` + `crates/vox-diagram/Cargo.toml` + `Cargo.lock`: `dagre =0.1.1` (default features), `unicode-width 0.2`, dev `quick-xml` (user-approved 2026-10-16, re-confirmed 2026-10-17; only plan touching manifests)
- [ ] `crates/vox-diagram/tests/svg_render.rs` + `tests/fixtures/svg/cases/00-minimal.{vdg,svg}` (golden loop over `cases/*.vdg`)
- [ ] `src/svg/writer.rs` and `src/layout/mod.rs` unit-test modules (`test(/^svg::/)`, `test(/^layout::/)`)
- [ ] later binaries created by their own plans, each RED first: `svg_errors` (85-03), `svg_hostile`, `svg_determinism`, `svg_geometry_props`, `svg_theme` + `tests/support/{svgcheck,theme}.rs` (85-04), `svg_corpus` + `tests/fixtures/svg/corpus/` (85-05); `src/layout/tests.rs` Engine doubles (85-02)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs: skip-verify in `lay_out` (85-02); each `esc` arm `&`, `<`, `>`, controls, whole-function bypass, attribute `"` and `'` (85-04); transcendental-float and dagre-seam guards (85-04) | REND-01, REND-02, REND-03 | deliberate break-then-restore of guards (repo rule) | save a copy, `grep -c` the guarded line before and after the mutation, run the named binary with `--retries 0`, see it fail, restore, `git diff --quiet -- crates/vox-diagram/src`; record arm, failing test and message in the plan SUMMARY |
| Contact-sheet visual review of the 10 spike diagrams in light and dark panes | REND-01, REND-04 | layout quality and legibility are judged by eye (SC2) | open `85-CONTACT-SHEET.html` in headless Chromium (docs-astro `playwright-core`, throwaway script outside the repo), screenshot each pane, record per diagram: labels inside shapes, titles in bands, arrows visible in both themes, soft defects; count DSL tweaks against the threshold of 2 |
| Linux byte comparison of the goldens (aarch64 Docker, x86_64 under QEMU) | REND-02 | needs Docker; hosted CI is the authoritative Linux run | standalone copy of the crate under `~/dev/_scratch/vd85-linux/` with lock-pinned versions, `docker run --rm -v …:/w -w /w rust:1.98.1 cargo test --test svg_corpus --test svg_render`, then `--platform linux/amd64`; record pass/fail/not-run in 85-05 SUMMARY |
| Hosted CI on the milestone PR (full workspace suite, cargo-deny on dependency change) | REND-02 | nothing is pushed during the phase | push the milestone branch as a draft PR; confirm the `tests` shards and the cargo-deny step pass against the macOS-blessed goldens |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 120s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
