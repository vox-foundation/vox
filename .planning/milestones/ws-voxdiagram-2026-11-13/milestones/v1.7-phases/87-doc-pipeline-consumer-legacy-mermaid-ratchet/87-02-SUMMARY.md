---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
plan: 02
subsystem: doc-pipeline
tags: [rust, doc-lint, ratchet, mermaid, baseline, vox-diagram]
status: complete

requires:
  - phase: 87-01
    provides: diagrams::near_miss_fences, LintKind delegation pattern, vox_diagram::fence::extract consumer
provides:
  - "Whole-tree legacy-Mermaid baseline lint pass (mermaid_gate::lint) run on every lint_in, scoped or not"
  - "--seed-mermaid-baseline / --tighten-mermaid-baseline tools (tighten is shrink-only and refuses before writing)"
  - "contracts/documentation/legacy-mermaid-baseline.v1.yaml seeded from the committed tree: 33 fences in 21 files"
  - "types::MermaidIssue + LintKind::LegacyMermaid; mermaid_gate::{MERMAID_BASELINE_REL, MERMAID_HEAL} for 87-03"
affects: [87-03, 88]

requirements-completed: []

actuals:
  tokens: 15000
  tasks: 2
  commits: 2
plan_head_before: d37692c97a56703159e9ad70c92bf423b37b0a9b
plan_head_after: e55ee4ac12b78b045aa35b84456fa3007734ad10

tech-stack:
  added: []
  patterns:
    - "Shrink-only per-file count ratchet (BTreeMap path -> n) mirroring facet_backlog.rs, with a generated header and render(parse(x)) == x byte stability"
    - "Whole-tree pass in lint_in beside lint_readme_sync_paths so unchanged files stay held to the baseline"

key-files:
  created:
    - crates/vox-doc-pipeline/src/pipeline/mermaid_gate.rs
    - crates/vox-doc-pipeline/src/pipeline/mermaid_gate_tests.rs
    - contracts/documentation/legacy-mermaid-baseline.v1.yaml
  modified:
    - crates/vox-doc-pipeline/src/pipeline/types.rs
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - crates/vox-doc-pipeline/src/pipeline/lint.rs
    - contracts/index.yaml

key-decisions:
  - "Seed numbers match the plan exactly (33/21); no hand edit and no reconciliation against git grep was needed"
  - "A missing baseline admits nothing; an invalid baseline or unreadable doc is one hard legacy-mermaid-unreadable error (fail closed)"

patterns-established:
  - "Mutation-proof a ratchet by shifting the growth comparison and by disabling the tighten refusal"

coverage:
  - id: D1
    description: "An extra mermaid fence in a listed file, or any fence in an unlisted file, fails lint_in with a message naming count and allowed; works when a different file is the --paths target"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "pipeline::mermaid_gate::tree_tests::{tracer_extra_fence_fails_lint,unlisted_file_fails_even_when_another_file_is_linted}"
        status: pass
  - id: D2
    description: "Counting is fence-aware: 4-backtick examples, prose mentions and docs/src/archive never count; tilde and list-nested fences do; near-miss info strings are hard errors"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "pipeline::mermaid_gate::tests::counts_are_fence_aware, tree_tests::{examples_prose_and_archive_are_not_counted,near_miss_mermaid_info_is_a_hard_error}"
        status: pass
  - id: D3
    description: "Fail closed: missing baseline admits nothing, invalid baseline is exactly one Unreadable, stale entry warns but exits 0, schema/unknown keys rejected"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "pipeline::mermaid_gate::{tests::parse_rejects_unknown_keys_and_other_schema_versions,tree_tests::missing_baseline_admits_nothing,invalid_baseline_is_one_hard_error,stale_entry_warns_but_exits_zero}"
        status: pass
  - id: D4
    description: "Seed is byte-stable, tighten only lowers/drops and refuses (file untouched) on growth, tighten without a baseline errors"
    requirement: DOCS-01
    verification:
      - kind: unit
        ref: "pipeline::mermaid_gate::tree_tests::{seed_then_tighten_then_refuse_growth,tighten_without_baseline_is_an_error}"
        status: pass
  - id: D5
    description: "Committed baseline = 33 fences in 21 files, second seed leaves identical bytes, contracts-index OK, scoped real-repo lint prints no legacy-mermaid line"
    requirement: DOCS-01
    verification:
      - kind: other
        ref: "cargo run -p vox-doc-pipeline -- --seed-mermaid-baseline (x2, diff -q); cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md rc 0; target/debug/vox ci contracts-index OK (198 entries)"
        status: pass
---

# Phase 87 Plan 02: Legacy-Mermaid Shrink-Only Baseline Summary

