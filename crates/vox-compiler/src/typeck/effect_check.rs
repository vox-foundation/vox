//! Effect propagation check: `caller.capabilities ⊇ callee.capabilities`.
//!
//! Only enforced when the caller has an explicit `uses` clause (or `@pure`).
//! Unannotated callers are unconstrained until stdlib intrinsics are fully annotated.

use std::collections::{HashMap, HashSet};

use crate::hir::{HirArg, HirCapability, HirExpr, HirFn, HirModule, HirStmt};
use crate::typeck::diagnostics::{Diagnostic, DiagnosticCategory};

/// Run effect propagation checks across all annotated functions in the module.
pub fn check_effect_compliance(module: &HirModule, source: &str) -> Vec<Diagnostic> {
    // Build name → effective capability set for every annotated function.
    let mut cap_map: HashMap<String, Vec<HirCapability>> = HashMap::new();
    for f in &module.functions {
        if is_annotated(f) {
            cap_map.insert(f.name.clone(), effective_caps(f));
        }
    }

    let mut diags = Vec::new();
    for f in &module.functions {
        check_fn(f, &cap_map, source, &mut diags);
    }
    diags
}

/// The effective capability set: `@pure` or `uses nothing` → empty (no permitted effects).
fn effective_caps(f: &HirFn) -> Vec<HirCapability> {
    if f.is_pure {
        return vec![];
    }
    f.capabilities
        .iter()
        .filter(|c| !matches!(c, HirCapability::Nothing))
        .cloned()
        .collect()
}

fn is_annotated(f: &HirFn) -> bool {
    !f.capabilities.is_empty() || f.is_pure
}

fn check_fn(
    caller: &HirFn,
    cap_map: &HashMap<String, Vec<HirCapability>>,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    if !is_annotated(caller) {
        return;
    }
    let caller_set: HashSet<HirCapability> = effective_caps(caller).into_iter().collect();
    for stmt in &caller.body {
        check_stmt(stmt, caller, &caller_set, cap_map, source, diags);
    }
}

