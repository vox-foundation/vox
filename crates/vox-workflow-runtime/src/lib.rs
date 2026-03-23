//! Minimal **interpreted** workflow runner: walks a [`vox_hir::HirModule`] workflow body for
//! activity calls and executes **no-op** steps with optional mesh hooks.
//!
//! - Activities whose name starts with `mesh_` are treated as [`MeshActivity`] steps when the
//!   **`mesh`** feature is enabled: they register with [`vox_mesh::publish_local_registry_best_effort`]
//!   and call the mesh HTTP control plane derived from **`VOX_MESH_CONTROL_ADDR`** / `Vox.toml`
//!   `[mesh]` (never a user-supplied URL in workflow source). Use `with { mesh: "noop" | "join" |
//!   "snapshot" | "heartbeat" }` to select the operation; see `mesh_noop`, `mesh_join`,
//!   `mesh_snapshot` shorthands.
//! - Other activities are recorded as local no-ops (journal only).
//!
//! **Codex journal:** when **`VOX_WORKFLOW_JOURNAL_CODEX=1`** (and DB config resolves), `vox-cli`
//! persists the interpreted journal after a successful run via `VoxDb::record_workflow_journal_entry`
//! (see `docs/src/architecture/orchestration-unified-ssot.md`). Journal rows include
//! **`ActivityStarted` / `ActivityCompleted`** with **`activity_id`** for idempotency hints.
//!
//! This crate is the MVP engine behind `vox populi workflow run` when `vox-cli` is built with
//! **`workflow-runtime`**.

#![deny(missing_docs)]

use anyhow::Context;
use serde_json::{Value, json};
use vox_hir::{HirExpr, HirModule, HirStmt};

/// Control-plane sub-step for a [`MeshActivity`] (URL always comes from env / `Vox.toml`, not source).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshHttpOp {
    /// `POST` heartbeat with the current node record.
    Heartbeat,
    /// Log only; still runs local registry publish when mesh is enabled.
    Noop,
    /// `POST /v1/mesh/join` for this process record.
    Join,
    /// `GET /v1/mesh/nodes` (counts in journal only; no arbitrary URLs).
    Snapshot,
}

/// One planned activity invocation extracted from workflow HIR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedActivity {
    /// Activity name as referenced in the workflow body.
    pub name: String,
    /// When true, run [`execute_mesh_step`].
    pub mesh: bool,
    /// Idempotency / journal key from `with { activity_id: "…" }` when set.
    pub activity_id: Option<String>,
    /// Wall-clock timeout for mesh HTTP sub-steps from `with { timeout: … }` (milliseconds).
    pub timeout_ms: Option<u64>,
    /// Mesh control-plane operation when [`Self::mesh`] is true.
    pub mesh_op: MeshHttpOp,
}

/// Mesh-tagged activity (name convention: `mesh_*`, plus [`MeshHttpOp`]).
#[derive(Debug, Clone)]
pub struct MeshActivity {
    /// Activity name from source.
    pub name: String,
    /// Resolved mesh HTTP operation.
    pub mesh_op: MeshHttpOp,
    /// Timeout for mesh HTTP client (defaults inside [`execute_mesh_step`] when unset).
    pub timeout_ms: Option<u64>,
    /// Stable id for journal / idempotency (`with { activity_id }` or generated).
    pub activity_id: String,
}

