use std::collections::HashSet;

use crate::ast::span::Span;
use crate::hir::*;
use crate::typeck::diagnostics::{Diagnostic, DiagnosticCategory, TypeckSeverity};
use crate::typeck::env::{Binding, BindingKind};
use crate::typeck::registration::resolve_hir_type;
use crate::typeck::ty::Ty;

use super::Checker;
use super::match_exhaust::check_hir_match_exhaustiveness;

impl<'a> Checker<'a> {
    fn check_db_select_projection(
        &mut self,
        fields: &[(String, Ty)],
        cols: &[String],
        span: Span,
    ) -> bool {
        let mut ok = true;
        if cols.is_empty() {
            self.diags.push(Diagnostic::error(
                "db select(...) requires at least one column name".into(),
                span,
                self.source,
            ));
            return false;
        }
        let mut seen = HashSet::new();
        for c in cols {
            if !seen.insert(c.as_str()) {
                self.diags.push(Diagnostic::error(
                    format!("duplicate column '{c}' in select(...)"),
                    span,
                    self.source,
                ));
                ok = false;
            }
            if fields.iter().all(|(n, _)| n != c) {
                self.diags.push(Diagnostic::error(
                    format!("Unknown field '{c}' in select(...)"),
                    span,
                    self.source,
                ));
                ok = false;
            }
        }
        if !ok {
            return false;
        }
        ok
    }

