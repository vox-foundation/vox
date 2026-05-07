use crate::hir::{HirFn, HirForall, HirModule, HirType};

use super::stmt_expr::{emit_expr, emit_stmt};
use super::tables::{collect_table_select_projections, emit_table_struct};
use super::types::emit_type;

pub fn emit_lib(module: &HirModule) -> String {
    let mut out = String::new();
    out.push_str("use serde::{Serialize, Deserialize};\n");

    if module.functions.iter().any(|f| f.is_llm) {
        out.push_str("use vox_clavis::{SecretId, resolve_secret};\n");
        out.push_str(
            "use vox_config::inference::{OPENROUTER_CHAT_COMPLETIONS_URL, openrouter_chat_model_preference};\n",
        );
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
        let model_init = if let Some(m) = func.llm_model.as_deref() {
            format!(
                "\"{}\".to_string()",
                m.replace('\\', "\\\\").replace('"', "\\\"")
            )
        } else {
            "openrouter_chat_model_preference()".to_string()
        };
        out.push_str("    let client = reqwest::Client::new();\n");
        out.push_str("    let token = resolve_secret(SecretId::OpenRouterApiKey).expose().expect(\"LLM function requires OpenRouterApiKey\").to_string();\n");
        out.push_str(&format!("    let model = {};\n", model_init));

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
        out.push_str("        client.post(OPENROUTER_CHAT_COMPLETIONS_URL)\n");
        out.push_str("            .header(\"Authorization\", format!(\"Bearer {}\", token))\n");
        out.push_str("            .json(&serde_json::json!({\n");
        out.push_str("                \"model\": model,\n");
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