**Doc lint now holds every `docs/src` file (archive excluded) to a seeded per-file count of legacy ```` ```mermaid ```` fences, fence-aware via `vox_diagram::fence::extract`, on every run; the baseline is seeded from the committed tree (33 fences / 21 files) and only the seed/tighten tools write it.**

## Accomplishments

- Task 1 (tracer, `5c2e935fe`): `types::MermaidIssue` + `LintKind::LegacyMermaid`; `lint::gather_md_files` made `pub(crate)`; `pipeline/mermaid_gate.rs` (baseline parse/render/load, `docs`, `counts`, `lint`, `MERMAID_HEAL`, `impl MermaidIssue`); one `mermaid_gate::lint(root, ..)` call after the README-sync pass plus the four `LegacyMermaid` match arms in `mod.rs`.
- Task 2 (`e55ee4ac1`): `seed` / `tighten`, the two CLI flags, the real seeded baseline (`seeded 33 fences in 21 files`, zero `archive/` entries) and the `documentation-legacy-mermaid-baseline` entry in `contracts/index.yaml`.
- `mod.rs` is +34/-3 against `2f0bd1d73` across 87-01 and 87-02 (limit 45). `mermaid_gate.rs` 304 and `mermaid_gate_tests.rs` 223 non-blank lines (< 400).

## TDD record

- Task 1 RED: with the `mermaid_gate::lint(root, &mut lint_errors)` call in `mod.rs` commented out (module and tests otherwise in place), 5 of 9 `pipeline::mermaid_gate::` tests failed (`tracer_extra_fence_fails_lint` at the exit-code assertion `left: 0, right: 1`; `unlisted_file_...`, `missing_baseline_...`, `invalid_baseline_...`, `near_miss_...` likewise). GREEN after restoring the one line (plus correcting my own test arithmetic: the fixture body starts on line 15, not 14). Final: 27 passed over `mermaid_gate | diagrams | tests::` filters.
- Task 2: honest note: the `seed`/`tighten` implementation and their tests were written in the same edit, so there is no pre-implementation RED for them; the failing-test evidence is the mutation proof below (the tighten refusal test fails when the refusal is disabled). Final: 11 `pipeline::mermaid_gate::` tests, 29 across all three filters, all green.

## Mutation proofs

1. Growth comparison (T-87-04): `if count > allowed {` (grep 1) mutated to `if count > allowed + 1 {` (grep 1, original grep 0). Failing: `tracer_extra_fence_fails_lint` (`mermaid_gate_tests.rs:62`, `left: 0, right: 1`), `unlisted_file_fails_even_when_another_file_is_linted` (`:83`, `left: 0, right: 1`) and `missing_baseline_admits_nothing` (`:114`, `left: []`, `right: [(15, Growth { count: 1, allowed: 0 })]`). Restored from a scratchpad copy; `diff -q` identical, mutated-marker grep 0, 9/9 green again.
2. Tighten refusal (T-87-05): `if !over.is_empty() {` mutated to `if false && !over.is_empty() {` (grep before 1, after 1). Failing: `seed_then_tighten_then_refuse_growth` (`mermaid_gate_tests.rs:226`, `called Result::unwrap_err() on an Ok value: "legacy-Mermaid baseline already tight"`). Restored, `diff -q` identical, marker grep 0, 11/11 green.

## Verification

- `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::mermaid_gate::/) | test(/^pipeline::diagrams::/) | test(/^pipeline::tests::/)'`: 29 passed.
- `cargo run -p vox-doc-pipeline -- --seed-mermaid-baseline` printed `seeded 33 fences in 21 files`; second run left identical bytes (`diff -q`); `grep -c '^  docs/src/'` = 21; `grep -c 'docs/src/archive/'` = 0.
- `cargo run -p vox-doc-pipeline -- --lint-only --paths architecture/where-things-live.md`: rc 0, 0 `legacy-mermaid` lines (the whole-tree pass found nothing against the real baseline).
- `target/debug/vox ci contracts-index` (binary rebuilt at HEAD; freshness check not skipped): `contracts-index OK (schema v1, 198 entries)`.
- `cargo clippy -p vox-doc-pipeline --all-targets`: no finding in `mermaid_gate*.rs`, `types.rs`, `mod.rs`, or the `lint.rs` line touched (pre-existing `claims/` findings untouched).
- No crate-edge baseline touched; no dependency added.

## Deviations from Plan

None - plan executed as written. (Only the fixture-line-number correction in my own tests, noted above.)

## Deferred Issues

Pre-existing clippy findings under `crates/vox-doc-pipeline/src/claims/` remain out of scope (see `deferred-items.md` from 87-01).

## Known Stubs

None.

## Threat Flags

None. New surface is the baseline read (T-87-05) and the fence counter (T-87-04), both covered by tests and the mutation proofs above.

## Self-Check: PASSED

- `mermaid_gate.rs`, `mermaid_gate_tests.rs`, the baseline YAML and this SUMMARY exist; commits `5c2e935fe` and `e55ee4ac1` are ancestors of HEAD; `git rev-list --count d37692c97..e55ee4ac1` = 2 (matches `commits: 2`).
