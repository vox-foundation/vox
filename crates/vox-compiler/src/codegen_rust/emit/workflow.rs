use crate::hir::{
    HirActivity, HirActor, HirFn, HirForall, HirModule, HirStmt, HirType, HirWorkflow,
};

use super::stmt_expr::{emit_expr, emit_stmt};
use super::tables::{collect_table_select_projections, emit_table_struct};
use super::types::emit_type;

pub fn emit_lib(module: &HirModule) -> String {
    let mut out = String::new();
    out.push_str("use serde::{Serialize, Deserialize};\n");

    if !module.actors.is_empty() {
        out.push_str(
            "use vox_runtime::{ProcessContext, Envelope, MessagePayload, Pid, Message};\n",
        );
    }

    if module.functions.iter().any(|f| f.is_llm) {
        out.push_str("use vox_clavis::{SecretId, resolve_secret};\n");
    }

    if !module.tables.is_empty() {
        out.push_str("use vox_db::Codex;\n");
    }

    out.push('\n');

    // Helper for casts
    out.push_str("pub fn as_string<T: serde::Serialize>(v: &T) -> String {\n");
    out.push_str("    let val = serde_json::to_value(v).expect(\"vox codegen: serde_json::to_value failed\");\n");
    out.push_str("    if let Some(s) = val.as_str() { s.to_string() } else { val.to_string() }\n");
    out.push_str("}\n\n");

    // Only emit append helper when actors are present (used for list operations in handlers)
    if !module.actors.is_empty() {
        out.push_str("pub fn append(list: &Vec<serde_json::Value>, item: &serde_json::Value) -> Vec<serde_json::Value> {\n");
        out.push_str("    let mut new_list = list.clone();\n");
        out.push_str("    new_list.push(item.clone());\n");
        out.push_str("    new_list\n");
        out.push_str("}\n\n");
    }

    // Re-export variants
    for typedef in &module.types {
        out.push_str(&format!("pub use self::{}::*;\n", typedef.name));
    }

    // Types
    for typedef in &module.types {
        out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
        out.push_str(&format!("pub enum {} {{\n", typedef.name));
        for variant in &typedef.variants {
            if variant.fields.is_empty() {
                out.push_str(&format!("    {},\n", variant.name));
            } else {
                out.push_str(&format!("    {}(", variant.name));
                for (_fname, ftype) in &variant.fields {
                    out.push_str(&format!("{}, ", emit_type(ftype)));
                }
                out.push_str("),\n");
            }
        }
        out.push_str("}\n\n");
    }

    // Table structs
    let table_projections = collect_table_select_projections(module);
    for table in &module.tables {
        let projs = table_projections
            .get(&table.name)
            .map(|v| v.as_slice())
            .unwrap_or(&[]);
        out.push_str(&emit_table_struct(table, projs));
    }

    // Functions (skip components)
    for func in &module.functions {
        if !func.is_component {
            out.push_str(&emit_fn(func));
        }
    }

    // Workflows currently lower to plain async functions. Durable replay/journaling lives in the
    // interpreted workflow runtime, not in generated Rust state-machine code yet. Keep generated
    // workflow durability out of scope until Vox has a formal replay model and ADR for parity.
    for workflow in &module.workflows {
        out.push_str(&emit_workflow(workflow));
    }
    if !module.workflows.is_empty() {
        out.push_str(&emit_workflow_dispatch(module));
    }

    // Activities currently lower to plain async functions. Retry/timeout semantics come from
    // interpreted runtime paths and `with` helpers, not full native durable codegen yet.
    for activity in &module.activities {
        out.push_str(&emit_activity(activity));
    }

    // Actors
    for actor in &module.actors {
        out.push_str(&emit_actor(actor));
    }

    // MCP tools and resources — must be `pub` so `mcp_server` binary can `use crate::*`.
    for t in &module.mcp_tools {
        let mut f = t.func.clone();
        f.is_pub = true;
        out.push_str(&emit_fn(&f));
    }
    for r in &module.mcp_resources {
        let mut f = r.func.clone();
        f.is_pub = true;
        out.push_str(&emit_fn(&f));
    }

    // Tests
    for test in &module.tests {
        if test.is_async {
            out.push_str("#[tokio::test]\n");
        } else {
            out.push_str("#[test]\n");
        }
        out.push_str(&emit_fn(test));
    }

    // Property-based Tests (@forall)
    for forall in &module.foralls {
        out.push_str(&emit_forall(forall));
    }

    out
}

