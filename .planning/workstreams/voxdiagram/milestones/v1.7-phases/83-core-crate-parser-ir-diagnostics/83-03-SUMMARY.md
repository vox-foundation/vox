---
phase: 83-core-crate-parser-ir-diagnostics
plan: 03
subsystem: diagram-core
tags: [rust, pulldown-cmark, fence, markdown, sequence-ir, serde]

requires:
  - phase: 83-01
    provides: "span::Location, lib.rs module/doc style, pulldown-cmark dependency"
provides:
  - "fence::extract(md, info) -> Vec<Fence>: CommonMark-correct fenced-block scanner (exact trimmed info match)"
  - "fence::normalize: CRLF/CR to LF + trim_end (Rust half of Phase 89 manifest key)"
  - "sequence_ir::{SequenceIr, Actor, SeqEvent::{Message, Note}} serde types (no parser)"
affects: [84, 86, 87, 88, 89]

key-files:
  created:
    - crates/vox-diagram/src/fence.rs
    - crates/vox-diagram/src/sequence_ir.rs
    - crates/vox-diagram/tests/fence_scan.rs
    - crates/vox-diagram/tests/sequence_ir.rs
  modified:
    - crates/vox-diagram/src/lib.rs

key-decisions:
  - "Fence body comes from pulldown-cmark Text events (indentation/blockquote prefix already stripped); open_line/body_range/closed are derived from the block's offset range, with closed requiring a distinct last line of only the opener's fence char at >= opener length"
  - "No lowercasing of the info string (deliberate difference from the claims extractor)"

patterns-established:
  - "Types-only modules (sequence_ir) expose no pub fn so tdd-guard has nothing to flag"

requirements-completed: []  # DGM-01 stays Pending: invalid-input diagnostics, fmt, fixes, mermaid are outstanding in Plans 04-07

duration: about 20min
completed: 2026-10-16
status: complete

actuals:
  tokens: 3100
  tasks: 2
  commits: 2
plan_head_before: 18722c0bbaa3578dc31fa438f68ca1b28eacd2ea
plan_head_after: de870d96dd6e96ceddc5f75a92899b09a8c413ee
---

# Phase 83 Plan 03: Fence scanner and sequence IR types Summary

**A greenfield pulldown-cmark fence scanner returns exactly the `vox-diagram` fences of any markdown/MDX string (body, 1-based open line, byte range, closed flag), plus serde-round-tripping sequence IR types with no parser.**

## Accomplishments

- `fence::extract` walks `Parser::new_ext(md, ENABLE_YAML_STYLE_METADATA_BLOCKS).into_offset_iter()`; a fence matches only when `info.trim() == info` exactly, so `vox`, `vox-diagram,x`, `mermaid`, prose/inline-code mentions and examples nested in a 4-backtick `md` block are all excluded.
- Tilde fences, list-item indentation, blockquote `>` prefixes, unclosed-at-EOF, shorter-than-opener closers, empty bodies, CRLF input and YAML frontmatter plus an MDX `import` line are all covered; CRLF and LF produce equal bodies after `normalize`.
- `sequence_ir.rs` defines `SequenceIr`, `Actor`, `SeqEvent` (`#[serde(tag = "type", rename_all = "kebab-case")]`, `Message{dashed}`, `Note{over}`) with zero `pub fn`; `lib.rs` gained `pub mod fence;` and `pub mod sequence_ir;`.
- Final run: 17 tests pass (`fence_scan` 11, `fence::tests` 4, `sequence_ir` 2); `cargo check -p vox-diagram --all-targets` clean; `cargo fmt -p vox-diagram` applied.

## Task Commits

1. Task 1 (tracer): fence scanner - `dfd8fbfb9`
2. Task 2: sequence IR types - `de870d96d`

## TDD Evidence

- Task 1 RED: with `extract` stubbed to return empty and `normalize` to the identity, `cargo nextest run -p vox-diagram -E 'binary(fence_scan) | test(fence::tests)'` ran 15 tests: 1 passed (`inline_code_mention_is_not_a_fence`, vacuously true on an empty stub), 14 failed. GREEN: 15/15 pass after implementation.
- Task 2 RED: `tests/sequence_ir.rs` failed to compile (`E0432: unresolved import vox_diagram::sequence_ir`). GREEN: 2/2 pass.
- Tracer gate: `<verify>` re-run end to end after Task 1 and again in the final combined run (17/17); no expansion issues.

## Deviations from Plan

None - plan executed as written. The plan described this as running in parallel with Plan 02, but Plan 02 was already complete and committed; only the five `files_modified` were touched (Cargo.toml/Cargo.lock untouched).

## Known Stubs

None.

## Threat Flags

None. T-83-08 (exact info matching) and T-83-09 (CommonMark parser instead of regex) mitigations are implemented and tested; mutation-style proof was not requested by the plan.

## Self-Check: PASSED

- fence.rs, sequence_ir.rs, fence_scan.rs, sequence_ir.rs (tests) exist; both commits are ancestors of HEAD.
