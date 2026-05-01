#![allow(missing_docs)]
//! External integration tests for `interpret_workflow_durable` and WorkflowTracker.

use jsonschema::validator_for;
use serde_json::Value;
use std::sync::{Arc, Mutex};
use vox_compiler::ast::span::Span;
use vox_compiler::hir::{DefId, HirExpr, HirMatchArm, HirModule, HirPattern, HirStmt};
use vox_compiler::hir::nodes::{DurabilityKind, HirFn};
use vox_db::{DbConfig, VoxDb};
use vox_workflow_runtime::{
    VoxDbTracker, WORKFLOW_JOURNAL_VERSION, WorkflowTracker, interpret_workflow_durable,
    plan_workflow_activities,
};

fn sp() -> Span {
    Span { start: 0, end: 0 }
}

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

fn expr_stmt(expr: HirExpr) -> HirStmt {
    HirStmt::Expr { expr, span: sp() }
}

fn call_stmt_with_int_arg(name: &str, value: i64) -> HirStmt {
    HirStmt::Expr {
        expr: HirExpr::Call(
            Box::new(HirExpr::Ident(name.to_string(), sp())),
            vec![vox_compiler::hir::HirArg {
                name: None,
                value: HirExpr::IntLit(value, sp()),
            }],
            false,
            sp(),
        ),
        span: sp(),
    }
}

fn call_stmt_with_string_arg(name: &str, value: &str) -> HirStmt {
    HirStmt::Expr {
        expr: HirExpr::Call(
            Box::new(HirExpr::Ident(name.to_string(), sp())),
            vec![vox_compiler::hir::HirArg {
                name: None,
                value: HirExpr::StringLit(value.to_string(), sp()),
            }],
            false,
            sp(),
        ),
        span: sp(),
    }
}

fn workflow(name: &str, stmts: Vec<HirStmt>) -> HirModule {
    let mut module = HirModule::default();
    module.functions.push(HirFn {
        id: DefId(0),
        name: name.to_string(),
        generics: vec![],
        params: vec![],
        return_type: None,
        body: stmts,
        is_component: false,
        is_async: false,
        is_pub: false,
        is_mobile_native: false,
        is_pure: false,
        effects: vec![],
        is_llm: false,
        llm_model: None,
        is_deprecated: false,
        schedule_interval: None,
        durability: Some(DurabilityKind::Workflow),
        actor_state_fields: vec![],
        postconditions: vec![],
        span: sp(),
    });
    module
}

fn assert_has_fields(entry: &Value, fields: &[&str]) {
    for field in fields {
        assert!(
            entry.get(*field).is_some(),
            "event `{}` missing required field `{field}`: {entry}",
            entry
                .get("event")
                .and_then(Value::as_str)
                .unwrap_or("<unknown>")
        );
    }
}

