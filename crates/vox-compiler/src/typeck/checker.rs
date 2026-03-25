//! HIR type Checker aligned with `crate::hir::hir` (current lowered AST).

use crate::ast::span::Span;
use crate::hir::hir::*;
use crate::typeck::builtins::BuiltinTypes;
use crate::typeck::diagnostics::{Diagnostic, Severity};
use crate::typeck::env::{Binding, BindingKind, TypeEnv};
use crate::typeck::registration::{register_hir_module, resolve_hir_type};
use crate::typeck::ty::Ty;
use crate::typeck::unify::InferenceContext;

pub struct Checker<'a> {
    pub env: &'a mut TypeEnv,
    pub builtins: &'a BuiltinTypes,
    pub uf: &'a mut InferenceContext,
    pub diags: &'a mut Vec<Diagnostic>,
    pub source: &'a str,
}

fn hir_expr_span(expr: &HirExpr) -> Span {
    match expr {
        HirExpr::IntLit(_, s)
        | HirExpr::FloatLit(_, s)
        | HirExpr::StringLit(_, s)
        | HirExpr::BoolLit(_, s)
        | HirExpr::Ident(_, s)
        | HirExpr::ObjectLit(_, s)
        | HirExpr::ListLit(_, s)
        | HirExpr::TupleLit(_, s)
        | HirExpr::Binary(_, _, _, s)
        | HirExpr::Unary(_, _, s)
        | HirExpr::Call(_, _, _, s)
        | HirExpr::MethodCall(_, _, _, s)
        | HirExpr::FieldAccess(_, _, s)
        | HirExpr::Match(_, _, s)
        | HirExpr::If(_, _, _, s)
        | HirExpr::For(_, _, _, s)
        | HirExpr::Lambda(_, _, _, s)
        | HirExpr::Pipe(_, _, s)
        | HirExpr::Spawn(_, s)
        | HirExpr::With(_, _, s)
        | HirExpr::Block(_, s) => *s,
        HirExpr::Jsx(el) => el.span,
        HirExpr::JsxSelfClosing(el) => el.span,
    }
}

impl<'a> Checker<'a> {
    pub fn new(
        env: &'a mut TypeEnv,
        builtins: &'a BuiltinTypes,
        uf: &'a mut InferenceContext,
        diags: &'a mut Vec<Diagnostic>,
        source: &'a str,
    ) -> Self {
        Self {
            env,
            builtins,
            uf,
            diags,
            source,
        }
    }

    pub fn check_module(&mut self, module: &HirModule) {
        register_hir_module(self.env, module);

        for f in &module.functions {
            self.check_function(f);
        }
        for a in &module.actors {
            self.check_actor(a);
        }
        for w in &module.workflows {
            self.check_workflow(w);
        }
        for act in &module.activities {
            self.check_activity(act);
        }
        for sf in &module.server_fns {
            self.check_server_fn(sf);
        }
        for t in &module.tests {
            self.check_function(t);
        }
        for t in &module.mcp_tools {
            self.check_function(&t.func);
        }
        for r in &module.routes {
            self.check_route(r);
        }
    }