fn check_stmt(
    stmt: &HirStmt,
    caller: &HirFn,
    caller_set: &HashSet<HirCapability>,
    cap_map: &HashMap<String, Vec<HirCapability>>,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match stmt {
        HirStmt::Let { value, .. } | HirStmt::Expr { expr: value, .. } => {
            check_expr(value, caller, caller_set, cap_map, source, diags);
        }
        HirStmt::Assign { value, .. } => {
            check_expr(value, caller, caller_set, cap_map, source, diags);
        }
        HirStmt::Return { value: Some(v), .. } => {
            check_expr(v, caller, caller_set, cap_map, source, diags);
        }
        HirStmt::Return { value: None, .. } => {}
        HirStmt::While { condition, body, .. } => {
            check_expr(condition, caller, caller_set, cap_map, source, diags);
            for s in body {
                check_stmt(s, caller, caller_set, cap_map, source, diags);
            }
        }
        HirStmt::Loop { body, .. } => {
            for s in body {
                check_stmt(s, caller, caller_set, cap_map, source, diags);
            }
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
}

fn check_expr(
    expr: &HirExpr,
    caller: &HirFn,
    caller_set: &HashSet<HirCapability>,
    cap_map: &HashMap<String, Vec<HirCapability>>,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    match expr {
        HirExpr::Call(callee_expr, args, _, span) => {
            // Only enforce when callee is a direct identifier (not a method/closure).
            if let HirExpr::Ident(callee_name, _) = callee_expr.as_ref() {
                if let Some(callee_caps) = cap_map.get(callee_name) {
                    for cap in callee_caps {
                        if !caller_set.contains(cap) {
                            let mut d = Diagnostic::error(
                                format!(
                                    "Function `{}` calls `{}` which requires `{}`, but `{}` does not declare `uses {}`",
                                    caller.name, callee_name, cap, caller.name, cap
                                ),
                                *span,
                                source,
                            );
                            d.category = DiagnosticCategory::EffectViolation;
                            diags.push(d);
                        }
                    }
                }
            }
            // Recurse into callee expression and arguments.
            check_expr(callee_expr, caller, caller_set, cap_map, source, diags);
            for arg in args {
                check_arg(arg, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::MethodCall(obj, _, args, _, _) => {
            check_expr(obj, caller, caller_set, cap_map, source, diags);
            for arg in args {
                check_arg(arg, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::Binary(_, left, right, _) => {
            check_expr(left, caller, caller_set, cap_map, source, diags);
            check_expr(right, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::Unary(_, operand, _) => {
            check_expr(operand, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::If(cond, then_stmts, else_stmts, _) => {
            check_expr(cond, caller, caller_set, cap_map, source, diags);
            for s in then_stmts {
                check_stmt(s, caller, caller_set, cap_map, source, diags);
            }
            if let Some(els) = else_stmts {
                for s in els {
                    check_stmt(s, caller, caller_set, cap_map, source, diags);
                }
            }
        }
        HirExpr::Block(stmts, _) => {
            for s in stmts {
                check_stmt(s, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::For(_, iterable, body, _) => {
            check_expr(iterable, caller, caller_set, cap_map, source, diags);
            check_expr(body, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::Lambda(_, _, body, _) => {
            check_expr(body, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::With(lhs, rhs, _) => {
            check_expr(lhs, caller, caller_set, cap_map, source, diags);
            check_expr(rhs, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::Match(subject, arms, _) => {
            check_expr(subject, caller, caller_set, cap_map, source, diags);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    check_expr(g, caller, caller_set, cap_map, source, diags);
                }
                check_expr(&arm.body, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::FieldAccess(obj, _, _) => {
            check_expr(obj, caller, caller_set, cap_map, source, diags);
        }
        HirExpr::ListLit(elems, _) => {
            for e in elems {
                check_expr(e, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::TupleLit(elems, _) => {
            for e in elems {
                check_expr(e, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                check_expr(v, caller, caller_set, cap_map, source, diags);
            }
        }
        HirExpr::Spawn(inner, _) => {
            check_expr(inner, caller, caller_set, cap_map, source, diags);
        }
        // Leaves.
        HirExpr::IntLit(..)
        | HirExpr::FloatLit(..)
        | HirExpr::DecimalLit(..)
        | HirExpr::StringLit(..)
        | HirExpr::BoolLit(..)
        | HirExpr::Ident(..)
        | HirExpr::JsxSelfClosing(_)
        | HirExpr::Jsx(_)
        | HirExpr::Try(_) => {}
    }
}

fn check_arg(
    arg: &HirArg,
    caller: &HirFn,
    caller_set: &HashSet<HirCapability>,
    cap_map: &HashMap<String, Vec<HirCapability>>,
    source: &str,
    diags: &mut Vec<Diagnostic>,
) {
    check_expr(&arg.value, caller, caller_set, cap_map, source, diags);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hir::lower::lower_module;
    use crate::lexer::lex;
    use crate::parser::parse;

    fn check(src: &str) -> Vec<Diagnostic> {
        let tokens = lex(src);
        let module = parse(tokens).expect("parse error");
        let hir = lower_module(&module);
        check_effect_compliance(&hir, src)
    }

    #[test]
    fn test_pure_fn_no_calls_ok() {
        let diags = check("fn add(a: int, b: int) uses nothing to int { a + b }");
        assert!(diags.is_empty(), "unexpected: {diags:?}");
    }

    #[test]
    fn test_unannotated_caller_not_enforced() {
        let diags = check(
            "fn fetch() uses net to str { \"ok\" }
fn caller() to str { fetch() }",
        );
        assert!(diags.is_empty(), "unexpected: {diags:?}");
    }

    #[test]
    fn test_annotated_caller_missing_capability_is_error() {
        let diags = check(
            "fn fetch() uses net to str { \"ok\" }
fn caller() uses nothing to str { fetch() }",
        );
        assert_eq!(diags.len(), 1, "expected one violation: {diags:?}");
        assert!(diags[0].message.contains("net"));
    }

    #[test]
    fn test_annotated_caller_with_superset_is_ok() {
        let diags = check(
            "fn fetch() uses net to str { \"ok\" }
fn caller() uses net, db to str { fetch() }",
        );
        assert!(diags.is_empty(), "unexpected: {diags:?}");
    }

    #[test]
    fn test_pure_decorator_treated_as_nothing() {
        let diags = check(
            "fn fetch() uses net to str { \"ok\" }
@pure
fn caller() to str { fetch() }",
        );
        assert_eq!(diags.len(), 1, "expected one violation: {diags:?}");
    }
}
