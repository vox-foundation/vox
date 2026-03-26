use crate::ast::expr::{self, BinOp, Expr, UnOp};
use crate::hir::*;

use super::LowerCtx;

impl LowerCtx {
    pub(crate) fn lower_expr(&mut self, e: &Expr) -> HirExpr {
        match e {
            Expr::IntLit { value, span } => HirExpr::IntLit(*value, *span),
            Expr::FloatLit { value, span } => HirExpr::FloatLit(*value, *span),
            Expr::StringLit { value, span } => HirExpr::StringLit(value.clone(), *span),
            Expr::BoolLit { value, span } => HirExpr::BoolLit(*value, *span),
            Expr::Ident { name, span } => HirExpr::Ident(name.clone(), *span),
            Expr::ObjectLit { fields, span } => HirExpr::ObjectLit(
                fields
                    .iter()
                    .map(|(k, v)| (k.clone(), self.lower_expr(v)))
                    .collect(),
                *span,
            ),
            Expr::ListLit { elements, span } => {
                HirExpr::ListLit(elements.iter().map(|e| self.lower_expr(e)).collect(), *span)
            }
            Expr::TupleLit { elements, span } => {
                HirExpr::TupleLit(elements.iter().map(|e| self.lower_expr(e)).collect(), *span)
            }
            Expr::Binary {
                op,
                left,
                right,
                span,
            } => {
                let hir_op = match op {
                    BinOp::Add => HirBinOp::Add,
                    BinOp::Sub => HirBinOp::Sub,
                    BinOp::Mul => HirBinOp::Mul,
                    BinOp::Div => HirBinOp::Div,
                    BinOp::Lt => HirBinOp::Lt,
                    BinOp::Gt => HirBinOp::Gt,
                    BinOp::Lte => HirBinOp::Lte,
                    BinOp::Gte => HirBinOp::Gte,
                    BinOp::And => HirBinOp::And,
                    BinOp::Or => HirBinOp::Or,
                    BinOp::Is => HirBinOp::Is,
                    BinOp::Isnt => HirBinOp::Isnt,
                    BinOp::Pipe => HirBinOp::Pipe,
                };
                HirExpr::Binary(
                    hir_op,
                    Box::new(self.lower_expr(left)),
                    Box::new(self.lower_expr(right)),
                    *span,
                )
            }
            Expr::Unary { op, operand, span } => {
                let hir_op = match op {
                    UnOp::Not => HirUnOp::Not,
                    UnOp::Neg => HirUnOp::Neg,
                };
                HirExpr::Unary(hir_op, Box::new(self.lower_expr(operand)), *span)
            }
            Expr::Call { callee, args, span } => HirExpr::Call(
                Box::new(self.lower_expr(callee)),
                args.iter()
                    .map(|a| HirArg {
                        name: a.name.clone(),
                        value: self.lower_expr(&a.value),
                    })
                    .collect(),
                false,
                *span,
            ),
            Expr::MethodCall {
                object,
                method,
                args,
                span,
            } => {
                if let Some(chain) = extract_db_query_chain(self, e) {
                    let plan = make_db_plan_from_chain(&chain);
                    return HirExpr::DbTableOp {
                        table: chain.table,
                        op: chain.op,
                        args: chain.args,
                        select_cols: chain.select_cols,
                        order_by: chain.order_by,
                        limit: chain.limit.map(Box::new),
                        plan: Some(plan),
                        span: *span,
                    };
                }
                let obj_hir = self.lower_expr(object);
                let hir_args: Vec<HirArg> = args
                    .iter()
                    .map(|a| HirArg {
                        name: a.name.clone(),
                        value: self.lower_expr(&a.value),
                    })
                    .collect();
                if method == "count"
                    && hir_args.is_empty()
                    && let Some((table, count_args)) = extract_count_chain_args(self, object)
                {
                    let plan = HirDbQueryPlan {
                        table: table.clone(),
                        op: HirDbTableOp::Count,
                        predicate: None,
                        projection: None,
                        order_by: None,
                        has_limit: false,
                        capabilities: HirDbPlanCapabilities::default(),
                    };
                    return HirExpr::DbTableOp {
                        table,
                        op: HirDbTableOp::Count,
                        args: count_args,
                        select_cols: None,
                        order_by: None,
                        limit: None,
                        plan: Some(plan),
                        span: *span,
                    };
                }
                if method == "filter"
                    && let Some(table) = db_table_handle_name(&obj_hir)
                    && let Some(filter_args) = extract_filter_record_args(&hir_args)
                    && !filter_args.is_empty()
                {
                    let plan = HirDbQueryPlan {
                        table: table.clone(),
                        op: HirDbTableOp::FilterRecord,
                        predicate: Some(HirDbPredicate::And(
                            filter_args
                                .iter()
                                .filter_map(|a| {
                                    Some(HirDbPredicate::Eq {
                                        field: a.name.clone()?,
                                    })
                                })
                                .collect(),
                        )),
                        projection: None,
                        order_by: None,
                        has_limit: false,
                        capabilities: HirDbPlanCapabilities::default(),
                    };
                    return HirExpr::DbTableOp {
                        table,
                        op: HirDbTableOp::FilterRecord,
                        args: filter_args,
                        select_cols: None,
                        order_by: None,
                        limit: None,
                        plan: Some(plan),
                        span: *span,
                    };
                }
                if let Some((table, op)) = db_table_op_from_field(&obj_hir, method.as_str()) {
                    let mut cap = HirDbPlanCapabilities::default();
                    if matches!(op, HirDbTableOp::UnsafeQueryRawClause) {
                        cap.emits_change_log = true;
                    }
                    HirExpr::DbTableOp {
                        table: table.clone(),
                        op,
                        args: hir_args,
                        select_cols: None,
                        order_by: None,
                        limit: None,
                        plan: Some(HirDbQueryPlan {
                            table,
                            op,
                            predicate: None,
                            projection: None,
                            order_by: None,
                            has_limit: false,
                            capabilities: cap,
                        }),
                        span: *span,
                    }
                } else {
                    HirExpr::MethodCall(Box::new(obj_hir), method.clone(), hir_args, *span)
                }
            }
            Expr::FieldAccess {
                object,
                field,
                span,
            } => HirExpr::FieldAccess(Box::new(self.lower_expr(object)), field.clone(), *span),
            Expr::Match {
                subject,
                arms,
                span,
            } => HirExpr::Match(
                Box::new(self.lower_expr(subject)),
                arms.iter()
                    .map(|a| HirMatchArm {
                        pattern: self.lower_pattern(&a.pattern),
                        guard: a.guard.as_ref().map(|g| Box::new(self.lower_expr(g))),
                        body: Box::new(self.lower_expr(&a.body)),
                        span: a.span,
                    })
                    .collect(),
                *span,
            ),
            Expr::If {
                condition,
                then_body,
                else_body,
                span,
            } => HirExpr::If(
                Box::new(self.lower_expr(condition)),
                then_body.iter().map(|s| self.lower_stmt(s)).collect(),
                else_body
                    .as_ref()
                    .map(|stmts| stmts.iter().map(|s| self.lower_stmt(s)).collect()),
                *span,
            ),
            Expr::For {
                binding,
                iterable,
                body,
                span,
            } => HirExpr::For(
                binding.clone(),
                Box::new(self.lower_expr(iterable)),
                Box::new(self.lower_expr(body)),
                *span,
            ),
            Expr::Lambda {
                params,
                return_type,
                body,
                span,
            } => {
                self.def_map.push_scope();
                let hir_params = params.iter().map(|p| self.lower_param(p)).collect();
                let hir_body = self.lower_expr(body);
                self.def_map.pop_scope();
                HirExpr::Lambda(
                    hir_params,
                    return_type.as_ref().map(|t| self.lower_type(t)),
                    Box::new(hir_body),
                    *span,
                )
            }
            Expr::Pipe { left, right, span } => HirExpr::Pipe(
                Box::new(self.lower_expr(left)),
                Box::new(self.lower_expr(right)),
                *span,
            ),
            Expr::Spawn { target, span } => {
                HirExpr::Spawn(Box::new(self.lower_expr(target)), *span)
            }
            Expr::With {
                operand,
                options,
                span,
            } => HirExpr::With(
                Box::new(self.lower_expr(operand)),
                Box::new(self.lower_expr(options)),
                *span,
            ),
            Expr::Jsx(el) => HirExpr::Jsx(HirJsxElement {
                tag: el.tag.clone(),
                attributes: el
                    .attributes
                    .iter()
                    .map(|a| HirJsxAttr {
                        name: a.name.clone(),
                        value: self.lower_expr(&a.value),
                    })
                    .collect(),
                children: el.children.iter().map(|c| self.lower_expr(c)).collect(),
                span: el.span,
            }),
            Expr::JsxSelfClosing(el) => HirExpr::JsxSelfClosing(HirJsxSelfClosing {
                tag: el.tag.clone(),
                attributes: el
                    .attributes
                    .iter()
                    .map(|a| HirJsxAttr {
                        name: a.name.clone(),
                        value: self.lower_expr(&a.value),
                    })
                    .collect(),
                span: el.span,
            }),
            Expr::StringInterp { parts, span } => {
                // Convert string interpolation to template literal-style
                // For now, represent as a string concat
                let mut result_parts = Vec::new();
                for part in parts {
                    match part {
                        expr::StringPart::Literal(s) => {
                            result_parts.push(HirExpr::StringLit(s.clone(), *span));
                        }
                        expr::StringPart::Interpolation(e) => {
                            result_parts.push(self.lower_expr(e));
                        }
                    }
                }
                if result_parts.len() == 1 {
                    result_parts.pop().unwrap()
                } else {
                    // Represent as a concat chain
                    let mut acc = result_parts.remove(0);
                    for part in result_parts {
                        acc = HirExpr::Binary(HirBinOp::Add, Box::new(acc), Box::new(part), *span);
                    }
                    acc
                }
            }
            Expr::Block { stmts, span } => {
                HirExpr::Block(stmts.iter().map(|s| self.lower_stmt(s)).collect(), *span)
            }
        }
    }
}

