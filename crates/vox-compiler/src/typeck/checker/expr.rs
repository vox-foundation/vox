use crate::ast::span::Span;
use crate::hir::*;
use crate::typeck::diagnostics::{Diagnostic, Severity};
use crate::typeck::env::{Binding, BindingKind};
use crate::typeck::registration::resolve_hir_type;
use crate::typeck::ty::Ty;

use super::Checker;
use super::match_exhaust::check_hir_match_exhaustiveness;

impl<'a> Checker<'a> {
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
}