    fn check_function(&mut self, f: &HirFn) {
        let ret_ty = f
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());

        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );

        for p in &f.params {
            let p_ty = p
                .type_ann
                .as_ref()
                .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
            self.env.define(
                p.name.clone(),
                Binding::new(p_ty, false, BindingKind::Parameter),
            );
        }

        let mut last_ty = Ty::Unit;
        for stmt in &f.body {
            last_ty = self.check_stmt(stmt);
        }
        let _ = self.uf.unify(&last_ty, &ret_ty);

        self.env.pop_return_type();
        self.env.pop_scope();
    }

    fn check_actor(&mut self, a: &HirActor) {
        for h in &a.handlers {
            self.check_actor_handler(h);
        }
    }

    fn check_actor_handler(&mut self, h: &HirActorHandler) {
        let ret_ty = h
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());

        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );

        for p in &h.params {
            let p_ty = p
                .type_ann
                .as_ref()
                .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
            self.env.define(
                p.name.clone(),
                Binding::new(p_ty, false, BindingKind::Parameter),
            );
        }
        for stmt in &h.body {
            let _ = self.check_stmt(stmt);
        }
        self.env.pop_return_type();
        self.env.pop_scope();
    }

    fn check_workflow(&mut self, w: &HirWorkflow) {
        let ret_ty = w
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());

        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );

        for p in &w.params {
            let p_ty = p
                .type_ann
                .as_ref()
                .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
            self.env.define(
                p.name.clone(),
                Binding::new(p_ty, false, BindingKind::Parameter),
            );
        }
        for stmt in &w.body {
            let _ = self.check_stmt(stmt);
        }
        self.env.pop_return_type();
        self.env.pop_scope();
    }

    fn check_activity(&mut self, a: &HirActivity) {
        if a.return_type.is_none() {
            self.diags.push(Diagnostic::warning(
                "Activity should have an explicit return type (typically Result[T])".into(),
                a.span,
                self.source,
            ));
        }
        let ret_ty = a
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        if let Some(rt) = &a.return_type {
            let declared = resolve_hir_type(rt, self.env);
            if !matches!(declared, Ty::Result(_)) {
                self.diags.push(Diagnostic::error(
                    "Activity must return a Result type (e.g. Result[str])".into(),
                    a.span,
                    self.source,
                ));
            }
        }
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());

        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );

        for p in &a.params {
            let p_ty = p
                .type_ann
                .as_ref()
                .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
            self.env.define(
                p.name.clone(),
                Binding::new(p_ty, false, BindingKind::Parameter),
            );
        }
        for stmt in &a.body {
            let _ = self.check_stmt(stmt);
        }
        self.env.pop_return_type();
        self.env.pop_scope();
    }

    fn check_server_fn(&mut self, sf: &HirServerFn) {
        let ret_ty = sf
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());

        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );

        for p in &sf.params {
            let p_ty = p
                .type_ann
                .as_ref()
                .map_or(self.uf.fresh_var(), |t| resolve_hir_type(t, self.env));
            self.env.define(
                p.name.clone(),
                Binding::new(p_ty, false, BindingKind::Parameter),
            );
        }
        for stmt in &sf.body {
            let _ = self.check_stmt(stmt);
        }
        self.env.pop_return_type();
        self.env.pop_scope();
    }

    fn check_route(&mut self, r: &HirRoute) {
        let ret_ty = r
            .return_type
            .as_ref()
            .map_or(Ty::Unit, |t| resolve_hir_type(t, self.env));
        self.env.push_scope();
        self.env.push_return_type(ret_ty.clone());
        // Align with AST `check::typecheck_module` HTTP scope: `request` + `db`.
        self.env.define(
            "request".into(),
            Binding::new(Ty::Named("Request".into()), false, BindingKind::Variable),
        );
        self.env.define(
            "db".into(),
            Binding::new(Ty::Database, false, BindingKind::Variable),
        );
        for stmt in &r.body {
            let _ = self.check_stmt(stmt);
        }
        self.env.pop_return_type();
        self.env.pop_scope();
    }

    pub fn check_stmt(&mut self, stmt: &HirStmt) -> Ty {
        match stmt {
            HirStmt::Let {
                pattern,
                type_ann,
                value,
                mutable,
                ..
            } => {
                let val_ty = self.check_expr(value);
                let target_ty = if let Some(ann) = type_ann {
                    let ann_ty = resolve_hir_type(ann, self.env);
                    if let Err(msg) = self.uf.unify(&val_ty, &ann_ty) {
                        self.diags.push(Diagnostic::error(
                            format!("Type mismatch in `let`: {msg}"),
                            hir_expr_span(value),
                            self.source,
                        ));
                    }
                    ann_ty
                } else {
                    val_ty
                };
                self.bind_pattern(pattern, &target_ty, *mutable);
                Ty::Unit
            }
            HirStmt::Assign { target, value, .. } => {
                if let HirExpr::Ident(name, name_span) = target {
                    if let Some(binding) = self.env.lookup(name) {
                        if !binding.mutable {
                            self.diags.push(Diagnostic::error(
                                format!("Cannot assign to immutable variable '{name}'"),
                                *name_span,
                                self.source,
                            ));
                        }
                    }
                }
                let target_ty = self.check_expr(target);
                let value_ty = self.check_expr(value);
                let _ = self.uf.unify(&target_ty, &value_ty);
                Ty::Unit
            }
            HirStmt::Return { value, span } => {
                let val_ty = value.as_ref().map_or(Ty::Unit, |v| self.check_expr(v));
                if let Some(expected) = self.env.current_return_type() {
                    if let Err(msg) = self.uf.unify(&val_ty, expected) {
                        self.diags.push(Diagnostic::error(
                            format!("Return type mismatch: {msg}"),
                            *span,
                            self.source,
                        ));
                    }
                }
                Ty::Never
            }
            HirStmt::Expr { expr, .. } => self.check_expr(expr),
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
                            severity: Severity::Warning,
                            message: format!("'{name}' is deprecated"),
                            span: *span,
                            expected_type: None,
                            found_type: None,
                            context: Some(Diagnostic::capture_context(self.source, *span)),
                            suggestions: vec![],
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

            HirExpr::FieldAccess(object, field, span) => {
                let raw_obj = self.check_expr(object);
                let obj_ty = self.uf.resolve(&raw_obj);
                match &obj_ty {
                    Ty::Named(n) if n == "JsonBody" => match field.as_str() {
                        "message" => Ty::Str,
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Field '{field}' not found on JsonBody"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "KeyboardEvent" => match field.as_str() {
                        "key" => Ty::Str,
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Field '{field}' not found on KeyboardEvent"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdNamespace" => match field.as_str() {
                        "fs" => Ty::Named("StdFsNs".into()),
                        "path" => Ty::Named("StdPathNs".into()),
                        "env" => Ty::Named("StdEnvNs".into()),
                        "process" => Ty::Named("StdProcessNs".into()),
                        "json" => Ty::Named("StdJsonNs".into()),
                        "args" => Ty::List(Box::new(Ty::Str)),
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std submodule or field '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdFsNs" => match field.as_str() {
                        "read" | "remove" | "mkdir" => {
                            Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str))))
                        }
                        "read_bytes" => {
                            Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Str))))
                        }
                        "write" => Ty::Fn(
                            vec![Ty::Str, Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::Unit))),
                        ),
                        "exists" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Bool)),
                        "list_dir" => Ty::Fn(
                            vec![Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::List(Box::new(Ty::Str))))),
                        ),
                        "glob" => Ty::Fn(
                            vec![Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::List(Box::new(Ty::Str))))),
                        ),
                        "remove_dir_all" => {
                            Ty::Fn(vec![Ty::Str], Box::new(Ty::Result(Box::new(Ty::Unit))))
                        }
                        "copy" => Ty::Fn(
                            vec![Ty::Str, Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::Unit))),
                        ),
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std.fs method '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdPathNs" => match field.as_str() {
                        "join" => Ty::Fn(vec![Ty::Str, Ty::Str], Box::new(Ty::Str)),
                        "join_many" => Ty::Fn(vec![Ty::List(Box::new(Ty::Str))], Box::new(Ty::Str)),
                        "basename" | "dirname" | "extension" => {
                            Ty::Fn(vec![Ty::Str], Box::new(Ty::Str))
                        }
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std.path method '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdEnvNs" => match field.as_str() {
                        "get" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Option(Box::new(Ty::Str)))),
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std.env method '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdProcessNs" => match field.as_str() {
                        "run" => Ty::Fn(
                            vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
                            Box::new(Ty::Result(Box::new(Ty::Int))),
                        ),
                        "run_ex" => Ty::Fn(
                            vec![
                                Ty::Str,
                                Ty::List(Box::new(Ty::Str)),
                                Ty::Str,
                                Ty::List(Box::new(Ty::Str)),
                            ],
                            Box::new(Ty::Result(Box::new(Ty::Int))),
                        ),
                        "run_capture" => Ty::Fn(
                            vec![Ty::Str, Ty::List(Box::new(Ty::Str))],
                            Box::new(Ty::Result(Box::new(Ty::Record(vec![
                                ("exit".into(), Ty::Int),
                                ("stdout".into(), Ty::Str),
                                ("stderr".into(), Ty::Str),
                            ])))),
                        ),
                        "run_capture_ex" => Ty::Fn(
                            vec![
                                Ty::Str,
                                Ty::List(Box::new(Ty::Str)),
                                Ty::Str,
                                Ty::List(Box::new(Ty::Str)),
                            ],
                            Box::new(Ty::Result(Box::new(Ty::Record(vec![
                                ("exit".into(), Ty::Int),
                                ("stdout".into(), Ty::Str),
                                ("stderr".into(), Ty::Str),
                            ])))),
                        ),
                        "exit" => Ty::Fn(vec![Ty::Int], Box::new(Ty::Never)),
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std.process method '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Named(n) if n == "StdJsonNs" => match field.as_str() {
                        "read_str" => Ty::Fn(
                            vec![Ty::Str, Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::Str))),
                        ),
                        "read_f64" => Ty::Fn(
                            vec![Ty::Str, Ty::Str],
                            Box::new(Ty::Result(Box::new(Ty::Float))),
                        ),
                        "quote" => Ty::Fn(vec![Ty::Str], Box::new(Ty::Str)),
                        _ => {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown std.json method '{field}'"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    },
                    Ty::Record(fields) | Ty::Table(_, fields) | Ty::Collection(_, fields) => fields
                        .iter()
                        .find(|(n, _)| n == field)
                        .map(|(_, t)| t.clone())
                        .unwrap_or_else(|| {
                            self.diags.push(Diagnostic::error(
                                format!("Field '{field}' not found on {obj_ty:?}"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }),
                    Ty::Database => {
                        if let Some(binding) = self.env.lookup(field) {
                            if binding.kind == BindingKind::Table {
                                binding.ty.clone()
                            } else {
                                self.diags.push(Diagnostic::error(
                                    format!("Unknown table '{field}' in database"),
                                    *span,
                                    self.source,
                                ));
                                Ty::Error
                            }
                        } else {
                            self.diags.push(Diagnostic::error(
                                format!("Unknown table '{field}' in database"),
                                *span,
                                self.source,
                            ));
                            Ty::Error
                        }
                    }
                    Ty::Error => Ty::Error,
                    _ => {
                        self.diags.push(Diagnostic::error(
                            format!("Cannot access field '{field}' on {obj_ty:?}"),
                            *span,
                            self.source,
                        ));
                        Ty::Error
                    }
                }
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
                res_ty
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

    fn extract_iterable_element(&mut self, ty: &Ty) -> Ty {
        let resolved = self.uf.resolve(ty);
        match resolved {
            Ty::List(inner) => inner.as_ref().clone(),
            Ty::Set(inner) => inner.as_ref().clone(),
            Ty::Map(k, v) => Ty::Tuple(vec![k.as_ref().clone(), v.as_ref().clone()]),
            Ty::Stream(inner) => inner.as_ref().clone(),
            Ty::Str => Ty::Char,
            _ => self.uf.fresh_var(),
        }
    }

    fn check_unary_op(&mut self, op: HirUnOp, ty: Ty) -> Ty {
        let ty = self.uf.resolve(&ty);
        match op {
            HirUnOp::Not => {
                let _ = self.uf.unify(&ty, &Ty::Bool);
                Ty::Bool
            }
            HirUnOp::Neg => {
                if ty == Ty::Int {
                    Ty::Int
                } else if ty == Ty::Float {
                    Ty::Float
                } else {
                    Ty::Error
                }
            }
        }
    }

    fn check_binary_op(&mut self, op: HirBinOp, l: Ty, r: Ty, span: Span) -> Ty {
        let l = self.uf.resolve(&l);
        let r = self.uf.resolve(&r);
        match op {
            HirBinOp::Add => {
                if l == Ty::Str || r == Ty::Str {
                    Ty::Str
                } else if l == Ty::Int && r == Ty::Int {
                    Ty::Int
                } else if (l == Ty::Float || l == Ty::Int) && (r == Ty::Float || r == Ty::Int) {
                    Ty::Float
                } else {
                    self.diags.push(Diagnostic::error(
                        format!("Invalid operands for +: {l:?} and {r:?}"),
                        span,
                        self.source,
                    ));
                    Ty::Error
                }
            }
            HirBinOp::Sub | HirBinOp::Mul | HirBinOp::Div => {
                if l == Ty::Int && r == Ty::Int {
                    Ty::Int
                } else if (l == Ty::Float || l == Ty::Int) && (r == Ty::Float || r == Ty::Int) {
                    Ty::Float
                } else {
                    Ty::Error
                }
            }
            HirBinOp::Lt | HirBinOp::Gt | HirBinOp::Lte | HirBinOp::Gte => Ty::Bool,
            HirBinOp::Is | HirBinOp::Isnt => Ty::Bool,
            HirBinOp::And | HirBinOp::Or => {
                let _ = self.uf.unify(&l, &Ty::Bool);
                let _ = self.uf.unify(&r, &Ty::Bool);
                Ty::Bool
            }
            HirBinOp::Pipe => match r {
                Ty::Fn(_, ret) => ret.as_ref().clone(),
                _ => l,
            },
        }
    }

    /// Validate `with { ... }` option bags (workflow / activity calls).
    fn check_with_options(&mut self, options: &HirExpr, _with_span: Span) {
        match options {
            HirExpr::ObjectLit(fields, _) => {
                for (name, expr) in fields {
                    let v_ty = self.check_expr(expr);
                    let v_res = self.uf.resolve(&v_ty);
                    match name.as_str() {
                        "retries" => {
                            if self.uf.unify(&Ty::Int, &v_res).is_err() {
                                self.diags.push(Diagnostic::warning(
                                    format!(
                                        "'with' option 'retries' expects Int; got incompatible type ({v_res:?})"
                                    ),
                                    hir_expr_span(expr),
                                    self.source,
                                ));
                            }
                        }
                        "timeout" | "activity_id" | "meta" => {
                            if self.uf.unify(&Ty::Str, &v_res).is_err() {
                                self.diags.push(Diagnostic::warning(
                                    format!(
                                        "'with' option '{name}' expects Str; got incompatible type ({v_res:?})"
                                    ),
                                    hir_expr_span(expr),
                                    self.source,
                                ));
                            }
                        }
                        other => {
                            self.diags.push(Diagnostic::warning(
                                format!("Unknown 'with' option '{other}'"),
                                hir_expr_span(expr),
                                self.source,
                            ));
                        }
                    }
                }
            }
            _ => {
                let _ = self.check_expr(options);
                self.diags.push(Diagnostic::error(
                    "'with' options must be a record literal".into(),
                    hir_expr_span(options),
                    self.source,
                ));
            }
        }
    }

    fn check_arguments(&mut self, expected: &[Ty], actual: &[HirArg], span: Span) {
        if expected.len() != actual.len() {
            self.diags.push(Diagnostic::error(
                crate::typeck::diagnostics::msg_arg_count_mismatch(expected.len(), actual.len()),
                span,
                self.source,
            ));
            for arg in actual {
                let _ = self.check_expr(&arg.value);
            }
            return;
        }
        for (e, a) in expected.iter().zip(actual.iter()) {
            let a_ty = self.check_expr(&a.value);
            if let Err(msg) = self.uf.unify(e, &a_ty) {
                let arg_span = hir_expr_span(&a.value);
                self.diags.push(Diagnostic {
                    severity: Severity::Error,
                    message: format!("Argument type mismatch: {msg}"),
                    span: arg_span,
                    expected_type: Some(format!("{e:?}")),
                    found_type: Some(format!("{a_ty:?}")),
                    context: Some(Diagnostic::capture_context(self.source, arg_span)),
                    suggestions: vec![],
                });
            }
        }
    }

    fn bind_pattern(&mut self, pattern: &HirPattern, ty: &Ty, mutable: bool) {
        let ty = self.uf.resolve(ty);
        match pattern {
            HirPattern::Ident(name, _) => {
                self.env.define(
                    name.clone(),
                    Binding::new(ty.clone(), mutable, BindingKind::Variable),
                );
            }
            HirPattern::Tuple(patterns, span) => {
                if let Ty::Tuple(tys) = &ty {
                    if tys.len() != patterns.len() {
                        self.diags.push(Diagnostic::error(
                            crate::typeck::diagnostics::msg_tuple_size_mismatch(
                                tys.len(),
                                patterns.len(),
                            ),
                            *span,
                            self.source,
                        ));
                    }
                    for (p, t) in patterns.iter().zip(tys.iter()) {
                        self.bind_pattern(p, t, mutable);
                    }
                } else {
                    self.diags.push(Diagnostic::error(
                        format!("Cannot destructure non-tuple type {ty:?}"),
                        *span,
                        self.source,
                    ));
                }
            }
            HirPattern::Wildcard(_) => {}
            HirPattern::Literal(_, _) => {}
            HirPattern::Constructor(name, fields, span) => match &ty {
                Ty::Option(inner) if name == "Some" && fields.len() == 1 => {
                    self.bind_pattern(&fields[0], inner.as_ref(), mutable);
                }
                Ty::Result(inner) if name == "Ok" && fields.len() == 1 => {
                    self.bind_pattern(&fields[0], inner.as_ref(), mutable);
                }
                Ty::Result(_) if (name == "Error" || name == "Err") && fields.len() == 1 => {
                    self.bind_pattern(&fields[0], &Ty::Str, mutable);
                }
                Ty::Option(_) if name == "None" && fields.is_empty() => {}
                _ => {
                    if let Some(field_defs) = self.env.lookup_adt_variant(name) {
                        for (i, p) in fields.iter().enumerate() {
                            let ft = field_defs
                                .get(i)
                                .map(|(_, t)| t.clone())
                                .unwrap_or(Ty::Error);
                            self.bind_pattern(p, &ft, mutable);
                        }
                    } else {
                        self.diags.push(Diagnostic::error(
                            format!("Unknown constructor or pattern mismatch: {name} on {ty:?}"),
                            *span,
                            self.source,
                        ));
                    }
                }
            },
        }
    }
}

/// ADT / option-style exhaustiveness (mirrors `check::patterns::check_match_exhaustiveness` for HIR).
fn check_hir_match_exhaustiveness(
    env: &TypeEnv,
    diags: &mut Vec<Diagnostic>,
    subject_ty: &Ty,
    arms: &[HirMatchArm],
    span: Span,
    source: &str,
) {
    let type_name = match subject_ty {
        Ty::Named(name) => name.as_str(),
        _ => return,
    };

    let adt = match env.lookup_adt(type_name) {
        Some(adt) => adt,
        None => return,
    };

    let mut covered_variants: Vec<String> = Vec::new();
    let mut has_wildcard = false;

    for arm in arms {
        match &arm.pattern {
            HirPattern::Wildcard(_) => {
                has_wildcard = true;
            }
            HirPattern::Ident(name, _) => {
                if adt.variants.iter().any(|v| v.name == *name) {
                    covered_variants.push(name.clone());
                } else {
                    has_wildcard = true;
                }
            }
            HirPattern::Constructor(name, _, _) => {
                covered_variants.push(name.clone());
            }
            HirPattern::Tuple(_, _) | HirPattern::Literal(_, _) => {}
        }
    }

    if has_wildcard {
        return;
    }

    let missing: Vec<&str> = adt
        .variants
        .iter()
        .filter(|v| !covered_variants.contains(&v.name))
        .map(|v| v.name.as_str())
        .collect();

    if !missing.is_empty() {
        diags.push(Diagnostic::error(
            format!(
                "Non-exhaustive match on type '{}'. Missing variant(s): {}",
                type_name,
                missing.join(", ")
            ),
            span,
            source,
        ));
    }
}

pub fn typecheck_hir(
    module: &HirModule,
    env: &mut TypeEnv,
    builtins: &BuiltinTypes,
    source: &str,
) -> Vec<Diagnostic> {
    let mut uf = InferenceContext::new();
    let mut diags = Vec::new();
    let mut checker = Checker::new(env, builtins, &mut uf, &mut diags, source);
    checker.check_module(module);
    diags
}
