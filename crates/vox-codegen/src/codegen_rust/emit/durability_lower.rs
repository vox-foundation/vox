//! P2-T7: lower `DurabilityKind` into specific runtime call shapes.
//!
//! Driven by `HirFn::durability`. The branch lives here so `emit_fn` stays
//! readable: header emit + delegate-by-kind.

use vox_compiler::hir::{DurabilityKind, HirFn};

use super::stmt_expr::emit_stmt;
use super::types::emit_type;

/// Emit the body of a workflow / activity / actor handler.
///
/// The function header (params, return type) is emitted by the caller; this
/// function owns everything inside the `{ ... }`.
pub(super) fn emit_durable_body(func: &HirFn) -> String {
    match func.durability {
        Some(DurabilityKind::Workflow) => emit_workflow_body(func),
        Some(DurabilityKind::Activity) => emit_activity_body(func),
        Some(DurabilityKind::Actor) => emit_actor_body(func),
        None => emit_plain_body(func),
    }
}

fn emit_workflow_body(func: &HirFn) -> String {
    let name = &func.name;
    let hash = func.generated_hash.as_deref().unwrap_or("UNSTAMPED");
    let mut out = String::new();
    out.push_str("    // P2-T7: workflow body lowered to interpret_workflow_durable\n");
    out.push_str(&format!(
        "    let __vox_fn_hash: &'static str = \"{hash}\";\n"
    ));
    out.push_str("    let _ = __vox_fn_hash;\n");
    out.push_str("    let __vox_hir = ::vox_workflow_runtime::workflow::current_hir_module();\n");
    out.push_str("    let mut __vox_tracker = ::vox_workflow_runtime::workflow::tracker::DefaultTracker;\n");
    out.push_str(&format!(
        "    let __vox_journal = ::vox_workflow_runtime::workflow::interpret_workflow_durable(&__vox_hir, \"{name}\", &mut __vox_tracker).await?;\n"
    ));
    if let Some(ret) = &func.return_type {
        out.push_str(&format!(
            "    ::vox_workflow_runtime::workflow::extract_terminal_return::<{ty}>(&__vox_journal).map_err(|e| anyhow::anyhow!(e))\n",
            ty = emit_type(ret),
        ));
    } else {
        out.push_str("    Ok(())\n");
    }
    out
}

fn emit_activity_body(func: &HirFn) -> String {
    let activity_id = func.generated_hash.clone().unwrap_or_else(|| func.name.clone());
    let mut out = String::new();
    out.push_str("    // P2-T7: activity body lowered to journal::execute\n");
    out.push_str(&format!(
        "    ::vox_workflow_runtime::journal::execute(\"{activity_id}\", async move {{\n"
    ));
    for stmt in &func.body {
        let inner = emit_stmt(stmt, 2, false, false, false);
        out.push_str(&inner);
    }
    out.push_str("    }).await\n");
    out
}

fn emit_actor_body(func: &HirFn) -> String {
    let actor_name = &func.name;
    let mut out = String::new();
    out.push_str("    // P2-T7: actor handler lowered to mailbox::spawn\n");
    out.push_str(&format!(
        "    ::vox_actor_runtime::mailbox::spawn(\"{actor_name}\", move || async move {{\n"
    ));
    for stmt in &func.body {
        let inner = emit_stmt(stmt, 2, false, false, false);
        out.push_str(&inner);
    }
    out.push_str("    }).await\n");
    out
}

pub(super) fn emit_plain_body(func: &HirFn) -> String {
    let mut out = String::new();
    for stmt in &func.body {
        out.push_str(&emit_stmt(stmt, 1, false, false, false));
    }
    out
}
