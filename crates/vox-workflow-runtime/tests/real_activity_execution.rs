#![allow(missing_docs)]
//! Task 1.4 acceptance: the runner executes the real workflow/activity body
//! through `WorkflowHost`, not a canned plan-walking replay.

use serde_json::json;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_workflow_runtime::workflow::{DefaultTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

#[tokio::test]
async fn activity_body_return_is_journalled() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Ok("tx_5") }
workflow checkout(amount: int) to Result[str] {
    return charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(5)], &mut tracker)
        .await
        .expect("runs");
    let completed = journal
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .expect("completed");
    assert_eq!(
        completed["return_value"],
        json!({"__vox": "Ok", "value": "tx_5"}),
        "canned LocalActivity payload is a failure"
    );
}

#[tokio::test]
async fn question_mark_sees_result_err() {
    const SRC: &str = r#"
activity charge(amount: int) to Result[str] { return Error("declined") }
workflow checkout(amount: int) to Result[str] {
    let tx = charge(amount)?
    return Ok(tx)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect("Err is a value, not a runner failure");
    let completed = journal
        .iter()
        .rev()
        .find(|e| e["event"] == "WorkflowCompleted")
        .expect("completed");
    assert_eq!(
        completed["return_value"],
        json!({"__vox": "Err", "value": "declined"})
    );
}

#[tokio::test]
async fn patch_request_is_journalled() {
    const SRC: &str = r#"
activity charge(amount: int) to int { return amount }
workflow checkout(amount: int) to int {
    workflow.version("add-audit-v2", 1, 2)
    return charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let journal = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect("runs");
    assert!(
        journal
            .iter()
            .any(|e| e["event"] == "WorkflowPatch" && e["change_id"] == "add-audit-v2"),
        "HostRequest::Patch must call handle_workflow_patch; got {journal:#?}"
    );
}

// Note: the task brief's literal source for this test used
// `@place(mens) activity remote_charge(...)`. That does not parse in this
// tree — `PlacementHint` (crates/vox-compiler/src/ast/decl/fundecl.rs) only
// accepts `native` / `gui` / `shared`, and an unrecognized `@place(...)`
// argument is a hard parse error by design (see the comment at
// crates/vox-compiler/src/parser/descent/decl/head_fn.rs:187-201: "Unknown or
// missing arguments are hard errors — placement is correctness-affecting").
// Mesh/PopuliActivity detection in this codebase (both the existing linearizer
// in `workflow::plan` and the new `activity_is_mesh` in `workflow::run`) keys
// off the `mesh_` name prefix instead, so this test uses that convention —
// it exercises the same runner code path the brief intended.
#[tokio::test]
async fn mesh_activity_is_a_loud_error() {
    const SRC: &str = r#"
activity mesh_remote_charge(amount: int) to int { return amount }
workflow checkout(amount: int) to int {
    return mesh_remote_charge(amount)
}
"#;
    let mut tracker = DefaultTracker;
    let err = interpret_workflow_durable(&hir(SRC), "checkout", vec![json!(1)], &mut tracker)
        .await
        .expect_err("mesh is out of this plan");
    let msg = format!("{err:#}");
    assert!(
        msg.contains("mesh") || msg.contains("Populi") || msg.contains("mens"),
        "must name mesh, not silently run local; got {msg}"
    );
}