fn assert_journal_entries_validate_v1_schema(entries: &[Value]) {
    let schema_json: Value = serde_json::from_str(include_str!(
        "../../../contracts/workflow/workflow-journal.v1.schema.json"
    ))
    .expect("parse workflow journal schema");
    let validator = validator_for(&schema_json).expect("compile workflow journal schema");
    for entry in entries {
        if let Err(err) = validator.validate(entry) {
            let event_name = entry["event"].as_str().unwrap_or("<unknown>");
            panic!(
                "journal event `{event_name}` failed v1 schema validation: {err}; entry={entry}"
            );
        }
    }
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

struct FailAfterPersistTracker {
    inner: VoxDbTracker,
    fail_after_first_complete: bool,
}

impl WorkflowTracker for FailAfterPersistTracker {
    fn is_activity_completed(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send {
        self.inner.is_activity_completed(workflow_name, activity_id)
    }

    fn load_activity_result(
        &self,
        workflow_name: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<Option<Value>>> + Send {
        self.inner.load_activity_result(workflow_name, activity_id)
    }

    fn on_workflow_started(
        &mut self,
        workflow_name: &str,
        plan_len: usize,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.inner.on_workflow_started(workflow_name, plan_len)
    }

    fn on_activity_started(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.inner
            .on_activity_started(workflow_name, activity_name, activity_id)
    }

    fn on_activity_completed(
        &mut self,
        workflow_name: &str,
        activity_name: &str,
        activity_id: &str,
        result: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        let should_fail = std::mem::take(&mut self.fail_after_first_complete);
        let fut =
            self.inner
                .on_activity_completed(workflow_name, activity_name, activity_id, result);
        async move {
            fut.await?;
            if should_fail {
                anyhow::bail!("simulated crash after activity completion persisted");
            }
            Ok(())
        }
    }

    fn on_workflow_completed(
        &mut self,
        workflow_name: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        self.inner.on_workflow_completed(workflow_name)
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
    assert!(
        events
            .last()
            .map(|e| e.starts_with("workflow_completed"))
            .unwrap_or(false)
    );

    // Journal must contain matching event entries
    let jevents: Vec<_> = journal.iter().filter_map(|v| v["event"].as_str()).collect();
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
    assert!(
        !tracker
            .events()
            .iter()
            .any(|e| e == "activity_started:step_a"),
        "skipped activity must not fire on_activity_started"
    );
    // step_b (w2-1) should still run
    assert!(
        tracker
            .events()
            .iter()
            .any(|e| e == "activity_started:step_b")
    );
    // Journal should contain a Skip event for step_a
    let jevents: Vec<_> = journal.iter().filter_map(|v| v["event"].as_str()).collect();
    assert!(
        jevents.contains(&"ActivitySkipped"),
        "journal must contain ActivitySkipped"
    );
}

#[tokio::test]
async fn empty_workflow_fires_started_and_completed() {
    let hir = workflow("empty_wf", vec![]);
    let mut tracker = RecordingTracker::default();
    interpret_workflow_durable(&hir, "empty_wf", &mut tracker)
        .await
        .expect("interpret");
    assert!(
        tracker
            .events()
            .iter()
            .any(|e| e.starts_with("workflow_started:empty_wf:0"))
    );
    assert!(
        tracker
            .events()
            .iter()
            .any(|e| e == "workflow_completed:empty_wf")
    );
}

#[tokio::test]
async fn workflow_not_found_returns_error() {
    let hir = workflow("present", vec![]);
    let mut tracker = RecordingTracker::default();
    let err = interpret_workflow_durable(&hir, "not_present", &mut tracker).await;
    assert!(err.is_err(), "unknown workflow must return Err");
}

#[tokio::test]
async fn same_run_id_skips_completed_steps_on_resume() {
    let hir = workflow(
        "resume_demo",
        vec![call_stmt("step_a"), call_stmt("step_b")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first_run = VoxDbTracker::new(db.clone(), "run-1");
    let first_journal = interpret_workflow_durable(&hir, "resume_demo", &mut first_run)
        .await
        .expect("first run");
    assert!(
        !first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivitySkipped"))
    );

    let mut resumed_run = VoxDbTracker::new(db, "run-1");
    let resumed_journal = interpret_workflow_durable(&hir, "resume_demo", &mut resumed_run)
        .await
        .expect("resumed run");
    let replayed = resumed_journal
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityReplayed"))
        .count();
    let replayed_local_activity = resumed_journal
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("LocalActivity"))
        .count();
    assert_eq!(replayed, 2, "resume should replay both completed steps");
    assert_eq!(
        replayed_local_activity, 2,
        "resume should surface stored step results, not only skip markers"
    );
}

#[tokio::test]
async fn different_run_id_starts_fresh_even_with_same_workflow_name() {
    let hir = workflow(
        "resume_demo",
        vec![call_stmt("step_a"), call_stmt("step_b")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first_run = VoxDbTracker::new(db.clone(), "run-1");
    interpret_workflow_durable(&hir, "resume_demo", &mut first_run)
        .await
        .expect("first run");

    let mut fresh_run = VoxDbTracker::new(db, "run-2");
    let fresh_journal = interpret_workflow_durable(&hir, "resume_demo", &mut fresh_run)
        .await
        .expect("fresh run");
    assert!(
        !fresh_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivitySkipped")),
        "new run id must not inherit prior completion state"
    );
}

#[tokio::test]
async fn planner_rejects_if_branches_for_durable_runs() {
    let hir = workflow(
        "branching",
        vec![expr_stmt(HirExpr::If(
            Box::new(HirExpr::Ident("dynamic_flag".to_string(), sp())),
            vec![call_stmt("step_a")],
            Some(vec![call_stmt("step_b")]),
            sp(),
        ))],
    );
    let mut tracker = RecordingTracker::default();
    let err = interpret_workflow_durable(&hir, "branching", &mut tracker)
        .await
        .expect_err("if branches should be rejected");
    assert!(err.to_string().contains("if"));
    assert!(err.to_string().contains("deterministic literal expression"));
}

#[tokio::test]
async fn planner_rejects_match_branches_for_durable_runs() {
    let hir = workflow(
        "matching",
        vec![expr_stmt(HirExpr::Match(
            Box::new(HirExpr::BoolLit(true, sp())),
            vec![HirMatchArm {
                pattern: HirPattern::Wildcard(sp()),
                guard: None,
                body: Box::new(HirExpr::Call(
                    Box::new(HirExpr::Ident("step_a".to_string(), sp())),
                    vec![],
                    false,
                    sp(),
                )),
                span: sp(),
            }],
            sp(),
        ))],
    );
    let mut tracker = RecordingTracker::default();
    let err = interpret_workflow_durable(&hir, "matching", &mut tracker)
        .await
        .expect_err("match branches should be rejected");
    assert!(err.to_string().contains("match"));
    assert!(err.to_string().contains("linear activity plans"));
}

#[test]
fn planner_captures_retry_and_backoff_options() {
    let hir = workflow(
        "retry_opts",
        vec![expr_stmt(HirExpr::With(
            Box::new(HirExpr::Call(
                Box::new(HirExpr::Ident("step_a".to_string(), sp())),
                vec![],
                false,
                sp(),
            )),
            Box::new(HirExpr::ObjectLit(
                vec![
                    ("retries".to_string(), HirExpr::IntLit(3, sp())),
                    (
                        "initial_backoff".to_string(),
                        HirExpr::StringLit("5ms".to_string(), sp()),
                    ),
                ],
                sp(),
            )),
            sp(),
        ))],
    );
    let plan = plan_workflow_activities(&hir, "retry_opts").expect("plan");
    assert_eq!(plan.len(), 1);
    assert_eq!(plan[0].retries, 3);
    assert_eq!(plan[0].initial_backoff_ms, Some(5));
}

#[tokio::test]
async fn db_tracker_persists_and_loads_step_results() {
    let hir = workflow("result_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first_run = VoxDbTracker::new(db.clone(), "run-result");
    let first_journal = interpret_workflow_durable(&hir, "result_demo", &mut first_run)
        .await
        .expect("first run");
    let stored = db
        .load_workflow_activity_result("run-result", "result_demo", "result_demo-0")
        .await
        .expect("load result")
        .expect("stored result");
    assert_eq!(
        stored["event"].as_str(),
        Some("LocalActivity"),
        "completed result should be stored for replay"
    );

    let mut replay_run = VoxDbTracker::new(db, "run-result");
    let replay_journal = interpret_workflow_durable(&hir, "result_demo", &mut replay_run)
        .await
        .expect("replay");
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed"))
    );
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("LocalActivity"))
    );
    assert_eq!(
        first_journal[0]["journal_version"].as_u64(),
        Some(WORKFLOW_JOURNAL_VERSION as u64)
    );
}

#[tokio::test]
async fn workflow_journal_v1_contract_shapes_are_stable() {
    let hir = workflow(
        "contract_demo",
        vec![call_stmt("step_a"), call_stmt("step_b")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first_run = VoxDbTracker::new(db.clone(), "contract-run");
    let first_journal = interpret_workflow_durable(&hir, "contract_demo", &mut first_run)
        .await
        .expect("first run");

    for entry in &first_journal {
        assert_eq!(
            entry["journal_version"].as_u64(),
            Some(WORKFLOW_JOURNAL_VERSION as u64),
            "all interpreted workflow events must carry journal_version=1"
        );
        let event = entry["event"].as_str().expect("event string");
        match event {
            "WorkflowStarted" => assert_has_fields(entry, &["workflow", "steps"]),
            "ActivityTask" => assert_has_fields(
                entry,
                &[
                    "workflow",
                    "activity",
                    "activity_id",
                    "execution_boundary",
                    "max_attempts",
                    "idempotency_key",
                ],
            ),
            "ActivityStarted" => assert_has_fields(entry, &["workflow", "activity", "activity_id"]),
            "LocalActivity" => assert_has_fields(
                entry,
                &["activity", "activity_id", "status", "classification"],
            ),
            "BranchDecision" => assert_has_fields(
                entry,
                &["activity", "activity_id", "branch", "decision_source"],
            ),
            "TimerWaitCompleted" => {
                assert_has_fields(entry, &["activity", "activity_id", "waited_ms"])
            }
            "SignalWaitSatisfied" => {
                assert_has_fields(entry, &["activity", "activity_id", "signal_key"])
            }
            "ActivityCompleted" => {
                assert_has_fields(entry, &["workflow", "activity", "activity_id"])
            }
            "WorkflowCompleted" => assert_has_fields(entry, &["workflow"]),
            other => panic!("unexpected first-run contract event: {other}"),
        }
    }
    assert_journal_entries_validate_v1_schema(&first_journal);

    let mut replay_run = VoxDbTracker::new(db, "contract-run");
    let replay_journal = interpret_workflow_durable(&hir, "contract_demo", &mut replay_run)
        .await
        .expect("replay run");
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "resume should emit ActivityReplayed in v1 contract"
    );
    for entry in replay_journal
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityReplayed"))
    {
        assert_has_fields(
            entry,
            &[
                "workflow",
                "activity",
                "activity_id",
                "replay_source",
                "result_event",
            ],
        );
        assert_eq!(
            entry["replay_source"].as_str(),
            Some("workflow_activity_log"),
            "v1 replay source is workflow_activity_log"
        );
        assert!(
            entry["result_event"]
                .as_str()
                .is_some_and(|name| !name.is_empty()),
            "v1 replay event should carry non-empty result_event"
        );
    }
    for entry in replay_journal
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityCompleted"))
    {
        assert_eq!(
            entry["replayed"].as_bool(),
            Some(true),
            "replayed ActivityCompleted events must carry replayed=true"
        );
    }
    assert_journal_entries_validate_v1_schema(&replay_journal);
}

#[tokio::test]
async fn workflow_journal_v1_schema_validates_branch_timer_signal_and_replay_events() {
    let hir = workflow(
        "schema_path_demo",
        vec![
            call_stmt("step_a"),
            expr_stmt(HirExpr::If(
                Box::new(HirExpr::Binary(
                    vox_compiler::hir::HirBinOp::Is,
                    Box::new(HirExpr::IntLit(1, sp())),
                    Box::new(HirExpr::IntLit(1, sp())),
                    sp(),
                )),
                vec![call_stmt("step_then")],
                Some(vec![call_stmt("step_else")]),
                sp(),
            )),
            call_stmt_with_int_arg("workflow_wait", 1),
            call_stmt_with_string_arg("workflow_wait_signal", "go"),
        ],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    db.record_workflow_signal("schema-path-run", "go", None)
        .await
        .expect("seed signal");

    let mut first = VoxDbTracker::new(db.clone(), "schema-path-run");
    let first_journal = interpret_workflow_durable(&hir, "schema_path_demo", &mut first)
        .await
        .expect("first run");
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("BranchDecision")),
        "first run should include branch decision event"
    );
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("TimerWaitCompleted")),
        "first run should include timer completion event"
    );
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("SignalWaitSatisfied")),
        "first run should include signal wait event"
    );
    assert_journal_entries_validate_v1_schema(&first_journal);

    let mut replay = VoxDbTracker::new(db, "schema-path-run");
    let replay_journal = interpret_workflow_durable(&hir, "schema_path_demo", &mut replay)
        .await
        .expect("replay run");
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "replay should include replay events"
    );
    assert_journal_entries_validate_v1_schema(&replay_journal);
}

