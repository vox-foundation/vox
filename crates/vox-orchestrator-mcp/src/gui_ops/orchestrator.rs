//! Orchestrator-group routes and ops for Axis (plan 07 Task 10): the persistent task hopper,
//! the chat context budget, live plan-DAG edits, the GUI-run journal and the activity timeline.
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{ConnectInfo, Path, Query, State};
use axum::http::HeaderMap;
use axum::routing::{get, post};
use axum::{Json, Router};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use vox_foundation::protocol::orch_daemon_method;
use vox_orchestrator::hopper::{
    HopperIntake, HopperItemId, InMemoryHopper, IntakeItem, IntakeSource, ItemState, PriorityHint,
    SqliteHopper,
};

use crate::gui_ops::dto::orchestrator::{
    ActivityRowDto, ContextBudgetPayload, FinishGuiRunInput, GuiRunRecord, HopperTaskDto,
    InsertPlanNodeInput, LatestPlanSessionForChat, PlanNodeDto, StartGuiRunInput,
    UpdatePlanNodeInput,
};
use crate::gui_ops::{api_db, enforce_route, enforce_write_route, require_db};
use crate::http_gateway::GatewayState;
use crate::params::ToolResult;
use crate::server_state::ServerState;
use crate::services::routes::{err, ok};

const HOPPER_SURFACES: &[&str] = &["tasks", "dashboard"];
const CONTROL_PLANE_SURFACES: &[&str] = &["chat", "dashboard", "tasks"];
const MISSION_CONTROL_SURFACES: &[&str] = &["sub-agents", "approvals", "tasks", "dashboard"];
const CONTEXT_SURFACES: &[&str] = &["chat"];
const PLAN_SURFACES: &[&str] = &["chat", "plan"];
const RUNS_SURFACES: &[&str] = &["runs", "models"];
const RUN_SURFACES: &[&str] = &["runs"];
const ACTIVITY_SURFACES: &[&str] = &["activity", "chat"];

/// Most-recent bound on completed items chained into the hopper list: the list is re-polled on
/// every tasks-changed event, so the done read must not grow with all-time history.
const DONE_HISTORY_LIMIT: u32 = 50;
const RUNS_DEFAULT_LIMIT: u32 = 100;
const ACTIVITY_DEFAULT_LIMIT: u32 = 50;
const ACTIVITY_MAX_LIMIT: u32 = 500;

// ─── Hopper ──────────────────────────────────────────────────────────────────

fn sqlite_hopper(state: &ServerState, db: &Arc<vox_db::VoxDb>) -> SqliteHopper {
    SqliteHopper::with_bus(db.clone(), Arc::new(state.orchestrator.event_bus.clone()))
}

/// The gateway's task hopper: persistent when the server has a DB (so Axis and the dashboard
/// see one queue), in-memory otherwise.
pub(crate) fn hopper_for(state: &ServerState) -> Arc<dyn HopperIntake> {
    match &state.db {
        Some(db) => Arc::new(sqlite_hopper(state, db)),
        None => Arc::new(InMemoryHopper::new(Arc::new(
            state.orchestrator.event_bus.clone(),
        ))),
    }
}

fn hopper_item_to_dto(item: &IntakeItem) -> HopperTaskDto {
    let agent_id = match &item.state {
        ItemState::Assigned { agent_id } => Some(agent_id.clone()),
        _ => None,
    };
    let remote_node = match &item.source {
        IntakeSource::Mesh { node_id } => Some(node_id.clone()),
        _ => None,
    };
    HopperTaskDto {
        item_id: item.item_id.0.clone(),
        intent: item.intent.clone(),
        priority: item.classified_priority as u8,
        state: item.state.kind().to_string(),
        task_id: vox_orchestrator::orchestrator::dispatch::stable_hash(&item.item_id.0),
        session_id: item.session_id.clone(),
        agent_id,
        remote_node,
    }
}

