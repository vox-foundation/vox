# Plan 20-04 Summary: Tutorial Verification & Corrections (TUT-01)

## Completed Tasks
- **Task 1: Verifier script, generated record, freshness test, and tut-getting-started rewrite**
  - Created `scripts/docs/tutorial-verify.vox` (`// vox:caps fs env subprocess`, supports `--check` and write mode) verifying `snippets_compile`, `commands_in_registry`, `install_versions_match`, `links_resolve`, and unreasoned `vox:skip`.
  - Created `contracts/documentation/tutorial-verification.v1.json` recording 40-hex commit hash, blob SHAs, and verification checks.
  - Created `docs-astro/tests/unit/tutorial-record.test.mjs` testing record structure, blob freshness against git, check statuses, and `skipsWithoutReason`.
  - Rewrote `docs/src/tutorials/tut-getting-started.md`: updated frontmatter with `status: "current"`, replaced outdated query/mutation code with the real `vox init` scaffold (`table Note`, `server add_note`, `server list_notes`, `component App`, `routes`), removed hardcoded Node/pnpm numbers in favor of linking to installation reference.
- **Task 2: Fix first-app, checkpoints, workflow-durability, and installation reference**
  - `docs/src/reference/installation.md`: updated prerequisites and doctor checks to consistently declare Rust 1.98.1, Node.js >= 18, pnpm >= 9 (aligning with `contracts/toolchain/workspace-toolchain.v1.yaml` and Vite/Astro tooling requirements).
  - `docs/src/tutorials/tut-first-app.md`: dropped redundant `--kind application` flag from `vox init`.
  - `docs/src/tutorials/tut-first-vox-app-checkpoints.md`: added `status: "current"`, removed stale `script-execution` caveat, noted `vox-ml-cli` with `populi` feature requirement, and clarified JSON diagnostics carry `error_code` and `severity`.
  - `docs/src/tutorials/tut-workflow-durability.md`: replaced mdBook `{{#include}}` with an inline verbatim snippet from `examples/golden/durable_workflow_real.vox` (under `ANCHOR: display`), deleted duplicated sentence, inlined a compiled `with { retries: 3, timeout: "30s", initial_backoff: "1s" }` code fence, and noted `vox-ml-cli` provides `vox mens workflow`.
- **Task 3: Actor basics, UI integration, React interop, and test tightening**
  - `docs/src/tutorials/tut-actor-basics.md`: added `status: "current"`, inlined `examples/golden/ref_actors.vox`, removed duplicate spawn sentence, and accurately described Rust codegen lowering based on `crates/vox-codegen/src/codegen_rust/emit/workflow.rs` (`emit_actor_state_structs`, `emit_fn_with_actor_handlers`, `vox_actor_runtime`).
  - `docs/src/tutorials/tut-ui-integration.md`: confirmed doctests and `Id[Task]` syntax pass.
  - `docs/src/tutorials/use-a-react-component-from-vox.md`: confirmed reasoned `vox:skip` markers and golden example matching.
  - Tightened `tutorial-record.test.mjs` to assert `equal(entry.checks[key], "pass")`.
  - Regenerated `tutorial-verification.v1.json` with 28/28 checks passing.

## Verification Evidence
- Record commit: `b64377d402f207a588f9c6b2e71ede6679340105`
- `tutorial-verify.vox --check`: exited 0 with all 7 tutorials passing all 4 checks.
- `node --test tests/unit/tutorial-record.test.mjs`: `3 passed, 0 failed`.
- `jq '[.tutorials[] | select([.checks[]] | all(. == "pass"))] | length'`: `7`.
- `vox ci doctest-md --strict docs/src/tutorials`: all doctests passed.
- `vox-doc-pipeline --lint-only`: passed with 0 hard errors.
- Actor codegen checked: `crates/vox-codegen/src/codegen_rust/emit/workflow.rs` (`emit_actor_state_structs`, `emit_fn_with_actor_handlers`).