#[cfg(feature = "mens")]
#[tokio::test]
async fn workflow_journal_v1_schema_validates_mesh_activity_event() {
    let hir = workflow("mesh_schema_demo", vec![call_stmt("mesh_join")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::new(db.clone(), "mesh-schema-run");
    let journal = interpret_workflow_durable(&hir, "mesh_schema_demo", &mut run)
        .await
        .expect("mesh run");
    let mesh_entry = journal
        .iter()
        .find(|entry| entry["event"].as_str() == Some("MeshActivity"))
        .expect("mesh activity event should exist");
    assert_has_fields(
        mesh_entry,
        &["activity", "activity_id", "mesh_op", "control"],
    );
    assert_journal_entries_validate_v1_schema(&journal);

    let mut replay = VoxDbTracker::new(db, "mesh-schema-run");
    let replay_journal = interpret_workflow_durable(&hir, "mesh_schema_demo", &mut replay)
        .await
        .expect("mesh replay run");
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "mesh replay should include replay event"
    );
    assert_journal_entries_validate_v1_schema(&replay_journal);
}

#[cfg(not(feature = "mens"))]
#[tokio::test]
async fn workflow_journal_v1_schema_validates_mesh_activity_skipped_event() {
    let hir = workflow("mesh_schema_demo", vec![call_stmt("mesh_join")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::new(db.clone(), "mesh-schema-run");
    let journal = interpret_workflow_durable(&hir, "mesh_schema_demo", &mut run)
        .await
        .expect("mesh skipped run");
    let skipped_entry = journal
        .iter()
        .find(|entry| entry["event"].as_str() == Some("MeshActivitySkipped"))
        .expect("mesh skipped event should exist");
    assert_has_fields(skipped_entry, &["activity", "activity_id", "reason"]);
    assert_journal_entries_validate_v1_schema(&journal);
}

#[tokio::test]
async fn deterministic_if_records_branch_decision_and_replays_it() {
    let hir = workflow(
        "if_replay_demo",
        vec![expr_stmt(HirExpr::If(
            Box::new(HirExpr::Binary(
                vox_compiler::hir::HirBinOp::Is,
                Box::new(HirExpr::IntLit(2, sp())),
                Box::new(HirExpr::IntLit(2, sp())),
                sp(),
            )),
            vec![call_stmt("step_then")],
            Some(vec![call_stmt("step_else")]),
            sp(),
        ))],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first = VoxDbTracker::new(db.clone(), "if-replay-run");
    let first_journal = interpret_workflow_durable(&hir, "if_replay_demo", &mut first)
        .await
        .expect("first run");
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("BranchDecision")),
        "first run should persist branch decision"
    );
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["activity"].as_str() == Some("step_then")),
        "then branch should execute"
    );
    assert!(
        !first_journal
            .iter()
            .any(|entry| entry["activity"].as_str() == Some("step_else")),
        "else branch should not execute"
    );

    let mut replay = VoxDbTracker::new(db, "if-replay-run");
    let replay_journal = interpret_workflow_durable(&hir, "if_replay_demo", &mut replay)
        .await
        .expect("replay run");
    let replayed_count = replay_journal
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityReplayed"))
        .count();
    assert!(
        replayed_count >= 2,
        "replay should include decision + then activity replay"
    );
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("BranchDecision")),
        "replay should emit stored branch decision payload"
    );
}

