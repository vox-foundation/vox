# Phase 81: `.vox` Language, Compiler, Interpreter & Codegen End-to-End Execution Parity — Context

**Gathered:** 2026-10-16
**Status:** Ready for execution
**Mode:** Autonomous

<domain>
## Phase Boundary

Deliver end-to-end execution parity across `.vox` first-party automation scripts (`scripts/*.vox`), full-stack and agentic constructs (`table`, `query`, `mutation`, `tool`, `component`, `workflow`, `activity`, `actor`), `vox init` project templates (`default`, `simulation-rpg`, `script`), span-accurate LLM compiler diagnostics (`vox check --for-llm`), and deterministic `@test` execution (`VOX-01..05`).

</domain>

<decisions>
## Implementation Decisions

### Scaffold Modernization (`VOX-03`)
- Modernize `crates/vox-project-scaffold/src/lib.rs` so `AGENT_KIND`, `WORKFLOW_KIND`, and application templates (`default`, `simulation-rpg`, `script`, `chatbot`, `dashboard`, `api`) emit canonical bare-keyword `.vox` syntax (`actor`, `activity`, `workflow`, `table`, `query`, `to` return syntax, and `//` line comments) instead of retired `@agent_def`, `@workflow_def`, `->`, or `#` comments.

### End-to-End Compiler, Interpreter & Multi-Target Codegen Parity (`VOX-01`, `VOX-02`, `VOX-04`, `VOX-05`)
- Verify all top-level first-party `.vox` scripts in `scripts/*.vox` through `run_frontend_str_with_options` (`script_mode: true`) and `Interpreter` without panics (`VOX-01`).
- Verify canonical full-stack + agentic `.vox` modules across `Interpreter`, `ProjectionBundle` (`Target::Interpreter`, `Target::RustAxum`, `Target::RustTauri`, `Target::TypeScript`), `vox_codegen::codegen_rust::generate` (`AxumLocalServer` + `TauriApp`), and `vox_codegen::codegen_ts::generate` (`VOX-02`).
- Verify `check_file` (`vox check --for-llm`) emits span-accurate `VoxCompilerDiagnosticPayload` JSON diagnostics without panicking across malformed inputs (`VOX-04`).
- Verify `@test` blocks lower into `hir.tests` and execute deterministically in `vox_compiler::eval::Interpreter` (`VOX-05`).

</decisions>

<canonical_refs>
## Canonical References

- `.planning/workstreams/unified/ROADMAP.md` — Phase 81 success criteria (`VOX-01..05`)
- `.planning/workstreams/unified/REQUIREMENTS.md` — `VOX-01..05`
- `crates/vox-project-scaffold/src/lib.rs` — Project and template scaffolding
- `crates/vox-compiler/src/pipeline.rs` — `run_frontend_str_with_options` and `check_file`
- `crates/vox-compiler/src/semcov_struct_pipeline_tests.rs` — Structural and end-to-end compiler/interpreter/codegen parity tests

</canonical_refs>