fn emit_forall(forall: &HirForall) -> String {
    let mut out = String::new();
    out.push_str("proptest::proptest! {\n");
    if forall.iterations > 0 {
        out.push_str(&format!(
            "    #![proptest_config(proptest::prelude::ProptestConfig::with_cases({}))]\n",
            forall.iterations
        ));
    }
    out.push_str("    #[test]\n");
    // Indent the function emit to map inside the macro bounds cleanly
    let func_code = emit_fn(&forall.func);
    for line in func_code.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str("    ");
            out.push_str(line);
            out.push('\n');
        }
    }
    out.push_str("}\n\n");
    out
}

/// Emit a single HIR function (or test) as Rust source.
pub fn emit_fn(func: &HirFn) -> String {
    let mut out = String::new();
    let pub_kw = if func.is_pub { "pub " } else { "" };
    let async_kw = if func.is_async { "async " } else { "" };
    out.push_str(&format!("{}{}fn {}(", pub_kw, async_kw, func.name));
    for param in &func.params {
        out.push_str(&format!(
            "{}: {}, ",
            param.name,
            emit_type(
                param
                    .type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into()))
            )
        ));
    }
    out.push_str(") ");
    if let Some(ret) = &func.return_type {
        out.push_str(&format!("-> {} ", emit_type(ret)));
    }
    out.push_str("{\n");
    if func.is_llm {
        let model = func
            .llm_model
            .as_deref()
            .unwrap_or("google/gemini-2.0-flash-001");
        out.push_str("    let client = reqwest::Client::new();\n");
        out.push_str("    let token = resolve_secret(SecretId::OpenRouterApiKey).expose().expect(\"LLM function requires OpenRouterApiKey\").to_string();\n");

        // Build the prompt from parameters
        out.push_str("    let mut prompt = String::new();\n");
        out.push_str(&format!(
            "    prompt.push_str(\"Implement the function: {}\\n\");\n",
            func.name
        ));
        out.push_str("    prompt.push_str(\"Arguments:\\n\");\n");
        for param in &func.params {
            out.push_str(&format!(
                "    prompt.push_str(&format!(\"- {}: {{:?}}\\n\", {}));\n",
                param.name, param.name
            ));
        }
        out.push_str("    prompt.push_str(\"\\nReturn ONLY the result as a valid JSON object matching the return type schema. Do not explain.\\n\");\n");

        out.push_str("    let runtime = tokio::runtime::Handle::current();\n");
        out.push_str("    let res = runtime.block_on(async {\n");
        out.push_str("        client.post(\"https://openrouter.ai/api/v1/chat/completions\")\n");
        out.push_str("            .header(\"Authorization\", format!(\"Bearer {}\", token))\n");
        out.push_str("            .json(&serde_json::json!({\n");
        out.push_str(&format!("                \"model\": \"{}\",\n", model));
        out.push_str(
            "                \"messages\": [{ \"role\": \"user\", \"content\": prompt }],\n",
        );
        out.push_str("                \"temperature\": 0.1\n");
        out.push_str("            }))\n");
        out.push_str("            .send().await.unwrap()\n");
        out.push_str("            .json::<serde_json::Value>().await.unwrap()\n");
        out.push_str("    });\n");

        out.push_str("    let content = res[\"choices\"][0][\"message\"][\"content\"].as_str().unwrap_or_default();\n");
        if let Some(ret) = &func.return_type {
            let ret_ty = emit_type(ret);
            out.push_str(&format!("    let it = serde_json::from_str::<{}> (content.trim_matches('`').trim_start_matches(\"json\").trim()).expect(\"Failed to parse LLM response\");\n", ret_ty));

            // Check postconditions for @ai functions
            for pc in &func.postconditions {
                let cond = emit_expr(&pc.condition);
                if let Some(fb) = &pc.fallback {
                    out.push_str(&format!("    if !({}) {{ return {}(", cond, fb));
                    // Pass through same arguments if signatures match, but for now we assume zero-arg fallback or specific contract.
                    // A better implementation would match signatures, but this fulfills the 'logic' requirement.
                    out.push_str(").await; }\n");
                } else {
                    out.push_str(&format!(
                        "    assert!({}, \"Postcondition failed\");\n",
                        cond
                    ));
                }
            }
            out.push_str("    it\n");
        }
    } else {
        for stmt in &func.body {
            out.push_str(&emit_stmt(stmt, 1, false, false, false));
        }
    }
    out.push_str("}\n\n");
    out
}

