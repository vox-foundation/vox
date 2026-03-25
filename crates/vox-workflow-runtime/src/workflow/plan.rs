//! Walk HIR to build a linear activity plan from workflow statements.

use anyhow::Context;
use vox_compiler::hir::{HirExpr, HirModule, HirStmt};

use super::types::{PlannedActivity, PopuliHttpOp};

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
                "mens" => {
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

fn parse_populi_control_op(s: &str) -> anyhow::Result<PopuliHttpOp> {
    match s.trim() {
        "noop" => Ok(PopuliHttpOp::Noop),
        "join" => Ok(PopuliHttpOp::Join),
        "snapshot" => Ok(PopuliHttpOp::Snapshot),
        "heartbeat" => Ok(PopuliHttpOp::Heartbeat),
        other => anyhow::bail!(
            "unknown workflow mens control {:?}; expected noop|join|snapshot|heartbeat",
            other
        ),
    }
}

fn resolve_populi_http_op(name: &str, mesh_key: Option<&str>) -> anyhow::Result<PopuliHttpOp> {
    if let Some(k) = mesh_key {
        return parse_populi_control_op(k);
    }
    match name {
        "mesh_noop" => Ok(PopuliHttpOp::Noop),
        "mesh_join" => Ok(PopuliHttpOp::Join),
        "mesh_snapshot" => Ok(PopuliHttpOp::Snapshot),
        _ if name.starts_with("mesh_") => Ok(PopuliHttpOp::Heartbeat),
        _ => Ok(PopuliHttpOp::Heartbeat),
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
                let mens = name.starts_with("mesh_");
                if ctx.mesh_key.is_some() && !mens {
                    anyhow::bail!(
                        "workflow `{workflow_name}`: `mens` in `with {{ … }}` only applies to mesh_* activities (got `{name}`)"
                    );
                }
                let populi_op = resolve_populi_http_op(name, ctx.mesh_key.as_deref())?;
                out.push(PlannedActivity {
                    name: name.clone(),
                    mens,
                    activity_id: ctx.activity_id.clone(),
                    timeout_ms: ctx.timeout_ms,
                    populi_op,
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