async fn hopper_list_items(hopper: &SqliteHopper) -> Vec<HopperTaskDto> {
    let inbox = hopper.inbox().await;
    let assigned = hopper.assigned().await;
    // Already scoped to `done` at the SQL layer; this filter is defense-in-depth.
    let done: Vec<_> = hopper
        .history_recent(DONE_HISTORY_LIMIT)
        .await
        .into_iter()
        .filter(|i| matches!(i.state, ItemState::Done))
        .collect();
    inbox
        .iter()
        .chain(assigned.iter())
        .chain(done.iter())
        .map(hopper_item_to_dto)
        .collect()
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HopperSubmitParams {
    pub intent: String,
    #[serde(default)]
    pub affinity: Vec<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HopperReprioritizeParams {
    pub item_id: String,
    /// `TaskPriority` wire value: 0 background, 1 normal, 2 urgent.
    pub priority: u8,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct HopperItemParams {
    pub item_id: String,
}

pub async fn hopper_submit(state: &ServerState, p: HopperSubmitParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<HopperTaskDto>::err(e).to_json(),
    };
    let item = sqlite_hopper(state, db)
        .submit(
            p.intent,
            p.affinity,
            PriorityHint::Normal,
            IntakeSource::Developer,
            None,
        )
        .await;
    ToolResult::ok(hopper_item_to_dto(&item)).to_json()
}

pub async fn hopper_reprioritize(state: &ServerState, p: HopperReprioritizeParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<HopperTaskDto>::err(e).to_json(),
    };
    let cap = vox_orchestrator::hopper::capability::DeveloperOverrideMint::new().mint(
        "Axis",
        "GUI Reprioritization",
        "gui-override",
    );
    let priority = vox_orchestrator::types::TaskPriority::from_u8(p.priority);
    match sqlite_hopper(state, db)
        .reprioritize(&HopperItemId(p.item_id), priority, cap)
        .await
    {
        Ok(item) => ToolResult::ok(hopper_item_to_dto(&item)).to_json(),
        Err(e) => ToolResult::<HopperTaskDto>::err(e.to_string()).to_json(),
    }
}

pub async fn hopper_cancel(state: &ServerState, p: HopperItemParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<HopperTaskDto>::err(e).to_json(),
    };
    match sqlite_hopper(state, db)
        .cancel(&HopperItemId(p.item_id))
        .await
    {
        Ok(item) => ToolResult::ok(hopper_item_to_dto(&item)).to_json(),
        Err(e) => ToolResult::<HopperTaskDto>::err(e.to_string()).to_json(),
    }
}

pub async fn hopper_mark_done(state: &ServerState, p: HopperItemParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<HopperTaskDto>::err(e).to_json(),
    };
    match sqlite_hopper(state, db)
        .complete(&HopperItemId(p.item_id))
        .await
    {
        Ok(item) => ToolResult::ok(hopper_item_to_dto(&item)).to_json(),
        Err(e) => ToolResult::<HopperTaskDto>::err(e.to_string()).to_json(),
    }
}

/// GET /api/v2/hopper/list — open (inbox + assigned) items plus the recent done window.
pub async fn get_hopper_list(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, HOPPER_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    ok(hopper_list_items(&sqlite_hopper(gs.server_state(), &db)).await)
}

// ─── Context budget ──────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct SessionQuery {
    pub session_id: Option<String>,
}

/// GET /api/v2/orchestrator/context-budget?session_id= — the compaction budget plus the
/// session's latest interaction size. Works without a DB (`used_tokens` is then zero).
pub async fn get_context_budget(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SessionQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, CONTEXT_SURFACES) {
        return e;
    }
    let cfg = vox_orchestrator::config::OrchestratorConfig::snapshot().compaction;
    let used_tokens = match (q.session_id.as_deref(), gs.server_state().db.as_ref()) {
        (Some(sid), Some(db)) => db.latest_interaction_tokens(sid).await.unwrap_or(0) as usize,
        _ => 0,
    };
    ok(ContextBudgetPayload {
        max_context_tokens: cfg.max_context_tokens,
        reserved_tokens: cfg.reserved_tokens,
        threshold_tokens: cfg.trigger_at(),
        usable_tokens: cfg.usable_budget(),
        strategy: cfg.strategy.to_string(),
        used_tokens,
    })
}

// ─── Plan DAG ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlanNodeUpsertMode {
    Update,
    Insert,
}

/// `update` edits a not-yet-dispatched node's description; `insert` adds a new step.
#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlanNodeUpsertParams {
    pub mode: PlanNodeUpsertMode,
    #[serde(default)]
    pub update: Option<UpdatePlanNodeInput>,
    #[serde(default)]
    pub insert: Option<InsertPlanNodeInput>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlanNodesApproveParams {
    pub plan_session_id: String,
}

/// Writes through the same `upsert_plan_node` primitive the orchestrator's plan synthesis uses;
/// `enqueue_runnable_plan_nodes` re-reads the row before dispatch, so the edit takes effect for
/// any node the scheduler hasn't reached yet.
async fn update_plan_node(db: &vox_db::VoxDb, input: UpdatePlanNodeInput) -> Result<(), String> {
    let rows = db
        .load_plan_nodes_with_status(&input.plan_session_id, input.plan_version)
        .await
        .map_err(|e| e.to_string())?;
    let existing = rows
        .iter()
        .find(|r| r.node_id == input.node_id)
        .ok_or_else(|| format!("plan node {} not found", input.node_id))?;
    db.upsert_plan_node(
        &input.plan_session_id,
        input.plan_version,
        &input.node_id,
        &input.description,
        &existing.dependencies_json,
        &existing.execution_policy_json,
        &existing.status,
        existing.workflow_invocation.as_deref(),
    )
    .await
    .map_err(|e| e.to_string())
}

