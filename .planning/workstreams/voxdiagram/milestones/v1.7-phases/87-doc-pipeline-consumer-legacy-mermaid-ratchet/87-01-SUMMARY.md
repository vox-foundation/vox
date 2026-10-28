---
phase: 87-doc-pipeline-consumer-legacy-mermaid-ratchet
plan: 01
subsystem: doc-pipeline
tags: [rust, doc-lint, vox-diagram, governance, crate-edges, pulldown-cmark]
status: complete

requires:
  - phase: 83-core-crate-parser-ir-diagnostics
    provides: vox_diagram::check, fence::extract, diagram/* diagnostic codes
provides:
  - "Doc lint validates every vox-diagram fence with diagram/* diagnostics at file:line:col (LintKind::Diagram)"
  - "near-miss info strings (vox-diagram,foo / vox-diagram foo) are hard diagram-fence-info errors"
  - "vox_doc_pipeline::diagram re-export of vox_diagram; the one authorized crate edge with its ledger entry"
  - "diagrams::fence_issues(md) / near_miss_fences(md, lang) text-level entry points for 87-02 (mermaid) and later callers"
affects: [87-02, 87-03, 88]

actuals:
  tokens: 6100
  tasks: 3
  commits: 3
plan_head_before: 13b3fabe12a86892fd3c7f8d0f4cea3f98d6bb8f
plan_head_after: 80ee4f58d3639c9f7e4054e3d2878c98679a404e

tech-stack:
  added: []
  patterns:
    - "LintKind delegation to an issue enum with rule_id/is_hard/workflow/render (copied from FacetIssue)"
    - "Container-prefix column mapping: raw line ends_with body line, prefix = char-count difference"
    - "Scanner-agreement: near_miss_fences uses fence::extract's parser options so lint and renderer agree on nesting"

key-files:
  created:
    - crates/vox-doc-pipeline/src/pipeline/diagrams.rs
  modified:
    - Cargo.toml
    - Cargo.lock
    - crates/vox-doc-pipeline/Cargo.toml
    - crates/vox-doc-pipeline/src/lib.rs
    - crates/vox-doc-pipeline/src/pipeline/types.rs
    - crates/vox-doc-pipeline/src/pipeline/lint.rs
    - crates/vox-doc-pipeline/src/pipeline/mod.rs
    - contracts/ci/crate-edges.allow.v1.json
    - contracts/ci/crate-graph.v1.json
    - docs/src/architecture/layers.toml
    - docs/src/architecture/where-things-live.md
    - docs/src/api/vox-doc-pipeline.generated.md

key-decisions:
  - "Exactly one exceptions entry (vox-doc-pipeline -> vox-diagram, 2026-10-16 authorization); edges array and fan-in snapshot untouched (numstat 7 0)"
  - "Info-string rule id diagram-fence-info; Check issues use the diagnostic's own diagram/* code"
  - "Container-prefix column term applied only when the body line is non-empty and the raw line ends with it, else 0"

patterns-established:
  - "Mutation-proof the ledger exception by deleting it and observing NEW EDGE"

requirements-completed: [DOCS-02]

coverage:
  - id: D1
    description: "Malformed vox-diagram fence fails lint_in with docs/src/<file>.md:<line>:<col>: diagram/<code>: <message>; valid passes; warning-only prints WARN and exits 0"
    requirement: DOCS-02
    verification:
      - kind: unit
        ref: "pipeline::diagrams::tests::{tracer_broken_fence_fails_lint_with_file_line_col,valid_fence_passes_lint,warnings_print_but_do_not_block}"
        status: pass
  - id: D2
    description: "vox doctest runner never compiles vox-diagram / vox-diagram,foo fences (control ```vox fence is rejected); near-miss info strings are hard errors the exact-match scanner would skip"
    requirement: DOCS-02
    verification:
      - kind: unit
        ref: "pipeline::diagrams::tests::{doctest_runner_never_compiles_diagram_fences,near_miss_info_strings_are_hard_errors}"
        status: pass
  - id: D3
    description: "Nested list/blockquote fences map container columns; 4-backtick examples ignored; oversized body is one limit-source error; all committed docs fences valid"
    requirement: DOCS-02
    verification:
      - kind: unit
        ref: "pipeline::diagrams::tests::{nested_fences_map_container_columns,fences_inside_four_backtick_examples_are_not_validated,oversized_fence_is_one_limit_source_error,committed_docs_diagram_fences_are_valid,every_error_in_a_fence_is_reported}"
        status: pass
  - id: D4
    description: "One authorized crate edge admitted by one exception, vox_doc_pipeline::diagram re-export, orphan_exempt removed, generated graph/API regenerated"
    requirement: DOCS-02
    verification:
      - kind: other
        ref: "target/debug/vox ci crate-edges; crate-api-sync; affected-crates --check; cargo run -p vox-arch-check; cargo hakari generate --diff"
        status: pass
---

# Phase 87 Plan 01: Doc-Pipeline Consumer for vox-diagram Summary

**Doc lint now validates every `vox-diagram` fence via `vox_diagram::check` with file:line:col diagnostics, rejects near-miss info strings, and the single authorized `vox-doc-pipeline -> vox-diagram` edge is live with its ledger entry and re-export.**

## Accomplishments

- Task 1 (tracer, `e6cac6d7d`): `LintKind::Diagram(DiagramIssue)`, `pipeline/diagrams.rs`, one added line in `lint_one_file`, six-ish lines in `mod.rs` (+12/-3 vs base; three test helpers made `pub(super)`). A broken fence prints `docs/src/diag.md:18:10: diagram/undeclared-node: ...` and `lint_in` returns 1; the fixed doc returns 0.
- Task 2 (`89f158c85`): exception entry (7-line pure insertion), `pub use vox_diagram as diagram;`, `orphan_exempt` removed from layers.toml, `crate-graph.v1.json` and `vox-doc-pipeline.generated.md` regenerated by their generators, new where-things-live row. `index.generated.md` did not change.
- Task 3 (`80ee4f58d`): `near_miss_fences`, `InfoString` pass, container-prefix columns, and the eight proof tests. `diagrams.rs` is 304 non-blank lines.

## TDD record

- Task 1 RED: with `mod.rs` wired but `check_diagram_fences` not yet called from `lint_one_file`, `tracer_broken_fence_fails_lint_with_file_line_col` failed at its exit-code assertion (`left: 0, right: 1`, lint printed "no hard errors"). GREEN after the one-line `lint.rs` wiring.
- Task 3 RED: with a stub `near_miss_fences` returning empty and no prefix term, `near_miss_info_strings_are_hard_errors` failed (`left: []`, `right: [(1, "vox-diagram,foo"), (5, "vox-diagram foo")]`) and `nested_fences_map_container_columns` failed (`left: col 10`, `right: col 12`); the other six passed (they assert pre-existing scanner/checker behavior). GREEN: 10/10 in the `pipeline::diagrams::` filter, 18/18 with `pipeline::tests::`.

## Mutation proofs

1. Exception is what admits the edge (T-87-01): deleted the new object (`grep -c '"to": "vox-diagram"'` 1 -> 0); `target/debug/vox ci crate-edges` exited 1 with `NEW EDGE not in baseline: vox-doc-pipeline -> vox-diagram`; restored from a scratchpad copy (grep back to 1, numstat `7 0`), `crate-edges: OK (701 live in-tree edges within baseline)`.
2. (a) `DiagramIssue::is_hard` for `Check` mutated to `false`: `tracer_broken_fence_fails_lint_with_file_line_col` failed with `assertion left == right failed, left: 0, right: 1` (diagrams.rs:191, the `lint_doc` exit-code assertion). Restored with `cmp` against the pre-mutation copy; marker grep 1 before, 0 mutated-marker after restore.
3. (b) `near_miss_fences` mutated to return an empty vector: `near_miss_info_strings_are_hard_errors` failed with `left: []`, `right: [(1, "vox-diagram,foo"), (5, "vox-diagram foo")]` (diagrams.rs:247). Restored with `cmp`; full filter green again (10/10).

## Verification

- `cargo nextest run -p vox-doc-pipeline --retries 0 -E 'test(/^pipeline::diagrams::/) | test(/^pipeline::tests::/)'`: 18 passed.
- `target/debug/vox ci crate-edges` OK (701 edges; only a pre-existing stale-baseline warning for vox-gui -> vox-db-types), `crate-api-sync` OK, `affected-crates --check` rc 0, `cargo run -p vox-arch-check` rc 0, `cargo hakari generate --diff` empty, scoped doc lint of `architecture/where-things-live.md` rc 0.
- `fan-in-snapshot.v1.json` unchanged vs `2f0bd1d73`; `doctest.rs` unchanged; `Cargo.toml` of the crate +1 line.
- `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings`: no finding in any file this plan touched (0 hits in `diagrams.rs`, `types.rs`, `mod.rs`, `lint.rs`, `lib.rs`); the command still exits non-zero because of 26 pre-existing findings under `src/claims/` (see Deferred Issues).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Clippy `never_loop` in my own test**
- **Found during:** Task 3 clippy
- **Issue:** `for ... { panic!() }` in `committed_docs_diagram_fences_are_valid`
- **Fix:** collect hard renders and `assert!(hard.is_empty(), ...)`
- **Files modified:** crates/vox-doc-pipeline/src/pipeline/diagrams.rs
- **Commit:** 80ee4f58d

**2. [Rule 2 - Robustness] Panic-free index in column mapping**
- `body_lines.get(span.line - 1)` could underflow on a zero line; used `saturating_sub(1)`.

**3. Stale `vox` binary (Rule 3):** `target/debug/vox` was 12 commits behind; rebuilt with `cargo build -p vox-cli` (background) and never set `VOX_SKIP_FRESHNESS_CHECK`.

`cargo hakari generate --diff` printed nothing, so no workspace-hack change was needed.

## Deferred Issues

Pre-existing, out of scope (not caused by this plan): 26 clippy errors (rust 1.98 lints: collapsible_if, io_other_error, never_loop, manual_strip, needless_range_loop, ...) in `crates/vox-doc-pipeline/src/claims/{mod,gate,ledger,prune,resolver,suppressions}.rs`. They make the plan's `cargo clippy -p vox-doc-pipeline --all-targets -- -D warnings` fail regardless of this plan. Logged in `deferred-items.md`; hosted CI runs the toolchain it pins.

## Known Stubs

None.

## Threat Flags

None. The new surface is the `vox-diagram` dependency edge itself (T-87-01, ledgered) and the lint (T-87-02/03, covered by `oversized_fence_is_one_limit_source_error` and mutation proof (b)).

## Self-Check: PASSED

- diagrams.rs, SUMMARY present; commits `e6cac6d7d`, `89f158c85`, `80ee4f58d` are ancestors of HEAD; `git rev-list --count 13b3fabe1..HEAD` = 3 (matches `commits: 3`).
