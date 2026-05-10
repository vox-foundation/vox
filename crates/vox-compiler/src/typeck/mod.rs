//! # Vox Type Checker
//!
//! This crate implements the name resolution and bidirectional type checking
//! logic for the Vox compiler. It operates on the High-Level Intermediate
//! Representation (HIR).
//!
//! Key components:
//! - Unification-based type inference
//! - ADT and pattern matching validation
//! - Builtin type registration
//! - Diagnostic reporting for semantic errors

#![allow(clippy::collapsible_if)]

mod ast_decl_lints;
mod async_handler_lint;
pub mod async_exhaustiveness;
pub mod boilerplate_grafts;
pub mod contrast;
mod effect_deps_lint;
pub mod form_check;
pub mod layer;
pub mod semantic_ui;
mod stale_capture_lint;

pub use ast_decl_lints::lint_ast_declarations;
/// Automated fix suggestions for type-check diagnostics.
pub mod autofix;
/// Logic for registering and accessing builtin types and functions.
pub mod builtins;
// All type checking flows through `typecheck_hir` in the `checker` module
// (implemented in `checker/mod.rs`).
/// Central state machine for the type checking process.
pub mod checker;
/// Diagnostic structures and error reporting for type checking.
pub mod diagnostics;
/// Effect propagation check: `caller.capabilities ⊇ callee.capabilities`.
pub mod effect_check;
/// Environment management for symbols, types, and scopes.
pub mod env;
/// Core logic for unification-based type inference.
pub mod infer;
pub mod policy;
/// Logic for registering declarations into the global environment.
pub mod registration;
/// State machine exhaustiveness and structural checks.
pub mod state_machine_check;
/// Representation of internal types used during inference and checking.
pub mod ty;
/// Logic for unification of types and solving constraints.
pub mod unify;

use crate::ast::decl::Module;
pub use crate::hir::HirModule;
use crate::hir::lower::lower_module;
pub use builtins::BuiltinTypes;
pub use checker::{Checker, typecheck_hir};
/// A single semantic diagnostic (error or warning) produced during type checking.
pub use diagnostics::{Diagnostic, DiagnosticCategory, DiagnosticFix, TypeckSeverity};
pub use env::TypeEnv;
pub use ty::ty_display;

/// Run the type Checker on a HirModule (replacement for the removed AST-only path).
#[must_use]
pub fn typecheck_hir_module(source: &str, hir: &mut HirModule) -> Vec<Diagnostic> {
    let mut env = TypeEnv::new();
    let builtins = BuiltinTypes::register_all(&mut env);
    let mut diags = typecheck_hir(hir, &mut env, &builtins, source);
    diags.extend(effect_check::check_effect_compliance(hir, source));
    diags.extend(state_machine_check::check_state_machines(hir, source));
    diags.extend(effect_deps_lint::check_effect_deps(hir, source));
    diags.extend(stale_capture_lint::check_stale_captures(hir, source));
    diags.extend(async_handler_lint::check_async_handlers(hir, source));
    diags.extend(form_check::check_forms(hir, source));
    // GA-20 / CC-23: contrast-ratio validation for design token color pairs.
    diags.extend(contrast::check_tokens(&hir.token_decls));
    // GA-19: a11y label enforcement for semantic UI primitives.
    diags.extend(semantic_ui::check_semantic_ui(&collect_semantic_ui_callsites(hir)));
    // GA-01: Async[T] view exhaustiveness (all four arms required).
    diags.extend(collect_async_views(hir).into_iter().filter_map(|v| async_exhaustiveness::check_async_view(&v)));
    diags
}

