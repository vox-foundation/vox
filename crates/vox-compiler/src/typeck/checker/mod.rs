//! HIR type Checker aligned with `crate::hir` HIR nodes (lowered AST).

use crate::ast::span::Span;
use crate::hir::*;
use crate::typeck::builtins::BuiltinTypes;
use crate::typeck::diagnostics::Diagnostic;
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

pub(crate) fn hir_expr_span(expr: &HirExpr) -> Span {
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

mod expr;
mod expr_field;
mod expr_ops;
mod match_exhaust;

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
