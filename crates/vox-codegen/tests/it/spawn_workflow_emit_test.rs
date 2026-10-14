/// Task 8D: Spawn Rust emission. `WorkflowVersion` emission (Task 6.1 update:
/// inert no-op, not a `compile_error!` stub — see `workflow_version_emits_inert_noop_not_compile_error`).
use vox_codegen::codegen_rust::emit::emit_expr;
use vox_compiler::ast::span::Span;
use vox_compiler::hir::{HirExpr, HirWorkflowVersion};

fn span() -> Span {
    Span { start: 0, end: 0 }
}

#[test]
fn spawn_emits_tokio_spawn() {
    let inner = HirExpr::IntLit(42, span());
    let expr = HirExpr::Spawn(Box::new(inner), span());
    let result = emit_expr(&expr);
    assert!(
        result.contains("tokio::spawn"),
        "Spawn must emit tokio::spawn, got: {result}"
    );
    assert!(
        result.contains("async move"),
        "Spawn must wrap in async move, got: {result}"
    );
    assert!(
        result.contains("42"),
        "Spawn must include inner expression, got: {result}"
    );
}

#[test]
fn workflow_version_emits_inert_noop_not_compile_error() {
    // Task 6.1 (ADR-021): generated `workflow` fns proxy their entire body to
    // `interpret_workflow_durable` (`emit_workflow_body` never runs
    // `emit_stmt`/`emit_expr` over the workflow's own statements), so the
    // *real* `workflow.version(...)` gate is the interpreter's
    // `WorkflowPatch` handling, not Rust codegen for this expression. This
    // arm is unreachable on the workflow path; asserting the old
    // `compile_error!` here would misrepresent already-backed (Task 1.2–1.4)
    // runtime support as unimplemented.
    let expr = HirExpr::WorkflowVersion(HirWorkflowVersion {
        change_id: "add-discount-field".to_string(),
        min: 1,
        max: 3,
        span: span(),
    });
    let result = emit_expr(&expr);
    assert!(
        !result.contains("compile_error!"),
        "WorkflowVersion is backed by the interpreter (ADR-021); it must not \
         hard-fail the build here, got: {result}"
    );
    assert_eq!(
        result.trim(),
        "()",
        "WorkflowVersion should emit an inert unit expression, got: {result}"
    );
}