fn db_table_handle_name(obj: &HirExpr) -> Option<String> {
    let HirExpr::FieldAccess(inner, table_name, _) = obj else {
        return None;
    };
    let HirExpr::Ident(db_name, _) = inner.as_ref() else {
        return None;
    };
    if db_name != "db" {
        return None;
    }
    Some(table_name.clone())
}

fn extract_filter_record_args(args: &[HirArg]) -> Option<Vec<HirArg>> {
    if args.len() != 1 {
        return None;
    }
    let v = &args[0].value;
    let HirExpr::ObjectLit(fields, _) = v else {
        return None;
    };
    Some(
        fields
            .iter()
            .map(|(k, val)| HirArg {
                name: Some(k.clone()),
                value: val.clone(),
            })
            .collect(),
    )
}

fn db_table_op_from_field(obj: &HirExpr, method: &str) -> Option<(String, HirDbTableOp)> {
    let HirExpr::FieldAccess(inner, table_name, _) = obj else {
        return None;
    };
    let HirExpr::Ident(db_name, _) = inner.as_ref() else {
        return None;
    };
    if db_name != "db" {
        return None;
    }
    let op = match method {
        "insert" => HirDbTableOp::Insert,
        "get" | "find" => HirDbTableOp::Get,
        "delete" => HirDbTableOp::Delete,
        "all" => HirDbTableOp::All,
        "count" => HirDbTableOp::Count,
        "query" => HirDbTableOp::UnsafeQueryRawClause,
        _ => return None,
    };
    Some((table_name.clone(), op))
}

