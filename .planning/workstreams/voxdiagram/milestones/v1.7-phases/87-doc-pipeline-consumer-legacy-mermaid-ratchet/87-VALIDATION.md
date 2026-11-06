---
phase: "87"
slug: "doc-pipeline-consumer-legacy-mermaid-ratchet"
# status lifecycle: draft (seeded by plan-phase) → validated (set by validate-phase §6)
status: draft
nyquist_compliant: false
wave_0_complete: false
created: "2026-10-17"
---

# Phase 87 — Validation Strategy

> Per-phase validation contract for feedback sampling during execution.

---

## Test Infrastructure

| Property | Value |
|----------|-------|
| **Framework** | cargo-nextest over `cargo test` binaries. `vox-doc-pipeline` lib unit tests (inline `#[cfg(test)] mod tests`, plus `mermaid_gate_tests.rs` declared with `#[path]` as `facets_tests.rs` is) using per-test temp roots under `std::env::temp_dir()` and the `pub(super)` fixture helpers of `pipeline/mod.rs` tests; `vox-cli` lib unit tests with a temporary git repository (`tempfile`, existing dev-dependency). No new dependency |
| **Config file** | `.config/nextest.toml` (profiles `default`, `ci`, `ci-gate`; no per-crate config) |
| **Quick run command** | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::diagrams::/) \| test(/^pipeline::mermaid_gate::/) \| test(/^pipeline::tests::/)'` |
| **Full suite command** | `cargo nextest run -p vox-doc-pipeline` and `cargo nextest run -p vox-cli --lib --retries 0 -E 'test(/commands::ci::mermaid_baseline::/) \| test(/commands::kb::facet_backlog::/)'` |
| **Estimated runtime** | ~10 s for the doc-pipeline tests after the first build; the vox-cli lib test binary takes minutes to build cold (background it), seconds to run |

Filter rules: all new tests are lib unit tests, selected by module path. `pipeline::diagrams::` (87-01),
`pipeline::mermaid_gate::` (87-02 inline and `tree_tests`, 87-03 `gate_text_cases`),
`commands::ci::mermaid_baseline::` (87-03). nextest exits non-zero when a filter selects no test, which is the
failing signal for a mistyped filter. Real-repository checks: `cargo run -p vox-doc-pipeline -- --lint-only
--paths architecture/where-things-live.md` exercises the whole-tree legacy-Mermaid pass against the committed
baseline plus one doc.

---

## Sampling Rate

- **After every task commit:** `cargo check -p vox-doc-pipeline --all-targets` plus the task's own filter (backgrounded; never block the foreground on cargo).
- **After every plan wave:** the quick run command; after 87-03 Task 1 also the vox-cli filter.
- **Before `/gsd-verify-work`:** 87-03 Task 2 closing gates: `cargo fmt -p vox-doc-pipeline --check`, `rustfmt --edition 2024 --check` on the two touched vox-cli files, `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings`, `cargo nextest run -p vox-doc-pipeline`, the vox-cli filter, `target/debug/vox ci crate-edges`, `crate-api-sync`, `affected-crates --check`, `contracts-index`, `cargo run -p vox-arch-check`, `cargo hakari generate --diff`, scoped doc lint of the two edited docs, `target/debug/vox ci pre-push` (fast tier), and `git diff --numstat 2f0bd1d73 -- contracts/ci/crate-edges.allow.v1.json` = `7 0`. Hosted CI on the milestone PR is authoritative (vox-cli clippy runs there).
- **Max feedback latency:** 60 seconds once the test binaries are built.

---

## Requirement → Automated Command Map

| Requirement | Success criterion | Automated command | Fails when |
|-------------|-------------------|-------------------|------------|
| DOCS-02 | SC2: malformed fence fails with `diagram/*` at file:line:col, valid passes, warnings do not block | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::diagrams::/)'` | non-zero exit, "0 tests run", any FAIL |
| DOCS-02 | SC2: the `vox` doctest runner never compiles a `vox-diagram` fence; comma split vs whole string | same filter (`doctest_runner_never_compiles_diagram_fences`, `near_miss_info_strings_are_hard_errors`) | non-zero exit, any FAIL |
| DOCS-02 | SC3: exactly one new edge, one exception, re-export, `orphan_exempt` removed | `target/debug/vox ci crate-edges && target/debug/vox ci crate-api-sync && target/debug/vox ci affected-crates --check` ; `cargo run -p vox-arch-check` | non-zero exit, `NEW EDGE`/`UPWARD EDGE`, regeneration request, arch finding naming vox-diagram |
| DOCS-01 | SC1: new fence fails doc lint, fence-aware counting, archive excluded, shrink-only tighten | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::mermaid_gate::/)'` | non-zero exit, "0 tests run", any FAIL |
| DOCS-01 | SC1: baseline seeded from the committed tree (33/21) and the repo lints clean | `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md` | non-zero exit or any `legacy-mermaid-` line |
| DOCS-01 | SC1: pre-push base-ref compare fails on growth, even with an upward-edited baseline | `cargo nextest run -p vox-cli --lib --retries 0 -E 'test(/commands::ci::mermaid_baseline::/)'` | non-zero exit, "0 tests run", any FAIL |

---

## Per-Task Verification Map