#[tokio::test]
async fn workflow_activity_log_schema_contains_replay_contract_columns() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let rows = db
        .query_all("PRAGMA table_info(workflow_activity_log)", ())
        .await
        .expect("table info");

    let mut columns = std::collections::HashSet::new();
    for row in rows {
        let name: String = row.get(1).expect("name column");
        columns.insert(name);
    }

    for required in [
        "run_id",
        "workflow_name",
        "activity_name",
        "activity_id",
        "status",
        "result_json",
        "recorded_at_ms",
    ] {
        assert!(
            columns.contains(required),
            "workflow_activity_log missing required replay column `{required}`"
        );
    }
}

#[tokio::test]
async fn workflow_run_log_schema_contains_lease_columns() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let rows = db
        .query_all("PRAGMA table_info(workflow_run_log)", ())
        .await
        .expect("table info");
    let mut columns = std::collections::HashSet::new();
    for row in rows {
        let name: String = row.get(1).expect("name column");
        columns.insert(name);
    }
    for required in [
        "run_id",
        "workflow_name",
        "status",
        "planned_steps",
        "completed_steps",
        "plan_session_id",
        "plan_node_id",
        "plan_version",
        "lease_owner",
        "lease_until_ms",
        "started_at_ms",
        "updated_at_ms",
    ] {
        assert!(
            columns.contains(required),
            "workflow_run_log missing required column `{required}`"
        );
    }
}

