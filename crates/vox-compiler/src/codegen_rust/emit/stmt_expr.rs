use crate::hir::{HirBinOp, HirExpr, HirPattern, HirStmt};

pub(super) fn emit_stmt(stmt: &HirStmt, indent: usize, is_route: bool, is_actor: bool) -> String {
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
                emit_pattern(pattern),
                emit_expr(value)
            )
        }
        HirStmt::Assign { target, value, .. } => {
            format!("{pad}{} = {};\n", emit_expr(target), emit_expr(value))
        }
        HirStmt::Return { value, .. } => {
            if is_actor {
                if let Some(v) = value {
                    format!(
                        "{pad}let _ = {}; // return ignored in actor; scaffolding only\n",
                        emit_expr(v)
                    )
                } else {
                    format!("{pad}// return ignored in actor; scaffolding only\n")
                }
            } else if let Some(v) = value {
                let expr_str = emit_expr(v);
                if is_route {
                    format!(
                        "{pad}return Json(serde_json::to_value({}).expect(\"vox codegen: route return JSON\"));\n",
                        expr_str
                    )
                } else {
                    format!("{pad}return {};\n", expr_str)
                }
            } else if is_route {
                format!("{pad}return Json(serde_json::Value::Null);\n")
            } else {
                format!("{pad}return;\n")
            }
        }
        HirStmt::Expr { expr, .. } => {
            format!("{pad}{};\n", emit_expr(expr))
        }
    }
}

/// Emit one statement for script-mode `main` (no route/actor return wrapping).
pub fn emit_main_stmt(stmt: &HirStmt, indent: usize) -> String {
    emit_stmt(stmt, indent, false, false)
}

fn emit_pattern(pat: &HirPattern) -> String {
    match pat {
        HirPattern::Ident(n, _) => n.clone(),
        HirPattern::Wildcard(_) => "_".into(),
        HirPattern::Literal(lit, _) => emit_expr(lit),
        HirPattern::Tuple(pats, _) => format!(
            "({})",
            pats.iter().map(emit_pattern).collect::<Vec<_>>().join(", ")
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
                pats.iter().map(emit_pattern).collect::<Vec<_>>().join(", ")
            )
        }
    }
}

