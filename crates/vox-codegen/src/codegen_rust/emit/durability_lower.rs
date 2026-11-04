//! P2-T7: lower `DurabilityKind` into specific runtime call shapes.
//!
//! Driven by `HirFn::durability`. The branch lives here so `emit_fn` stays
//! readable: header emit + delegate-by-kind.

use std::collections::HashMap;
use vox_compiler::ast::span::Span;
use vox_compiler::hir::{DurabilityKind, HirFn, HirStmt, HirType};

use super::script_db::with_activity_journal_inner;
use super::stmt_expr::emit_stmt;
use super::types::emit_type;

fn activity_inner_return_type(ret: Option<&HirType>) -> String {
    match ret {
        Some(HirType::Generic(name, args)) if name == "Result" => {
            let ok = args
                .first()
                .map(emit_type)
                .unwrap_or_else(|| "serde_json::Value".into());
            format!("Result<{ok}, anyhow::Error>")
        }
        Some(other) => format!("Result<{}, anyhow::Error>", emit_type(other)),
        None => "Result<(), anyhow::Error>".into(),
    }
}

/// Emit the body of a workflow / activity / actor handler.
///
/// The function header (params, return type) is emitted by the caller; this
/// function owns everything inside the `{ ... }`.
pub(super) fn emit_durable_body(
    func: &HirFn,
    inferred_types: Option<&HashMap<Span, HirType>>,
    usage: Option<&super::usage::UsageTracker>,
    actor_handlers: &[&HirFn],
) -> String {
    match func.durability {
        Some(DurabilityKind::Workflow) => emit_workflow_body(func),
        Some(DurabilityKind::Activity) => emit_activity_body(func, inferred_types, usage),
        Some(DurabilityKind::Actor) => emit_actor_body(func, inferred_types, usage, actor_handlers),
        None => emit_plain_body(func, inferred_types, usage),
    }
}

fn is_result_type(ret: &HirType) -> bool {
    matches!(ret, HirType::Generic(name, _) if name == "Result")
}

/// Rust return type of a workflow fn: `Result<R, String>` for a declared `R`
/// (`Result<(), String>` when none), with no exception for a declared `Result`.
///
/// The journal records the workflow's own return value, so a domain `Err` is a
/// value of `R`; the outer `Err` means only that the durable runtime failed
/// (journal, replay). Keeping the two apart lets `R` carry any error type with no
/// conversion from the runtime's, and gives callers one rule to follow.
pub(super) fn workflow_rust_return_type(ret: Option<&HirType>) -> String {
    match ret {
        Some(r) => format!("Result<{}, String>", emit_type(r)),
        None => "Result<(), String>".into(),
    }
}

/// Resolve + connect the durable `VoxDb` for generated workflow wrappers.
/// Same resolution as `main_boot::emit_durable_boot_prelude`: always
/// `DbConfig::resolve_canonical()` (honors `VOX_DB_*`, else the user-global
/// file). Isolation is the caller's job — the ADR-021 gate already sets
/// `VOX_DB_PATH`.
fn emit_vox_db_connect_snippet() -> &'static str {
    "    let __vox_db_cfg = ::vox_db::DbConfig::resolve_canonical()\n\
     \x20       .map_err(|e| format!(\"vox durable workflow: resolve VoxDb config (VOX_DB_URL+TOKEN, or VOX_DB_PATH): {e}\"))?;\n\
     \x20   let __vox_db = ::std::sync::Arc::new(\n\
     \x20       ::vox_db::VoxDb::connect(__vox_db_cfg)\n\
     \x20           .await\n\
     \x20           .map_err(|e| format!(\"vox durable workflow: connect VoxDb: {e}\"))?,\n\
     \x20   );\n"
}

