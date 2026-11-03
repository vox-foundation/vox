#![allow(missing_docs)]
//! Task 1.4 acceptance: the runner executes the real workflow/activity body
//! through `WorkflowHost`, not a canned plan-walking replay.

use serde_json::Value;
use serde_json::json;
use std::sync::Mutex;
use vox_compiler::hir::lower_module;
use vox_compiler::lexer::cursor::lex;
use vox_compiler::parser::parse;
use vox_workflow_runtime::workflow::{DefaultTracker, WorkflowTracker, interpret_workflow_durable};

fn hir(src: &str) -> vox_compiler::hir::HirModule {
    lower_module(&parse(lex(src)).expect("parses"))
}

#[derive(Default)]
struct RunArgsTracker {
    args: Mutex<Option<Value>>,
    persist_calls: usize,
}

impl WorkflowTracker for RunArgsTracker {
    fn persist_run_args(
        &mut self,
        args_json: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.persist_calls += 1;
        let result = self
            .args
            .lock()
            .map_err(|error| anyhow::anyhow!("run args mutex poisoned: {error}"))
            .map(|mut stored| *stored = Some(args_json.clone()));
        async move { result }
    }

    fn load_run_args(
        &self,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Value>>> + Send {
        let result = self
            .args
            .lock()
            .map_err(|error| anyhow::anyhow!("run args mutex poisoned: {error}"))
            .map(|stored| stored.clone());
        async move { result }
    }
}

fn completed_return(journal: &[Value]) -> &Value {
    &journal
        .iter()
        .rev()
        .find(|event| event["event"] == "WorkflowCompleted")
        .expect("workflow completed")["return_value"]
}

#[tokio::test]
async fn resume_without_args_reuses_stored_args_without_overwriting_them() {
    const SRC: &str = r#"
workflow echo(amount: int) to int {
    return amount
}
"#;
    let mut tracker = RunArgsTracker::default();

    let first = interpret_workflow_durable(&hir(SRC), "echo", vec![json!(5)], &mut tracker)
        .await
        .expect("first run succeeds");
    assert_eq!(completed_return(&first), &json!(5));

    let resumed = interpret_workflow_durable(&hir(SRC), "echo", vec![], &mut tracker)
        .await
        .expect("resume uses stored arguments");
    assert_eq!(completed_return(&resumed), &json!(5));
    assert_eq!(
        tracker.load_run_args().await.expect("loads stored args"),
        Some(json!([5]))
    );
    assert_eq!(tracker.persist_calls, 1, "run args are write-once");
}

#[tokio::test]
async fn resume_rejects_incoming_args_that_differ_from_stored_args() {
    const SRC: &str = r#"
workflow echo(amount: int) to int {
    return amount
}
"#;
    let mut tracker = RunArgsTracker::default();
    interpret_workflow_durable(&hir(SRC), "echo", vec![json!(5)], &mut tracker)
        .await
        .expect("first run succeeds");

    let error = interpret_workflow_durable(&hir(SRC), "echo", vec![json!(6)], &mut tracker)
        .await
        .expect_err("different resume arguments must be rejected");
    assert!(
        error.to_string().contains("arguments") && error.to_string().contains("mismatch"),
        "error should identify the argument mismatch: {error:#}"
    );
    assert_eq!(
        tracker.load_run_args().await.expect("loads stored args"),
        Some(json!([5]))
    );
    assert_eq!(tracker.persist_calls, 1, "mismatch must not overwrite args");
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
