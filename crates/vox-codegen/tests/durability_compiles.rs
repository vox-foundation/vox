//! Phase 1 closure proof: the codegen output for `workflow` + `activity`
//! resolves against `vox-workflow-runtime`.
//!
//! Approach: **Option A + compile-link belt-and-suspenders**.
//!
//! Why this approach (and not a temp-cargo-crate or `trybuild`):
//! - The existing `durability_lowering.rs` test pattern already drives `emit_fn`
//!   and asserts on the emitted Rust. We reuse that pattern for speed.
//! - To also prove the *paths* codegen emits actually resolve at link time,
//!   the top of this file `use`s every runtime symbol at the exact public
//!   path the emitter writes (`::vox_workflow_runtime::workflow::*` and
//!   `::vox_workflow_runtime::VoxDbTracker`). If any symbol is removed or
//!   relocated, this test file fails to *compile* — which is stronger than
//!   the string assertion and faster than spinning up a temp crate.
//! - `vox-workflow-runtime` is added as a `[dev-dependencies]` entry of
//!   `vox-codegen` solely to enable these `use` statements. It does not
//!   reach production codegen (which still emits strings, not links).
//!
//! Closes Phase 1 of
//! `docs/superpowers/plans/2026-05-23-durable-functions-completion.md`.
//! Task 6.1 swaps the Phase 1 `DefaultTracker` / `journal::execute` proof for
//! the real ADR-021 path: generated workflows persist through `VoxDbTracker`
//! (feature `sql`) and activities are no longer wrapped in `journal::execute`
//! (deleted in the same change — the interpreter's `WorkflowTracker` owns
//! record/replay, not a second wrapper around the generated fn body).

// --- Compile-link proof ---------------------------------------------------
// These `use` statements fail to compile if any runtime symbol is removed,
// renamed, or moved away from the path codegen emits.
#[allow(unused_imports)]
use vox_workflow_runtime::VoxDbTracker as _adr021_vox_db_tracker;
#[allow(unused_imports)]
use vox_workflow_runtime::workflow::{
    current_hir_module as _phase1_current_hir_module,
    extract_terminal_return as _phase1_extract_terminal_return,
};
// `interpret_workflow_durable` is also written by the emitter; pin it too so
// a rename is caught at compile time.
#[allow(unused_imports)]
use vox_workflow_runtime::workflow::interpret_workflow_durable as _phase1_interpret_workflow_durable;

// --- Emit-string proof ----------------------------------------------------
use vox_codegen::codegen_rust::emit::emit_fn;
use vox_compiler::hir::{DurabilityKind, lower_module};
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;

/// Workflow emit must call every Phase 1 runtime entry-point that
/// `durability_lower.rs` writes: `current_hir_module`,
/// `interpret_workflow_durable`, and `extract_terminal_return`.
#[test]
fn workflow_emit_references_all_phase_1_workflow_symbols() {
    let src = "workflow wf() to int { return 7 }";
    let module = parse(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow))
        .expect("workflow lowered");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);

    assert!(
        rust.contains("::vox_workflow_runtime::workflow::current_hir_module"),
        "workflow emit MUST call current_hir_module (Task 1.1); got:\n{rust}"
    );
    assert!(
        rust.contains("::vox_workflow_runtime::workflow::interpret_workflow_durable"),
        "workflow emit MUST call interpret_workflow_durable; got:\n{rust}"
    );
    assert!(
        rust.contains("::vox_workflow_runtime::workflow::extract_terminal_return"),
        "workflow emit MUST call extract_terminal_return (Task 1.2); got:\n{rust}"
    );
}

/// Task 6.1 review: generated wrappers must use the same DB resolution as
/// `emit_durable_boot_prelude` — always `DbConfig::resolve_canonical()`, never
/// a CWD `vox-workflow-debug.db` fallback when `VOX_DB_*` is unset. The
/// ADR-021 gate already sets `VOX_DB_PATH` for isolation.
#[test]
fn workflow_emit_uses_canonical_db_resolution() {
    let src = "workflow wf() to int { return 7 }";
    let module = parse(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow))
        .expect("workflow lowered");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);

    assert!(
        rust.contains("DbConfig::resolve_canonical"),
        "workflow emit MUST resolve DB via DbConfig::resolve_canonical (same as durable boot); got:\n{rust}"
    );
    assert!(
        !rust.contains("vox-workflow-debug.db"),
        "workflow emit must NOT fall back to CWD vox-workflow-debug.db; got:\n{rust}"
    );
    assert!(
        !rust.contains("DbConfig::local"),
        "workflow emit must NOT use DbConfig::local; got:\n{rust}"
    );
}

/// Task 6.1 review: `emit_fn` keeps `-> i64` for `workflow wf() to int`.
/// The body must not use `?` — that is only valid in a `Result` function.
/// Failures use expect/match so the generated signature still matches the
/// Vox type.
#[test]
fn workflow_to_int_wrapper_has_no_try_operator() {
    let src = "workflow wf() to int { return 7 }";
    let module = parse(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Workflow))
        .expect("workflow lowered");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);

    assert!(
        rust.contains("async fn wf() -> i64"),
        "to int wrapper must keep the Vox return type (i64), not Result; got:\n{rust}"
    );
    assert!(
        !rust.contains('?'),
        "to int wrapper must not use ? (invalid in a non-Result fn); got:\n{rust}"
    );
}

/// Task 6.1: activities are invoked by the workflow interpreter via
/// `HostRequest::Begin` / `HostDecision::Execute`|`Replay` — the
/// interpreter's `WorkflowTracker` (`VoxDbTracker` in the generated path)
/// owns record/replay. Wrapping the generated activity fn body in
/// `journal::execute` was dead code (unreachable on the interpreter's call
/// path) with a second, conflicting id scheme; `journal::execute` was
/// deleted in the same change. See the equivalent, more detailed assertion
/// in `durability_lowering.rs`
/// (`activity_lowers_to_plain_async_body_not_journal_execute`).
#[test]
fn activity_emit_does_not_reference_journal_execute() {
    let src = "activity act() to int { return 9 }";
    let module = parse(lex(src)).expect("parse");
    let hir = lower_module(&module);
    let func = hir
        .functions
        .iter()
        .find(|f| f.durability == Some(DurabilityKind::Activity))
        .expect("activity lowered");
    let rust = emit_fn(func, Some(&hir.inferred_types), &[]);

    assert!(
        !rust.contains("journal::execute"),
        "activity emit must NOT call journal::execute (deleted Task 6.1); got:\n{rust}"
    );
}