fn emit_workflow_body(func: &HirFn) -> String {
    let name = &func.name;
    let hash = func.generated_hash.as_deref().unwrap_or("UNSTAMPED");
    let mut out = String::new();
    out.push_str(
        "    // Task 6.1: workflow body lowered to interpret_workflow_durable via the SQL tracker\n",
    );
    out.push_str(&format!(
        "    let __vox_fn_hash: &'static str = \"{hash}\";\n"
    ));
    out.push_str("    let _ = __vox_fn_hash;\n");
    out.push_str("    let __vox_hir = ::vox_workflow_runtime::workflow::current_hir_module();\n");
    // ADR-021: generated workflows persist through the same SQL tracker
    // (feature `sql`) that `vox mens workflow run` uses, not an in-memory
    // no-op tracker. `VOX_WORKFLOW_RUN_ID` lets a caller resume/inspect a
    // specific run; unset falls back to a fresh id per invocation.
    out.push_str(&format!(
        "    let __vox_run_id: String = ::std::env::var(\"VOX_WORKFLOW_RUN_ID\").unwrap_or_else(|_| format!(\"{name}-run-{{}}-{{}}\", ::std::process::id(), ::std::time::SystemTime::now().duration_since(::std::time::UNIX_EPOCH).unwrap_or_default().as_nanos()));\n"
    ));
    out.push_str(emit_vox_db_connect_snippet());
    out.push_str(
        "    let mut __vox_tracker = ::vox_workflow_runtime::VoxDbTracker::new(__vox_db, __vox_run_id);\n",
    );
    // G27 / Task 1.4: forward the workflow's actual call arguments (already
    // bound as Rust locals by the emitted function header) as journal-JSON,
    // so `interpret_workflow_durable` can persist `args_json` on the run row.
    out.push_str("    let __vox_args: ::std::vec::Vec<::serde_json::Value> = ::std::vec![");
    for param in &func.params {
        out.push_str(&format!(
            "::serde_json::to_value(&{p}).unwrap_or(::serde_json::Value::Null), ",
            p = param.name
        ));
    }
    out.push_str("];\n");
    out.push_str(&format!(
        "    let __vox_journal = ::vox_workflow_runtime::workflow::interpret_workflow_durable(__vox_hir, \"{name}\", __vox_args, &mut __vox_tracker).await.map_err(|e| e.to_string())?;\n"
    ));
    // ADR-021: `VOX_RUN_WORKFLOW=<this workflow>` debug dispatch calls this
    // generated wrapper and expects the journal JSON on stdout (see
    // `tests/adr021_parity.rs`). Match the value, not mere presence — an
    // inherited env from another workflow must not print or kill the host.
    out.push_str(&format!(
        "    let __vox_debug_dispatch = ::std::env::var(\"VOX_RUN_WORKFLOW\").ok().as_deref().map(str::trim) == Some({name:?});\n\
         \x20   if __vox_debug_dispatch {{\n\
         \x20       println!(\"{{}}\", ::serde_json::to_string(&__vox_journal).map_err(|e| e.to_string())?);\n\
         \x20   }}\n"
    ));
    out.push_str(
        "    let __vox_completed = __vox_journal.iter().rev().any(|__e| {\n\
         \x20       __e.get(\"event\").and_then(::serde_json::Value::as_str) == Some(\"WorkflowCompleted\")\n\
         \x20   });\n\
         \x20   if !__vox_completed {\n\
         \x20       if __vox_debug_dispatch {\n\
         \x20           ::std::process::exit(0);\n\
         \x20       }\n\
         \x20       return Err(\"vox durable workflow: run parked (no WorkflowCompleted); resume with the same VOX_WORKFLOW_RUN_ID\".to_string());\n\
         \x20   }\n",
    );
    match &func.return_type {
        Some(ret) => out.push_str(&format!(
            "    Ok(::vox_workflow_runtime::workflow::extract_terminal_return::<{ty}>(&__vox_journal).map_err(|e| e.to_string())?)\n",
            ty = emit_type(ret),
        )),
        None => out.push_str("    Ok(())\n"),
    }
    out
}