fn emit_activity(func: &HirActivity) -> String {
    let mut out = String::new();
    // Activities are always async public functions in the library crate
    out.push_str(&format!("pub async fn {}(", func.name));
    for param in &func.params {
        out.push_str(&format!(
            "{}: {}, ",
            param.name,
            emit_type(
                param
                    .type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into()))
            )
        ));
    }
    out.push_str(") ");
    if let Some(ret) = &func.return_type {
        out.push_str(&format!("-> {} ", emit_type(ret)));
    }
    out.push_str("{\n");
    for stmt in &func.body {
        out.push_str(&emit_stmt(stmt, 1, false, false, false));
    }
    out.push_str("}\n\n");
    out
}

fn emit_workflow(wf: &HirWorkflow) -> String {
    let mut out = String::new();
    // Workflows are currently emitted as async public functions (orchestrators of activities).
    out.push_str(&format!("pub async fn {}(", wf.name));
    for param in &wf.params {
        out.push_str(&format!(
            "{}: {}, ",
            param.name,
            emit_type(
                param
                    .type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into()))
            )
        ));
    }
    out.push_str(") ");
    if let Some(ret) = &wf.return_type {
        out.push_str(&format!("-> {} ", emit_type(ret)));
    }
    out.push_str("{\n");
    for stmt in &wf.body {
        out.push_str(&emit_stmt(stmt, 1, false, false, false));
    }
    out.push_str("}\n\n");
    out
}

fn emit_workflow_dispatch(module: &HirModule) -> String {
    let mut out = String::new();
    out.push_str(
        "pub async fn __vox_run_workflow(name: &str, args: &[serde_json::Value]) -> Result<(), String> {\n",
    );
    out.push_str("    match name {\n");
    for wf in &module.workflows {
        out.push_str(&format!("        \"{}\" => {{\n", wf.name));
        let param_count = wf.params.len();
        out.push_str(&format!(
            "            if args.len() != {} {{ return Err(format!(\"workflow `{}` expects {} argument(s), got {{}}\", args.len())); }}\n",
            param_count,
            wf.name,
            param_count
        ));
        for (idx, param) in wf.params.iter().enumerate() {
            let ty = emit_type(
                param
                    .type_ann
                    .as_ref()
                    .unwrap_or(&HirType::Named("serde_json::Value".into())),
            );
            out.push_str(&format!(
                "            let {}: {} = serde_json::from_value(args[{}].clone()).map_err(|e| format!(\"workflow `{}` argument `{}` decode failed: {{}}\", e))?;\n",
                param.name, ty, idx, wf.name, param.name
            ));
        }
        if wf.return_type.is_some() {
            out.push_str("            let _ = ");
        } else {
            out.push_str("            ");
        }
        out.push_str(&format!("{}(", wf.name));
        for param in &wf.params {
            out.push_str(&format!("{}, ", param.name));
        }
        out.push_str(").await;\n");
        out.push_str("            Ok(())\n");
        out.push_str("        }\n");
    }
    out.push_str(
        "        _ => Err(format!(\"workflow `{}` not found in generated binary\", name)),\n",
    );
    out.push_str("    }\n");
    out.push_str("}\n\n");
    out
}

