---
phase: "86"
slug: "sequence-kind"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-17"
---

# Phase 86 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest over `cargo test` binaries; `proptest` (existing dev-dep); byte-golden helper `assert_golden` (`tests/support/roundtrip.rs`, `UPDATE_GOLDEN=1`); `insta` for the `fixes_repair` snapshots; the independent quick-xml SVG allowlist checker `tests/support/svgcheck.rs` (split by 86-01 into `svgcheck.rs` + `svgcheck/allow.rs`, extended by 86-03 with `svgcheck/seq.rs`); the Node oracle `docs-astro/tests/unit/mermaid-oracle.test.mjs` with the pinned `mermaid` 11.17.2. No new dependency |
| **Config file** | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`; no per-crate config) |
| **Quick run command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(<test file stem>) \| test(/^<module path>::/)'` |
| **Full suite command** | `INSTA_UPDATE=no cargo nextest run -p vox-diagram` (464 pass / 2 skipped before this phase) |
| **Oracle command** | `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` (run from the repository root; `docs-astro/node_modules` is installed) |
| **Estimated runtime** | ~150 s for the full crate suite after the first build (property suites dominate); the oracle ~20 s |

Filter rules: integration-test files are selected with `binary(<file stem>)`. New binaries of this phase:
`sequence_dsl`, `svg_sequence` (86-01); `sequence_check`, `sequence_mermaid`, `sequence_props` (86-02);
`svg_sequence_geometry`, `svg_sequence_safety` (86-03); `sequence_corpus` (86-04). Library unit tests in the
new private modules are selected by name: `test(/^parser::seq::/)`, `test(/^resolve::seq::/)`,
`test(/^layout::seq::/)`, `test(/^svg::seq::/)`, plus the existing `test(/^print::/)`, `test(/^fixgen::/)`,
`test(/^codes::/)`, `test(/^check::/)`, `test(/^mermaid::import::/)`. Goldens are blessed with
`UPDATE_GOLDEN=1` on the single golden test (`binary(svg_render) & test(svg_cases)`, `binary(mermaid_roundtrip)`,
`binary(sequence_corpus)`) and must pass without it; insta snapshots with `INSTA_UPDATE=always` on
`binary(fixes_repair) & test(fixes_repair_seeded_set)` only. nextest exits non-zero when a filter selects no
test, which is the failing signal for a mistyped filter.

---

## Sampling Rate