/// `pub async fn __vox_run_workflow(name, args)` — the dispatcher `main.rs` calls when
/// `VOX_RUN_WORKFLOW` is set. Args are positional JSON values; the wrapper prints the journal.
/// Empty when the module declares no workflows.
pub(super) fn emit_workflow_dispatcher(functions: &[HirFn]) -> String {
    let workflows: Vec<&HirFn> = functions
        .iter()
        .filter(|f| f.durability == Some(DurabilityKind::Workflow))
        .collect();
    if workflows.is_empty() {
        return String::new();
    }
    let mut out = String::new();
    out.push_str("/// Runs a workflow by name with positional JSON args (`VOX_RUN_WORKFLOW` / `VOX_WORKFLOW_ARGS`).\n");
    out.push_str("pub async fn __vox_run_workflow(__vox_wf_name: &str, __vox_wf_args: &[serde_json::Value]) -> Result<(), String> {\n");
    out.push_str("    match __vox_wf_name {\n");
    for wf in workflows {
        let rust_name = wf.name.replace("::", "_");
        out.push_str(&format!("        \"{}\" => {{\n", wf.name));
        let mut call_args = Vec::new();
        for (i, p) in wf.params.iter().enumerate() {
            let ty = emit_type(
                p.type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into())),
            );
            out.push_str(&format!(
                "            let {name} = serde_json::from_value::<{ty}>(__vox_wf_args.get({i}).cloned().unwrap_or(serde_json::Value::Null)).map_err(|e| format!(\"workflow {wf} arg {i} ({name}): {{e}}\"))?;\n",
                name = p.name,
                wf = wf.name,
            ));
            call_args.push(p.name.clone());
        }
        // `?` here surfaces only runtime failure; `out` is the declared value.
        out.push_str(&format!(
            "            let out = {rust_name}({}).await?;\n",
            call_args.join(", ")
        ));
        match &wf.return_type {
            None => out.push_str("            let () = out;\n            Ok(())\n"),
            Some(ret) => {
                // No second print here: under `VOX_RUN_WORKFLOW` the wrapper already
                // printed the journal, whose `WorkflowCompleted.return_value` carries the
                // value, and the ADR-021 gate parses stdout as exactly one JSON document.
                if is_result_type(ret) {
                    // A domain `Err` is a failed run for the caller's exit status.
                    out.push_str(&format!(
                        "            match out {{ Ok(_) => Ok(()), Err(_) => Err(\"workflow {} returned Err\".to_string()) }}\n",
                        wf.name
                    ));
                } else {
                    out.push_str("            let _ = out;\n            Ok(())\n");
                }
            }
        }
        out.push_str("        }\n");
    }
    out.push_str("        _ => Err(format!(\"unknown workflow: {__vox_wf_name}\")),\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out
}

fn emit_activity_body(
    func: &HirFn,
    inferred_types: Option<&HashMap<Span, HirType>>,
    usage: Option<&super::usage::UsageTracker>,
) -> String {
    let activity_id = func
        .generated_hash
        .clone()
        .unwrap_or_else(|| func.name.clone());
    let inner_name = format!("__vox_activity_body_{}", func.name);
    let mut param_decls = String::new();
    let mut call_args = String::new();
    for param in &func.params {
        param_decls.push_str(&format!(
            "{}: {}, ",
            param.name,
            emit_type(
                param
                    .type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into()))
            )
        ));
        call_args.push_str(&format!("{}, ", param.name));
    }
    let inner_ret = activity_inner_return_type(func.return_type.as_ref());
    let mut body = String::new();
    with_activity_journal_inner(|| {
        for stmt in &func.body {
            body.push_str(&emit_stmt(
                stmt,
                2,
                false,
                false,
                false,
                inferred_types,
                usage,
                None,
                func.return_type.as_ref(),
            ));
        }
    });
    // Task 6.1: activities are invoked by the workflow interpreter via
    // `HostRequest::Begin` / `HostDecision::Execute`|`Replay` — the
    // interpreter itself owns record/replay through the workflow's
    // `WorkflowTracker` (`VoxDbTracker` in the generated path). Wrapping the
    // activity body in `journal::execute` here was dead weight: this
    // generated fn is not on the interpreter's call path, so its own
    // recording was unreachable and carried a second, conflicting id
    // scheme. Emit the plain async body instead.
    let mut out = String::new();
    out.push_str(&format!(
        "    // Task 6.1: activity body (activity_id \"{activity_id}\"; recorded by the workflow's WorkflowTracker, not here)\n\
         \x20   async fn {inner_name}({param_decls}) -> {inner_ret} {{\n{body}\
         \x20   }}\n\
         \x20   {inner_name}({call_args}).await.map_err(|e| e.to_string())\n",
    ));
    out
}

