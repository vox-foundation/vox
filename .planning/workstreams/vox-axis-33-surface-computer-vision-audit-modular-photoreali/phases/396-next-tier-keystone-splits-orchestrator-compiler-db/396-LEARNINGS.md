---
phase: 396
phase_name: "next-tier-keystone-splits-orchestrator-compiler-db"
project: "Vox"
generated: "2026-11-17"
counts:
  decisions: 7
  lessons: 0
  patterns: 0
  surprises: 0
missing_artifacts:
  - "UAT.md"
---

# Phase 396 Learnings: next-tier-keystone-splits-orchestrator-compiler-db

## Decisions

### Applied #[inline(never)] monomorphization firewalls across task_submit.r...
Applied `#[inline(never)]` monomorphization firewalls across `task_submit.rs`, `complete/success/mod.rs`, `complete/fail.rs`, `remote_worker.rs`, `dispatch.rs`, `observer_loop.rs`, `runtime.rs`, `services/routes.rs`, `session/manager/*`, `session/state.rs`, and `hopper/sqlite_store.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-01-SUMMARY.md

---

### Applied #[inline(never)] compilation boundaries across pipeline.rs, lexe...
Applied `#[inline(never)]` compilation boundaries across `pipeline.rs`, `lexer/cursor.rs`, `parser/descent/mod.rs`, `hir/{lower/mod.rs,validate.rs}`, `typeck/{mod.rs,ast_decl_lints.rs}`, and `eval/{mod.rs,stmt.rs,expr.rs}`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-01-SUMMARY.md

---

### Applied #[inline(never)] boundaries across ops_scientia.rs, facade/scien...
Applied `#[inline(never)]` boundaries across `ops_scientia.rs`, `facade/scientia.rs`, `ops_mens_intelligence.rs`, `ops_mens_cloud.rs`, `research.rs`, `codex_legacy.rs`, `store/open.rs`, and `store/ops.rs`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-01-SUMMARY.md

---

### cargo check -p vox-orchestrator
`cargo check -p vox-orchestrator`: `119s -> 11.53s` warm/incremental.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-01-SUMMARY.md

---

### cargo check -p vox-compiler
`cargo check -p vox-compiler`: `55.6s -> 1.65s` warm/incremental.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-01-SUMMARY.md

---

### Added #[inline(never)] boundaries across vox-populi (26.9s -> 6.41s), vo...
Added `#[inline(never)]` boundaries across `vox-populi` (`26.9s -> 6.41s`), `vox-publisher` (`19.1s -> 7.33s`), `vox-scientia` (`14.7s -> 7.31s`), `vox-codegen` (`10.9s -> 7.67s`), `vox-corpus` (`8.6s -> 7.57s`), `vox-cli-ci`, `vox-cli-core`, `vox-sql`, `vox-gamify`, `vox-research-shim`, and `vox-search`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-02-SUMMARY.md

---

### Verified clean compilation across all 11 crates via cargo check and form...
Verified clean compilation across all 11 crates via `cargo check` and formatted modified `.rs` files with `rustfmt --edition 2024`.

**Rationale:** Documented implementation outcome from phase summary.
**Source:** 396-02-SUMMARY.md

## Lessons

## Patterns

## Surprises

