//! Compatibility helpers for interpreted workflow planning.

use anyhow::Context;
use vox_compiler::hir::{HirExpr, HirModule};

use super::types::PopuliHttpOp;

/// Return the names of activities declared alongside a workflow.
///
/// Body-first execution discovers actual activity calls at runtime. This thin
/// compatibility export therefore reports declared activity names without
/// attempting to linearize control flow.
pub fn plan_workflow_activities(
    hir: &HirModule,
    workflow_name: &str,
) -> anyhow::Result<Vec<String>> {
    use vox_compiler::hir::nodes::DurabilityKind;

    if !hir
        .functions
        .iter()
        .any(|f| f.durability == Some(DurabilityKind::Workflow) && f.name == workflow_name)
    {
        anyhow::bail!("workflow `{workflow_name}` not found");
    }

    Ok(hir
        .functions
        .iter()
        .filter(|f| f.durability == Some(DurabilityKind::Activity))
        .map(|f| f.name.clone())
        .collect())
}

#[allow(dead_code)]
fn parse_timeout_ms(expr: &HirExpr) -> anyhow::Result<u64> {
    match expr {
        HirExpr::IntLit(ms, _) if *ms >= 0 => Ok(*ms as u64),
        HirExpr::IntLit(_, _) => {
            anyhow::bail!("workflow `timeout` must be a non-negative integer")
        }
        HirExpr::StringLit(s, _) => {
            parse_duration_ms_str(s).with_context(|| format!("invalid workflow timeout {:?}", s))
        }
        _ => anyhow::bail!("workflow `timeout` must be an int (milliseconds) or duration string"),
    }
}

/// Adapt the shared SSOT duration parser to the `u64`-millisecond shape the
/// planner stores. Delegates to
/// [`crate::duration_literal::parse_duration_str`] so the runtime and the
/// `vox-codegen` emit agree on every unit (ms / s / m / h / d) and on error
/// messages (ADR-041 M-7).
///
/// **Behavior note (post-M-7 consolidation, 2026-05-24):** a bare integer like
/// `workflow_wait(30)` is now interpreted as 30 **seconds** (the cron-style
/// ergonomic default the shared parser uses), where the legacy local parser
/// here interpreted it as 30 **milliseconds**. No golden uses the bare-integer
/// form so blast radius is zero, but if a workflow author wants milliseconds
/// they must write `workflow_wait("30ms")` explicitly.
#[allow(dead_code)]
fn parse_duration_ms_str(s: &str) -> anyhow::Result<u64> {
    crate::duration_literal::parse_duration_str(s)
        .map(|d| d.as_millis().min(u64::MAX as u128) as u64)
        .map_err(|e| anyhow::anyhow!("{e}"))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
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