fn extract_count_chain_args(ctx: &mut LowerCtx, object: &Expr) -> Option<(String, Vec<HirArg>)> {
    let Expr::MethodCall {
        object: inner_obj,
        method: inner_method,
        args: inner_args,
        ..
    } = object
    else {
        return None;
    };
    let inner_obj_hir = ctx.lower_expr(inner_obj);
    let table = db_table_handle_name(&inner_obj_hir)?;
    match inner_method.as_str() {
        "all" => Some((table, Vec::new())),
        "filter" | "where" => {
            let lowered: Vec<HirArg> = inner_args
                .iter()
                .map(|a| HirArg {
                    name: a.name.clone(),
                    value: ctx.lower_expr(&a.value),
                })
                .collect();
            let filter_args = extract_filter_record_args(&lowered)?;
            if filter_args.is_empty() {
                None
            } else {
                Some((table, filter_args))
            }
        }
        _ => None,
    }
}

struct DbQueryChain {
    table: String,
    op: HirDbTableOp,
    args: Vec<HirArg>,
    predicate: Option<HirDbPredicate>,
    select_cols: Option<Vec<String>>,
    order_by: Option<(String, bool)>,
    limit: Option<HirExpr>,
    capabilities: HirDbPlanCapabilities,
}

fn make_db_plan_from_chain(chain: &DbQueryChain) -> HirDbQueryPlan {
    HirDbQueryPlan {
        table: chain.table.clone(),
        op: chain.op,
        predicate: chain.predicate.clone(),
        projection: chain.select_cols.clone(),
        order_by: chain.order_by.clone(),
        has_limit: chain.limit.is_some(),
        capabilities: chain.capabilities.clone(),
    }
}

