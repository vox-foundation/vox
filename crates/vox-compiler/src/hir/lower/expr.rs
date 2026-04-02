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
                if let Some(chain) = super::expr_db::extract_db_query_chain(self, e) {
                    let plan = super::expr_db::make_db_plan_from_chain(&chain);
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
                    && let Some((table, count_args)) =
                        super::expr_db::extract_count_chain_args(self, object)
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
                    && let Some(table) = super::expr_db::db_table_handle_name(&obj_hir)
                    && let Some(filter_args) = super::expr_db::extract_filter_record_args(&hir_args)
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
                if let Some((table, op)) =
                    super::expr_db::db_table_op_from_field(&obj_hir, method.as_str())
                {
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
            Expr::Try { target, span } => HirExpr::Try(HirTry {
                target: Box::new(self.lower_expr(target)),
                span: *span,
            }),
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