- **After every task commit:** `cargo check -p vox-diagram --all-targets` + the task's own filter (backgrounded; never block the foreground on cargo).
- **After every plan wave:** `INSTA_UPDATE=no cargo nextest run -p vox-diagram -E 'binary(/^sequence_/) | binary(/^svg_/) | binary(/^mermaid_/) | binary(diagnostics_all_errors) | binary(fixes_repair) | binary(determinism_guard) | test(/::seq::/)'`.
- **Before `/gsd-verify-work`:** full crate suite, `cargo clippy -p vox-diagram --all-targets -- -D warnings`, `cargo fmt -p vox-diagram --check`, `RUSTDOCFLAGS='-D warnings' cargo doc -p vox-diagram --no-deps`, `target/debug/vox ci crate-api-sync`, `target/debug/vox ci crate-edges`, `cargo run -p vox-arch-check`, `cargo hakari generate --diff`, scoped doc lint of `architecture/where-things-live.md`, the Node oracle, and `git diff --quiet 883a3e4b1 -- Cargo.toml Cargo.lock crates/vox-diagram/Cargo.toml crates/workspace-hack contracts/ci docs/src/architecture/layers.toml` (86-04 Task 3). Hosted CI on the milestone PR is authoritative.
- **Max feedback latency:** 150 seconds.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| DGM-02 | SC1: actors, ordered messages, dashed `reply` returns, self-messages, `note over` one or two actors | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_dsl) \| test(/^parser::seq::/) \| test(/^resolve::seq::/)'` | non-zero exit, "0 tests run", any FAIL |
| DGM-02 | SC1: `check` parity (all errors at once, codes, spans, fixes; undeclared-actor declare fix; Mermaid-isms with rewrites) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_check) \| binary(sequence_mermaid) \| binary(diagnostics_all_errors) \| binary(fixes_repair) \| test(/^fixgen::/)'` | non-zero exit, "0 tests run", any FAIL, pending snapshot |
| DGM-02 | SC1: `fmt` canonical and idempotent, `parse(fmt(x)) == parse(x)` (S1-S4) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_props)'` | non-zero exit, any FAIL, a written `proptest-regressions` file |
| DGM-02 | SC3: corpus block imports, prints as DSL, re-parses to the same IR; every sequence import path round-trips | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_corpus) \| binary(mermaid_roundtrip) \| binary(mermaid_fixpoint) \| binary(mermaid_import_seq) \| binary(mermaid_import_props) \| binary(mermaid_corpus)'` | non-zero exit, any FAIL, stale/missing golden |
| DGM-02 | SC3: re-export accepted by the pinned real Mermaid parser | `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | non-zero exit or any `not ok` line |
| REND-05 | SC2: width-aware lanes; 80+ character messages neither overlap nor clip (oracle + property) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence) \| binary(svg_sequence_geometry) \| test(/^layout::seq::/) \| test(/^svg::seq::/)'` | non-zero exit, "0 tests run", any FAIL |
| REND-05 | SC2: same safe writer, byte-identical across runs, fail closed (caps, InvalidIr, hostile text, integer-only geometry) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence_safety) \| binary(determinism_guard) \| binary(svg_render) \| binary(svg_theme)'` | non-zero exit, any FAIL, stale/missing golden |
| REND-05 | SC3: the corpus block renders (golden + geometry oracle) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_corpus)'` | non-zero exit, any FAIL, stale/missing golden |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 86-01-01 | 01 | 1 | DGM-02, REND-05 | T-86-01, T-86-02, T-86-03 | sequence text only through the Phase 85 writer; parser fuel and caps cover sequence statements; unknown actor is `InvalidIr` | tracer (integration + unit) | `cargo check -p vox-diagram --all-targets` ; `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_dsl) \| binary(svg_sequence) \| test(/^parser::seq::/) \| test(/^resolve::seq::/) \| test(/^layout::seq::/) \| test(/^svg::seq::/) \| binary(diagnostics_all_errors) \| binary(svg_render) \| binary(svg_corpus) \| binary(mermaid_export) \| binary(svg_theme) \| test(/^codes::/) \| test(/^check::/) \| test(/^print::/)'` | ❌ W0 | ⬜ pending |
| 86-01-02 | 01 | 1 | DGM-02 | — | contextual keywords never shadow actor ids | integration | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_dsl) \| binary(svg_sequence) \| test(/^parser::/) \| binary(fixes_repair) \| binary(fmt_golden) \| binary(diagnostics_all_errors)'` | ❌ W0 | ⬜ pending |
| 86-02-01 | 02 | 2 | DGM-02 | T-86-04, T-86-05 | fixes valid by construction (`declarable`, `quote`); adversarial inputs bounded | tracer (integration + unit) + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_check) \| binary(diagnostics_all_errors) \| binary(fixes_repair) \| test(/^resolve::seq::/) \| test(/^fixgen::/) \| test(/^parser::/)'` | ❌ W0 | ⬜ pending |
| 86-02-02 | 02 | 2 | DGM-02 | T-86-04, T-86-06 | Mermaid rewrites only quoted labels and validated ids; repair and importer agree on the corpus block | catalogue + cross-check + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_mermaid) \| binary(sequence_dsl) \| binary(fixes_repair) \| binary(diagnostics_all_errors) \| binary(mermaid_catalog) \| test(/^parser::/)'` | ❌ W0 | ⬜ pending |
| 86-02-03 | 02 | 2 | DGM-02 | T-86-04 | corrupt-then-repair returns the original IR | property | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_props) \| binary(fmt_props) \| binary(repair_props)'` | ❌ W0 | ⬜ pending |
| 86-03-01 | 03 | 2 | REND-05 | T-86-07 | long labels wrap and fit between lifelines (independent oracle) | tracer (integration + golden) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence_geometry) \| binary(svg_sequence) \| binary(svg_render) \| binary(svg_theme) \| test(/^layout::seq::/)'` | ❌ W0 | ⬜ pending |
| 86-03-02 | 03 | 2 | REND-05 | — | no overlap, no clip for generated diagrams | property + golden + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence_geometry) \| binary(svg_render) \| binary(svg_theme) \| test(/^layout::seq::/)'` | ❌ W0 | ⬜ pending |
| 86-03-03 | 03 | 2 | REND-05 | T-86-07, T-86-08, T-86-09, T-86-10 | caps before layout; `InvalidIr` for inconsistent IR; hostile text never escapes; byte determinism; integer-only geometry | integration + property + guard + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(svg_sequence_safety) \| binary(svg_sequence) \| binary(svg_sequence_geometry) \| binary(determinism_guard) \| binary(svg_errors) \| binary(svg_render) \| test(/^layout::/) \| test(/^svg::/)'` | ❌ W0 | ⬜ pending |
| 86-04-01 | 04 | 3 | DGM-02, REND-05 | T-86-11 | imported DSL kept only when `fmt` accepts it (fail closed) | tracer (integration + golden + unit) | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(sequence_corpus) \| binary(mermaid_import_seq) \| binary(mermaid_import_props) \| binary(mermaid_corpus) \| test(/^print::/) \| test(/^mermaid::import::/)'` | ❌ W0 | ⬜ pending |
| 86-04-02 | 04 | 3 | DGM-02 | T-86-11, T-86-12 | every accepted import re-parses to the same IR; export goldens unchanged; real-parser oracle | integration + property + oracle + mutation | `INSTA_UPDATE=no cargo nextest run -p vox-diagram --retries 0 -E 'binary(mermaid_roundtrip) \| binary(mermaid_fixpoint) \| binary(mermaid_import_seq) \| binary(mermaid_import_props) \| binary(mermaid_corpus) \| binary(mermaid_import_words) \| binary(mermaid_import_review) \| binary(svg_corpus) \| binary(sequence_corpus)'` ; `cd docs-astro && node --test tests/unit/mermaid-oracle.test.mjs` | ✅ (existing binaries) / ❌ W0 (`sequence_corpus`) | ⬜ pending |
| 86-04-03 | 04 | 3 | DGM-02, REND-05 | T-86-13, T-86-SC | no manifest/baseline change; API page via sync only | gates | `cargo fmt -p vox-diagram --check && cargo clippy -p vox-diagram --all-targets -- -D warnings` ; `INSTA_UPDATE=no cargo nextest run -p vox-diagram` ; `target/debug/vox ci crate-api-sync && target/debug/vox ci crate-edges` ; `git diff --quiet 883a3e4b1 -- Cargo.toml Cargo.lock crates/vox-diagram/Cargo.toml crates/workspace-hack contracts/ci docs/src/architecture/layers.toml` | ✅ (existing gates) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is folded into 86-01 (its tracer creates the harness every later task uses):