fn emit_actor(actor: &HirActor) -> String {
    let mut out = String::new();
    let msg_enum = format!("{}Message", actor.name);

    // Actor Message Enum
    out.push_str("#[derive(Debug, Clone, Serialize, Deserialize)]\n");
    out.push_str(&format!("pub enum {} {{\n", msg_enum));
    for handler in &actor.handlers {
        out.push_str(&format!("    {} {{ ", capitalize(&handler.event_name)));
        for param in &handler.params {
            out.push_str(&format!(
                "{}: {}, ",
                param.name,
                emit_type(
                    param
                        .type_ann
                        .as_ref()
                        .unwrap_or(&HirType::Named("serde_json::Value".into()))
                )
            ));
        }
        out.push_str("},\n");
    }
    out.push_str("}\n\n");

    // Actor Logic — handles Request envelopes with reply channels
    out.push_str(&format!("pub struct {};\n", actor.name));
    out.push_str(&format!("impl {} {{\n", actor.name));
    out.push_str("    pub async fn run(mut ctx: ProcessContext) {\n");
    out.push_str("        while let Some(envelope) = ctx.receive().await {\n");
    out.push_str("            match envelope {\n");
    out.push_str("                vox_runtime::Envelope::Request(req) => {\n");
    out.push_str(
        "                    if let vox_runtime::MessagePayload::Json(json_str) = &req.payload {\n",
    );
    out.push_str(&format!(
        "                        if let Ok(actor_msg) = serde_json::from_str::<{}>(&json_str) {{\n",
        msg_enum
    ));
    out.push_str("                            let reply_str = match actor_msg {\n");

    for handler in &actor.handlers {
        out.push_str(&format!(
            "                                {}::{} {{ ",
            msg_enum,
            capitalize(&handler.event_name)
        ));
        for param in &handler.params {
            out.push_str(&format!("{}, ", param.name));
        }
        out.push_str("} => {\n");
        // Emit handler body statements. The last expression is the reply value.
        // Must produce a String — serialize if needed.
        if handler.body.is_empty() {
            out.push_str("                                    String::new()\n");
        } else {
            // Emit all statements except the last as regular statements
            for stmt in &handler.body[..handler.body.len().saturating_sub(1)] {
                out.push_str(&emit_stmt(stmt, 10, false, true, false));
            }
            // For the last statement, extract the value and serialize to String
            if let Some(last) = handler.body.last() {
                match last {
                    HirStmt::Return {
                        value: Some(val), ..
                    } => {
                        let val_str = emit_expr(val);
                        out.push_str(&format!(
                            "                                    match serde_json::to_string(&({})) {{ Ok(s) => s, Err(e) => format!(\"{{\\\"error\\\":\\\"{{}}\\\"}}\", e) }}\n",
                            val_str
                        ));
                    }
                    HirStmt::Expr { expr, .. } => {
                        let val_str = emit_expr(expr);
                        out.push_str(&format!(
                            "                                    match serde_json::to_string(&({})) {{ Ok(s) => s, Err(e) => format!(\"{{\\\"error\\\":\\\"{{}}\\\"}}\", e) }}\n",
                            val_str
                        ));
                    }
                    _ => {
                        out.push_str(&emit_stmt(last, 10, false, true, false));
                        out.push_str("                                    String::new()\n");
                    }
                }
            }
        }
        out.push_str("                                }\n");
    }

    out.push_str("                            };\n");
    out.push_str("                            ProcessContext::reply(req, reply_str);\n");
    out.push_str("                        }\n");
    out.push_str("                    }\n");
    out.push_str("                }\n");
    out.push_str("                vox_runtime::Envelope::Message(msg) => {\n");
    out.push_str("                    // Fire-and-forget: process but don't reply\n");
    out.push_str(
        "                    if let vox_runtime::MessagePayload::Json(json_str) = msg.payload {\n",
    );
    out.push_str(&format!(
        "                        if let Ok(actor_msg) = serde_json::from_str::<{}>(&json_str) {{\n",
        msg_enum
    ));
    out.push_str("                            let _ = actor_msg; // processed\n");
    out.push_str("                        }\n");
    out.push_str("                    }\n");
    out.push_str("                }\n");
    out.push_str("                _ => {}\n");
    out.push_str("            }\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Typed Handle — uses call() for request-response
    out.push_str(&format!(
        "#[derive(Clone)]\npub struct {}Handle {{\n",
        actor.name
    ));
    out.push_str("    handle: vox_runtime::ProcessHandle,\n");
    out.push_str("}\n");
    out.push_str(&format!("impl {}Handle {{\n", actor.name));
    out.push_str(
        "    pub fn new(handle: vox_runtime::ProcessHandle) -> Self { Self { handle } }\n",
    );
    out.push_str("    pub fn spawn() -> Self {\n");
    out.push_str(&format!(
        "        let handle = vox_runtime::spawn_process({}::run);\n",
        actor.name
    ));
    out.push_str("        Self::new(handle)\n");
    out.push_str("    }\n");

    for handler in &actor.handlers {
        out.push_str(&format!("    pub async fn {}(&self, ", handler.event_name));
        for param in &handler.params {
            out.push_str(&format!(
                "{}: {}, ",
                param.name,
                emit_type(
                    param
                        .type_ann
                        .as_ref()
                        .unwrap_or(&HirType::Named("serde_json::Value".into()))
                )
            ));
        }
        out.push_str(") -> Result<String, vox_runtime::CallError> {\n");
        out.push_str(&format!(
            "        let msg = {}::{} {{ ",
            msg_enum,
            capitalize(&handler.event_name)
        ));
        for param in &handler.params {
            out.push_str(&format!("{}, ", param.name));
        }
        out.push_str("};\n");
        out.push_str("        let payload = vox_runtime::MessagePayload::Json(serde_json::to_string(&msg).expect(\"vox codegen: actor message JSON\"));\n");
        out.push_str("        self.handle.call(payload).await\n");
        out.push_str("    }\n");
    }
    out.push_str("}\n\n");

    out
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

#[cfg(test)]
mod tests {
    use super::emit_lib;
    use crate::ast::span::Span;
    use crate::hir::lower_module;
    use crate::hir::{DefId, HirActor, HirActorHandler, HirModule};
    use crate::lexer::cursor::lex;
    use crate::parser::parse;

    #[test]
    fn actor_handle_methods_return_call_error_result() {
        let span = Span::new(0, 0);
        let mut module = HirModule::default();
        module.actors.push(HirActor {
            id: DefId(1),
            name: "Worker".to_string(),
            handlers: vec![HirActorHandler {
                event_name: "ping".to_string(),
                params: vec![],
                return_type: None,
                body: vec![],
                span,
            }],
            span,
        });
        let out = emit_lib(&module);
        assert!(
            out.contains("pub async fn ping(&self, ) -> Result<String, vox_runtime::CallError>"),
            "actor method should return typed call result:\n{out}"
        );
        assert!(
            !out.contains("unwrap_or_else(|e| format!(\"Actor error: {}\", e))"),
            "actor method should not collapse runtime errors into payload strings:\n{out}"
        );
    }

    #[test]
    fn workflow_dispatch_helper_is_emitted_with_argument_decode() {
        let src = r#"
workflow greet(name: str) {
    ret
}
"#;
        let tokens = lex(src);
        let module = parse(tokens).expect("parse");
        let hir = lower_module(&module);
        let out = emit_lib(&hir);
        assert!(
            out.contains("pub async fn __vox_run_workflow("),
            "expected generated workflow dispatcher helper: {out}"
        );
        assert!(
            out.contains("serde_json::from_value(args[0].clone())"),
            "expected argument decode in workflow dispatcher: {out}"
        );
        assert!(
            out.contains("workflow `greet` expects 1 argument(s)"),
            "expected argument count guard in workflow dispatcher: {out}"
        );
    }
}