/// Extract ordered activity call names from a workflow's statements.
pub fn plan_workflow_activities(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<PlannedActivity>> {
    let wf = hir
        .workflows
        .iter()
        .find(|w| w.name == workflow_name)
        .with_context(|| format!("workflow '{workflow_name}' not found"))?;
    let mut raw = Vec::new();
    collect_activity_calls_from_stmts(
        workflow_name,
        &wf.body,
        &ActivityWithOpts::default(),
        &mut raw,
    )?;
    Ok(raw)
}

#[derive(Clone, Default, Debug)]
struct ActivityWithOpts {
    activity_id: Option<String>,
    timeout_ms: Option<u64>,
    mesh_key: Option<String>,
}

impl ActivityWithOpts {
    fn merged_with(&self, opts: &HirExpr) -> anyhow::Result<Self> {
        let mut n = self.clone();
        let HirExpr::ObjectLit(fields, _) = opts else {
            return Ok(n);
        };
        for (k, v) in fields {
            match k.as_str() {
                "activity_id" | "id" => {
                    if let HirExpr::StringLit(s, _) = v {
                        n.activity_id = Some(s.clone());
                    }
                }
                "timeout" => {
                    n.timeout_ms = Some(parse_timeout_ms(v)?);
                }
                "mesh" => {
                    if let HirExpr::StringLit(s, _) = v {
                        n.mesh_key = Some(s.clone());
                    }
                }
                _ => {}
            }
        }
        Ok(n)
    }
}

fn parse_timeout_ms(expr: &HirExpr) -> anyhow::Result<u64> {
    match expr {
        HirExpr::IntLit(ms, _) => Ok(*ms as u64),
        HirExpr::StringLit(s, _) => {
            parse_duration_ms_str(s).with_context(|| format!("invalid workflow timeout {:?}", s))
        }
        _ => anyhow::bail!("workflow `timeout` must be an int (milliseconds) or duration string"),
    }
}

fn parse_duration_ms_str(s: &str) -> anyhow::Result<u64> {
    let s = s.trim();
    if let Ok(n) = s.parse::<u64>() {
        return Ok(n);
    }
    if let Some(rest) = s.strip_suffix("ms") {
        return Ok(rest.trim().parse()?);
    }
    if let Some(rest) = s.strip_suffix('s') {
        let n: u64 = rest.trim().parse()?;
        return Ok(n.saturating_mul(1000));
    }
    if let Some(rest) = s.strip_suffix('m') {
        let n: u64 = rest.trim().parse()?;
        return Ok(n.saturating_mul(60_000));
    }
    anyhow::bail!("expected duration like 5000, \"30s\", \"500ms\", \"2m\"");
}

fn parse_mesh_control_op(s: &str) -> anyhow::Result<MeshHttpOp> {
    match s.trim() {
        "noop" => Ok(MeshHttpOp::Noop),
        "join" => Ok(MeshHttpOp::Join),
        "snapshot" => Ok(MeshHttpOp::Snapshot),
        "heartbeat" => Ok(MeshHttpOp::Heartbeat),
        other => anyhow::bail!(
            "unknown workflow mesh control {:?}; expected noop|join|snapshot|heartbeat",
            other
        ),
    }
}

fn resolve_mesh_http_op(name: &str, mesh_key: Option<&str>) -> anyhow::Result<MeshHttpOp> {
    if let Some(k) = mesh_key {
        return parse_mesh_control_op(k);
    }
    match name {
        "mesh_noop" => Ok(MeshHttpOp::Noop),
        "mesh_join" => Ok(MeshHttpOp::Join),
        "mesh_snapshot" => Ok(MeshHttpOp::Snapshot),
        _ if name.starts_with("mesh_") => Ok(MeshHttpOp::Heartbeat),
        _ => Ok(MeshHttpOp::Heartbeat),
    }
}

fn collect_activity_calls_from_stmts(
    workflow_name: &str,
    stmts: &[HirStmt],
    ctx: &ActivityWithOpts,
    out: &mut Vec<PlannedActivity>,
) -> anyhow::Result<()> {
    for s in stmts {
        match s {
            HirStmt::Let { value, .. } => collect_from_expr(workflow_name, value, ctx, out)?,
            HirStmt::Assign { value, .. } => collect_from_expr(workflow_name, value, ctx, out)?,
            HirStmt::Return { value, .. } => {
                if let Some(v) = value {
                    collect_from_expr(workflow_name, v, ctx, out)?;
                }
            }
            HirStmt::Expr { expr, .. } => collect_from_expr(workflow_name, expr, ctx, out)?,
        }
    }
    Ok(())
}

fn collect_from_expr(
    workflow_name: &str,
    expr: &HirExpr,
    ctx: &ActivityWithOpts,
    out: &mut Vec<PlannedActivity>,
) -> anyhow::Result<()> {
    match expr {
        HirExpr::With(inner, opts, _) => {
            let merged = ctx.merged_with(opts)?;
            collect_from_expr(workflow_name, inner, &merged, out)?;
        }
        HirExpr::Call(callee, _, _, _) => {
            if let HirExpr::Ident(name, _) = &**callee {
                let mesh = name.starts_with("mesh_");
                if ctx.mesh_key.is_some() && !mesh {
                    anyhow::bail!(
                        "workflow `{workflow_name}`: `mesh` in `with {{ … }}` only applies to mesh_* activities (got `{name}`)"
                    );
                }
                let mesh_op = resolve_mesh_http_op(name, ctx.mesh_key.as_deref())?;
                out.push(PlannedActivity {
                    name: name.clone(),
                    mesh,
                    activity_id: ctx.activity_id.clone(),
                    timeout_ms: ctx.timeout_ms,
                    mesh_op,
                });
            } else {
                collect_from_expr(workflow_name, callee, ctx, out)?;
            }
        }
        HirExpr::If(_, then_b, else_b, _) => {
            collect_activity_calls_from_stmts(workflow_name, then_b, ctx, out)?;
            if let Some(e) = else_b {
                collect_activity_calls_from_stmts(workflow_name, e, ctx, out)?;
            }
        }
        HirExpr::Block(stmts, _) => {
            collect_activity_calls_from_stmts(workflow_name, stmts, ctx, out)?
        }
        HirExpr::Binary(_, a, b, _) => {
            collect_from_expr(workflow_name, a, ctx, out)?;
            collect_from_expr(workflow_name, b, ctx, out)?;
        }
        HirExpr::Unary(_, a, _) => collect_from_expr(workflow_name, a, ctx, out)?,
        HirExpr::Match(scrut, arms, _) => {
            collect_from_expr(workflow_name, scrut, ctx, out)?;
            for arm in arms {
                if let Some(g) = &arm.guard {
                    collect_from_expr(workflow_name, g, ctx, out)?;
                }
                collect_from_expr(workflow_name, &arm.body, ctx, out)?;
            }
        }
        HirExpr::MethodCall(recv, _, args, _) => {
            collect_from_expr(workflow_name, recv, ctx, out)?;
            for a in args {
                collect_from_expr(workflow_name, &a.value, ctx, out)?;
            }
        }
        HirExpr::FieldAccess(recv, _, _) => collect_from_expr(workflow_name, recv, ctx, out)?,
        HirExpr::Pipe(a, b, _) => {
            collect_from_expr(workflow_name, a, ctx, out)?;
            collect_from_expr(workflow_name, b, ctx, out)?;
        }
        HirExpr::Lambda(_, _, body, _) => collect_from_expr(workflow_name, body, ctx, out)?,
        HirExpr::For(_, iter, body, _) => {
            collect_from_expr(workflow_name, iter, ctx, out)?;
            collect_from_expr(workflow_name, body, ctx, out)?;
        }
        HirExpr::Spawn(inner, _) => collect_from_expr(workflow_name, inner, ctx, out)?,
        HirExpr::ListLit(items, _) => {
            for it in items {
                collect_from_expr(workflow_name, it, ctx, out)?;
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                collect_from_expr(workflow_name, v, ctx, out)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// An engine tracker that allows the interpreted runner to persist durable states.
pub trait WorkflowTracker: Send + Sync {
    /// Check if a specific step was already completed in a prior, durable run.
    fn is_activity_completed(
        &self,
        _workflow_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<bool>> + Send {
        async { Ok(false) }
    }

    /// Called when the workflow plan begins.
    fn on_workflow_started(
        &mut self,
        _workflow_name: &str,
        _plan_len: usize,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when an activity starts execution.
    fn on_activity_started(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when an activity fully completes.
    fn on_activity_completed(
        &mut self,
        _workflow_name: &str,
        _activity_name: &str,
        _activity_id: &str,
        _result: &Value,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }

    /// Called when the workflow successfully completes all steps.
    fn on_workflow_completed(
        &mut self,
        _workflow_name: &str,
    ) -> impl std::future::Future<Output = anyhow::Result<()>> + Send {
        async { Ok(()) }
    }
}

/// A default no-op tracker used if none is provided.
#[derive(Default)]
pub struct DefaultTracker;

impl WorkflowTracker for DefaultTracker {}

/// Execute a planned workflow and append journal entries.
pub async fn interpret_workflow(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<Value>> {
    let mut tracker = DefaultTracker;
    interpret_workflow_durable(hir, workflow_name, &mut tracker).await
}

/// Execute a planned workflow with a durable state engine, returning journal entries.
pub async fn interpret_workflow_durable(
    hir: &HirModule,
    workflow_name: &str,
    tracker: &mut impl WorkflowTracker,
) -> anyhow::Result<Vec<Value>> {
    let plan = plan_workflow_activities(hir, workflow_name)?;
    let mut journal = Vec::new();
    tracker.on_workflow_started(workflow_name, plan.len()).await?;
    journal.push(json!({
        "event": "WorkflowStarted",
        "workflow": workflow_name,
        "steps": plan.len(),
    }));
    for (idx, step) in plan.iter().enumerate() {
        let activity_id = step
            .activity_id
            .clone()
            .unwrap_or_else(|| format!("{workflow_name}-{idx}"));

        if tracker.is_activity_completed(workflow_name, &activity_id).await? {
            journal.push(json!({
                "event": "ActivitySkipped",
                "workflow": workflow_name,
                "activity": step.name,
                "activity_id": activity_id,
                "reason": "already completed in prior durable run",
            }));
            continue;
        }

        tracker.on_activity_started(workflow_name, &step.name, &activity_id).await?;
        journal.push(json!({
            "event": "ActivityStarted",
            "workflow": workflow_name,
            "activity": step.name,
            "activity_id": activity_id,
        }));
        
        let mut entry = if step.mesh {
            #[cfg(feature = "mesh")]
            {
                let m = MeshActivity {
                    name: step.name.clone(),
                    mesh_op: step.mesh_op,
                    timeout_ms: step.timeout_ms,
                    activity_id: activity_id.clone(),
                };
                execute_mesh_step(&m).await?
            }
            #[cfg(not(feature = "mesh"))]
            {
                json!({
                    "event": "MeshActivitySkipped",
                    "activity": step.name,
                    "activity_id": activity_id,
                    "reason": "vox-workflow-runtime built without mesh feature",
                })
            }
        } else {
            json!({
                "event": "LocalActivity",
                "activity": step.name,
                "activity_id": activity_id,
                "status": "noop",
            })
        };
        
        tracker.on_activity_completed(workflow_name, &step.name, &activity_id, &entry).await?;
        journal.push(entry);

        journal.push(json!({
            "event": "ActivityCompleted",
            "workflow": workflow_name,
            "activity": step.name,
            "activity_id": activity_id,
        }));
    }
    tracker.on_workflow_completed(workflow_name).await?;
    journal.push(json!({
        "event": "WorkflowCompleted",
        "workflow": workflow_name,
    }));
    Ok(journal)
}

/// Best-effort mesh registration + optional control-plane HTTP (env-derived base URL only).
#[cfg(feature = "mesh")]
pub async fn execute_mesh_step(activity: &MeshActivity) -> anyhow::Result<Value> {
    let _ = vox_mesh::publish_local_registry_best_effort();
    let vox = vox_mesh::resolve_vox_toml_best_effort();
    let env = vox_mesh::mesh_env_resolved(vox.as_deref());
    let timeout = std::time::Duration::from_millis(activity.timeout_ms.unwrap_or(30_000).max(250));
    if let Some(base) = env.control_addr.clone() {
        let client = vox_mesh::http_client::MeshHttpClient::new_with_timeout(
            normalize_control_base(&base),
            timeout,
        )
        .with_env_token();
        let id = env
            .node_id
            .clone()
            .unwrap_or_else(|| format!("wf-{}", activity.name.replace(' ', "_")));
        let node = vox_mesh::node_record_for_current_process(id, Some(base.clone()));
        let mesh_op = mesh_op_json(activity.mesh_op);
        match activity.mesh_op {
            MeshHttpOp::Noop => Ok(json!({
                "event": "MeshActivity",
                "activity": activity.name,
                "activity_id": activity.activity_id,
                "mesh_op": mesh_op,
                "control": "noop",
            })),
            MeshHttpOp::Join => match client.join(&node).await {
                Ok(n) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "join_ok",
                    "node_id": n.id,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "join_err",
                    "error": e.to_string(),
                })),
            },
            MeshHttpOp::Snapshot => match client.list_nodes().await {
                Ok(f) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "snapshot_ok",
                    "node_count": f.nodes.len(),
                    "schema_version": f.schema_version,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "snapshot_err",
                    "error": e.to_string(),
                })),
            },
            MeshHttpOp::Heartbeat => match client.heartbeat(&node).await {
                Ok(n) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "heartbeat_ok",
                    "node_id": n.id,
                })),
                Err(e) => Ok(json!({
                    "event": "MeshActivity",
                    "activity": activity.name,
                    "activity_id": activity.activity_id,
                    "mesh_op": mesh_op,
                    "control": "heartbeat_err",
                    "error": e.to_string(),
                })),
            },
        }
    } else {
        Ok(json!({
            "event": "MeshActivity",
            "activity": activity.name,
            "activity_id": activity.activity_id,
            "mesh_op": mesh_op_json(activity.mesh_op),
            "control": "local_registry_only",
        }))
    }
}