/// A node inserted into a plan that still has `blocked_on_approval` nodes is itself inserted
/// blocked — otherwise the GUI could create a node the scheduler dispatches immediately,
/// bypassing the approval gate.
async fn insert_plan_node(db: &vox_db::VoxDb, input: InsertPlanNodeInput) -> Result<(), String> {
    let deps_json = serde_json::to_string(&input.depends_on).map_err(|e| e.to_string())?;
    let existing = db
        .load_plan_nodes_with_status(&input.plan_session_id, input.plan_version)
        .await
        .map_err(|e| e.to_string())?;
    let status = if existing.iter().any(|r| r.status == "blocked_on_approval") {
        "blocked_on_approval"
    } else {
        "pending"
    };
    db.upsert_plan_node(
        &input.plan_session_id,
        input.plan_version,
        &input.node_id,
        &input.description,
        &deps_json,
        "{}",
        status,
        None,
    )
    .await
    .map_err(|e| e.to_string())
}

pub async fn plan_node_upsert(state: &ServerState, p: PlanNodeUpsertParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    let result = match (p.mode, p.update, p.insert) {
        (PlanNodeUpsertMode::Update, Some(input), _) => update_plan_node(db, input).await,
        (PlanNodeUpsertMode::Insert, _, Some(input)) => insert_plan_node(db, input).await,
        (mode, _, _) => Err(format!("mode {mode:?} requires its matching payload")),
    };
    match result {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e).to_json(),
    }
}

/// Approve every `blocked_on_approval` node in a plan session; returns how many were released.
pub async fn plan_nodes_approve(state: &ServerState, p: PlanNodesApproveParams) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<u64>::err(e).to_json(),
    };
    match db.approve_all_blocked_plan_nodes(&p.plan_session_id).await {
        Ok(n) => ToolResult::ok(n).to_json(),
        Err(e) => ToolResult::<u64>::err(e.to_string()).to_json(),
    }
}

#[derive(Debug, Deserialize)]
pub struct PlanVersionQuery {
    pub version: i64,
}

#[derive(Debug, Deserialize)]
pub struct SessionIdsQuery {
    /// Comma-separated chat session ids.
    pub session_ids: Option<String>,
}