/// Emit one HIR expression as a Rust expression string (for nested codegen / tools).
pub fn emit_expr(expr: &HirExpr) -> String {
    match expr {
        HirExpr::IntLit(v, _) => v.to_string(),
        HirExpr::FloatLit(v, _) => v.to_string(),
        HirExpr::StringLit(v, _) => format!("\"{}\".to_string()", v),
        HirExpr::BoolLit(v, _) => v.to_string(),
        HirExpr::ListLit(elements, _) => format!(
            "vec![{}]",
            elements
                .iter()
                .map(emit_expr)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        HirExpr::TupleLit(elements, _) => format!(
            "({})",
            elements
                .iter()
                .map(emit_expr)
                .collect::<Vec<_>>()
                .join(", ")
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
                HirBinOp::Pipe => return format!("{}({})", emit_expr(r), emit_expr(l)),
            };
            if matches!(
                op,
                HirBinOp::Add | HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Div
            ) {
                format!("({} {} &{})", emit_expr(l), op_str, emit_expr(r))
            } else {
                format!("({} {} {})", emit_expr(l), op_str, emit_expr(r))
            }
        }
        HirExpr::Call(callee, args, is_await, _) => {
            if let HirExpr::Ident(n, _) = &**callee {
                if n == "str" && args.len() == 1 {
                    return format!("as_string(&{})", emit_expr(&args[0].value));
                }
                if n == "assert" && args.len() == 1 {
                    if let HirExpr::Binary(HirBinOp::Is, l, r, _) = &args[0].value {
                        return format!("assert_eq!({}, {})", emit_expr(l), emit_expr(r));
                    }
                    return format!("assert!({})", emit_expr(&args[0].value));
                }
                if n == "assert_eq" && args.len() >= 2 {
                    return format!(
                        "assert_eq!({}, {})",
                        emit_expr(&args[0].value),
                        emit_expr(&args[1].value)
                    );
                }
                if n == "assert_ne" && args.len() >= 2 {
                    return format!(
                        "assert_ne!({}, {})",
                        emit_expr(&args[0].value),
                        emit_expr(&args[1].value)
                    );
                }
                if n == "print" && args.len() == 1 {
                    return format!("println!(\"{{}}\", {})", emit_expr(&args[0].value));
                }
            }
            // std.* call forms: std.fs.read(path) → FieldAccess(FieldAccess(Ident("std"), "fs"), "read")
            if let HirExpr::FieldAccess(namespace_expr, fn_name, _) = &**callee {
                if let HirExpr::Ident(std_kw, _) = &**namespace_expr {
                    if std_kw == "std" {
                        let a: Vec<_> = args.iter().map(|arg| emit_expr(&arg.value)).collect();
                        let builtin: Option<String> = match fn_name.as_str() {
                            "uuid" => Some("vox_runtime::builtins::vox_uuid()".to_string()),
                            "now_ms" => Some("vox_runtime::builtins::vox_now_ms()".to_string()),
                            "hash_fast" if !a.is_empty() => {
                                Some(format!("vox_runtime::builtins::vox_hash_fast(&{})", a[0]))
                            }
                            "hash_secure" if !a.is_empty() => {
                                Some(format!("vox_runtime::builtins::vox_hash_secure(&{})", a[0]))
                            }
                            _ => None,
                        };
                        if let Some(b) = builtin {
                            return if *is_await { format!("{}.await", b) } else { b };
                        }
                    }
                }
                if let HirExpr::FieldAccess(std_expr, ns_name, _) = &**namespace_expr {
                    if let HirExpr::Ident(std_kw, _) = &**std_expr {
                        if std_kw == "std" {
                            let a: Vec<_> = args.iter().map(|arg| emit_expr(&arg.value)).collect();
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
            let c = emit_expr(callee);
            let a: Vec<_> = args.iter().map(|arg| emit_expr(&arg.value)).collect();
            if *is_await {
                format!("{}({}).await", c, a.join(", "))
            } else {
                format!("{}({})", c, a.join(", "))
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            let props: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, emit_expr(v)))
                .collect();
            format!("serde_json::json!({{ {} }})", props.join(", "))
        }
        HirExpr::MethodCall(obj, method, args, _) => {
            super::method_emit::emit_method_call(emit_expr, obj.as_ref(), method.as_str(), args)
        }
        HirExpr::Spawn(target, _) => {
            if let HirExpr::Ident(n, _) = &**target {
                format!("{}Handle::spawn()", n)
            } else {
                "/* error: spawn target must be actor name */".into()
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            let mut s = format!("if {} {{\n", emit_expr(cond));
            for stmt in then_b {
                s.push_str(&emit_stmt(stmt, 1, false, false));
            }
            s.push('}');
            if let Some(else_stmts) = else_b {
                s.push_str(" else {\n");
                for stmt in else_stmts {
                    s.push_str(&emit_stmt(stmt, 1, false, false));
                }
                s.push('}');
            }
            s
        }
        HirExpr::FieldAccess(obj, field, _) => {
            if let HirExpr::Ident(root, _) = obj.as_ref() {
                if root == "std" && field == "args" {
                    return "std::env::args().skip(1).map(|s| s.to_string()).collect::<Vec<String>>()"
                        .to_string();
                }
            }
            format!("{}[\"{}\"].clone()", emit_expr(obj), field)
        }
        HirExpr::Block(stmts, _) => {
            let mut s = String::from("{\n");
            for stmt in stmts {
                s.push_str(&emit_stmt(stmt, 1, false, false));
            }
            s.push('}');
            s
        }
        HirExpr::With(operand, options, _) => {
            super::with_emit::emit_with(emit_expr, operand.as_ref(), options.as_ref())
        }
        HirExpr::Lambda(params, _ret_ty, body, _) => {
            let mut s = String::new();
            s.push('|');
            let param_strs: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            s.push_str(&param_strs.join(", "));
            s.push_str("| ");
            s.push_str(&emit_expr(body));
            s
        }
        HirExpr::Pipe(left, right, _) => {
            format!("({})({})", emit_expr(right), emit_expr(left))
        }
        HirExpr::For(name, iter, body, _) => {
            let mut s = format!("for {} in {} {{\n", name, emit_expr(iter));
            if let HirExpr::Block(stmts, _) = &**body {
                for stmt in stmts {
                    s.push_str(&emit_stmt(stmt, 1, false, false));
                }
            } else {
                s.push_str(&format!("  {};\n", emit_expr(body)));
            }
            s.push_str("}\n");
            s
        }
        HirExpr::Jsx(_) | HirExpr::JsxSelfClosing(_) => {
            "panic!(\"JSX cannot be rendered via the Rust backend yet\")".into()
        }

        HirExpr::Unary(op, expr, _) => {
            let op_str = match op {
                crate::hir::HirUnOp::Not => "!",
                crate::hir::HirUnOp::Neg => "-",
            };
            format!("{}({})", op_str, emit_expr(expr))
        }
        HirExpr::Match(obj, arms, _) => {
            let mut s = format!("match {} {{\n", emit_expr(obj));
            for arm in arms {
                s.push_str(&format!("    {} => {{\n", emit_pattern(&arm.pattern)));
                s.push_str(&emit_expr(&arm.body));
                s.push_str("\n    }\n");
            }
            s.push('}');
            s
        }
    }
}
