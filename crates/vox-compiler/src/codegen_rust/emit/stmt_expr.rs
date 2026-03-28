use crate::builtin_registry::lookup_builtin;
use crate::hir::{HirBinOp, HirExpr, HirPattern, HirStmt};

pub(super) fn emit_stmt(
    stmt: &HirStmt,
    indent: usize,
    is_route: bool,
    is_actor: bool,
    mutation_tx: bool,
) -> String {
    let pad = " ".repeat(indent * 4);
    match stmt {
        HirStmt::Let {
            pattern,
            value,
            mutable,
            ..
        } => {
            let mut_kw = if *mutable { "mut " } else { "" };
            format!(
                "{pad}let {}{} = {};\n",
                mut_kw,
                emit_pattern(pattern, is_route, is_actor, mutation_tx),
                emit_expr_with(value, is_route, is_actor, mutation_tx)
            )
        }
        HirStmt::Assign { target, value, .. } => {
            format!(
                "{pad}{} = {};\n",
                emit_expr_with(target, is_route, is_actor, mutation_tx),
                emit_expr_with(value, is_route, is_actor, mutation_tx)
            )
        }
        HirStmt::Return { value, .. } => {
            if is_actor {
                if let Some(v) = value {
                    format!(
                        "{pad}let _ = {}; // return ignored in actor; scaffolding only\n",
                        emit_expr_with(v, is_route, is_actor, mutation_tx)
                    )
                } else {
                    format!("{pad}// return ignored in actor; scaffolding only\n")
                }
            } else if let Some(v) = value {
                let expr_str = emit_expr_with(v, is_route, is_actor, mutation_tx);
                if is_route && mutation_tx {
                    format!(
                        "{pad}return Ok(Json(serde_json::to_value({}).map_err(|e| vox_db::StoreError::Serialization(format!(\"{{}}\", e)))?));\n",
                        expr_str
                    )
                } else if is_route {
                    format!(
                        "{pad}return Json(serde_json::to_value({}).expect(\"vox codegen: route return JSON\"));\n",
                        expr_str
                    )
                } else {
                    format!("{pad}return {};\n", expr_str)
                }
            } else if is_route && mutation_tx {
                format!("{pad}return Ok(Json(serde_json::Value::Null));\n")
            } else if is_route {
                format!("{pad}return Json(serde_json::Value::Null);\n")
            } else {
                format!("{pad}return;\n")
            }
        }
        HirStmt::Expr { expr, .. } => {
            format!(
                "{pad}{};\n",
                emit_expr_with(expr, is_route, is_actor, mutation_tx)
            )
        }
    }
}

/// Emit one statement for script-mode `main` (no route/actor return wrapping).
pub fn emit_main_stmt(stmt: &HirStmt, indent: usize) -> String {
    emit_stmt(stmt, indent, false, false, false)
}