/// GET /api/v2/plan/sessions/{plan_session_id}/nodes?version=
pub async fn get_plan_nodes(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(plan_session_id): Path<String>,
    Query(q): Query<PlanVersionQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, PLAN_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db
        .load_plan_nodes_with_status(&plan_session_id, q.version)
        .await
    {
        Ok(rows) => ok(rows.into_iter().map(PlanNodeDto::from).collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/plan/open-task-counts?session_ids=a,b — sessions with no open tasks are absent.
pub async fn get_plan_open_task_counts(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SessionIdsQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, PLAN_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let ids: Vec<String> = q
        .session_ids
        .as_deref()
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    match db.open_task_counts_for_sessions(&ids).await {
        Ok(counts) => ok::<HashMap<String, i64>>(counts),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/plan/latest-for-chat/{session_id} — `data: null` when the chat has no plan.
pub async fn get_latest_plan_for_chat(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(session_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, PLAN_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let plan_session_id = match db.latest_plan_session_id_for_origin(&session_id).await {
        Ok(Some(id)) => id,
        Ok(None) => return ok(None::<LatestPlanSessionForChat>),
        Err(e) => return err("db_error", &e.to_string()),
    };
    match db.get_plan_session_by_id(&plan_session_id).await {
        Ok(row) => ok(row.map(|row| LatestPlanSessionForChat {
            plan_session_id,
            plan_version: row.current_version,
        })),
        Err(e) => err("db_error", &e.to_string()),
    }
}

// ─── GUI runs ────────────────────────────────────────────────────────────────

fn blank_run(run_id: String, workflow_name: String, status: &str, ts: i64) -> vox_db::AgentRunRow {
    vox_db::AgentRunRow {
        run_id,
        workflow_name,
        command: None,
        repo: None,
        worktree: None,
        model: None,
        status: status.to_string(),
        planned_steps: 1,
        completed_steps: 0,
        cost_usd: 0.0,
        tokens_in: 0,
        tokens_out: 0,
        logs_ref: None,
        artifacts_json: "[]".to_string(),
        approval_ref: None,
        started_at_ms: ts,
        updated_at_ms: ts,
        completed_at_ms: None,
        last_error: None,
    }
}

pub async fn gui_run_start(state: &ServerState, input: StartGuiRunInput) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    let ts = chrono::Utc::now().timestamp_millis();
    let row = vox_db::AgentRunRow {
        command: input.command,
        repo: input.repo,
        worktree: input.worktree,
        model: input.model,
        planned_steps: input.planned_steps.unwrap_or(1),
        ..blank_run(input.run_id, input.workflow_name, "running", ts)
    };
    match db.agent_runs_upsert(&row).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e.to_string()).to_json(),
    }
}

async fn finish_run(db: &vox_db::VoxDb, input: FinishGuiRunInput) -> Result<(), String> {
    let ts = chrono::Utc::now().timestamp_millis();
    let status = if input.success { "completed" } else { "failed" };
    // Preserve start metadata; a run never started gets a minimal fallback row.
    let mut row = match db
        .agent_runs_get(&input.run_id)
        .await
        .map_err(|e| e.to_string())?
    {
        Some(existing) => existing,
        None => vox_db::AgentRunRow {
            planned_steps: input.completed_steps.unwrap_or(0),
            ..blank_run(input.run_id.clone(), input.run_id.clone(), status, ts)
        },
    };
    row.status = status.to_string();
    if let Some(steps) = input.completed_steps {
        row.completed_steps = steps;
    }
    if let Some(cost) = input.cost_usd {
        row.cost_usd = cost;
    }
    if let Some(tin) = input.tokens_in {
        row.tokens_in = tin;
    }
    if let Some(tout) = input.tokens_out {
        row.tokens_out = tout;
    }
    row.updated_at_ms = ts;
    row.completed_at_ms = Some(ts);
    row.last_error = input.error;

    // Best-effort joins against durable correlation data: a missing approval or a not-yet-flushed
    // telemetry event leaves the placeholder value.
    if let Some(tid) = input.task_id.as_deref().filter(|s| !s.trim().is_empty()) {
        if row.approval_ref.is_none()
            && let Some(approval_id) = db.find_approval_id_for_run(tid).await
        {
            row.approval_ref = Some(approval_id);
        }
        if row.cost_usd == 0.0
            && row.tokens_in == 0
            && row.tokens_out == 0
            && let Some((cost, tin, tout)) = db.find_task_root_summary_totals(tid).await
        {
            row.cost_usd = cost;
            row.tokens_in = tin;
            row.tokens_out = tout;
        }
    }
    db.agent_runs_upsert(&row).await.map_err(|e| e.to_string())
}

pub async fn gui_run_finish(state: &ServerState, input: FinishGuiRunInput) -> String {
    let db = match require_db(state) {
        Ok(db) => db,
        Err(e) => return ToolResult::<()>::err(e).to_json(),
    };
    match finish_run(db, input).await {
        Ok(()) => ToolResult::ok(()).to_json(),
        Err(e) => ToolResult::<()>::err(e).to_json(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LimitQuery {
    pub limit: Option<u32>,
}

/// GET /api/v2/runs/gui?limit=
pub async fn get_gui_runs(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<LimitQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RUNS_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let limit = i64::from(q.limit.unwrap_or(RUNS_DEFAULT_LIMIT));
    match db.agent_runs_recent(limit).await {
        Ok(rows) => ok(rows.into_iter().map(GuiRunRecord::from).collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

/// GET /api/v2/runs/gui/{run_id} — `data: null` for an unknown run.
pub async fn get_gui_run(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Path(run_id): Path<String>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, RUN_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    match db.agent_runs_get(&run_id).await {
        Ok(row) => ok(row.map(GuiRunRecord::from)),
        Err(e) => err("db_error", &e.to_string()),
    }
}

// ─── Activity ────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ActivityQuery {
    pub agent_id: Option<String>,
    pub kind: Option<String>,
    pub session_id: Option<String>,
    pub limit: Option<u32>,
    pub before_id: Option<i64>,
}

/// GET /api/v2/activity?agent_id=&kind=&session_id=&limit=&before_id= — newest first.
pub async fn get_activity(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<ActivityQuery>,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, ACTIVITY_SURFACES) {
        return e;
    }
    let db = match api_db(&gs) {
        Ok(db) => db,
        Err(e) => return e,
    };
    let filter = vox_db::ActivityLogFilter {
        agent_id: q.agent_id,
        kind: q.kind,
        session_id: q.session_id,
        limit: q
            .limit
            .unwrap_or(ACTIVITY_DEFAULT_LIMIT)
            .min(ACTIVITY_MAX_LIMIT),
        before_id: q.before_id,
    };
    match db.query_activity_log(&filter).await {
        Ok(rows) => ok(rows
            .into_iter()
            .map(ActivityRowDto::from)
            .collect::<Vec<_>>()),
        Err(e) => err("db_error", &e.to_string()),
    }
}

// ─── Orchestrator Control Plane & Mission Control (Remote Axis Parity) ───────

async fn dispatch_orch_rpc(gs: &GatewayState, method: &str, params: Value) -> Json<Value> {
    let state = gs.server_state();
    let req = vox_foundation::protocol::DispatchRequest {
        id: "http".to_string(),
        method: method.to_string(),
        params,
        auth_token: None,
        permission_mode: None,
    };
    let resp = vox_orchestrator::orch_daemon::dispatch_request(
        &state.repository.repository_id,
        Arc::clone(&state.orchestrator),
        &req,
    )
    .await;
    match resp.payload {
        vox_foundation::protocol::DispatchPayload::Result { value } => ok(value),
        vox_foundation::protocol::DispatchPayload::Error { message, .. } => {
            err("orchestrator_error", &message)
        }
        other => err(
            "orchestrator_error",
            &format!("unexpected dispatch payload: {other:?}"),
        ),
    }
}

/// GET /api/v2/orchestrator/tasks — live queue snapshot (`orch.list_tasks`).
pub async fn get_orchestrator_tasks(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::LIST_TASKS, serde_json::json!({})).await
}

/// POST /api/v2/orchestrator/tasks/submit — submit a task (`orch.submit_task`).
pub async fn post_submit_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::SUBMIT_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/edit — update queued task description (`orch.edit_task`).
pub async fn post_edit_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::EDIT_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/cancel — cancel a task (`orch.cancel_task`).
pub async fn post_cancel_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::CANCEL_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/interrupt — interrupt a task (`orch.interrupt_task`).
pub async fn post_interrupt_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::INTERRUPT_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/reorder — change task priority (`orch.reorder_task`).
pub async fn post_reorder_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::REORDER_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/doubt — mark a task as suspect (`orch.doubt_task`).
pub async fn post_doubt_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::DOUBT_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/overrule — overrule a doubted task (`orch.overrule_task`).
pub async fn post_overrule_task(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::OVERRULE_TASK, body).await
}

/// POST /api/v2/orchestrator/tasks/approve-plan — advance PAV loop (`orch.approve_plan`).
pub async fn post_approve_plan(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::APPROVE_PLAN, body).await
}

/// POST /api/v2/orchestrator/tasks/skip-verify — skip PAV verify (`orch.skip_verify`).
pub async fn post_skip_verify(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::SKIP_VERIFY, body).await
}

/// POST /api/v2/orchestrator/tasks/force-verify — force PAV verify (`orch.force_verify`).
pub async fn post_force_verify(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::FORCE_VERIFY, body).await
}

/// POST /api/v2/orchestrator/agents/pause — pause an agent (`orch.pause_agent`).
pub async fn post_pause_agent(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::PAUSE_AGENT, body).await
}

/// POST /api/v2/orchestrator/agents/resume — resume an agent (`orch.resume_agent`).
pub async fn post_resume_agent(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, CONTROL_PLANE_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::RESUME_AGENT, body).await
}

