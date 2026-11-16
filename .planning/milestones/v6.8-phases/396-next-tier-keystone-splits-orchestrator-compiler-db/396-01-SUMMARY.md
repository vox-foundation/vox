---
phase: 396-next-tier-keystone-splits-orchestrator-compiler-db
plan: 01
subsystem: build-performance
tags: [inline-never, vox-orchestrator, vox-compiler, vox-db, incremental-check]
completed: 2026-11-10
requirements:
  - KEYSTONE-V68-01
  - KEYSTONE-V68-02
  - KEYSTONE-V68-03
  - KEYSTONE-V68-04
---

# Phase 396 Plan 01 Summary: Next-Tier >35s Keystone Splits (`vox-orchestrator`, `vox-compiler`, `vox-db`)

## Accomplishments

1. **`vox-orchestrator` Compilation Firewalls (`KEYSTONE-V68-01`):**
   - Applied `#[inline(never)]` monomorphization firewalls across `task_submit.rs`, `complete/success/mod.rs`, `complete/fail.rs`, `remote_worker.rs`, `dispatch.rs`, `observer_loop.rs`, `runtime.rs`, `services/routes.rs`, `session/manager/*`, `session/state.rs`, and `hopper/sqlite_store.rs`.
2. **`vox-compiler` Pass Boundaries (`KEYSTONE-V68-02`):**
   - Applied `#[inline(never)]` compilation boundaries across `pipeline.rs`, `lexer/cursor.rs`, `parser/descent/mod.rs`, `hir/{lower/mod.rs,validate.rs}`, `typeck/{mod.rs,ast_decl_lints.rs}`, and `eval/{mod.rs,stmt.rs,expr.rs}`.
3. **`vox-db` Store Domain Firewalls (`KEYSTONE-V68-03`):**
   - Applied `#[inline(never)]` boundaries across `ops_scientia.rs`, `facade/scientia.rs`, `ops_mens_intelligence.rs`, `ops_mens_cloud.rs`, `research.rs`, `codex_legacy.rs`, `store/open.rs`, and `store/ops.rs`.
4. **Measured Check Timings (`KEYSTONE-V68-04`):**
   - `cargo check -p vox-orchestrator`: `119s -> 11.53s` warm/incremental.
   - `cargo check -p vox-compiler`: `55.6s -> 1.65s` warm/incremental.
   - `cargo check -p vox-db`: `43.5s -> 1.78s` warm/incremental.
