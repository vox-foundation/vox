#![allow(missing_docs)]
//! External integration tests for `interpret_workflow_durable` and WorkflowTracker.

use serde_json::Value;
use std::sync::{Arc, Mutex};
use vox_compiler::ast::span::Span;
use vox_compiler::hir::{DefId, HirExpr, HirModule, HirStmt, HirWorkflow};
use vox_workflow_runtime::{interpret_workflow_durable, WorkflowTracker};

fn sp() -> Span { Span { start: 0, end: 0 } }

fn call_stmt(name: &str) -> HirStmt {
    HirStmt::Expr {
        expr: HirExpr::Call(
            Box::new(HirExpr::Ident(name.to_string(), sp())),
            vec![],
            false,
            sp(),
        ),
        span: sp(),
    }
}

fn workflow(name: &str, stmts: Vec<HirStmt>) -> HirModule {
    let mut module = HirModule::default();
    module.workflows.push(HirWorkflow {
        id: DefId(0),
        name: name.to_string(),
        params: vec![],
        return_type: None,
        body: stmts,
        span: sp(),
    });
    module
}

// ── RecordingTracker ─────────────────────────────────────────────────────────

#[derive(Default)]
struct RecordingTracker {
    events: Arc<Mutex<Vec<String>>>,
    /// Activity ids that should be reported "already completed".
    skip_ids: Vec<String>,
}

impl RecordingTracker {
    fn with_skip(ids: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            skip_ids: ids.into_iter().map(|s| s.into()).collect(),
            ..Default::default()
        }
    }
    fn events(&self) -> Vec<String> {
        self.events.lock().unwrap().clone()
    }
    fn push(&self, s: impl Into<String>) {
        self.events.lock().unwrap().push(s.into());
    }
}

impl WorkflowTracker for RecordingTracker {
    fn is_activity_completed(
        &self,
        _wf: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send {
        let done = self.skip_ids.contains(&activity_id.to_string());
        async move { Ok(done) }
    }

    fn on_workflow_started(
        &mut self,
        wf: &str,
        steps: usize,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.push(format!("workflow_started:{wf}:{steps}"));
        async { Ok(()) }
    }

    fn on_activity_started(
        &mut self,
        _wf: &str,
        name: &str,
        _id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.push(format!("activity_started:{name}"));
        async { Ok(()) }
    }

    fn on_activity_completed(
        &mut self,
        _wf: &str,
        name: &str,
        _id: &str,
        _result: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.push(format!("activity_completed:{name}"));
        async { Ok(()) }
    }

    fn on_workflow_completed(
        &mut self,
        wf: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.push(format!("workflow_completed:{wf}"));
        async { Ok(()) }
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[tokio::test]
async fn durable_tracker_receives_all_hooks_for_two_activities() {
    let hir = workflow("w", vec![call_stmt("step_a"), call_stmt("step_b")]);
    let mut tracker = RecordingTracker::default();
    let journal = interpret_workflow_durable(&hir, "w", &mut tracker)
        .await
        .expect("interpret");
    let events = tracker.events();
    assert!(events.iter().any(|e| e.starts_with("workflow_started")));
    assert!(events.iter().any(|e| e == "activity_started:step_a"));
    assert!(events.iter().any(|e| e == "activity_completed:step_a"));
    assert!(events.iter().any(|e| e == "activity_started:step_b"));
    assert!(events.iter().any(|e| e == "activity_completed:step_b"));
    assert!(events.last().map(|e| e.starts_with("workflow_completed")).unwrap_or(false));

    // Journal must contain matching event entries
    let jevents: Vec<_> = journal.iter()
        .filter_map(|v| v["event"].as_str())
        .collect();
    assert!(jevents.contains(&"WorkflowStarted"));
    assert!(jevents.contains(&"ActivityStarted"));
    assert!(jevents.contains(&"ActivityCompleted"));
    assert!(jevents.contains(&"WorkflowCompleted"));
}

#[tokio::test]
async fn durable_tracker_skips_already_completed_activity() {
    let hir = workflow("w2", vec![call_stmt("step_a"), call_stmt("step_b")]);
    // The default activity_id for idx=0 is "w2-0"
    let mut tracker = RecordingTracker::with_skip(["w2-0"]);
    let journal = interpret_workflow_durable(&hir, "w2", &mut tracker)
        .await
        .expect("interpret");
    // step_a (w2-0) should be skipped
    assert!(!tracker.events().iter().any(|e| e == "activity_started:step_a"),
        "skipped activity must not fire on_activity_started");
    // step_b (w2-1) should still run
    assert!(tracker.events().iter().any(|e| e == "activity_started:step_b"));
    // Journal should contain a Skip event for step_a
    let jevents: Vec<_> = journal.iter()
        .filter_map(|v| v["event"].as_str())
        .collect();
    assert!(jevents.contains(&"ActivitySkipped"), "journal must contain ActivitySkipped");
}

#[tokio::test]
async fn empty_workflow_fires_started_and_completed() {
    let hir = workflow("empty_wf", vec![]);
    let mut tracker = RecordingTracker::default();
    interpret_workflow_durable(&hir, "empty_wf", &mut tracker)
        .await
        .expect("interpret");
    assert!(tracker.events().iter().any(|e| e.starts_with("workflow_started:empty_wf:0")));
    assert!(tracker.events().iter().any(|e| e == "workflow_completed:empty_wf"));
}

#[tokio::test]
async fn workflow_not_found_returns_error() {
    let hir = workflow("present", vec![]);
    let mut tracker = RecordingTracker::default();
    let err = interpret_workflow_durable(&hir, "not_present", &mut tracker).await;
    assert!(err.is_err(), "unknown workflow must return Err");
}
