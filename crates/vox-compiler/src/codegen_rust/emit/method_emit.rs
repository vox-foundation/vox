//! `emit_expr` helpers for `MethodCall` (db.*, tracing, oratio, etc.).

use crate::hir::HirExpr;

pub(super) fn emit_method_call(
    emit_expr: fn(&HirExpr) -> String,
    obj: &HirExpr,
    method: &str,
    args: &[crate::hir::HirArg],
) -> String {
    if let HirExpr::Ident(obj_name, _) = obj {
        if obj_name == "Speech" && method == "transcribe" && args.len() == 1 {
            let p = emit_expr(&args[0].value);
            return format!(
                "(match vox_oratio::transcribe_path(std::path::Path::new(({}).as_str())) {{ Ok(t) => Ok(t.display_text().to_string()), Err(e) => Error(format!(\"{{}}\", e)) }})",
                p
            );
        }
        if obj_name == "log" && !args.is_empty() {
            let mut args_iter = args.iter();
            if let Some(first_arg) = args_iter.next() {
                let fmt = match &first_arg.value {
                    HirExpr::StringLit(s, _) => format!("\"{}\"", s),
                    other => emit_expr(other),
                };
                let remaining: Vec<String> = args_iter.map(|a| emit_expr(&a.value)).collect();
                let macro_name = match method {
                    "info" => "info",
                    "warn" => "warn",
                    "error" => "error",
                    "debug" => "debug",
                    _ => "info",
                };
                if remaining.is_empty() {
                    return format!("tracing::{}!(\"{{:?}}\", {})", macro_name, fmt);
                }
                return format!(
                    "tracing::{}!({}, {})",
                    macro_name,
                    fmt,
                    remaining.join(", ")
                );
            }
        }
    }
    // Check for db.Table.method pattern
    if let HirExpr::FieldAccess(inner, table_name, _) = obj {
        if let HirExpr::Ident(n, _) = inner.as_ref() {
            if n == "db" {
                let args_str: Vec<String> = args.iter().map(|a| emit_expr(&a.value)).collect();
                return match method {
                    "insert" => {
                        format!(
                            "{{ let item: {} = serde_json::from_value({}).expect(\"vox codegen: db insert from_value\"); {}::insert(&*db, &item).await.expect(\"vox codegen: db insert\") }}",
                            table_name,
                            args_str
                                .first()
                                .unwrap_or(&"serde_json::json!({})".to_string()),
                            table_name
                        )
                    }
                    "get" => {
                        format!(
                            "{}::get(&*db, {}).await.expect(\"vox codegen: db get\")",
                            table_name,
                            args_str.first().unwrap_or(&"0".to_string())
                        )
                    }
                    "delete" => {
                        format!(
                            "{}::delete(&*db, {}).await.expect(\"vox codegen: db delete\")",
                            table_name,
                            args_str.first().unwrap_or(&"0".to_string())
                        )
                    }
                    "query" => {
                        format!(
                            "{}::query(&*db, {}).await.expect(\"vox codegen: db query\")",
                            table_name,
                            args_str.first().unwrap_or(&"\"\"".to_string())
                        )
                    }
                    _ => format!("/* unsupported db method {} */", method),
                };
            }
        }
    }

    let o = emit_expr(obj);
    if method == "json" && o == "request" {
        return "request.clone()".into();
    }
    let call = format!(
        "{}.{}({})",
        o,
        method,
        args.iter()
            .map(|a| emit_expr(&a.value))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if method == "send" {
        format!("{}.await", call)
    } else {
        call
    }
}