#[tokio::test]
async fn workflow_signal_log_schema_contains_required_columns() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let rows = db
        .query_all("PRAGMA table_info(workflow_signal_log)", ())
        .await
        .expect("table info");
    let mut columns = std::collections::HashSet::new();
    for row in rows {
        let name: String = row.get(1).expect("name column");
        columns.insert(name);
    }
    for required in [
        "id",
        "run_id",
        "signal_key",
        "payload_json",
        "recorded_at_ms",
        "consumed_at_ms",
    ] {
        assert!(
            columns.contains(required),
            "workflow_signal_log missing required column `{required}`"
        );
    }
}

#[tokio::test]
async fn workflow_activity_attempt_log_schema_contains_required_columns() {
    let db = VoxDb::connect(DbConfig::Memory).await.expect("memory db");
    let rows = db
        .query_all("PRAGMA table_info(workflow_activity_attempt_log)", ())
        .await
        .expect("table info");
    let mut columns = std::collections::HashSet::new();
    for row in rows {
        let name: String = row.get(1).expect("name column");
        columns.insert(name);
    }
    for required in [
        "run_id",
        "workflow_name",
        "activity_id",
        "attempt_no",
        "status",
        "worker_owner",
        "lease_until_ms",
        "error",
        "recorded_at_ms",
    ] {
        assert!(
            columns.contains(required),
            "workflow_activity_attempt_log missing required column `{required}`"
        );
    }
}

