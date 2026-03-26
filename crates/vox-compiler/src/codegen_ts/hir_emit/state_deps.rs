use crate::hir::*;
use std::collections::HashSet;

#[must_use]
pub fn extract_state_deps(expr: &HirExpr, state_names: &HashSet<String>) -> Vec<String> {
    let mut deps = HashSet::new();
    collect_deps(expr, state_names, &mut deps);
    let mut sorted: Vec<String> = deps.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_deps(expr: &HirExpr, state_names: &HashSet<String>, deps: &mut HashSet<String>) {
    match expr {
        HirExpr::Ident(name, _) => {
            if state_names.contains(name) {
                deps.insert(name.clone());
            }
        }
        HirExpr::Binary(_, left, right, _) => {
            collect_deps(left, state_names, deps);
            collect_deps(right, state_names, deps);
        }
        HirExpr::Unary(_, expr, _) => {
            collect_deps(expr, state_names, deps);
        }
        HirExpr::Block(stmts, _) => {
            for stmt in stmts {
                collect_deps_stmt(stmt, state_names, deps);
            }
        }
        HirExpr::Jsx(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, deps);
            }
            for child in &el.children {
                collect_deps(child, state_names, deps);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, deps);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, val) in fields {
                collect_deps(val, state_names, deps);
            }
        }
        HirExpr::ListLit(elems, _) | HirExpr::TupleLit(elems, _) => {
            for e in elems {
                collect_deps(e, state_names, deps);
            }
        }
        HirExpr::Call(callee, args, _, _) => {
            collect_deps(callee, state_names, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, deps);
            }
        }
        HirExpr::DbTableOp { args, .. } => {
            for arg in args {
                collect_deps(&arg.value, state_names, deps);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            collect_deps(obj, state_names, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, deps);
            }
        }
        HirExpr::FieldAccess(obj, _, _) => {
            collect_deps(obj, state_names, deps);
        }
        HirExpr::If(cond, then_body, else_body, _) => {
            collect_deps(cond, state_names, deps);
            for stmt in then_body {
                collect_deps_stmt(stmt, state_names, deps);
            }
            if let Some(estmts) = else_body {
                for stmt in estmts {
                    collect_deps_stmt(stmt, state_names, deps);
                }
            }
        }
        HirExpr::For(_, iterable, body, _) => {
            collect_deps(iterable, state_names, deps);
            collect_deps(body, state_names, deps);
        }
        HirExpr::Lambda(_, _, body, _) => {
            collect_deps(body, state_names, deps);
        }
        HirExpr::Match(subject, arms, _) => {
            collect_deps(subject, state_names, deps);
            for arm in arms {
                collect_deps(&arm.body, state_names, deps);
            }
        }
        HirExpr::Pipe(left, right, _) | HirExpr::With(left, right, _) => {
            collect_deps(left, state_names, deps);
            collect_deps(right, state_names, deps);
        }
        HirExpr::Spawn(expr, _) => {
            collect_deps(expr, state_names, deps);
        }
        _ => {}
    }
}

fn collect_deps_stmt(stmt: &HirStmt, state_names: &HashSet<String>, deps: &mut HashSet<String>) {
    match stmt {
        HirStmt::Let { value, .. } => {
            collect_deps(value, state_names, deps);
        }
        HirStmt::Assign { target, value, .. } => {
            collect_deps(target, state_names, deps);
            collect_deps(value, state_names, deps);
        }
        HirStmt::Expr { expr, .. } => {
            collect_deps(expr, state_names, deps);
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_deps(v, state_names, deps);
            }
        }
    }
}
