//! Walk HIR to build a linear activity plan from workflow statements.

use anyhow::Context;
use vox_compiler::hir::{HirBinOp, HirExpr, HirModule, HirStmt, HirUnOp};

use super::types::{PlannedActivity, PopuliHttpOp, ReplayNode, WorkflowReplayIr};

/// Extract ordered activity call names from a workflow's statements.
pub fn plan_workflow_activities(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<PlannedActivity>> {
    let ir = plan_workflow_replay_ir(hir, workflow_name)?;
    Ok(ir
        .nodes
        .into_iter()
        .map(|node| match node {
            ReplayNode::Activity(activity) => activity,
        })
        .collect())
}

/// Build replay-oriented linear IR from workflow statements.
pub fn plan_workflow_replay_ir(
    _hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<WorkflowReplayIr> {
    // Workflow primitives are retired (TASK-2.6 Path B). The parser tombstones
    // `workflow`/`activity` source forms. This planner is kept as a stub so the
    // crate compiles; it returns an empty replay IR. Use `@durable fn` instead.
    let _ = workflow_name;
    Ok(WorkflowReplayIr { nodes: vec![] })
}

#[derive(Clone, Default, Debug)]
struct ActivityWithOpts {
    activity_id: Option<String>,
    timeout_ms: Option<u64>,
    mesh_key: Option<String>,
    retries: u32,
    initial_backoff_ms: Option<u64>,
    required_labels: Option<Vec<String>>,
    is_detached: bool,
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
                "retries" => {
                    n.retries = parse_retries(v)?;
                }
                "initial_backoff" => {
                    n.initial_backoff_ms = Some(parse_timeout_ms(v)?);
                }
                "labels" => {
                    if let HirExpr::ListLit(items, _) = v {
                        let mut labels = Vec::new();
                        for it in items {
                            if let HirExpr::StringLit(s, _) = it {
                                labels.push(s.clone());
                            }
                        }
                        n.required_labels = Some(labels);
                    }
                }
                "detach" => {
                    if let HirExpr::BoolLit(b, _) = v {
                        n.is_detached = *b;
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

fn parse_retries(expr: &HirExpr) -> anyhow::Result<u32> {
    match expr {
        HirExpr::IntLit(n, _) if *n >= 0 => {
            u32::try_from(*n).map_err(|_| anyhow::anyhow!("workflow `retries` is too large"))
        }
        HirExpr::IntLit(_, _) => anyhow::bail!("workflow `retries` must be a non-negative integer"),
        _ => anyhow::bail!("workflow `retries` must be an int"),
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
        "dispatch" => Ok(PopuliHttpOp::Dispatch),
        "wait" => Ok(PopuliHttpOp::Wait),
        other => anyhow::bail!(
            "unknown workflow mens control {:?}; expected noop|join|snapshot|heartbeat|dispatch|wait",
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
        "mesh_dispatch" => Ok(PopuliHttpOp::Dispatch),
        "mesh_wait" => Ok(PopuliHttpOp::Wait),
        _ if name.starts_with("mesh_") => Ok(PopuliHttpOp::Heartbeat),
        _ => Ok(PopuliHttpOp::Heartbeat),
    }
}

fn collect_activity_calls_from_stmts(
    workflow_name: &str,
    stmts: &[HirStmt],
    ctx: &ActivityWithOpts,
    out: &mut Vec<PlannedActivity>,
    branch_counter: &mut usize,
) -> anyhow::Result<()> {
    for s in stmts {
        match s {
            HirStmt::Let { value, .. } => {
                collect_from_expr(workflow_name, value, ctx, out, branch_counter)?
            }
            HirStmt::Assign { value, .. } => {
                collect_from_expr(workflow_name, value, ctx, out, branch_counter)?
            }
            HirStmt::Return { value, .. } => {
                if let Some(v) = value {
                    collect_from_expr(workflow_name, v, ctx, out, branch_counter)?;
                }
            }
            HirStmt::Expr { expr, .. } => {
                collect_from_expr(workflow_name, expr, ctx, out, branch_counter)?
            }
            HirStmt::While { .. } | HirStmt::Loop { .. } => {
                anyhow::bail!(
                    "workflow `{workflow_name}`: interpreted durable planning does not support `while` or `loop` statements"
                )
            }
            HirStmt::Break { .. } | HirStmt::Continue { .. } => {
                anyhow::bail!(
                    "workflow `{workflow_name}`: interpreted durable planning does not support `break` or `continue`"
                )
            }
        }
    }
    Ok(())
}

fn collect_from_expr(
    workflow_name: &str,
    expr: &HirExpr,
    ctx: &ActivityWithOpts,
    out: &mut Vec<PlannedActivity>,
    branch_counter: &mut usize,
) -> anyhow::Result<()> {
    match expr {
        HirExpr::With(inner, opts, _) => {
            let merged = ctx.merged_with(opts)?;
            collect_from_expr(workflow_name, inner, &merged, out, branch_counter)?;
        }
        HirExpr::Call(callee, args, _, _) => {
            if let HirExpr::Ident(name, _) = &**callee {
                if name == "workflow_wait" {
                    let wait_ms = parse_workflow_wait_ms(args)?;
                    out.push(PlannedActivity {
                        name: "__durable_timer_wait".to_string(),
                        mens: false,
                        activity_id: ctx.activity_id.clone(),
                        timeout_ms: Some(wait_ms),
                        retries: 0,
                        initial_backoff_ms: None,
                        populi_op: PopuliHttpOp::Noop,
                        required_labels: None,
                        is_detached: false,
                    });
                    return Ok(());
                }
                if name == "workflow_wait_signal" {
                    let signal_key = parse_workflow_signal_key(args)?;
                    out.push(PlannedActivity {
                        name: format!("__durable_signal_wait:{signal_key}"),
                        mens: false,
                        activity_id: ctx.activity_id.clone(),
                        timeout_ms: None,
                        retries: ctx.retries,
                        initial_backoff_ms: ctx.initial_backoff_ms,
                        populi_op: PopuliHttpOp::Noop,
                        required_labels: None,
                        is_detached: false,
                    });
                    return Ok(());
                }
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
                    retries: ctx.retries,
                    initial_backoff_ms: ctx.initial_backoff_ms,
                    populi_op,
                    required_labels: ctx.required_labels.clone(),
                    is_detached: ctx.is_detached,
                });
            } else {
                collect_from_expr(workflow_name, callee, ctx, out, branch_counter)?;
            }
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            let take_then = eval_const_bool(cond).with_context(|| {
                format!(
                    "workflow `{workflow_name}`: interpreted durable planning supports `if` only when the condition is a deterministic literal expression (bool/int/string literals with basic logical/comparison operators)"
                )
            })?;
            let branch_id = *branch_counter;
            *branch_counter = branch_counter.saturating_add(1);
            out.push(PlannedActivity {
                name: if take_then {
                    "__branch_decision_then".to_string()
                } else {
                    "__branch_decision_else".to_string()
                },
                mens: false,
                activity_id: Some(format!("{workflow_name}-branch-{branch_id}")),
                timeout_ms: None,
                retries: 0,
                initial_backoff_ms: None,
                populi_op: PopuliHttpOp::Noop,
                required_labels: None,
                is_detached: false,
            });
            if take_then {
                collect_activity_calls_from_stmts(
                    workflow_name,
                    then_stmts,
                    ctx,
                    out,
                    branch_counter,
                )?
            } else if let Some(else_branch) = else_stmts {
                collect_activity_calls_from_stmts(
                    workflow_name,
                    else_branch,
                    ctx,
                    out,
                    branch_counter,
                )?;
            }
        }
        HirExpr::Block(stmts, _) => {
            collect_activity_calls_from_stmts(workflow_name, stmts, ctx, out, branch_counter)?
        }
        HirExpr::Binary(_, a, b, _) => {
            collect_from_expr(workflow_name, a, ctx, out, branch_counter)?;
            collect_from_expr(workflow_name, b, ctx, out, branch_counter)?;
        }
        HirExpr::Unary(_, a, _) => collect_from_expr(workflow_name, a, ctx, out, branch_counter)?,
        HirExpr::Match(_, _, _) => anyhow::bail!(
            "workflow `{workflow_name}`: interpreted durable planning currently supports only linear activity plans; `match` branches are not replay-safe yet"
        ),
        HirExpr::MethodCall(recv, _, args, _, _) => {
            collect_from_expr(workflow_name, recv, ctx, out, branch_counter)?;
            for a in args {
                collect_from_expr(workflow_name, &a.value, ctx, out, branch_counter)?;
            }
        }
        HirExpr::FieldAccess(recv, _, _) => {
            collect_from_expr(workflow_name, recv, ctx, out, branch_counter)?
        }
        HirExpr::Lambda(_, _, body, _) => {
            collect_from_expr(workflow_name, body, ctx, out, branch_counter)?
        }
        HirExpr::For(_, iter, body, _) => {
            // Replay-safe bounded loops: literal lists are deterministic and can be unrolled.
            const MAX_STATIC_LOOP_UNROLL: usize = 64;
            match iter.as_ref() {
                HirExpr::ListLit(items, _) => {
                    if items.len() > MAX_STATIC_LOOP_UNROLL {
                        anyhow::bail!(
                            "workflow `{workflow_name}`: static `for` loop exceeds max unroll ({MAX_STATIC_LOOP_UNROLL})"
                        );
                    }
                    for _ in items {
                        collect_from_expr(workflow_name, body, ctx, out, branch_counter)?;
                    }
                }
                _ => anyhow::bail!(
                    "workflow `{workflow_name}`: interpreted durable planning supports `for` only over literal list values (bounded deterministic replay)"
                ),
            }
        }
        HirExpr::Spawn(inner, _) => {
            collect_from_expr(workflow_name, inner, ctx, out, branch_counter)?
        }
        HirExpr::ListLit(items, _) => {
            for it in items {
                collect_from_expr(workflow_name, it, ctx, out, branch_counter)?;
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                collect_from_expr(workflow_name, v, ctx, out, branch_counter)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn parse_workflow_wait_ms(args: &[vox_compiler::hir::HirArg]) -> anyhow::Result<u64> {
    let Some(first) = args.first() else {
        anyhow::bail!("workflow_wait requires one duration argument");
    };
    parse_timeout_ms(&first.value)
}

fn parse_workflow_signal_key(args: &[vox_compiler::hir::HirArg]) -> anyhow::Result<String> {
    let Some(first) = args.first() else {
        anyhow::bail!("workflow_wait_signal requires one string signal key argument");
    };
    let HirExpr::StringLit(value, _) = &first.value else {
        anyhow::bail!("workflow_wait_signal key must be a string literal");
    };
    let key = value.trim();
    if key.is_empty() {
        anyhow::bail!("workflow_wait_signal key must not be empty");
    }
    Ok(key.to_string())
}

fn eval_const_bool(expr: &HirExpr) -> anyhow::Result<bool> {
    match expr {
        HirExpr::BoolLit(v, _) => Ok(*v),
        HirExpr::IntLit(v, _) => Ok(*v != 0),
        HirExpr::StringLit(v, _) => Ok(!v.is_empty()),
        HirExpr::Unary(HirUnOp::Not, inner, _) => Ok(!eval_const_bool(inner)?),
        HirExpr::Binary(op, lhs, rhs, _) => match op {
            HirBinOp::And => Ok(eval_const_bool(lhs)? && eval_const_bool(rhs)?),
            HirBinOp::Or => Ok(eval_const_bool(lhs)? || eval_const_bool(rhs)?),
            HirBinOp::Is => eval_const_eq(lhs, rhs),
            HirBinOp::Isnt => Ok(!eval_const_eq(lhs, rhs)?),
            HirBinOp::Lt => eval_const_ord(lhs, rhs, |a, b| a < b),
            HirBinOp::Lte => eval_const_ord(lhs, rhs, |a, b| a <= b),
            HirBinOp::Gt => eval_const_ord(lhs, rhs, |a, b| a > b),
            HirBinOp::Gte => eval_const_ord(lhs, rhs, |a, b| a >= b),
            _ => anyhow::bail!("unsupported binary operator in deterministic `if` condition"),
        },
        _ => anyhow::bail!("unsupported expression in deterministic `if` condition"),
    }
}

fn eval_const_eq(lhs: &HirExpr, rhs: &HirExpr) -> anyhow::Result<bool> {
    match (lhs, rhs) {
        (HirExpr::BoolLit(a, _), HirExpr::BoolLit(b, _)) => Ok(a == b),
        (HirExpr::IntLit(a, _), HirExpr::IntLit(b, _)) => Ok(a == b),
        (HirExpr::StringLit(a, _), HirExpr::StringLit(b, _)) => Ok(a == b),
        _ => anyhow::bail!("`is`/`isnt` supports only bool/int/string literal comparisons"),
    }
}

fn eval_const_ord(
    lhs: &HirExpr,
    rhs: &HirExpr,
    cmp: impl FnOnce(i64, i64) -> bool,
) -> anyhow::Result<bool> {
    match (lhs, rhs) {
        (HirExpr::IntLit(a, _), HirExpr::IntLit(b, _)) => Ok(cmp(*a, *b)),
        _ => anyhow::bail!("ordering comparisons support only int literals"),
    }
}