/// GET /api/v2/orchestrator/subagent-tree — delegation tree (`orch.subagent_tree`).
pub async fn get_subagent_tree(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, MISSION_CONTROL_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(
        &gs,
        orch_daemon_method::SUBAGENT_TREE,
        serde_json::json!({}),
    )
    .await
}

/// GET /api/v2/orchestrator/approvals — pending HITL approvals (`orch.list_pending_approvals`).
pub async fn get_pending_approvals(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Json<Value> {
    if let Err(e) = enforce_route(&gs, &connect.0, &headers, MISSION_CONTROL_SURFACES) {
        return e;
    }
    ok(serde_json::json!({
        "approvals": gs.server_state().pending_approvals.list(),
    }))
}

/// POST /api/v2/orchestrator/mesh-policy — update task mesh policy (`orch.set_mesh_policy`).
pub async fn post_mesh_policy(
    State(gs): State<GatewayState>,
    connect: ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Json<Value> {
    if let Err(e) = enforce_write_route(&gs, &connect.0, &headers, MISSION_CONTROL_SURFACES) {
        return e;
    }
    dispatch_orch_rpc(&gs, orch_daemon_method::SET_MESH_POLICY, body).await
}

pub fn api_v2_router() -> Router<GatewayState> {
    Router::new()
        .route("/hopper/list", get(get_hopper_list))
        .route("/orchestrator/context-budget", get(get_context_budget))
        .route("/orchestrator/tasks", get(get_orchestrator_tasks))
        .route("/orchestrator/tasks/submit", post(post_submit_task))
        .route("/orchestrator/tasks/edit", post(post_edit_task))
        .route("/orchestrator/tasks/cancel", post(post_cancel_task))
        .route("/orchestrator/tasks/interrupt", post(post_interrupt_task))
        .route("/orchestrator/tasks/reorder", post(post_reorder_task))
        .route("/orchestrator/tasks/doubt", post(post_doubt_task))
        .route("/orchestrator/tasks/overrule", post(post_overrule_task))
        .route("/orchestrator/tasks/approve-plan", post(post_approve_plan))
        .route("/orchestrator/tasks/skip-verify", post(post_skip_verify))
        .route("/orchestrator/tasks/force-verify", post(post_force_verify))
        .route("/orchestrator/agents/pause", post(post_pause_agent))
        .route("/orchestrator/agents/resume", post(post_resume_agent))
        .route("/orchestrator/subagent-tree", get(get_subagent_tree))
        .route("/orchestrator/approvals", get(get_pending_approvals))
        .route("/orchestrator/mesh-policy", post(post_mesh_policy))
        .route("/plan/open-task-counts", get(get_plan_open_task_counts))
        .route(
            "/plan/latest-for-chat/{session_id}",
            get(get_latest_plan_for_chat),
        )
        .route(
            "/plan/sessions/{plan_session_id}/nodes",
            get(get_plan_nodes),
        )
        .route("/runs/gui", get(get_gui_runs))
        .route("/runs/gui/{run_id}", get(get_gui_run))
        .route("/activity", get(get_activity))
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use axum::extract::{Path, Query, State};
    use axum::http::HeaderMap;

    use super::*;
    use crate::gui_ops::test_support::{body, gateway_with_memory_db, peer, tool_json};

    #[test]
    fn router_builds() {
        let _ = api_v2_router();
    }

    #[test]
    fn done_history_limit_is_the_agreed_bound() {
        assert_eq!(DONE_HISTORY_LIMIT, 50);
    }

    #[test]
    fn hopper_dto_carries_persisted_session_agent_and_mesh_fields() {
        use vox_orchestrator::hopper::IntakeItem;
        use vox_orchestrator::hopper::types::{IntakeSource, ItemState, PriorityHint};
        let mut item = IntakeItem::new(
            "wired intent".to_string(),
            vec![],
            PriorityHint::Normal,
            IntakeSource::Mesh {
                node_id: "did:vox:peer-1".into(),
            },
            Some("gui-session-9".to_string()),
        );
        item.state = ItemState::Assigned {
            agent_id: "agent-42".into(),
        };
        let dto = hopper_item_to_dto(&item);
        assert_eq!(dto.session_id.as_deref(), Some("gui-session-9"));
        assert_eq!(dto.agent_id.as_deref(), Some("agent-42"));
        assert_eq!(dto.remote_node.as_deref(), Some("did:vox:peer-1"));
        assert_eq!(
            dto.task_id,
            vox_orchestrator::orchestrator::dispatch::stable_hash(&item.item_id.0)
        );
    }

    #[test]
    fn task_priority_wire_values_match_frontend_constants() {
        // Mirror of crates/vox-gui/ui/src/lib/taskPriority.ts (TASK_PRIORITY_WIRE).
        use vox_orchestrator::types::TaskPriority;
        assert_eq!(TaskPriority::Background as u8, 0);
        assert_eq!(TaskPriority::Normal as u8, 1);
        assert_eq!(TaskPriority::Urgent as u8, 2);
    }

    async fn hopper_rows(gs: &GatewayState) -> Vec<Value> {
        let out = body(get_hopper_list(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        out["data"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| panic!("{out}"))
    }

    #[tokio::test]
    async fn hopper_submit_list_mark_done_share_the_gateway_queue() {
        let (gs, _db) = gateway_with_memory_db().await;
        let submitted = tool_json(
            &hopper_submit(
                gs.server_state(),
                HopperSubmitParams {
                    intent: "remote intent".into(),
                    affinity: vec![],
                },
            )
            .await,
        );
        assert_eq!(submitted["success"], true, "{submitted}");
        let item_id = submitted["data"]["item_id"].as_str().unwrap().to_string();

        let rows = hopper_rows(&gs).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["intent"], "remote intent");
        assert_eq!(rows[0]["state"], "inbox");

        let inbox = body(
            crate::http_gateway::dashboard_api::get_hopper_inbox(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
            )
            .await,
        )
        .await;
        assert_eq!(
            inbox["data"].as_array().map(Vec::len),
            Some(1),
            "the gateway's existing /hopper/inbox reads the same persistent queue: {inbox}"
        );

        let reprioritized = tool_json(
            &hopper_reprioritize(
                gs.server_state(),
                HopperReprioritizeParams {
                    item_id: item_id.clone(),
                    priority: 2,
                },
            )
            .await,
        );
        assert_eq!(reprioritized["data"]["priority"], 2, "{reprioritized}");

        let done = tool_json(
            &hopper_mark_done(
                gs.server_state(),
                HopperItemParams {
                    item_id: item_id.clone(),
                },
            )
            .await,
        );
        assert_eq!(done["data"]["state"], "done", "{done}");
        let rows = hopper_rows(&gs).await;
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0]["state"], "done");
    }

    #[tokio::test]
    async fn hopper_cancel_removes_the_item_from_the_open_list() {
        let (gs, _db) = gateway_with_memory_db().await;
        let submitted = tool_json(
            &hopper_submit(
                gs.server_state(),
                HopperSubmitParams {
                    intent: "drop me".into(),
                    affinity: vec![],
                },
            )
            .await,
        );
        let item_id = submitted["data"]["item_id"].as_str().unwrap().to_string();
        let cancelled =
            tool_json(&hopper_cancel(gs.server_state(), HopperItemParams { item_id }).await);
        assert_eq!(cancelled["success"], true, "{cancelled}");
        assert!(hopper_rows(&gs).await.is_empty());
    }

    #[tokio::test]
    async fn hopper_ops_need_a_db() {
        let state = ServerState::new_test().await;
        let out = tool_json(
            &hopper_submit(
                &state,
                HopperSubmitParams {
                    intent: "x".into(),
                    affinity: vec![],
                },
            )
            .await,
        );
        assert_eq!(out["success"], false, "{out}");
    }

    async fn seed_plan(db: &vox_db::VoxDb, id: &str, origin: Option<&str>, status: &str) {
        db.create_plan_session(id, origin, "goal", "sequential")
            .await
            .unwrap();
        db.append_plan_version(id, 1, None, None, None)
            .await
            .unwrap();
        db.upsert_plan_node(id, 1, "n1", "first step", "[]", "{}", status, None)
            .await
            .unwrap();
    }

    fn insert(id: &str, node: &str) -> PlanNodeUpsertParams {
        PlanNodeUpsertParams {
            mode: PlanNodeUpsertMode::Insert,
            update: None,
            insert: Some(InsertPlanNodeInput {
                plan_session_id: id.into(),
                plan_version: 1,
                node_id: node.into(),
                description: "a step the user added".into(),
                depends_on: vec![],
            }),
        }
    }

    #[tokio::test]
    async fn plan_node_update_insert_list_and_approve() {
        let (gs, db) = gateway_with_memory_db().await;
        seed_plan(&db, "ps1", None, "pending").await;
        let updated = tool_json(
            &plan_node_upsert(
                gs.server_state(),
                PlanNodeUpsertParams {
                    mode: PlanNodeUpsertMode::Update,
                    update: Some(UpdatePlanNodeInput {
                        plan_session_id: "ps1".into(),
                        plan_version: 1,
                        node_id: "n1".into(),
                        description: "edited description".into(),
                    }),
                    insert: None,
                },
            )
            .await,
        );
        assert_eq!(updated["success"], true, "{updated}");
        let inserted = tool_json(&plan_node_upsert(gs.server_state(), insert("ps1", "n2")).await);
        assert_eq!(inserted["success"], true, "{inserted}");

        let nodes = body(
            get_plan_nodes(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("ps1".to_string()),
                Query(PlanVersionQuery { version: 1 }),
            )
            .await,
        )
        .await;
        let nodes = nodes["data"]
            .as_array()
            .unwrap_or_else(|| panic!("{nodes}"));
        let by_id = |id: &str| nodes.iter().find(|n| n["node_id"] == id).unwrap().clone();
        assert_eq!(by_id("n1")["description"], "edited description");
        assert_eq!(by_id("n2")["status"], "pending");

        seed_plan(&db, "ps-gate", None, "blocked_on_approval").await;
        tool_json(&plan_node_upsert(gs.server_state(), insert("ps-gate", "n-new")).await);
        let rows = db.load_plan_nodes_with_status("ps-gate", 1).await.unwrap();
        assert_eq!(
            rows.iter().find(|r| r.node_id == "n-new").unwrap().status,
            "blocked_on_approval",
            "a node inserted into an unapproved plan must not be immediately runnable"
        );
        let approved = tool_json(
            &plan_nodes_approve(
                gs.server_state(),
                PlanNodesApproveParams {
                    plan_session_id: "ps-gate".into(),
                },
            )
            .await,
        );
        assert_eq!(approved["data"], 2, "{approved}");
    }

    #[tokio::test]
    async fn plan_upsert_rejects_a_mode_without_its_payload() {
        let (gs, _db) = gateway_with_memory_db().await;
        let out = tool_json(
            &plan_node_upsert(
                gs.server_state(),
                PlanNodeUpsertParams {
                    mode: PlanNodeUpsertMode::Update,
                    update: None,
                    insert: None,
                },
            )
            .await,
        );
        assert_eq!(out["success"], false, "{out}");
    }

    #[tokio::test]
    async fn plan_counts_and_latest_for_chat_routes() {
        let (gs, db) = gateway_with_memory_db().await;
        seed_plan(&db, "ps-count", Some("chat-x"), "pending").await;
        let counts = body(
            get_plan_open_task_counts(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(SessionIdsQuery {
                    session_ids: Some("chat-x,chat-y".into()),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(counts["data"]["chat-x"], 1, "{counts}");
        assert!(counts["data"].get("chat-y").is_none());

        let latest = body(
            get_latest_plan_for_chat(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("chat-x".to_string()),
            )
            .await,
        )
        .await;
        assert_eq!(latest["data"]["plan_session_id"], "ps-count", "{latest}");
        assert_eq!(latest["data"]["plan_version"], 1);
        let none = body(
            get_latest_plan_for_chat(
                State(gs),
                peer(),
                HeaderMap::new(),
                Path("chat-none".to_string()),
            )
            .await,
        )
        .await;
        assert!(none["data"].is_null(), "{none}");
    }

    #[tokio::test]
    async fn gui_run_start_finish_list_get() {
        let (gs, _db) = gateway_with_memory_db().await;
        let started = tool_json(
            &gui_run_start(
                gs.server_state(),
                StartGuiRunInput {
                    run_id: "gui-1".into(),
                    workflow_name: "submit".into(),
                    planned_steps: Some(2),
                    command: None,
                    repo: None,
                    worktree: None,
                    model: Some("m".into()),
                },
            )
            .await,
        );
        assert_eq!(started["success"], true, "{started}");
        let finished = tool_json(
            &gui_run_finish(
                gs.server_state(),
                FinishGuiRunInput {
                    run_id: "gui-1".into(),
                    success: false,
                    completed_steps: Some(1),
                    error: Some("boom".into()),
                    cost_usd: Some(0.25),
                    tokens_in: None,
                    tokens_out: None,
                    task_id: None,
                },
            )
            .await,
        );
        assert_eq!(finished["success"], true, "{finished}");

        let list = body(
            get_gui_runs(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(LimitQuery { limit: Some(10) }),
            )
            .await,
        )
        .await;
        let runs = list["data"].as_array().unwrap_or_else(|| panic!("{list}"));
        assert_eq!(runs.len(), 1);
        assert_eq!(runs[0]["status"], "failed");
        assert_eq!(runs[0]["model"], "m");
        let one = body(
            get_gui_run(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Path("gui-1".to_string()),
            )
            .await,
        )
        .await;
        assert_eq!(one["data"]["last_error"], "boom", "{one}");
        assert_eq!(one["data"]["cost_usd"], 0.25);
        let missing = body(
            get_gui_run(
                State(gs),
                peer(),
                HeaderMap::new(),
                Path("nope".to_string()),
            )
            .await,
        )
        .await;
        assert!(missing["data"].is_null(), "{missing}");
    }

    #[tokio::test]
    async fn activity_route_applies_the_filter() {
        let (gs, db) = gateway_with_memory_db().await;
        for (ts, agent) in [(1, "A1"), (2, "A2"), (3, "A1")] {
            db.insert_activity_log_row(ts, Some(agent), None, "TaskCompleted", "s", "{}")
                .await
                .unwrap();
        }
        let out = body(
            get_activity(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(ActivityQuery {
                    agent_id: Some("A1".into()),
                    kind: None,
                    session_id: None,
                    limit: Some(10),
                    before_id: None,
                }),
            )
            .await,
        )
        .await;
        let ts: Vec<i64> = out["data"]
            .as_array()
            .unwrap_or_else(|| panic!("{out}"))
            .iter()
            .map(|r| r["ts_ms"].as_i64().unwrap())
            .collect();
        assert_eq!(ts, vec![3, 1]);
    }

    #[tokio::test]
    async fn context_budget_reads_the_latest_interaction() {
        let (gs, db) = gateway_with_memory_db().await;
        db.log_interaction("s1", None, "p", "r", "m", Some(0), Some(30), Some(12))
            .await
            .unwrap();
        let none = body(
            get_context_budget(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Query(SessionQuery { session_id: None }),
            )
            .await,
        )
        .await;
        assert_eq!(none["data"]["used_tokens"], 0, "{none}");
        assert!(none["data"]["max_context_tokens"].as_u64().unwrap() > 0);
        let out = body(
            get_context_budget(
                State(gs),
                peer(),
                HeaderMap::new(),
                Query(SessionQuery {
                    session_id: Some("s1".into()),
                }),
            )
            .await,
        )
        .await;
        assert_eq!(out["data"]["used_tokens"], 42, "{out}");
    }

    #[tokio::test]
    async fn orchestrator_routes_are_profile_gated() {
        let named = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["mesh"]).expect("mesh profile"),
        );
        let gs = GatewayState::for_test_with_state(named, HashSet::new()).await;
        let denied = body(get_hopper_list(State(gs), peer(), HeaderMap::new()).await).await;
        assert_eq!(denied["error"]["code"], "tool_not_in_profile", "{denied}");
        let orch = ServerState::new_test().await.with_profile(
            crate::profiles::ActiveProfile::resolve(&["orchestrator"]).expect("orchestrator"),
        );
        let gs = GatewayState::for_test_with_state(orch, HashSet::new()).await;
        let served = body(get_hopper_list(State(gs), peer(), HeaderMap::new()).await).await;
        assert_ne!(served["error"]["code"], "tool_not_in_profile", "{served}");
    }

    #[tokio::test]
    async fn submit_list_mesh_policy_subagent_tree_and_approvals_routes_round_trip() {
        let (gs, _db) = gateway_with_memory_db().await;
        let submitted = body(
            post_submit_task(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Json(serde_json::json!({
                    "description": "remote routed task",
                })),
            )
            .await,
        )
        .await;
        let task_id = submitted["data"]["task_id"]
            .as_u64()
            .unwrap_or_else(|| panic!("{submitted}"));

        let listed =
            body(get_orchestrator_tasks(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert_eq!(listed["data"]["tasks"][0]["id"], task_id, "{listed}");

        let policy_resp = body(
            post_mesh_policy(
                State(gs.clone()),
                peer(),
                HeaderMap::new(),
                Json(serde_json::json!({
                    "task_id": task_id,
                    "policy": "local_only",
                })),
            )
            .await,
        )
        .await;
        assert_eq!(policy_resp["data"]["ok"], true, "{policy_resp}");

        let tree_resp =
            body(get_subagent_tree(State(gs.clone()), peer(), HeaderMap::new()).await).await;
        assert!(tree_resp["data"]["tree"].is_array(), "{tree_resp}");

        let approvals_resp =
            body(get_pending_approvals(State(gs), peer(), HeaderMap::new()).await).await;
        assert!(
            approvals_resp["data"]["approvals"].is_array(),
            "{approvals_resp}"
        );
    }
}
