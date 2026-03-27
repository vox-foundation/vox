//! `emit_expr` helpers for `MethodCall` (db.*, tracing, oratio, etc.).

use crate::hir::{HirDbPredicate, HirDbQueryPlan, HirDbTableOp, HirExpr};

fn db_ref(fallible: bool) -> &'static str {
    if fallible { "&db" } else { "&*db" }
}

fn await_or_expect_suffix(fallible: bool, expect_msg: &str) -> String {
    if fallible {
        ".await?".into()
    } else {
        format!(".await.expect(\"{expect_msg}\")")
    }
}

/// Emit lowered `db.<Table>.<op>(...)` (canonical Codex IR).
pub(super) fn emit_db_table_op<F>(
    emit_expr: &F,
    table_name: &str,
    op: HirDbTableOp,
    args: &[crate::hir::HirArg],
    select_cols: &Option<Vec<String>>,
    order_by: &Option<(String, bool)>,
    limit: &Option<Box<HirExpr>>,
    plan: Option<&HirDbQueryPlan>,
    fallible: bool,
) -> String
where
    F: Fn(&HirExpr) -> String,
{
    let db = db_ref(fallible);
    let args_str: Vec<String> = args.iter().map(|a| emit_expr(&a.value)).collect();
    fn emit_predicate_sql<F>(
        _emit_expr: &F,
        pred: &HirDbPredicate,
        _args: &[crate::hir::HirArg],
        next_param: &mut usize,
        next_arg: &mut usize,
    ) -> String
    where
        F: Fn(&HirExpr) -> String,
    {
        match pred {
            HirDbPredicate::Eq { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} = ?{idx}")
            }
            HirDbPredicate::Neq { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} <> ?{idx}")
            }
            HirDbPredicate::Lt { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} < ?{idx}")
            }
            HirDbPredicate::Lte { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} <= ?{idx}")
            }
            HirDbPredicate::Gt { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} > ?{idx}")
            }
            HirDbPredicate::Gte { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} >= ?{idx}")
            }
            HirDbPredicate::Contains { field } => {
                let idx = *next_param;
                *next_param += 1;
                *next_arg += 1;
                format!("{field} LIKE '%' || ?{idx} || '%'")
            }
            HirDbPredicate::IsNull { field } => format!("{field} IS NULL"),
            HirDbPredicate::In { field, arity } => {
                let mut slots = Vec::with_capacity(*arity);
                for _ in 0..*arity {
                    let idx = *next_param;
                    *next_param += 1;
                    *next_arg += 1;
                    slots.push(format!("?{idx}"));
                }
                format!("{field} IN ({})", slots.join(", "))
            }
            HirDbPredicate::And(parts) => parts
                .iter()
                .map(|p| {
                    format!(
                        "({})",
                        emit_predicate_sql(_emit_expr, p, _args, next_param, next_arg)
                    )
                })
                .collect::<Vec<_>>()
                .join(" AND "),
            HirDbPredicate::Or(parts) => parts
                .iter()
                .map(|p| {
                    format!(
                        "({})",
                        emit_predicate_sql(_emit_expr, p, _args, next_param, next_arg)
                    )
                })
                .collect::<Vec<_>>()
                .join(" OR "),
            HirDbPredicate::Not(inner) => {
                format!(
                    "NOT ({})",
                    emit_predicate_sql(_emit_expr, inner, _args, next_param, next_arg)
                )
            }
        }
    }

    fn emit_params_values<F>(emit_expr: &F, args: &[crate::hir::HirArg]) -> String
    where
        F: Fn(&HirExpr) -> String,
    {
        args.iter()
            .map(|a| emit_expr(&a.value))
            .collect::<Vec<_>>()
            .join(", ")
    }

    let rendered = match op {
        HirDbTableOp::Insert => {
            let val = args_str
                .first()
                .cloned()
                .unwrap_or_else(|| "serde_json::json!({})".to_string());
            if fallible {
                format!(
                    "{{ let item: {table_name} = serde_json::from_value({val}).map_err(|e| vox_db::StoreError::Serialization(format!(\"{{}}\", e)))?; {table_name}::insert({db}, &item).await?; }}"
                )
            } else {
                format!(
                    "{{ let item: {table_name} = serde_json::from_value({val}).expect(\"vox codegen: db insert from_value\"); {table_name}::insert({db}, &item){} }}",
                    await_or_expect_suffix(false, "vox codegen: db insert")
                )
            }
        }
        HirDbTableOp::Get => {
            format!(
                "{}::get({}, {}){}",
                table_name,
                db,
                args_str.first().unwrap_or(&"0".to_string()),
                await_or_expect_suffix(fallible, "vox codegen: db get")
            )
        }
        HirDbTableOp::Delete => {
            format!(
                "{}::delete({}, {}){}",
                table_name,
                db,
                args_str.first().unwrap_or(&"0".to_string()),
                await_or_expect_suffix(fallible, "vox codegen: db delete")
            )
        }
        HirDbTableOp::All => {
            if order_by.is_some() || limit.is_some() {
                let order_sql = order_by
                    .as_ref()
                    .map(|(col, asc)| format!("{col} {}", if *asc { "ASC" } else { "DESC" }))
                    .unwrap_or_default();
                let limit_sql = limit
                    .as_ref()
                    .map(|e| format!("Some(({}) as i64)", emit_expr(e.as_ref())))
                    .unwrap_or_else(|| "None".to_string());
                format!(
                    "{}::all_order_limit({}, \"{}\", {}){}",
                    table_name,
                    db,
                    order_sql,
                    limit_sql,
                    await_or_expect_suffix(fallible, "vox codegen: db all_order_limit")
                )
            } else {
                format!(
                    "{}::all({}){}",
                    table_name,
                    db,
                    await_or_expect_suffix(fallible, "vox codegen: db all")
                )
            }
        }
        HirDbTableOp::Count => {
            if order_by.is_some() || limit.is_some() {
                return "/* vox codegen: invalid count modifiers (typecheck should reject) */ 0"
                    .into();
            }
            if args.is_empty() {
                format!(
                    "{}::count({}){}",
                    table_name,
                    db,
                    await_or_expect_suffix(fallible, "vox codegen: db count")
                )
            } else {
                let where_sql = if let Some(pred) = plan.and_then(|p| p.predicate.as_ref()) {
                    let mut next_param = 1usize;
                    let mut next_arg = 0usize;
                    emit_predicate_sql(emit_expr, pred, args, &mut next_param, &mut next_arg)
                } else {
                    args.iter()
                        .enumerate()
                        .map(|(i, a)| {
                            let col = a
                                .name
                                .as_deref()
                                .expect("count filter args must be named columns");
                            format!("{col} = ?{}", i + 1)
                        })
                        .collect::<Vec<_>>()
                        .join(" AND ")
                };
                let params = emit_params_values(emit_expr, args);
                format!(
                    "{}::count_where({}, \"{}\", turso::params![{}]){}",
                    table_name,
                    db,
                    where_sql,
                    params,
                    await_or_expect_suffix(fallible, "vox codegen: db count_where")
                )
            }
        }
        HirDbTableOp::FilterRecord => {
            if args.is_empty() {
                return format!(
                    "{{ /* vox codegen: empty filter */ {}::all({}){} }}",
                    table_name,
                    db,
                    await_or_expect_suffix(fallible, "")
                );
            }
            let where_sql = if let Some(pred) = plan.and_then(|p| p.predicate.as_ref()) {
                let mut next_param = 1usize;
                let mut next_arg = 0usize;
                emit_predicate_sql(emit_expr, pred, args, &mut next_param, &mut next_arg)
            } else {
                args.iter()
                    .enumerate()
                    .map(|(i, a)| {
                        let col = a
                            .name
                            .as_deref()
                            .expect("filter_record args must be named columns");
                        format!("{col} = ?{}", i + 1)
                    })
                    .collect::<Vec<_>>()
                    .join(" AND ")
            };
            let params = emit_params_values(emit_expr, args);
            let proj = select_cols.as_ref().and_then(|c| {
                if c.is_empty() {
                    None
                } else {
                    Some(super::tables::db_projection_method_suffix(c.as_slice()))
                }
            });
            if order_by.is_some() || limit.is_some() {
                let order_sql = order_by
                    .as_ref()
                    .map(|(col, asc)| format!("{col} {}", if *asc { "ASC" } else { "DESC" }))
                    .unwrap_or_default();
                let limit_sql = limit
                    .as_ref()
                    .map(|e| format!("Some(({}) as i64)", emit_expr(e.as_ref())))
                    .unwrap_or_else(|| "None".to_string());
                if let Some(sfx) = proj {
                    format!(
                        "{}::filter_where_order_limit_proj_{}({}, \"{}\", turso::params![{}], \"{}\", {}){}",
                        table_name,
                        sfx,
                        db,
                        where_sql,
                        params,
                        order_sql,
                        limit_sql,
                        await_or_expect_suffix(
                            fallible,
                            "vox codegen: db filter_where_order_limit_proj"
                        )
                    )
                } else {
                    format!(
                        "{}::filter_where_order_limit({}, \"{}\", turso::params![{}], \"{}\", {}){}",
                        table_name,
                        db,
                        where_sql,
                        params,
                        order_sql,
                        limit_sql,
                        await_or_expect_suffix(
                            fallible,
                            "vox codegen: db filter_where_order_limit"
                        )
                    )
                }
            } else if let Some(sfx) = proj {
                format!(
                    "{}::filter_where_proj_{}({}, \"{}\", turso::params![{}]){}",
                    table_name,
                    sfx,
                    db,
                    where_sql,
                    params,
                    await_or_expect_suffix(fallible, "vox codegen: db filter_where_proj")
                )
            } else {
                format!(
                    "{}::filter_where({}, \"{}\", turso::params![{}]){}",
                    table_name,
                    db,
                    where_sql,
                    params,
                    await_or_expect_suffix(fallible, "vox codegen: db filter_where")
                )
            }
        }
        HirDbTableOp::UnsafeQueryRawClause => {
            format!(
                "{}::unsafe_query_raw_clause({}, {}){}",
                table_name,
                db,
                args_str.first().unwrap_or(&"\"\"".to_string()),
                await_or_expect_suffix(fallible, "vox codegen: db unsafe_query_raw_clause")
            )
        }
    };
    if plan.is_some_and(|p| p.capabilities.requires_sync)
        && matches!(
            op,
            HirDbTableOp::Get
                | HirDbTableOp::All
                | HirDbTableOp::FilterRecord
                | HirDbTableOp::Count
        )
    {
        if fallible {
            format!("{{ db.sync().await?; {rendered} }}")
        } else {
            format!("{{ db.sync().await.expect(\"vox codegen: db sync\"); {rendered} }}")
        }
    } else {
        rendered
    }
}

pub(super) fn emit_method_call<F>(
    emit_expr: &F,
    obj: &HirExpr,
    method: &str,
    args: &[crate::hir::HirArg],
    fallible_db: bool,
) -> String
where
    F: Fn(&HirExpr) -> String,
{
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
    // Fallback: `db.Table.method` if lowering missed (should be rare).
    if let HirExpr::FieldAccess(inner, table_name, _) = obj {
        if let HirExpr::Ident(n, _) = inner.as_ref() {
            if n == "db" {
                let op = match method {
                    "insert" => Some(HirDbTableOp::Insert),
                    "get" | "find" => Some(HirDbTableOp::Get),
                    "delete" => Some(HirDbTableOp::Delete),
                    "all" => Some(HirDbTableOp::All),
                    "count" => Some(HirDbTableOp::Count),
                    "query" => Some(HirDbTableOp::UnsafeQueryRawClause),
                    _ => None,
                };
                if let Some(op) = op {
                    return emit_db_table_op(
                        emit_expr,
                        table_name,
                        op,
                        args,
                        &None,
                        &None,
                        &None,
                        None,
                        fallible_db,
                    );
                }
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