- [ ] `crates/vox-diagram/tests/sequence_dsl.rs` and `crates/vox-diagram/tests/svg_sequence.rs` (tracer binaries)
- [ ] `crates/vox-diagram/tests/support/svgcheck/allow.rs` (allowlist tables moved out of `svgcheck.rs`, sequence classes, `stroke-dasharray`) and the `lifelines` / `notes` / `edge_paths` read-back fields
- [ ] unit-test modules for the new `src` files (`parser::seq`, `resolve::seq`, `layout::seq` via `src/layout/seq/tests.rs`, `svg::seq`)
- [ ] later binaries created by their own plans, each RED first: `sequence_check`, `sequence_mermaid`, `sequence_props` + `tests/support/seq.rs` (86-02); `svg_sequence_geometry`, `svg_sequence_safety` + `tests/support/svgcheck/seq.rs` (86-03); `sequence_corpus` + `tests/fixtures/svg/sequence-corpus/` (86-04)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs: declare-fix insertion point (86-02 T1), `->>` rewrite text (86-02 T2), crossing-label term of the gap rule (86-03 T2), render caps check (86-03 T3), dashed flag in `sequence_source` (86-04 T2) | DGM-02, REND-05 | deliberate break-then-restore of guards (repo rule) | save a copy, `grep -c` the guarded line before and after the mutation, run the named binary with `--retries 0`, see the named test fail, restore from the copy, `git diff --quiet -- <file>`, rerun green; record file, mutation, failing test and message in the plan SUMMARY |
| Visual review of the corpus sequence SVG and the four `3x-seq` goldens | REND-05 | legibility of lanes, labels, loops and notes is judged by eye (SC2) | headless Chromium via `playwright-core` from `docs-astro/node_modules/.pnpm`, throwaway script in the scratchpad; screenshot each at 1.5x; record per diagram: labels inside boxes or between lifelines, arrows and loops visible, notes over the right lanes, nothing clipped; a visible defect is fixed in `layout/seq.rs` with a regression test, not just recorded (86-04 T3) |
| Hosted CI on the milestone PR (full workspace suite on `ubuntu-latest` against macOS-blessed goldens) | REND-05 | nothing is pushed during the phase | push the milestone branch as a draft PR; the `tests` shards must pass; sequence geometry is integer-only, so Linux bytes must equal the goldens |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 150s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