fn parse_where_object_predicate(
    obj: &HirExpr,
    out_args: &mut Vec<HirArg>,
) -> Option<HirDbPredicate> {
    let HirExpr::ObjectLit(fields, _) = obj else {
        return None;
    };
    let mut out = Vec::new();
    for (k, v) in fields {
        match k.as_str() {
            "and" => {
                let HirExpr::ListLit(items, _) = v else {
                    return None;
                };
                let mut parts = Vec::new();
                for item in items {
                    parts.push(parse_where_object_predicate(item, out_args)?);
                }
                out.push(HirDbPredicate::And(parts));
            }
            "or" => {
                let HirExpr::ListLit(items, _) = v else {
                    return None;
                };
                let mut parts = Vec::new();
                for item in items {
                    parts.push(parse_where_object_predicate(item, out_args)?);
                }
                out.push(HirDbPredicate::Or(parts));
            }
            "not" => {
                out.push(HirDbPredicate::Not(Box::new(parse_where_object_predicate(
                    v, out_args,
                )?)));
            }
            field => {
                if let HirExpr::ObjectLit(op_fields, _) = v
                    && op_fields.len() == 1
                {
                    let (op_name, op_val) = &op_fields[0];
                    match op_name.as_str() {
                        "eq" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Eq {
                                field: field.to_string(),
                            });
                        }
                        "neq" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Neq {
                                field: field.to_string(),
                            });
                        }
                        "lt" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Lt {
                                field: field.to_string(),
                            });
                        }
                        "lte" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Lte {
                                field: field.to_string(),
                            });
                        }
                        "gt" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Gt {
                                field: field.to_string(),
                            });
                        }
                        "gte" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Gte {
                                field: field.to_string(),
                            });
                        }
                        "contains" => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: op_val.clone(),
                            });
                            out.push(HirDbPredicate::Contains {
                                field: field.to_string(),
                            });
                        }
                        "is_null" => {
                            out.push(HirDbPredicate::IsNull {
                                field: field.to_string(),
                            });
                        }
                        "in" => {
                            let HirExpr::ListLit(items, _) = op_val else {
                                return None;
                            };
                            let arity = items.len();
                            if arity == 0 {
                                return None;
                            }
                            for item in items {
                                out_args.push(HirArg {
                                    name: Some(field.to_string()),
                                    value: item.clone(),
                                });
                            }
                            out.push(HirDbPredicate::In {
                                field: field.to_string(),
                                arity,
                            });
                        }
                        _ => {
                            out_args.push(HirArg {
                                name: Some(field.to_string()),
                                value: v.clone(),
                            });
                            out.push(HirDbPredicate::Eq {
                                field: field.to_string(),
                            });
                        }
                    }
                    continue;
                }
                out_args.push(HirArg {
                    name: Some(field.to_string()),
                    value: v.clone(),
                });
                out.push(HirDbPredicate::Eq {
                    field: field.to_string(),
                });
            }
        }
    }
    if out.len() == 1 {
        out.into_iter().next()
    } else {
        Some(HirDbPredicate::And(out))
    }
}