| Task ID | Plan | Wave | Requirement | Threat Ref | Secure Behavior | Test Type | Automated Command | File Exists | Status |
|---------|------|------|-------------|------------|-----------------|-----------|-------------------|-------------|--------|
| 87-01-01 | 01 | 1 | DOCS-02 | T-87-01 | one dependency line added; errors fail the lint | tracer (lib unit + lint_in) | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::diagrams::/) \| test(/^pipeline::tests::/)'` | ❌ W0 (`diagrams.rs` created by the task) | ⬜ pending |
| 87-01-02 | 01 | 1 | DOCS-02 | T-87-01, T-87-SC | single 7-line exception; edge proven to need it (mutation); generated artifacts from generators only | gates + mutation | `target/debug/vox ci crate-edges && target/debug/vox ci crate-api-sync && target/debug/vox ci affected-crates --check` ; `cargo run -p vox-arch-check` | ✅ (existing gates) | ⬜ pending |
| 87-01-03 | 01 | 1 | DOCS-02 | T-87-02, T-87-03 | size gate, near-miss info strings fail, doctest runner never compiles diagrams | lib unit + repo walk + mutation | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::diagrams::/)'` ; `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings` | ✅ after 87-01-01 | ⬜ pending |
| 87-02-01 | 02 | 2 | DOCS-01 | T-87-04, T-87-05 | fence-aware whole-tree count; absent baseline admits nothing; invalid baseline is a hard error | tracer (lib unit + fixture trees) + mutation | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::mermaid_gate::/) \| test(/^pipeline::diagrams::/) \| test(/^pipeline::tests::/)'` | ❌ W0 (`mermaid_gate.rs`, `mermaid_gate_tests.rs` created by the task) | ⬜ pending |
| 87-02-02 | 02 | 2 | DOCS-01 | T-87-05 | tighten refuses growth and leaves the file untouched; seed is byte-stable | lib unit + real-repo lint + contracts index | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::mermaid_gate::/)'` ; `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md` ; `target/debug/vox ci contracts-index` | ✅ after 87-02-01 | ⬜ pending |
| 87-03-01 | 03 | 3 | DOCS-01 | T-87-06, T-87-07 | head count may not exceed base; git failure fails closed | tracer (pure + temp git repo) + mutation | `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::mermaid_gate::/)'` ; `cargo nextest run -p vox-cli --lib --retries 0 -E 'test(/commands::ci::mermaid_baseline::/) \| test(/commands::kb::facet_backlog::/)'` | ❌ W0 (`commands/ci/mermaid_baseline.rs` created by the task) | ⬜ pending |
| 87-03-02 | 03 | 3 | DOCS-01, DOCS-02 | T-87-SC | no baseline or vox-diagram change; one exception in the phase | gates | `cargo nextest run -p vox-doc-pipeline` ; `target/debug/vox ci crate-edges && target/debug/vox ci crate-api-sync && target/debug/vox ci affected-crates --check && target/debug/vox ci contracts-index` ; `git diff --numstat 2f0bd1d73 -- contracts/ci/crate-edges.allow.v1.json && git diff --quiet 2f0bd1d73 -- contracts/ci/fan-in-snapshot.v1.json crates/vox-diagram` | ✅ (existing gates) | ⬜ pending |

*Status: ⬜ pending · ✅ green · ❌ red · ⚠️ flaky*

---

## Wave 0 Requirements

Wave 0 is folded into each plan's tracer task (each creates the module and its failing tests first):

- [ ] `crates/vox-doc-pipeline/src/pipeline/diagrams.rs` with its inline test module (87-01 Task 1); `write_readme_sync_fixture`, `write_facet_contracts`, `BASE_FM` in `pipeline/mod.rs` tests made `pub(super)`
- [ ] `crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs` inline tests and `mermaid_gate_tests.rs` (87-02 Task 1)
- [ ] `crates/vox-cli/src/commands/ci/mermaid_baseline.rs` with its temp-git-repo test (87-03 Task 1)

---

## Manual-Only Verifications

| Behavior | Requirement | Why Manual | Test Instructions |
|----------|-------------|------------|-------------------|
| Mutation proofs: crate-edges exception removal (87-01 T2), `is_hard` for `Check` and empty `near_miss_fences` (87-01 T3), lint growth comparison (87-02 T1), gate comparison (87-03 T1) | DOCS-01, DOCS-02 | deliberate break-then-restore of guards (repo rule) | copy the file to the scratchpad, `grep -c` the guarded line before and after, run the named test or gate, see it fail, restore from the copy (`cmp` or `git diff --quiet`), rerun green; record file, mutation, failing test and message in the plan SUMMARY |
| Real baseline seed equals the committed tree | DOCS-01 | the expected numbers (33/21) are a fact about the tree at execution time | `cargo run -p vox-doc-pipeline -- --seed-mermaid-baseline` prints `seeded 33 fences in 21 files`; any difference is explained per file in the 87-02 SUMMARY against `git grep -c` at `2f0bd1d73` |
| Hosted CI on the milestone PR (`linux` runs the fast tier with the new step; full workspace suite because manifests and `contracts/` changed) | DOCS-01, DOCS-02 | nothing is pushed during the phase | push the milestone branch as a draft PR; `Check, Build, and Test (Rust)` must pass; the legacy-Mermaid step bootstraps against `origin/main` (no baseline there) |

---

## Validation Sign-Off

- [ ] All tasks have `<automated>` verify or Wave 0 dependencies
- [ ] Sampling continuity: no 3 consecutive tasks without automated verify
- [ ] Wave 0 covers all MISSING references
- [ ] No watch-mode flags
- [ ] Feedback latency < 60s
- [ ] `nyquist_compliant: true` set in frontmatter

**Approval:** pending