/// Emit the per-handler dispatch arm for one `on <event>(params)` handler.
///
/// Decodes the positional `args` array into each typed param, then calls the
/// emitted `Actor_event(&mut _state, …)` function. On the request path
/// (`is_request`) the return value is serialized back through
/// `ProcessContext::reply`; on the message path the return (if any) is dropped.
/// A malformed/short args array drops the envelope (`continue`s the loop).
fn emit_actor_dispatch_arm(handler: &HirFn, is_request: bool) -> String {
    let event = handler.name.split("::").nth(1).unwrap_or(&handler.name);
    let call_name = handler.name.replace("::", "_");
    let mut arm = String::new();
    arm.push_str(&format!("                    {event:?} => {{\n"));
    let mut call_args = String::from("&mut _state");
    for (i, param) in handler.params.iter().enumerate() {
        let ty = emit_type(
            param
                .type_ann
                .as_ref()
                .unwrap_or(&HirType::Named("serde_json::Value".into())),
        );
        arm.push_str(&format!(
            "                        let {name}: {ty} = match __vox_arg(&__args, {i}usize) {{ \
             Some(__a) => __a, None => continue }};\n",
            name = param.name,
        ));
        call_args.push_str(&format!(", {}", param.name));
    }
    if is_request {
        if handler.return_type.is_some() {
            arm.push_str(&format!(
                "                        let __ret = {call_name}({call_args});\n"
            ));
            arm.push_str(
                "                        ::vox_actor_runtime::ProcessContext::reply(__req, \
                 ::serde_json::to_vec(&__ret).unwrap_or_default());\n",
            );
        } else {
            arm.push_str(&format!(
                "                        {call_name}({call_args});\n"
            ));
            arm.push_str(
                "                        ::vox_actor_runtime::ProcessContext::reply(__req, \
                 ::std::vec::Vec::new());\n",
            );
        }
    } else {
        arm.push_str(&format!(
            "                        let _ = {call_name}({call_args});\n"
        ));
    }
    arm.push_str("                    }\n");
    arm
}

