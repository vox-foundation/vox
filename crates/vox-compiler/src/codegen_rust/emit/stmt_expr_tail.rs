use crate::hir::HirExpr;

/// Object literals, method/table ops, control flow, and block expressions (split from `stmt_expr` for line budget).
pub(super) fn try_emit_expr_tail<F>(
    expr: &HirExpr,
    is_route: bool,
    is_actor: bool,
    mutation_tx: bool,
    fallible_db: bool,
    emit: &F,
) -> Option<String>
where
    F: Fn(&HirExpr) -> String,
{
    Some(match expr {
        HirExpr::ObjectLit(fields, _) => {
            let props: Vec<String> = fields
                .iter()
                .map(|(k, v)| format!("\"{}\": {}", k, emit(v)))
                .collect();
            format!("serde_json::json!({{ {} }})", props.join(", "))
        }
        HirExpr::MethodCall(obj, method, args, _) => super::method_emit::emit_method_call(
            emit,
            obj.as_ref(),
            method.as_str(),
            args,
            fallible_db,
        ),
        HirExpr::DbTableOp {
            table,
            op,
            args,
            select_cols,
            order_by,
            limit,
            plan,
            ..
        } => super::method_emit::emit_db_table_op(
            emit,
            table.as_str(),
            *op,
            args,
            select_cols,
            order_by,
            limit,
            plan.as_ref(),
            fallible_db,
        ),
        HirExpr::Spawn(target, _) => {
            if let HirExpr::Ident(n, _) = &**target {
                format!("{}Handle::spawn()", n)
            } else {
                "/* error: spawn target must be actor name */".into()
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            let mut s = format!("if {} {{\n", emit(cond));
            for stmt in then_b {
                s.push_str(&super::stmt_expr::emit_stmt(
                    stmt,
                    1,
                    is_route,
                    false,
                    mutation_tx,
                ));
            }
            s.push('}');
            if let Some(else_stmts) = else_b {
                s.push_str(" else {\n");
                for stmt in else_stmts {
                    s.push_str(&super::stmt_expr::emit_stmt(
                        stmt,
                        1,
                        is_route,
                        false,
                        mutation_tx,
                    ));
                }
                s.push('}');
            }
            s
        }
        HirExpr::FieldAccess(obj, field, _) => {
            let o = emit(obj);
            if o == "std" && field == "args" {
                "std::env::args().skip(1).map(|s| s.to_string()).collect::<Vec<String>>()"
                    .to_string()
            } else if o == "std" || o == "::std" || o.starts_with("std::") || o.starts_with("::std::") {
                let prefix = if o.starts_with("::") { o } else { format!("::{}", o) };
                format!("{}::{}", prefix, field)
            } else {
                format!("{}[\"{}\"].clone()", o, field)
            }
        }
        HirExpr::Block(stmts, _) => {
            let mut s = String::from("{\n");
            for stmt in stmts {
                s.push_str(&super::stmt_expr::emit_stmt(
                    stmt,
                    1,
                    is_route,
                    false,
                    mutation_tx,
                ));
            }
            s.push('}');
            s
        }
        HirExpr::With(operand, options, _) => {
            super::with_emit::emit_with(emit, operand.as_ref(), options.as_ref())
        }
        HirExpr::Lambda(params, _ret_ty, body, _) => {
            let mut s = String::new();
            s.push('|');
            let param_strs: Vec<String> = params.iter().map(|p| p.name.clone()).collect();
            s.push_str(&param_strs.join(", "));
            s.push_str("| ");
            s.push_str(&emit(body));
            s
        }
        HirExpr::Pipe(left, right, _) => {
            format!("({})({})", emit(right), emit(left))
        }
        HirExpr::For(name, iter, body, _) => {
            let mut s = format!("for {} in {} {{\n", name, emit(iter));
            if let HirExpr::Block(stmts, _) = &**body {
                for stmt in stmts {
                    s.push_str(&super::stmt_expr::emit_stmt(
                        stmt,
                        1,
                        is_route,
                        false,
                        mutation_tx,
                    ));
                }
            } else {
                s.push_str(&format!("  {};\n", emit(body)));
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
            format!("{}({})", op_str, emit(expr))
        }
        HirExpr::Match(obj, arms, _) => {
            let mut s = format!("match {} {{\n", emit(obj));
            for arm in arms {
                s.push_str(&format!(
                    "    {} => {{\n",
                    super::stmt_expr::emit_pattern(&arm.pattern, is_route, is_actor, mutation_tx,)
                ));
                s.push_str(&emit(&arm.body));
                s.push_str("\n    }\n");
            }
            s.push('}');
            s
        }
        HirExpr::Try(h) => format!("({})?", emit(h.target.as_ref())),

        _ => return None,
    })
}