pub(super) fn emit_pattern(
    pat: &HirPattern,
    is_route: bool,
    is_actor: bool,
    mutation_tx: bool,
) -> String {
    match pat {
        HirPattern::Ident(n, _) => n.clone(),
        HirPattern::Wildcard(_) => "_".into(),
        HirPattern::Literal(lit, _) => emit_expr_with(lit, is_route, is_actor, mutation_tx),
        HirPattern::Tuple(pats, _) => format!(
            "({})",
            pats.iter()
                .map(|p| emit_pattern(p, is_route, is_actor, mutation_tx))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        HirPattern::Constructor(n, pats, _) => {
            // Rust struct variant syntax: Name { field: val }
            // HirPattern::Constructor has positional args?
            // "Ok(text: str)" -> Constructor("Ok", [Ident("text")])
            // Rust enum: Ok { text: ... } or Ok(...) depending on def.
            // Vox ADTs use named fields. So we matched on Struct names.
            // Wait, parse_typedef uses named fields.
            // But pattern matching? "Ok(r) -> r". This is positional.
            // My ADT generator emitted named fields: `Variant { field: Type }`.
            // Rust requires named matching if defined with names.
            // Or use tuple variants if positional.
            // Vox defines `| Ok(text: str)`. This is named.
            // So `Ok(t)` in match needs to be `Ok { text: t }`.
            // BUT the parser/HIR doesn't resolve positional match to named fields yet.
            // This is a semantic gap.
            // Workaround: Use tuple variants in Rust if possible, or assume names match?
            // "Ok(r)" -> pattern is Constructor("Ok", [Ident("r")]).
            // We don't know the field name "text" here without looking up the definition.
            // For now, emit as tuple style `Ok(p1, p2)` and hope the ADT generation uses tuple variants?
            // In emit_lib: `variant.fields` are named.
            // If I change emit_lib to use tuple variants if fields are present?
            // Or structs?
            // Vox syntax `Ok(text: str)` looks like named.
            // But usage `Ok("hi")` looks positional.
            // Let's generate Tuple variants in Rust for simplicity: `Ok(String)`.
            // And ignore field names in TypeDef?
            // Or use the names?
            format!(
                "{}({})",
                n,
                pats.iter()
                    .map(|p| emit_pattern(p, is_route, is_actor, mutation_tx))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

/// Emit one HIR expression as a Rust expression string (for nested codegen / tools).
pub fn emit_expr(expr: &HirExpr) -> String {
    emit_expr_with(expr, false, false, false)
}

fn emit_expr_with(expr: &HirExpr, is_route: bool, is_actor: bool, mutation_tx: bool) -> String {
    let fallible_db = mutation_tx;
    let emit = |e: &HirExpr| emit_expr_with(e, is_route, is_actor, mutation_tx);
    if let Some(s) = super::stmt_expr_tail::try_emit_expr_tail(
        expr,
        is_route,
        is_actor,
        mutation_tx,
        fallible_db,
        &emit,
    ) {
        return s;
    }
    match expr {
        HirExpr::IntLit(v, _) => v.to_string(),
        HirExpr::FloatLit(v, _) => v.to_string(),
        HirExpr::StringLit(v, _) => format!("\"{}\".to_string()", v),
        HirExpr::BoolLit(v, _) => v.to_string(),
        HirExpr::ListLit(elements, _) => format!(
            "vec![{}]",
            elements.iter().map(emit).collect::<Vec<_>>().join(", ")
        ),
        HirExpr::TupleLit(elements, _) => format!(
            "({})",
            elements.iter().map(emit).collect::<Vec<_>>().join(", ")
        ),

        HirExpr::Ident(n, _) => {
            if n == "request" {
                "request".into()
            } else if n.chars().next().map(|c| c.is_uppercase()).unwrap_or(false) {
                n.clone()
            } else {
                format!("{}.clone()", n)
            }
        }
        HirExpr::Binary(op, l, r, _) => {
            let op_str = match op {
                HirBinOp::Add => "+",
                HirBinOp::Sub => "-",
                HirBinOp::Mul => "*",
                HirBinOp::Div => "/",
                HirBinOp::Lt => "<",
                HirBinOp::Gt => ">",
                HirBinOp::Lte => "<=",
                HirBinOp::Gte => ">=",
                HirBinOp::And => "&&",
                HirBinOp::Or => "||",
                HirBinOp::Is => "==",
                HirBinOp::Isnt => "!=",
                HirBinOp::Pipe => return format!("{}({})", emit(r), emit(l)),
            };
            if matches!(
                op,
                HirBinOp::Add | HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Div
            ) {
                format!("({} {} &{})", emit(l), op_str, emit(r))
            } else {
                format!("({} {} {})", emit(l), op_str, emit(r))
            }
        }
        HirExpr::Call(callee, args, is_await, _) => {
            if let HirExpr::Ident(n, _) = &**callee {
                if n == "str" && args.len() == 1 {
                    return format!("as_string(&{})", emit(&args[0].value));
                }
                if n == "assert" && args.len() == 1 {
                    if let HirExpr::Binary(HirBinOp::Is, l, r, _) = &args[0].value {
                        return format!("assert_eq!({}, {})", emit(l), emit(r));
                    }
                    return format!("assert!({})", emit(&args[0].value));
                }
                if n == "assert_eq" && args.len() >= 2 {
                    return format!(
                        "assert_eq!({}, {})",
                        emit(&args[0].value),
                        emit(&args[1].value)
                    );
                }
                if n == "assert_ne" && args.len() >= 2 {
                    return format!(
                        "assert_ne!({}, {})",
                        emit(&args[0].value),
                        emit(&args[1].value)
                    );
                }
                if n == "print" && args.len() == 1 {
                    return format!("println!(\"{{}}\", {})", emit(&args[0].value));
                }
                if n == "len" && args.len() == 1 {
                    // Vec, String, &str, etc. — use Rust `.len()` (db.Table.all() lowers to Vec)
                    return format!("({}).len()", emit(&args[0].value));
                }
            }
            // std.* call forms: std.fs.read(path) → FieldAccess(FieldAccess(Ident("std"), "fs"), "read")
            if let HirExpr::FieldAccess(namespace_expr, fn_name, _) = &**callee {
                if let HirExpr::Ident(module_name, _) = &**namespace_expr
                    && module_name == "OpenClaw"
                {
                    let a: Vec<_> = args.iter().map(|arg| emit(&arg.value)).collect();
                    if let Some(call) = emit_registry_runtime_call("OpenClaw", fn_name, &a) {
                        return format!("(match {call} {{ Ok(v) => Ok(v), Err(m) => Error(m) }})");
                    }
                }
                if let HirExpr::Ident(std_kw, _) = &**namespace_expr {
                    if std_kw == "std" {
                        let a: Vec<_> = args.iter().map(|arg| emit(&arg.value)).collect();
                        if let Some(call) = emit_registry_runtime_call("std", fn_name, &a) {
                            return if *is_await {
                                format!("{}.await", call)
                            } else {
                                call
                            };
                        }
                    }
                }
                if let HirExpr::FieldAccess(std_expr, ns_name, _) = &**namespace_expr {
                    if let HirExpr::Ident(std_kw, _) = &**std_expr {
                        if std_kw == "std" {
                            let a: Vec<_> = args.iter().map(|arg| emit(&arg.value)).collect();
                            let builtin: Option<String> = match (ns_name.as_str(), fn_name.as_str())
                            {
                                ("crypto", "hash_fast") if !a.is_empty() => {
                                    Some(format!("vox_runtime::builtins::vox_hash_fast(&{})", a[0]))
                                }
                                ("crypto", "hash_secure") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_hash_secure(&{})",
                                    a[0]
                                )),
                                ("crypto", "uuid") => {
                                    Some("vox_runtime::builtins::vox_uuid()".to_string())
                                }
                                ("time", "now_ms") => {
                                    Some("vox_runtime::builtins::vox_now_ms()".to_string())
                                }
                                ("log", "debug") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_log_debug(({}).as_str())",
                                    a[0]
                                )),
                                ("log", "info") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_log_info(({}).as_str())",
                                    a[0]
                                )),
                                ("log", "warn") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_log_warn(({}).as_str())",
                                    a[0]
                                )),
                                ("log", "error") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_log_error(({}).as_str())",
                                    a[0]
                                )),
                                ("fs", "read") if !a.is_empty() => Some(format!(
                                    "std::fs::read_to_string({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
                                    a[0]
                                )),
                                ("fs", "write") if a.len() >= 2 => Some(format!(
                                    "std::fs::write({}, {}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
                                    a[0], a[1]
                                )),
                                ("fs", "exists") if !a.is_empty() => {
                                    Some(format!("std::path::Path::new(&{}).exists()", a[0]))
                                }
                                ("fs", "remove") if !a.is_empty() => Some(format!(
                                    "std::fs::remove_file({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
                                    a[0]
                                )),
                                ("fs", "read_bytes") if !a.is_empty() => Some(format!(
                                    "std::fs::read({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
                                    a[0]
                                )),
                                ("fs", "mkdir") if !a.is_empty() => Some(format!(
                                    "std::fs::create_dir_all({}).map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?",
                                    a[0]
                                )),
                                ("path", "join") if a.len() >= 2 => Some(format!(
                                    "std::path::Path::new(&{}).join(&{}).to_string_lossy().to_string()",
                                    a[0], a[1]
                                )),
                                ("path", "basename") if !a.is_empty() => Some(format!(
                                    "std::path::Path::new(&{}).file_name().unwrap_or_default().to_string_lossy().to_string()",
                                    a[0]
                                )),
                                ("path", "dirname") if !a.is_empty() => Some(format!(
                                    "std::path::Path::new(&{}).parent().unwrap_or(std::path::Path::new(\".\")).to_string_lossy().to_string()",
                                    a[0]
                                )),
                                ("path", "extension") if !a.is_empty() => Some(format!(
                                    "std::path::Path::new(&{}).extension().unwrap_or_default().to_string_lossy().to_string()",
                                    a[0]
                                )),
                                ("env", "get") if !a.is_empty() => Some(format!(
                                    "(vox_runtime::builtins::vox_env_get(({}).as_str()))",
                                    a[0]
                                )),
                                ("process", "run") if a.len() >= 2 => Some(format!(
                                    "(match vox_runtime::builtins::vox_process_run(({}).as_str(), {}.as_slice()) {{ Ok(c) => Ok(c as i64), Err(m) => Error(m) }})",
                                    a[0], a[1]
                                )),
                                ("process", "run_ex") if a.len() >= 4 => Some(format!(
                                    "(match vox_runtime::builtins::vox_process_run_ex(({}).as_str(), {}.as_slice(), ({}).as_str(), {}.as_slice()) {{ Ok(c) => Ok(c as i64), Err(m) => Error(m) }})",
                                    a[0], a[1], a[2], a[3]
                                )),
                                ("process", "run_capture") if a.len() >= 2 => Some(format!(
                                    "(match vox_runtime::builtins::vox_process_run_capture(({}).as_str(), {}.as_slice()) {{ Ok(p) => Ok(serde_json::json!({{ \"exit\": p.exit as i64, \"stdout\": p.stdout, \"stderr\": p.stderr }})), Err(m) => Error(m) }})",
                                    a[0], a[1]
                                )),
                                ("process", "run_capture_ex") if a.len() >= 4 => Some(format!(
                                    "(match vox_runtime::builtins::vox_process_run_capture_ex(({}).as_str(), {}.as_slice(), ({}).as_str(), {}.as_slice()) {{ Ok(p) => Ok(serde_json::json!({{ \"exit\": p.exit as i64, \"stdout\": p.stdout, \"stderr\": p.stderr }})), Err(m) => Error(m) }})",
                                    a[0], a[1], a[2], a[3]
                                )),
                                ("process", "exit") if !a.is_empty() => {
                                    Some(format!("{{ std::process::exit({} as i32) }}", a[0]))
                                }
                                ("fs", "list_dir") if !a.is_empty() => Some(format!(
                                    "(match vox_runtime::builtins::vox_list_dir(({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
                                    a[0]
                                )),
                                ("fs", "glob") if !a.is_empty() => Some(format!(
                                    "(match vox_runtime::builtins::vox_fs_glob(({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
                                    a[0]
                                )),
                                ("fs", "remove_dir_all") if !a.is_empty() => Some(format!(
                                    "(match vox_runtime::builtins::vox_fs_remove_dir_all(({}).as_str()) {{ Ok(()) => Ok(()), Err(m) => Error(m) }})",
                                    a[0]
                                )),
                                ("fs", "copy") if a.len() >= 2 => Some(format!(
                                    "(match vox_runtime::builtins::vox_fs_copy(({}).as_str(), ({}).as_str()) {{ Ok(()) => Ok(()), Err(m) => Error(m) }})",
                                    a[0], a[1]
                                )),
                                ("path", "join_many") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_path_join_many({}.as_slice())",
                                    a[0]
                                )),
                                ("json", "read_str") if a.len() >= 2 => Some(format!(
                                    "(match vox_runtime::builtins::vox_json_read_str(({}).as_str(), ({}).as_str()) {{ Ok(s) => Ok(s), Err(m) => Error(m) }})",
                                    a[0], a[1]
                                )),
                                ("json", "read_f64") if a.len() >= 2 => Some(format!(
                                    "(match vox_runtime::builtins::vox_json_read_f64(({}).as_str(), ({}).as_str()) {{ Ok(v) => Ok(v), Err(m) => Error(m) }})",
                                    a[0], a[1]
                                )),
                                ("json", "quote") if !a.is_empty() => Some(format!(
                                    "vox_runtime::builtins::vox_json_quote(({}).as_str())",
                                    a[0]
                                )),
                                _ => None,
                            };
                            if let Some(b) = builtin {
                                return if *is_await { format!("{}.await", b) } else { b };
                            }
                            let call = format!("std::{}::{}({})", ns_name, fn_name, a.join(", "));
                            return if *is_await {
                                format!("{}.await", call)
                            } else {
                                call
                            };
                        }
                    }
                }
            }
            let c = emit(callee);
            let a: Vec<_> = args.iter().map(|arg| emit(&arg.value)).collect();
            if *is_await {
                format!("{}({}).await", c, a.join(", "))
            } else {
                format!("{}({})", c, a.join(", "))
            }
        }
        _ => unreachable!(
            "HIR expr variants not handled in stmt_expr::emit_expr_with must be handled by stmt_expr_tail (delegate order bug)"
        ),
    }
}

fn emit_registry_runtime_call(namespace: &str, fn_name: &str, args: &[String]) -> Option<String> {
    let entry = lookup_builtin(namespace, fn_name, args.len())?;
    let symbol = entry.runtime_symbol?;
    let call = match args.len() {
        0 => format!("{symbol}()"),
        1 => format!("{symbol}(({}).as_str())", args[0]),
        2 => format!("{symbol}(({}).as_str(), ({}).as_str())", args[0], args[1]),
        _ => return None,
    };
    Some(call)
}