fn emit_actor_body(
    func: &HirFn,
    inferred_types: Option<&HashMap<Span, HirType>>,
    usage: Option<&super::usage::UsageTracker>,
    handlers: &[&HirFn],
) -> String {
    // An actor "handler" function has a name like "ChatRoom::join" and
    // carries the executable handler body lowered from the `on event(...)`
    // declaration. It is NOT a process-spawn site — it's a plain function
    // that the runtime dispatcher calls when it receives a matching
    // envelope. So emit it like any other function. Per the 2026-05-23
    // slot-3 chat bring-up: the prior implementation re-emitted the
    // spawn_process loop inside every handler body, referencing
    // bogus types like `ChatRoom::joinState`.
    if func.name.contains("::") {
        return emit_plain_body(func, inferred_types, usage);
    }

    // Actor SHELL: spawn the process loop and dispatch envelopes to handlers.
    // State struct is emitted as a sibling by `emit_actor_state_structs` in
    // workflow.rs; here we instantiate it and route each `Envelope` to the
    // matching `on <event>` handler via the `{"event","args"}` JSON wire format
    // (docs/superpowers/specs/2026-06-03-actor-message-wire-format-design.md):
    // `Message` is fire-and-forget; `Request` runs the handler and replies with
    // the serialized return value; `Signal` is lifecycle (dropped for now).
    let actor_name = &func.name;
    let state_struct = format!("{}State", actor_name);

    let message_arms: String = handlers
        .iter()
        .map(|h| emit_actor_dispatch_arm(h, false))
        .collect();
    let request_arms: String = handlers
        .iter()
        .map(|h| emit_actor_dispatch_arm(h, true))
        .collect();

    let mut out = String::new();
    out.push_str("    // actor shell — spawn the mailbox loop and dispatch envelopes.\n");
    out.push_str(&format!(
        "    #[allow(unused_mut, unused_variables)]\n    let mut _state = {state_struct}::default();\n"
    ));
    out.push_str(
        "    let _handle = ::vox_actor_runtime::spawn_process(move |mut ctx| async move {\n",
    );
    // Local helper: decode positional arg `i` from the `args` JSON array.
    out.push_str(
        "        fn __vox_arg<__T: ::serde::de::DeserializeOwned>(args: &::serde_json::Value, i: usize) -> ::std::option::Option<__T> {\n",
    );
    out.push_str(
        "            args.get(i).and_then(|__a| ::serde_json::from_value(__a.clone()).ok())\n",
    );
    out.push_str("        }\n");
    out.push_str("        while let Some(envelope) = ctx.receive().await {\n");
    out.push_str("            match envelope {\n");
    // Fire-and-forget messages.
    out.push_str("                ::vox_actor_runtime::Envelope::Message(__m) => {\n");
    out.push_str("                    let __v: ::serde_json::Value = match __m.payload.deserialize_json() { Ok(__v) => __v, Err(_) => continue };\n");
    out.push_str("                    let __ev = __v.get(\"event\").and_then(|__e| __e.as_str()).unwrap_or(\"\");\n");
    out.push_str("                    let __args = __v.get(\"args\").cloned().unwrap_or(::serde_json::Value::Null);\n");
    out.push_str("                    match __ev {\n");
    out.push_str(&message_arms);
    out.push_str("                    _ => {}\n");
    out.push_str("                    }\n");
    out.push_str("                }\n");
    // Request/reply.
    out.push_str("                ::vox_actor_runtime::Envelope::Request(__req) => {\n");
    out.push_str("                    let __v: ::serde_json::Value = match __req.payload.deserialize_json() { Ok(__v) => __v, Err(_) => continue };\n");
    out.push_str("                    let __ev = __v.get(\"event\").and_then(|__e| __e.as_str()).unwrap_or(\"\").to_string();\n");
    out.push_str("                    let __args = __v.get(\"args\").cloned().unwrap_or(::serde_json::Value::Null);\n");
    out.push_str("                    match __ev.as_str() {\n");
    out.push_str(&request_arms);
    out.push_str("                    _ => {}\n");
    out.push_str("                    }\n");
    out.push_str("                }\n");
    out.push_str("                ::vox_actor_runtime::Envelope::Signal(_) => {}\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    });\n");
    out
}

pub(super) fn emit_plain_body(
    func: &HirFn,
    inferred_types: Option<&HashMap<Span, HirType>>,
    usage: Option<&super::usage::UsageTracker>,
) -> String {
    let mut out = String::new();
    let n = func.body.len();
    let returns_value = func
        .return_type
        .as_ref()
        .is_some_and(|t| !matches!(t, HirType::Unit));
    for (i, stmt) in func.body.iter().enumerate() {
        let s = emit_stmt(
            stmt,
            1,
            false,
            false,
            false,
            inferred_types,
            usage,
            None,
            func.return_type.as_ref(),
        );
        // Vox last-expr return: a fn with a non-Unit return type whose final
        // statement is a bare expression (e.g. `fn area(..) to float { match .. }`)
        // returns that expression's value. Drop the trailing `;` so the Rust fn
        // returns the value instead of `()` (otherwise E0308 mismatched types).
        if i + 1 == n && returns_value && matches!(stmt, HirStmt::Expr { .. }) {
            match s.strip_suffix(";\n") {
                Some(body) => {
                    out.push_str(body);
                    out.push('\n');
                }
                None => out.push_str(&s),
            }
        } else {
            out.push_str(&s);
        }
    }
    out
}