#[tokio::test]
async fn workflow_run_log_tracks_lifecycle_and_progress() {
    let hir = workflow(
        "run_log_demo",
        vec![call_stmt("step_a"), call_stmt("step_b")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut run = VoxDbTracker::new(db.clone(), "run-log-1");
    interpret_workflow_durable(&hir, "run_log_demo", &mut run)
        .await
        .expect("run");

    let rows = db
        .query_all(
            "SELECT workflow_name, status, planned_steps, completed_steps, completed_at_ms, lease_owner, lease_until_ms
             FROM workflow_run_log WHERE run_id = ?1",
            ("run-log-1".to_string(),),
        )
        .await
        .expect("workflow run rows");
    assert_eq!(rows.len(), 1, "workflow run row should exist");

    let row = &rows[0];
    let workflow_name: String = row.get(0).expect("workflow_name");
    let status: String = row.get(1).expect("status");
    let planned_steps: i64 = row.get(2).expect("planned_steps");
    let completed_steps: i64 = row.get(3).expect("completed_steps");
    let completed_at_ms: Option<i64> = row.get(4).expect("completed_at_ms");
    let lease_owner: Option<String> = row.get(5).expect("lease_owner");
    let lease_until_ms: Option<i64> = row.get(6).expect("lease_until_ms");

    assert_eq!(workflow_name, "run_log_demo");
    assert_eq!(status, "completed");
    assert_eq!(planned_steps, 2);
    assert_eq!(completed_steps, 2);
    assert!(
        completed_at_ms.is_some(),
        "completed workflow run should have completed_at_ms"
    );
    assert!(
        lease_owner.is_none(),
        "completed run should release lease owner"
    );
    assert!(
        lease_until_ms.is_none(),
        "completed run should release lease expiration"
    );
}

#[tokio::test]
async fn lease_conflict_blocks_second_owner_for_same_run() {
    let hir = workflow("lease_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first = VoxDbTracker::with_owner(db.clone(), "lease-run", "owner-a", 120_000);
    interpret_workflow_durable(&hir, "lease_demo", &mut first)
        .await
        .expect("first run");

    db.record_workflow_run_started("lease-run", "lease_demo", 1)
        .await
        .expect("restart run row");
    let claimed = db
        .try_claim_workflow_run_lease("lease-run", "owner-a", 120_000)
        .await
        .expect("owner-a reclaims");
    assert!(claimed, "owner-a should hold lease");

    let mut second = VoxDbTracker::with_owner(db, "lease-run", "owner-b", 120_000);
    let err = interpret_workflow_durable(&hir, "lease_demo", &mut second)
        .await
        .expect_err("second owner should fail lease claim");
    assert!(
        err.to_string().contains("lease"),
        "conflict error should mention lease: {err}"
    );
}

#[tokio::test]
async fn crash_window_after_persist_replays_without_duplicate_execution() {
    let hir = workflow("crash_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let inner = VoxDbTracker::with_owner(db.clone(), "crash-run", "owner-crash", 120_000);
    let mut crashing_tracker = FailAfterPersistTracker {
        inner,
        fail_after_first_complete: true,
    };
    let first = interpret_workflow_durable(&hir, "crash_demo", &mut crashing_tracker).await;
    assert!(
        first.is_err(),
        "first run should simulate post-persist crash"
    );

    let mut resumed = VoxDbTracker::with_owner(db, "crash-run", "owner-crash", 120_000);
    let replay = interpret_workflow_durable(&hir, "crash_demo", &mut resumed)
        .await
        .expect("resumed run");
    let replayed = replay
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityReplayed"))
        .count();
    let started = replay
        .iter()
        .filter(|entry| entry["event"].as_str() == Some("ActivityStarted"))
        .count();
    assert_eq!(replayed, 1, "resumed run should replay persisted result");
    assert_eq!(
        started, 0,
        "resumed run should not re-execute already persisted activity"
    );
}

#[tokio::test]
async fn durable_timer_wait_is_recorded_and_replayed_without_rewaiting() {
    let hir = workflow(
        "timer_demo",
        vec![call_stmt_with_int_arg("workflow_wait", 25)],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    let mut first = VoxDbTracker::new(db.clone(), "timer-run");
    let started = std::time::Instant::now();
    let first_journal = interpret_workflow_durable(&hir, "timer_demo", &mut first)
        .await
        .expect("first run");
    let first_elapsed = started.elapsed().as_millis();
    assert!(
        first_elapsed >= 20,
        "initial timer execution should wait roughly requested duration"
    );
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("TimerWaitCompleted")),
        "timer wait event should be emitted"
    );

    let mut replay = VoxDbTracker::new(db, "timer-run");
    let replay_started = std::time::Instant::now();
    let replay_journal = interpret_workflow_durable(&hir, "timer_demo", &mut replay)
        .await
        .expect("replay run");
    let replay_elapsed = replay_started.elapsed().as_millis();
    assert!(
        replay_elapsed < 20,
        "replay should not block on already completed durable timer"
    );
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "replay should use stored timer result"
    );
}

#[tokio::test]
async fn cancelled_run_refuses_new_activity_execution() {
    let hir = workflow(
        "cancel_demo",
        vec![call_stmt("step_a"), call_stmt("step_b")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::with_owner(db.clone(), "cancel-run", "owner-cancel", 120_000);

    db.record_workflow_run_started("cancel-run", "cancel_demo", 2)
        .await
        .expect("seed run");
    db.record_workflow_run_cancelled("cancel-run", "operator requested cancel")
        .await
        .expect("cancel run");

    let err = interpret_workflow_durable(&hir, "cancel_demo", &mut run)
        .await
        .expect_err("cancelled run should reject new activity execution");
    assert!(
        err.to_string().contains("cancelled"),
        "cancel error should mention cancellation: {err}"
    );
}

#[tokio::test]
async fn workflow_run_plan_context_is_persisted() {
    let hir = workflow("plan_ctx_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::new(db.clone(), "plan-ctx-run").with_plan_context(
        "plan-session-42",
        "node-verify",
        7,
    );
    interpret_workflow_durable(&hir, "plan_ctx_demo", &mut run)
        .await
        .expect("run with plan context");

    let rows = db
        .query_all(
            "SELECT plan_session_id, plan_node_id, plan_version
             FROM workflow_run_log WHERE run_id = ?1",
            ("plan-ctx-run".to_string(),),
        )
        .await
        .expect("workflow run row");
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    let plan_session_id: Option<String> = row.get(0).expect("plan_session_id");
    let plan_node_id: Option<String> = row.get(1).expect("plan_node_id");
    let plan_version: Option<i64> = row.get(2).expect("plan_version");
    assert_eq!(plan_session_id.as_deref(), Some("plan-session-42"));
    assert_eq!(plan_node_id.as_deref(), Some("node-verify"));
    assert_eq!(plan_version, Some(7));
}

#[tokio::test]
async fn activity_attempt_rows_are_persisted_for_successful_execution() {
    let hir = workflow("attempt_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::with_owner(db.clone(), "attempt-run", "worker-a", 120_000);
    interpret_workflow_durable(&hir, "attempt_demo", &mut run)
        .await
        .expect("run");

    let rows = db
        .query_all(
            "SELECT attempt_no, status, worker_owner, lease_until_ms
             FROM workflow_activity_attempt_log
             WHERE run_id = ?1 AND workflow_name = ?2 AND activity_id = ?3
             ORDER BY attempt_no, status",
            (
                "attempt-run".to_string(),
                "attempt_demo".to_string(),
                "attempt_demo-0".to_string(),
            ),
        )
        .await
        .expect("attempt rows");
    assert_eq!(rows.len(), 2, "started and completed attempt rows expected");
    let first_status: String = rows[0].get(1).expect("first status");
    let second_status: String = rows[1].get(1).expect("second status");
    let worker_owner: Option<String> = rows[0].get(2).expect("worker owner");
    assert_eq!(first_status, "completed");
    assert_eq!(second_status, "started");
    assert_eq!(worker_owner.as_deref(), Some("worker-a"));
}

#[tokio::test]
async fn active_attempt_lease_blocks_second_worker_execution() {
    let hir = workflow("attempt_guard_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    db.record_workflow_run_started("attempt-guard-run", "attempt_guard_demo", 1)
        .await
        .expect("seed run");
    db.record_workflow_activity_attempt_started(
        "attempt-guard-run",
        "attempt_guard_demo",
        "attempt_guard_demo-0",
        1,
        "worker-b",
        120_000,
    )
    .await
    .expect("seed active attempt");

    let mut run = VoxDbTracker::with_owner(db, "attempt-guard-run", "worker-a", 120_000);
    let err = interpret_workflow_durable(&hir, "attempt_guard_demo", &mut run)
        .await
        .expect_err("active attempt lease should block second worker");
    assert!(
        err.to_string().contains("attempt lease active"),
        "error should surface attempt lease conflict: {err}"
    );
}

#[tokio::test]
async fn stale_attempt_lease_recovers_with_next_attempt_number() {
    let hir = workflow("attempt_recover_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    db.record_workflow_run_started("attempt-recover-run", "attempt_recover_demo", 1)
        .await
        .expect("seed run");
    db.record_workflow_activity_attempt_started(
        "attempt-recover-run",
        "attempt_recover_demo",
        "attempt_recover_demo-0",
        1,
        "worker-b",
        1,
    )
    .await
    .expect("seed stale attempt");
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;

    let mut run = VoxDbTracker::with_owner(db.clone(), "attempt-recover-run", "worker-a", 120_000);
    let journal = interpret_workflow_durable(&hir, "attempt_recover_demo", &mut run)
        .await
        .expect("recover run");
    assert!(
        journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityAttemptRecovered")),
        "stale attempt recovery should be journaled"
    );

    let rows = db
        .query_all(
            "SELECT attempt_no, status FROM workflow_activity_attempt_log
             WHERE run_id = ?1 AND workflow_name = ?2 AND activity_id = ?3
             ORDER BY attempt_no, status",
            (
                "attempt-recover-run".to_string(),
                "attempt_recover_demo".to_string(),
                "attempt_recover_demo-0".to_string(),
            ),
        )
        .await
        .expect("attempt rows");
    assert!(
        rows.iter().any(|r| {
            let attempt_no: i64 = r.get(0).expect("attempt_no");
            let status: String = r.get(1).expect("status");
            attempt_no == 2 && status == "completed"
        }),
        "second attempt should complete after stale lease recovery"
    );
}

#[tokio::test]
async fn stale_attempt_recovery_event_shape_is_stable_and_replay_skips_recovery_path() {
    let hir = workflow("attempt_recover_shape_demo", vec![call_stmt("step_a")]);
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));

    db.record_workflow_run_started("attempt-recover-shape-run", "attempt_recover_shape_demo", 1)
        .await
        .expect("seed run");
    db.record_workflow_activity_attempt_started(
        "attempt-recover-shape-run",
        "attempt_recover_shape_demo",
        "attempt_recover_shape_demo-0",
        1,
        "worker-b",
        1,
    )
    .await
    .expect("seed stale attempt");
    tokio::time::sleep(std::time::Duration::from_millis(3)).await;

    let mut first =
        VoxDbTracker::with_owner(db.clone(), "attempt-recover-shape-run", "worker-a", 120_000);
    let first_journal = interpret_workflow_durable(&hir, "attempt_recover_shape_demo", &mut first)
        .await
        .expect("recovered run should succeed");
    let recovered_entry = first_journal
        .iter()
        .find(|entry| entry["event"].as_str() == Some("ActivityAttemptRecovered"))
        .expect("journal should include recovery entry");
    assert_has_fields(
        recovered_entry,
        &[
            "event",
            "workflow",
            "activity",
            "activity_id",
            "resume_attempt",
            "max_attempts_window",
            "journal_version",
        ],
    );
    assert_eq!(
        recovered_entry["resume_attempt"].as_u64(),
        Some(2),
        "recovery should resume from the second attempt"
    );

    let mut second =
        VoxDbTracker::with_owner(db.clone(), "attempt-recover-shape-run", "worker-c", 120_000);
    let second_journal =
        interpret_workflow_durable(&hir, "attempt_recover_shape_demo", &mut second)
            .await
            .expect("second run should replay");
    assert!(
        second_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "completed step should replay on subsequent runs"
    );
    assert!(
        !second_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityAttemptRecovered")),
        "recovery event should not be emitted once step completion is durable"
    );
}

#[tokio::test]
async fn signal_wait_fails_when_signal_is_missing() {
    let hir = workflow(
        "signal_demo",
        vec![call_stmt_with_string_arg("workflow_wait_signal", "go")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    let mut run = VoxDbTracker::with_owner(db, "signal-run-missing", "owner-signal", 120_000);
    let err = interpret_workflow_durable(&hir, "signal_demo", &mut run)
        .await
        .expect_err("missing signal should fail");
    assert!(
        err.to_string().contains("waiting for signal"),
        "missing signal error should mention waiting state: {err}"
    );
}

#[tokio::test]
async fn signal_wait_consumes_signal_and_replays_satisfied_event() {
    let hir = workflow(
        "signal_demo",
        vec![call_stmt_with_string_arg("workflow_wait_signal", "go")],
    );
    let db = Arc::new(VoxDb::connect(DbConfig::Memory).await.expect("memory db"));
    db.record_workflow_signal("signal-run", "go", None)
        .await
        .expect("record signal");

    let mut first = VoxDbTracker::with_owner(db.clone(), "signal-run", "owner-signal", 120_000);
    let first_journal = interpret_workflow_durable(&hir, "signal_demo", &mut first)
        .await
        .expect("signal run");
    assert!(
        first_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("SignalWaitSatisfied")),
        "first run should consume and emit signal satisfied event"
    );

    let mut replay = VoxDbTracker::with_owner(db, "signal-run", "owner-signal", 120_000);
    let replay_journal = interpret_workflow_durable(&hir, "signal_demo", &mut replay)
        .await
        .expect("replay run");
    assert!(
        replay_journal
            .iter()
            .any(|entry| entry["event"].as_str() == Some("ActivityReplayed")),
        "replay should use persisted signal wait completion"
    );
}