/// Walk all statements in a function body looking for `Async[T]` view nodes.
fn collect_async_views(hir: &HirModule) -> Vec<crate::hir::nodes::async_view::HirAsyncView> {
    use crate::hir::HirExpr;
    use crate::hir::HirStmt;

    fn visit_expr(expr: &HirExpr, out: &mut Vec<crate::hir::nodes::async_view::HirAsyncView>) {
        match expr {
            HirExpr::AsyncView(v) => {
                // Check nested arms too before pushing this node.
                if let Some(arm) = &v.fetching_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.empty_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.error_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.ok_arm { visit_expr(arm, out); }
                out.push(*v.clone());
            }
            HirExpr::Block(stmts, _) => {
                for s in stmts { visit_stmt(s, out); }
            }
            HirExpr::If(cond, then_stmts, else_stmts, _) => {
                visit_expr(cond, out);
                for s in then_stmts { visit_stmt(s, out); }
                if let Some(es) = else_stmts { for s in es { visit_stmt(s, out); } }
            }
            HirExpr::Binary(_, l, r, _) => { visit_expr(l, out); visit_expr(r, out); }
            HirExpr::Unary(_, e, _) => visit_expr(e, out),
            HirExpr::Call(f, args, _, _) => {
                visit_expr(f, out);
                for a in args { visit_expr(&a.value, out); }
            }
            HirExpr::MethodCall(recv, _, args, _, _) => {
                visit_expr(recv, out);
                for a in args { visit_expr(&a.value, out); }
            }
            HirExpr::Lambda(_, _, body, _, _) => visit_expr(body, out),
            HirExpr::For(_, _, iter, body, key, _) => {
                visit_expr(iter, out);
                visit_expr(body, out);
                if let Some(k) = key { visit_expr(k, out); }
            }
            HirExpr::Match(e, arms, _) => {
                visit_expr(e, out);
                for arm in arms { visit_expr(&arm.body, out); }
            }
            HirExpr::Jsx(el) => {
                for a in &el.attributes { visit_expr(&a.value, out); }
                for c in &el.children { visit_expr(c, out); }
            }
            HirExpr::JsxSelfClosing(el) => {
                for a in &el.attributes { visit_expr(&a.value, out); }
            }
            HirExpr::JsxFragment(children, _) => {
                for c in children { visit_expr(c, out); }
            }
            HirExpr::Try(t) => visit_expr(&t.target, out),
            HirExpr::Index(a, b, _) => { visit_expr(a, out); visit_expr(b, out); }
            HirExpr::With(a, b, _) => { visit_expr(a, out); visit_expr(b, out); }
            HirExpr::FieldAccess(e, _, _) => visit_expr(e, out),
            HirExpr::Spawn(e, _) => visit_expr(e, out),
            HirExpr::ObjectLit(fields, _) => {
                for (_, v) in fields { visit_expr(v, out); }
            }
            HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
                for v in items { visit_expr(v, out); }
            }
            _ => {}
        }
    }

    fn visit_stmt(stmt: &HirStmt, out: &mut Vec<crate::hir::nodes::async_view::HirAsyncView>) {
        match stmt {
            HirStmt::Let { value, .. } => visit_expr(value, out),
            HirStmt::Assign { target, value, .. } => {
                visit_expr(target, out);
                visit_expr(value, out);
            }
            HirStmt::Expr { expr, .. } => visit_expr(expr, out),
            HirStmt::Return { value: Some(e), .. } => visit_expr(e, out),
            HirStmt::While { condition, body, .. } => {
                visit_expr(condition, out);
                for s in body { visit_stmt(s, out); }
            }
            HirStmt::Loop { body, .. } => {
                for s in body { visit_stmt(s, out); }
            }
            _ => {}
        }
    }

    let mut out = vec![];
    for f in &hir.functions {
        for s in &f.body { visit_stmt(s, &mut out); }
    }
    for comp in &hir.components {
        use crate::hir::nodes::HirReactiveMember;
        for m in &comp.members {
            match m {
                HirReactiveMember::State(s) => visit_expr(&s.init, &mut out),
                HirReactiveMember::Derived(d) => visit_expr(&d.expr, &mut out),
                HirReactiveMember::Effect(e) => visit_expr(&e.body, &mut out),
                HirReactiveMember::OnMount(m) => visit_expr(&m.body, &mut out),
                HirReactiveMember::OnCleanup(c) => visit_expr(&c.body, &mut out),
                HirReactiveMember::Stmt(s) => visit_stmt(s, &mut out),
            }
        }
        if let Some(view) = &comp.view {
            visit_expr(view, &mut out);
        }
    }
    out
}