    fn validate_db_predicate(
        &mut self,
        pred: &HirDbPredicate,
        fields: &[(String, Ty)],
        args: &[HirArg],
        arg_ix: &mut usize,
        table: &str,
        span: Span,
    ) -> bool {
        let expect_value = |field: &str| -> Option<Ty> {
            let (_, f_ty) = fields.iter().find(|(n, _)| n == field).cloned()?;
            Some(f_ty)
        };
        match pred {
            HirDbPredicate::Eq { field }
            | HirDbPredicate::Neq { field }
            | HirDbPredicate::Lt { field }
            | HirDbPredicate::Lte { field }
            | HirDbPredicate::Gt { field }
            | HirDbPredicate::Gte { field }
            | HirDbPredicate::Contains { field } => {
                let Some(f_ty) = expect_value(field) else {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown field '{field}' on table '{table}' for predicate"),
                        span,
                        self.source,
                    ));
                    return false;
                };
                if *arg_ix >= args.len() {
                    self.diags.push(Diagnostic::error(
                        "db predicate/value mismatch (internal arg ordering error)".into(),
                        span,
                        self.source,
                    ));
                    return false;
                }
                let v_ty = self.check_expr(&args[*arg_ix].value);
                *arg_ix += 1;
                let _ = self.uf.unify(&f_ty, &v_ty);
                true
            }
            HirDbPredicate::IsNull { field } => {
                if expect_value(field).is_none() {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown field '{field}' on table '{table}' for predicate"),
                        span,
                        self.source,
                    ));
                    return false;
                }
                true
            }
            HirDbPredicate::In { field, arity } => {
                let Some(f_ty) = expect_value(field) else {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown field '{field}' on table '{table}' for predicate"),
                        span,
                        self.source,
                    ));
                    return false;
                };
                if *arity == 0 {
                    self.diags.push(Diagnostic::error(
                        "db where(...): `in` must have at least one value".into(),
                        span,
                        self.source,
                    ));
                    return false;
                }
                for _ in 0..*arity {
                    if *arg_ix >= args.len() {
                        self.diags.push(Diagnostic::error(
                            "db predicate/value mismatch (internal arg ordering error)".into(),
                            span,
                            self.source,
                        ));
                        return false;
                    }
                    let v_ty = self.check_expr(&args[*arg_ix].value);
                    *arg_ix += 1;
                    let _ = self.uf.unify(&f_ty, &v_ty);
                }
                true
            }
            HirDbPredicate::And(parts) | HirDbPredicate::Or(parts) => parts
                .iter()
                .all(|p| self.validate_db_predicate(p, fields, args, arg_ix, table, span)),
            HirDbPredicate::Not(inner) => {
                self.validate_db_predicate(inner, fields, args, arg_ix, table, span)
            }
        }
    }
    pub fn check_expr(&mut self, expr: &HirExpr) -> Ty {
        match expr {
            HirExpr::IntLit(_, _) => Ty::Int,
            HirExpr::FloatLit(_, _) => Ty::Float,
            HirExpr::StringLit(_, _) => Ty::Str,
            HirExpr::BoolLit(_, _) => Ty::Bool,
            HirExpr::TupleLit(exprs, _) => {
                let tys = exprs.iter().map(|e| self.check_expr(e)).collect();
                Ty::Tuple(tys)
            }

            HirExpr::Ident(name, span) => {
                if let Some(binding) = self.env.lookup(name) {
                    if binding.is_deprecated {
                        self.diags.push(Diagnostic {
                            severity: TypeckSeverity::Warning,
                            message: format!("'{name}' is deprecated"),
                            span: *span,
                            expected_type: None,
                            found_type: None,
                            context: Some(Diagnostic::capture_context(self.source, *span)),
                            suggestions: vec![],
                            category: DiagnosticCategory::Typecheck,
                            code: Some("typecheck.deprecated_ident".into()),
                            fixes: vec![],
                        });
                    }
                    binding.ty.clone()
                } else if let Some(ty) = self.builtins.lookup_var(name) {
                    ty
                } else {
                    self.diags.push(Diagnostic::error(
                        format!("Undefined variable: {name}"),
                        *span,
                        self.source,
                    ));
                    Ty::Error
                }
            }

            HirExpr::ObjectLit(fields, _span) => {
                let typed_fields: Vec<(String, Ty)> = fields
                    .iter()
                    .map(|(name, expr)| (name.clone(), self.check_expr(expr)))
                    .collect();
                Ty::Record(typed_fields)
            }
            HirExpr::ListLit(elements, _span) => {
                let elem_ty = if elements.is_empty() {
                    self.uf.fresh_var()
                } else {
                    let first = self.check_expr(&elements[0]);
                    for e in &elements[1..] {
                        let t = self.check_expr(e);
                        let _ = self.uf.unify(&first, &t);
                    }
                    first
                };
                Ty::List(Box::new(elem_ty))
            }

            HirExpr::Binary(op, left, right, span) => {
                let l_ty = self.check_expr(left);
                let r_ty = self.check_expr(right);
                self.check_binary_op(*op, l_ty, r_ty, *span)
            }
            HirExpr::Unary(op, operand, _span) => {
                let ty = self.check_expr(operand);
                self.check_unary_op(*op, ty)
            }

            HirExpr::Call(callee, args, _tail, span) => {
                let raw_callee = self.check_expr(callee);
                let callee_ty = self.uf.resolve(&raw_callee);
                let callee_ty = self.uf.instantiate(&callee_ty);
                match callee_ty {
                    Ty::Fn(params, ret) => {
                        self.check_arguments(&params, args, *span);
                        ret.as_ref().clone()
                    }
                    Ty::Error => Ty::Error,
                    _ => {
                        self.diags.push(Diagnostic::error(
                            format!("Not a function: {callee_ty:?}"),
                            *span,
                            self.source,
                        ));
                        Ty::Error
                    }
                }
            }
            HirExpr::MethodCall(object, method, args, span) => {
                if matches!(object.as_ref(), HirExpr::DbTableOp { .. })
                    && matches!(method.as_str(), "limit" | "order_by")
                {
                    self.diags.push(Diagnostic::error(
                        format!(
                            "db query chaining via '.{method}(...)' is not supported yet; use typed db.Table operations directly"
                        ),
                        *span,
                        self.source,
                    ));
                    return Ty::Error;
                }
                let obj_ty = self.check_expr(object);
                let obj_ty = self.uf.resolve(&obj_ty);
                if let Some(method_ty) = self.builtins.lookup_method(&obj_ty, method) {
                    let method_ty = self.uf.instantiate(&method_ty);
                    if let Ty::Fn(params, ret) = method_ty {
                        self.check_arguments(&params, args, *span);
                        ret.as_ref().clone()
                    } else {
                        Ty::Error
                    }
                } else if let Ty::ActorRef(actor_name) = &obj_ty {
                    if let Some(handlers) = self.env.lookup_actor(actor_name) {
                        if let Some(h) = handlers.iter().find(|h| h.event_name == *method) {
                            let params: Vec<Ty> = h.params.iter().map(|(_, t)| t.clone()).collect();
                            let method_ty = self
                                .uf
                                .instantiate(&Ty::Fn(params, Box::new(h.return_type.clone())));
                            if let Ty::Fn(params, ret) = method_ty {
                                self.check_arguments(&params, args, *span);
                                ret.as_ref().clone()
                            } else {
                                Ty::Error
                            }
                        } else {
                            self.diags.push(Diagnostic::error(
                                format!("Method '{method}' not found on actor '{actor_name}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    } else {
                        self.diags.push(Diagnostic::error(
                            format!("Unknown actor '{actor_name}' for method call"),
                            *span,
                            self.source,
                        ));
                        Ty::Error
                    }
                } else {
                    self.diags.push(Diagnostic::error(
                        format!("Method '{method}' not found on {obj_ty:?}"),
                        *span,
                        self.source,
                    ));
                    Ty::Error
                }
            }

            HirExpr::DbTableOp {
                table,
                op,
                args,
                select_cols,
                order_by,
                limit,
                plan,
                span,
                ..
            } => {
                if matches!(op, HirDbTableOp::UnsafeQueryRawClause) {
                    self.diags.push(Diagnostic {
                        severity: TypeckSeverity::Error,
                        message:
                            "`.query(clause)` builds dynamic SQL; prefer `.all()` or `.get(id)`. \
                                  (This IR maps to `unsafe_query_raw_clause` in generated Rust.)"
                                .into(),
                        span: *span,
                        expected_type: None,
                        found_type: None,
                        context: Some(Diagnostic::capture_context(self.source, *span)),
                        suggestions: vec![
                            "Use `db.Table.all()` for full scans.".into(),
                            "Use `db.Table.get(id)` or `db.Table.find(id)` for primary-key reads."
                                .into(),
                        ],
                        category: DiagnosticCategory::Lint,
                        code: Some("lint.db_unsafe_query".into()),
                        fixes: vec![],
                    });
                }
                let Some(binding) = self.env.lookup(table) else {
                    self.diags.push(Diagnostic::error(
                        format!("Unknown table '{table}' for db operation"),
                        *span,
                        self.source,
                    ));
                    return Ty::Error;
                };
                let obj_ty = binding.ty.clone();
                if let Some(limit_expr) = limit {
                    let lim_ty = self.check_expr(limit_expr.as_ref());
                    let _ = self.uf.unify(&lim_ty, &Ty::Int);
                }
                if let Some(cols) = select_cols.as_ref() {
                    if matches!(*op, HirDbTableOp::Count) {
                        self.diags.push(Diagnostic::error(
                            ".select(...) cannot be combined with `.count()`; use `.count()` on the table or filter without narrowing columns"
                                .into(),
                            *span,
                            self.source,
                        ));
                        return Ty::Error;
                    }
                    if !matches!(*op, HirDbTableOp::All | HirDbTableOp::FilterRecord) {
                        self.diags.push(Diagnostic::error(
                            ".select(...) is only supported on db.Table.all() and db.Table.filter(...)"
                                .into(),
                            *span,
                            self.source,
                        ));
                        return Ty::Error;
                    }
                    let Ty::Table(_, fields) = &obj_ty else {
                        self.diags.push(Diagnostic::error(
                            format!("Expected table type for db select on '{table}'"),
                            *span,
                            self.source,
                        ));
                        return Ty::Error;
                    };
                    if !self.check_db_select_projection(fields, cols, *span) {
                        return Ty::Error;
                    }
                }
                match *op {
                    HirDbTableOp::FilterRecord => {
                        let Ty::Table(_, fields) = &obj_ty else {
                            self.diags.push(Diagnostic::error(
                                format!("Expected table type for db filter on '{table}'"),
                                *span,
                                self.source,
                            ));
                            return Ty::Error;
                        };
                        if args.is_empty()
                            && plan.as_ref().and_then(|p| p.predicate.as_ref()).is_none()
                        {
                            self.diags.push(Diagnostic::error(
                                "db.Table.filter({ ... }) requires at least one field predicate"
                                    .into(),
                                *span,
                                self.source,
                            ));
                            return Ty::Error;
                        }
                        if let Some(pred) = plan.as_ref().and_then(|p| p.predicate.as_ref()) {
                            let mut arg_ix = 0usize;
                            if !self.validate_db_predicate(
                                pred,
                                fields,
                                args,
                                &mut arg_ix,
                                table,
                                *span,
                            ) {
                                return Ty::Error;
                            }
                        } else {
                            for arg in args {
                                let Some(col) = arg.name.as_deref() else {
                                    self.diags.push(Diagnostic::error(
                                        "filter object keys must be field names (internal error)"
                                            .into(),
                                        *span,
                                        self.source,
                                    ));
                                    return Ty::Error;
                                };
                                let Some((_, f_ty)) = fields.iter().find(|(n, _)| n == col) else {
                                    self.diags.push(Diagnostic::error(
                                        format!(
                                            "Unknown field '{col}' on table '{table}' for filter"
                                        ),
                                        *span,
                                        self.source,
                                    ));
                                    return Ty::Error;
                                };
                                let v_ty = self.check_expr(&arg.value);
                                let _ = self.uf.unify(f_ty, &v_ty);
                            }
                        }
                        if let Some((col, _)) = order_by {
                            if fields.iter().all(|(n, _)| n != col) {
                                self.diags.push(Diagnostic::error(
                                    format!(
                                        "Unknown field '{col}' on table '{table}' for order_by"
                                    ),
                                    *span,
                                    self.source,
                                ));
                                return Ty::Error;
                            }
                        }
                        let record_ty = Ty::Record(fields.clone());
                        Ty::Result(Box::new(Ty::List(Box::new(record_ty))))
                    }
                    HirDbTableOp::Count => {
                        if order_by.is_some() || limit.is_some() {
                            self.diags.push(Diagnostic::error(
                                "db.Table.count() does not support order_by/limit modifiers".into(),
                                *span,
                                self.source,
                            ));
                            return Ty::Error;
                        }
                        if !args.is_empty()
                            || plan.as_ref().and_then(|p| p.predicate.as_ref()).is_some()
                        {
                            let Ty::Table(_, fields) = &obj_ty else {
                                self.diags.push(Diagnostic::error(
                                    format!("Expected table type for db count on '{table}'"),
                                    *span,
                                    self.source,
                                ));
                                return Ty::Error;
                            };
                            if let Some(pred) = plan.as_ref().and_then(|p| p.predicate.as_ref()) {
                                let mut arg_ix = 0usize;
                                if !self.validate_db_predicate(
                                    pred,
                                    fields,
                                    args,
                                    &mut arg_ix,
                                    table,
                                    *span,
                                ) {
                                    return Ty::Error;
                                }
                            } else {
                                for arg in args {
                                    let Some(col) = arg.name.as_deref() else {
                                        self.diags.push(Diagnostic::error(
                                            "count filter keys must be field names (internal error)"
                                                .into(),
                                            *span,
                                            self.source,
                                        ));
                                        return Ty::Error;
                                    };
                                    let Some((_, f_ty)) = fields.iter().find(|(n, _)| n == col)
                                    else {
                                        self.diags.push(Diagnostic::error(
                                            format!(
                                                "Unknown field '{col}' on table '{table}' for count filter"
                                            ),
                                            *span,
                                            self.source,
                                        ));
                                        return Ty::Error;
                                    };
                                    let v_ty = self.check_expr(&arg.value);
                                    let _ = self.uf.unify(f_ty, &v_ty);
                                }
                            }
                        }
                        Ty::Result(Box::new(Ty::Int))
                    }
                    HirDbTableOp::Insert
                    | HirDbTableOp::Get
                    | HirDbTableOp::Delete
                    | HirDbTableOp::All
                    | HirDbTableOp::UnsafeQueryRawClause => {
                        if matches!(
                            op,
                            HirDbTableOp::Insert
                                | HirDbTableOp::Get
                                | HirDbTableOp::Delete
                                | HirDbTableOp::UnsafeQueryRawClause
                        ) && (order_by.is_some() || limit.is_some())
                        {
                            self.diags.push(Diagnostic::error(
                                "order_by/limit modifiers are only supported on db.Table.all()/filter(...)"
                                    .into(),
                                *span,
                                self.source,
                            ));
                            return Ty::Error;
                        }
                        if let Some((col, _)) = order_by {
                            if let Ty::Table(_, fields) = &obj_ty {
                                if fields.iter().all(|(n, _)| n != col) {
                                    self.diags.push(Diagnostic::error(
                                        format!(
                                            "Unknown field '{col}' on table '{table}' for order_by"
                                        ),
                                        *span,
                                        self.source,
                                    ));
                                    return Ty::Error;
                                }
                            }
                        }
                        let method = match op {
                            HirDbTableOp::Insert => "insert",
                            HirDbTableOp::Get => "get",
                            HirDbTableOp::Delete => "delete",
                            HirDbTableOp::All => "all",
                            HirDbTableOp::UnsafeQueryRawClause => "query",
                            HirDbTableOp::FilterRecord | HirDbTableOp::Count => {
                                unreachable!("handled in earlier match arms")
                            }
                        };
                        if let Some(method_ty) = self.builtins.lookup_method(&obj_ty, method) {
                            let method_ty = self.uf.instantiate(&method_ty);
                            if let Ty::Fn(params, ret) = method_ty {
                                self.check_arguments(&params, args, *span);
                                ret.as_ref().clone()
                            } else {
                                Ty::Error
                            }
                        } else {
                            self.diags.push(Diagnostic::error(
                                format!("Invalid db operation '{method}' on table '{table}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    }
                }
            }

            HirExpr::FieldAccess(object, field, span) => {
                self.check_expr_field_access(object, field.as_str(), *span)
            }

            HirExpr::Match(subject, arms, span) => {
                let sub_ty = self.check_expr(subject);
                let sub_resolved = self.uf.resolve(&sub_ty);
                check_hir_match_exhaustiveness(
                    self.env,
                    self.diags,
                    &sub_resolved,
                    arms,
                    *span,
                    self.source,
                );
                let ret_ty = self.uf.fresh_var();
                for arm in arms {
                    self.env.push_scope();
                    self.bind_pattern(&arm.pattern, &sub_ty, false);
                    if let Some(guard) = &arm.guard {
                        let guard_ty = self.check_expr(guard);
                        let _ = self.uf.unify(&guard_ty, &Ty::Bool);
                    }
                    let arm_ty = self.check_expr(arm.body.as_ref());
                    let _ = self.uf.unify(&ret_ty, &arm_ty);
                    self.env.pop_scope();
                }
                ret_ty
            }

            HirExpr::If(cond, then_body, else_body, _span) => {
                let cond_ty = self.check_expr(cond);
                let _ = self.uf.unify(&cond_ty, &Ty::Bool);
                self.env.push_scope();
                let mut then_ty = Ty::Unit;
                for stmt in then_body {
                    then_ty = self.check_stmt(stmt);
                }
                self.env.pop_scope();
                if let Some(eb) = else_body {
                    self.env.push_scope();
                    let mut else_ty = Ty::Unit;
                    for stmt in eb {
                        else_ty = self.check_stmt(stmt);
                    }
                    self.env.pop_scope();
                    let _ = self.uf.unify(&then_ty, &else_ty);
                }
                then_ty
            }

            HirExpr::Block(stmts, _span) => {
                self.env.push_scope();
                let mut last_ty = Ty::Unit;
                for stmt in stmts {
                    last_ty = self.check_stmt(stmt);
                }
                self.env.pop_scope();
                last_ty
            }

            HirExpr::For(binding, iterable, body, _span) => {
                let iter_ty = self.check_expr(iterable);
                let element_ty = self.extract_iterable_element(&iter_ty);
                self.env.push_scope();
                self.bind_pattern(
                    &HirPattern::Ident(binding.clone(), Span::new(0, 0)),
                    &element_ty,
                    false,
                );
                let _ = self.check_expr(body);
                self.env.pop_scope();
                Ty::Unit
            }

            HirExpr::Lambda(params, ret_ann, body, _span) => {
                self.env.push_scope();
                let param_tys: Vec<Ty> = params
                    .iter()
                    .map(|p| {
                        let p_ty = p
                            .type_ann
                            .as_ref()
                            .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
                        self.env.define(
                            p.name.clone(),
                            Binding::new(p_ty.clone(), false, BindingKind::Parameter),
                        );
                        p_ty
                    })
                    .collect();
                let ret_ty = ret_ann
                    .as_ref()
                    .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
                self.env.push_return_type(ret_ty.clone());
                let body_ty = self.check_expr(body.as_ref());
                let _ = self.uf.unify(&body_ty, &ret_ty);
                self.env.pop_return_type();
                self.env.pop_scope();
                Ty::Fn(param_tys, Box::new(ret_ty))
            }

            HirExpr::Pipe(left, right, _span) => {
                let l_ty = self.check_expr(left);
                let raw_right = self.check_expr(right);
                let r_ty = self.uf.resolve(&raw_right);
                match r_ty {
                    Ty::Fn(params, ret) => {
                        if let Some(first) = params.first() {
                            let _ = self.uf.unify(&l_ty, first);
                        }
                        ret.as_ref().clone()
                    }
                    _ => l_ty,
                }
            }

            HirExpr::Spawn(inner, span) => {
                let inner_ty = self.check_expr(inner);
                let actor_name = match inner.as_ref() {
                    HirExpr::Ident(name, _) => name.clone(),
                    _ => {
                        self.diags.push(Diagnostic::error(
                            "spawn target must be a bare actor name (identifier)".into(),
                            *span,
                            self.source,
                        ));
                        return Ty::Error;
                    }
                };
                if self.uf.resolve(&inner_ty) == Ty::Error {
                    return Ty::Error;
                }
                if !self
                    .env
                    .lookup(&actor_name)
                    .is_some_and(|b| b.kind == BindingKind::Actor)
                {
                    self.diags.push(Diagnostic::error(
                        format!("'{actor_name}' is not an actor"),
                        *span,
                        self.source,
                    ));
                    return Ty::Error;
                }
                if self.env.lookup_actor(&actor_name).is_none() {
                    self.diags.push(Diagnostic::error(
                        format!(
                            "actor '{actor_name}' has no handler signatures in this module (declare `actor {actor_name}:`)"
                        ),
                        *span,
                        self.source,
                    ));
                    return Ty::Error;
                }
                Ty::ActorRef(actor_name)
            }

            HirExpr::With(resource, options, span) => {
                let res_ty = self.check_expr(resource);
                self.check_with_options(options.as_ref(), *span);
                let expected = Ty::Result(Box::new(self.uf.fresh_var()));
                if let Err(msg) = self.uf.unify(&res_ty, &expected) {
                    self.diags.push(Diagnostic::error(
                        format!(
                            "'with' operand must have type Result[T]; found incompatible type ({:?}): {msg}",
                            self.uf.resolve(&res_ty)
                        ),
                        *span,
                        self.source,
                    ));
                    Ty::Error
                } else {
                    self.uf.resolve(&expected)
                }
            }

            HirExpr::Jsx(el) => {
                for attr in &el.attributes {
                    let _ = self.check_expr(&attr.value);
                }
                for child in &el.children {
                    let _ = self.check_expr(child);
                }
                Ty::Element
            }
            HirExpr::JsxSelfClosing(el) => {
                for attr in &el.attributes {
                    let _ = self.check_expr(&attr.value);
                }
                Ty::Element
            }
        }
    }
}