#[cfg(feature = "mesh")]
fn mesh_op_json(op: MeshHttpOp) -> &'static str {
    match op {
        MeshHttpOp::Heartbeat => "heartbeat",
        MeshHttpOp::Noop => "noop",
        MeshHttpOp::Join => "join",
        MeshHttpOp::Snapshot => "snapshot",
    }
}

#[cfg(feature = "mesh")]
fn normalize_control_base(addr: &str) -> String {
    let a = addr.trim();
    if a.starts_with("http://") || a.starts_with("https://") {
        a.to_string()
    } else {
        format!("http://{a}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_ast::span::Span;
    use vox_hir::{DefId, HirWorkflow};

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn minimal_wf(body: Vec<HirStmt>) -> HirModule {
        HirModule {
            imports: vec![],
            functions: vec![],
            types: vec![],
            routes: vec![],
            actors: vec![],
            workflows: vec![HirWorkflow {
                id: DefId(0),
                name: "demo".to_string(),
                params: vec![],
                return_type: None,
                body,
                span: span(),
            }],
            activities: vec![],
            tests: vec![],
            server_fns: vec![],
            tables: vec![],
            indexes: vec![],
            mcp_tools: vec![],
        }
    }

    #[test]
    fn plan_finds_mesh_and_local_calls() {
        let body = vec![
            HirStmt::Expr {
                expr: HirExpr::Call(
                    Box::new(HirExpr::Ident("local_step".to_string(), span())),
                    vec![],
                    false,
                    span(),
                ),
                span: span(),
            },
            HirStmt::Expr {
                expr: HirExpr::With(
                    Box::new(HirExpr::Call(
                        Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                        vec![],
                        false,
                        span(),
                    )),
                    Box::new(HirExpr::ObjectLit(vec![], span())),
                    span(),
                ),
                span: span(),
            },
        ];
        let hir = minimal_wf(body);
        let p = plan_workflow_activities(&hir, "demo").expect("plan");
        assert_eq!(p.len(), 2);
        assert!(!p[0].mesh);
        assert_eq!(p[0].mesh_op, MeshHttpOp::Heartbeat);
        assert!(p[1].mesh);
        assert_eq!(p[1].mesh_op, MeshHttpOp::Heartbeat);
    }

    #[test]
    fn plan_mesh_snapshot_from_with_block() {
        let body = vec![HirStmt::Expr {
            expr: HirExpr::With(
                Box::new(HirExpr::Call(
                    Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                    vec![],
                    false,
                    span(),
                )),
                Box::new(HirExpr::ObjectLit(
                    vec![(
                        "mesh".to_string(),
                        HirExpr::StringLit("snapshot".to_string(), span()),
                    )],
                    span(),
                )),
                span(),
            ),
            span: span(),
        }];
        let hir = minimal_wf(body);
        let p = plan_workflow_activities(&hir, "demo").expect("plan");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].mesh_op, MeshHttpOp::Snapshot);
    }

    #[tokio::test]
    async fn interpret_workflow_journal_local_and_mesh() {
        let body = vec![
            HirStmt::Expr {
                expr: HirExpr::Call(
                    Box::new(HirExpr::Ident("local_step".to_string(), span())),
                    vec![],
                    false,
                    span(),
                ),
                span: span(),
            },
            HirStmt::Expr {
                expr: HirExpr::With(
                    Box::new(HirExpr::Call(
                        Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                        vec![],
                        false,
                        span(),
                    )),
                    Box::new(HirExpr::ObjectLit(vec![], span())),
                    span(),
                ),
                span: span(),
            },
        ];
        let hir = minimal_wf(body);
        let journal = interpret_workflow(&hir, "demo").await.expect("interpret");
        let events: Vec<_> = journal
            .iter()
            .filter_map(|v| v.get("event").and_then(|e| e.as_str()))
            .collect();
        assert!(events.contains(&"WorkflowStarted"));
        assert!(events.contains(&"ActivityStarted"));
        assert!(events.contains(&"ActivityCompleted"));
        assert!(events.contains(&"LocalActivity"));
        assert!(events.contains(&"MeshActivity"));
        assert!(events.contains(&"WorkflowCompleted"));
    }
}