/// Collect semantic UI primitive callsites from all JSX in the HIR.
///
/// Finds `<Dialog>`, `<Menu>`, `<Listbox>`, `<Combobox>`, `<Tabs>` elements and
/// records whether they carry a `label` attribute, so `check_semantic_ui` can
/// enforce the a11y requirement.
fn collect_semantic_ui_callsites(hir: &HirModule) -> Vec<semantic_ui::SemanticUiCallSite> {
    use crate::hir::HirExpr;
    use crate::hir::HirStmt;
    use crate::hir::HirJsxAttr;

    const PRIMITIVES: &[&str] = &["Dialog", "Menu", "Listbox", "Combobox", "Tabs"];

    fn check_attrs(attrs: &[HirJsxAttr]) -> bool {
        attrs.iter().any(|a| a.name == "label")
    }

    fn visit_expr(expr: &HirExpr, out: &mut Vec<semantic_ui::SemanticUiCallSite>) {
        match expr {
            HirExpr::Jsx(el) => {
                if PRIMITIVES.contains(&el.tag.as_str()) {
                    out.push(semantic_ui::SemanticUiCallSite {
                        primitive: el.tag.clone(),
                        has_label: check_attrs(&el.attributes),
                        span: el.span,
                    });
                }
                for a in &el.attributes { visit_expr(&a.value, out); }
                for c in &el.children { visit_expr(c, out); }
            }
            HirExpr::JsxSelfClosing(el) => {
                if PRIMITIVES.contains(&el.tag.as_str()) {
                    out.push(semantic_ui::SemanticUiCallSite {
                        primitive: el.tag.clone(),
                        has_label: check_attrs(&el.attributes),
                        span: el.span,
                    });
                }
                for a in &el.attributes { visit_expr(&a.value, out); }
            }
            HirExpr::JsxFragment(children, _) => {
                for c in children { visit_expr(c, out); }
            }
            HirExpr::Block(stmts, _) => {
                for s in stmts { visit_stmt(s, out); }
            }
            HirExpr::If(cond, then_stmts, else_stmts, _) => {
                visit_expr(cond, out);
                for s in then_stmts { visit_stmt(s, out); }
                if let Some(es) = else_stmts { for s in es { visit_stmt(s, out); } }
            }
            HirExpr::For(_, _, iter, body, key, _) => {
                visit_expr(iter, out);
                visit_expr(body, out);
                if let Some(k) = key { visit_expr(k, out); }
            }
            HirExpr::Lambda(_, _, body, _, _) => visit_expr(body, out),
            HirExpr::Match(e, arms, _) => {
                visit_expr(e, out);
                for arm in arms { visit_expr(&arm.body, out); }
            }
            HirExpr::AsyncView(v) => {
                if let Some(arm) = &v.fetching_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.empty_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.error_arm { visit_expr(arm, out); }
                if let Some(arm) = &v.ok_arm { visit_expr(arm, out); }
            }
            HirExpr::Call(f, args, _, _) => {
                visit_expr(f, out);
                for a in args { visit_expr(&a.value, out); }
            }
            HirExpr::MethodCall(recv, _, args, _, _) => {
                visit_expr(recv, out);
                for a in args { visit_expr(&a.value, out); }
            }
            _ => {}
        }
    }

    fn visit_stmt(stmt: &HirStmt, out: &mut Vec<semantic_ui::SemanticUiCallSite>) {
        match stmt {
            HirStmt::Let { value, .. } => visit_expr(value, out),
            HirStmt::Assign { target, value, .. } => {
                visit_expr(target, out);
                visit_expr(value, out);
            }
            HirStmt::Expr { expr, .. } => visit_expr(expr, out),
            HirStmt::Return { value: Some(e), .. } => visit_expr(e, out),
            HirStmt::While { condition, body, .. } => {
                visit_expr(condition, out);
                for s in body { visit_stmt(s, out); }
            }
            HirStmt::Loop { body, .. } => {
                for s in body { visit_stmt(s, out); }
            }
            _ => {}
        }
    }

    let mut out = vec![];
    for f in &hir.functions {
        for s in &f.body { visit_stmt(s, &mut out); }
    }
    for comp in &hir.components {
        use crate::hir::nodes::HirReactiveMember;
        for m in &comp.members {
            match m {
                HirReactiveMember::State(s) => visit_expr(&s.init, &mut out),
                HirReactiveMember::Derived(d) => visit_expr(&d.expr, &mut out),
                HirReactiveMember::Effect(e) => visit_expr(&e.body, &mut out),
                HirReactiveMember::OnMount(m) => visit_expr(&m.body, &mut out),
                HirReactiveMember::OnCleanup(c) => visit_expr(&c.body, &mut out),
                HirReactiveMember::Stmt(s) => visit_stmt(s, &mut out),
            }
        }
        if let Some(view) = &comp.view {
            visit_expr(view, &mut out);
        }
    }
    out
}

/// Lower `module` to HIR and run the type Checker (replacement for the removed AST-only path).
#[must_use]
pub fn typecheck_ast_module(source: &str, module: &Module) -> Vec<Diagnostic> {
    let mut diags = ast_decl_lints::lint_ast_declarations(module, source);
    let mut hir = lower_module(module);
    diags.extend(typecheck_hir_module(source, &mut hir));
    diags
}

/// Type-check a parsed module. Pass `source` for diagnostic context (may be `""`).
///
/// This is the stable name used by the CLI, LSP, and integration tests; it delegates to the HIR pipeline.
#[must_use]
pub fn typecheck_module(module: &Module, source: &str) -> Vec<Diagnostic> {
    typecheck_ast_module(source, module)
}