fn parse_select_columns(args: &[expr::Arg]) -> Option<Vec<String>> {
    if args.is_empty() {
        return None;
    }
    let mut out = Vec::new();
    if args.len() == 1 {
        match &args[0].value {
            Expr::ListLit { elements, .. } => {
                for e in elements {
                    match e {
                        Expr::StringLit { value, .. } => out.push(value.clone()),
                        Expr::Ident { name, .. } => out.push(name.clone()),
                        _ => return None,
                    }
                }
            }
            Expr::StringLit { value, .. } => out.push(value.clone()),
            Expr::Ident { name, .. } => out.push(name.clone()),
            _ => return None,
        }
    } else {
        for a in args {
            match &a.value {
                Expr::StringLit { value, .. } => out.push(value.clone()),
                Expr::Ident { name, .. } => out.push(name.clone()),
                _ => return None,
            }
        }
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

fn extract_db_query_chain(ctx: &mut LowerCtx, expr: &Expr) -> Option<DbQueryChain> {
    let Expr::MethodCall {
        object,
        method,
        args,
        ..
    } = expr
    else {
        return None;
    };

    if let Some(mut chain) = extract_db_query_chain(ctx, object) {
        match method.as_str() {
            "order_by" => {
                if args.is_empty() || args.len() > 2 {
                    return None;
                }
                let col = match &args[0].value {
                    Expr::StringLit { value, .. } => value.clone(),
                    Expr::Ident { name, .. } => name.clone(),
                    _ => return None,
                };
                let mut asc = true;
                if args.len() == 2 {
                    asc = match &args[1].value {
                        Expr::StringLit { value, .. } => !value.eq_ignore_ascii_case("desc"),
                        Expr::BoolLit { value, .. } => *value,
                        _ => return None,
                    };
                }
                chain.order_by = Some((col, asc));
                return Some(chain);
            }
            "limit" => {
                if args.len() != 1 {
                    return None;
                }
                chain.limit = Some(ctx.lower_expr(&args[0].value));
                return Some(chain);
            }
            "select" => {
                if chain.select_cols.is_some() {
                    return None;
                }
                let cols = parse_select_columns(args)?;
                chain.select_cols = Some(cols);
                return Some(chain);
            }
            "sync" => {
                if !args.is_empty() {
                    return None;
                }
                chain.capabilities.requires_sync = true;
                return Some(chain);
            }
            "using" => {
                if args.len() != 1 {
                    return None;
                }
                let mode = match &args[0].value {
                    Expr::StringLit { value, .. } => value.to_ascii_lowercase(),
                    Expr::Ident { name, .. } => name.to_ascii_lowercase(),
                    _ => return None,
                };
                chain.capabilities.retrieval_mode = match mode.as_str() {
                    "fts" | "search" => Some(HirDbRetrievalMode::Fts),
                    "vector" => Some(HirDbRetrievalMode::Vector),
                    "hybrid" => Some(HirDbRetrievalMode::Hybrid),
                    _ => None,
                };
                return Some(chain);
            }
            "live" => {
                if args.len() != 1 {
                    return None;
                }
                let topic = match &args[0].value {
                    Expr::StringLit { value, .. } => value.clone(),
                    Expr::Ident { name, .. } => name.clone(),
                    _ => return None,
                };
                chain.capabilities.emits_change_log = true;
                chain.capabilities.live_topic = Some(topic);
                return Some(chain);
            }
            "scope" => {
                if args.len() != 1 {
                    return None;
                }
                let scope = match &args[0].value {
                    Expr::StringLit { value, .. } => value.clone(),
                    Expr::Ident { name, .. } => name.clone(),
                    _ => return None,
                };
                chain.capabilities.orchestration_scope = Some(scope);
                return Some(chain);
            }
            _ => return None,
        }
    }

    let obj_hir = ctx.lower_expr(object);
    let table = db_table_handle_name(&obj_hir)?;
    match method.as_str() {
        "all" if args.is_empty() => Some(DbQueryChain {
            table,
            op: HirDbTableOp::All,
            args: Vec::new(),
            predicate: None,
            select_cols: None,
            order_by: None,
            limit: None,
            capabilities: HirDbPlanCapabilities::default(),
        }),
        "filter" => {
            let lowered: Vec<HirArg> = args
                .iter()
                .map(|a| HirArg {
                    name: a.name.clone(),
                    value: ctx.lower_expr(&a.value),
                })
                .collect();
            let filter_args = extract_filter_record_args(&lowered)?;
            if filter_args.is_empty() {
                return None;
            }
            Some(DbQueryChain {
                table,
                op: HirDbTableOp::FilterRecord,
                args: filter_args.clone(),
                predicate: Some(HirDbPredicate::And(
                    filter_args
                        .iter()
                        .filter_map(|a| {
                            let k = a.name.clone()?;
                            Some(HirDbPredicate::Eq { field: k })
                        })
                        .collect(),
                )),
                select_cols: None,
                order_by: None,
                limit: None,
                capabilities: HirDbPlanCapabilities::default(),
            })
        }
        "where" => {
            if args.len() != 1 {
                return None;
            }
            let lowered = ctx.lower_expr(&args[0].value);
            let mut filter_args = Vec::new();
            let predicate = parse_where_object_predicate(&lowered, &mut filter_args)?;
            if filter_args.is_empty() && !matches!(predicate, HirDbPredicate::IsNull { .. }) {
                return None;
            }
            Some(DbQueryChain {
                table,
                op: HirDbTableOp::FilterRecord,
                args: filter_args,
                predicate: Some(predicate),
                select_cols: None,
                order_by: None,
                limit: None,
                capabilities: HirDbPlanCapabilities::default(),
            })
        }
        _ => None,
    }
}
