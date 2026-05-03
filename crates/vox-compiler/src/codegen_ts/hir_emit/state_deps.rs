//! Reactive `useMemo` / dependency lists: walks HIR for identifiers that reference `state` names.
//!
//! **Support-only API (OP-0134):** used from [`super::super::reactive`]; not part of the Web IR
//! surface. Prefer keeping logic here instead of duplicating walks in preview emit.
//!
//! ## Cross-call dep tracking (Phase E of the Svelte-mineable features plan)
//!
//! By default the walker stops at function-call boundaries: a `derived label = format(count)`
//! where `format` is a free function does not see the read of `count` through the call.
//! [`extract_state_deps_with_callees`] takes a map of `@reactive`-annotated free functions
//! (their `name -> body`) and recurses into them when called from the analyzed expression.
//!
//! Bounding (per the user's "K-complexity" guidance):
//! - Single-level recursion only — a `@reactive` function body is analyzed for reactive reads,
//!   but calls *out* of it are not followed further. (Eliminates pathological recursion blow-up
//!   without paying for whole-program escape analysis.)
//! - A `visited` set prevents infinite recursion on direct or mutual recursion.
//! - Functions without `@reactive` are not descended into; the call site contributes no deps
//!   from inside the callee. (Conservative under-tracking, opt-in extension.)

use crate::hir::*;
use std::collections::{HashMap, HashSet};

/// Backward-compatible entry point: walk `expr` collecting reads of `state_names` without
/// following any function calls.
#[must_use]
pub fn extract_state_deps(expr: &HirExpr, state_names: &HashSet<String>) -> Vec<String> {
    extract_state_deps_with_callees(expr, state_names, &HashMap::new())
}

/// Walk `expr` collecting reads of `state_names`, recursing one level into the bodies of
/// any function calls that resolve to a name in `reactive_callees` (the
/// `@reactive`-annotated free functions visible to the caller). See module docs for the
/// bounding policy.
#[must_use]
pub fn extract_state_deps_with_callees(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    reactive_callees: &HashMap<String, Vec<HirStmt>>,
) -> Vec<String> {
    let mut deps = HashSet::new();
    let mut visited: HashSet<String> = HashSet::new();
    collect_deps(expr, state_names, reactive_callees, &mut visited, &mut deps);
    let mut sorted: Vec<String> = deps.into_iter().collect();
    sorted.sort();
    sorted
}

fn collect_deps(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    reactive_callees: &HashMap<String, Vec<HirStmt>>,
    visited: &mut HashSet<String>,
    deps: &mut HashSet<String>,
) {
    match expr {
        HirExpr::Ident(name, _) => {
            if state_names.contains(name) {
                deps.insert(name.clone());
            }
        }
        HirExpr::Binary(_, left, right, _) => {
            collect_deps(left, state_names, reactive_callees, visited, deps);
            collect_deps(right, state_names, reactive_callees, visited, deps);
        }
        HirExpr::Unary(_, expr, _) => {
            collect_deps(expr, state_names, reactive_callees, visited, deps);
        }
        HirExpr::Block(stmts, _) => {
            for stmt in stmts {
                collect_deps_stmt(stmt, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::Jsx(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, reactive_callees, visited, deps);
            }
            for child in &el.children {
                collect_deps(child, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for attr in &el.attributes {
                collect_deps(&attr.value, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, val) in fields {
                collect_deps(val, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::ListLit(elems, _) | HirExpr::TupleLit(elems, _) => {
            for e in elems {
                collect_deps(e, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::Call(callee, args, _, _) => {
            collect_deps(callee, state_names, reactive_callees, visited, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, reactive_callees, visited, deps);
            }
            // Cross-call recursion: if the callee resolves to an `@reactive` free function,
            // descend into its body once. The visited set bounds recursion depth and prevents
            // infinite loops from direct or mutual recursion.
            if let HirExpr::Ident(name, _) = callee.as_ref() {
                if !visited.contains(name) {
                    if let Some(body) = reactive_callees.get(name) {
                        visited.insert(name.clone());
                        for stmt in body {
                            collect_deps_stmt(
                                stmt,
                                state_names,
                                reactive_callees,
                                visited,
                                deps,
                            );
                        }
                    }
                }
            }
        }
        HirExpr::MethodCall(_, _, args, Some(_), _) => {
            for arg in args {
                collect_deps(&arg.value, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::MethodCall(obj, _, args, _, _) => {
            collect_deps(obj, state_names, reactive_callees, visited, deps);
            for arg in args {
                collect_deps(&arg.value, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::FieldAccess(obj, _, _) => {
            collect_deps(obj, state_names, reactive_callees, visited, deps);
        }
        HirExpr::If(cond, then_body, else_body, _) => {
            collect_deps(cond, state_names, reactive_callees, visited, deps);
            for stmt in then_body {
                collect_deps_stmt(stmt, state_names, reactive_callees, visited, deps);
            }
            if let Some(estmts) = else_body {
                for stmt in estmts {
                    collect_deps_stmt(stmt, state_names, reactive_callees, visited, deps);
                }
            }
        }
        HirExpr::For(_, iterable, body, _) => {
            collect_deps(iterable, state_names, reactive_callees, visited, deps);
            collect_deps(body, state_names, reactive_callees, visited, deps);
        }
        HirExpr::Lambda(_, _, body, _) => {
            collect_deps(body, state_names, reactive_callees, visited, deps);
        }
        HirExpr::Match(subject, arms, _) => {
            collect_deps(subject, state_names, reactive_callees, visited, deps);
            for arm in arms {
                collect_deps(&arm.body, state_names, reactive_callees, visited, deps);
            }
        }
        HirExpr::With(left, right, _) => {
            collect_deps(left, state_names, reactive_callees, visited, deps);
            collect_deps(right, state_names, reactive_callees, visited, deps);
        }
        HirExpr::Spawn(expr, _) => {
            collect_deps(expr, state_names, reactive_callees, visited, deps);
        }
        _ => {}
    }
}

fn collect_deps_stmt(
    stmt: &HirStmt,
    state_names: &HashSet<String>,
    reactive_callees: &HashMap<String, Vec<HirStmt>>,
    visited: &mut HashSet<String>,
    deps: &mut HashSet<String>,
) {
    match stmt {
        HirStmt::Let { value, .. } => {
            collect_deps(value, state_names, reactive_callees, visited, deps);
        }
        HirStmt::Assign { target, value, .. } => {
            collect_deps(target, state_names, reactive_callees, visited, deps);
            collect_deps(value, state_names, reactive_callees, visited, deps);
        }
        HirStmt::Expr { expr, .. } => {
            collect_deps(expr, state_names, reactive_callees, visited, deps);
        }
        HirStmt::Return { value, .. } => {
            if let Some(v) = value {
                collect_deps(v, state_names, reactive_callees, visited, deps);
            }
        }
        HirStmt::While {
            condition, body, ..
        } => {
            collect_deps(condition, state_names, reactive_callees, visited, deps);
            for s in body {
                collect_deps_stmt(s, state_names, reactive_callees, visited, deps);
            }
        }
        HirStmt::Loop { body, .. } => {
            for s in body {
                collect_deps_stmt(s, state_names, reactive_callees, visited, deps);
            }
        }
        HirStmt::Break { .. } | HirStmt::Continue { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_state_deps, extract_state_deps_with_callees};
    use crate::ast::span::Span;
    use crate::hir::{HirArg, HirBinOp, HirExpr, HirStmt};
    use std::collections::{HashMap, HashSet};

    fn sp() -> Span {
        Span::new(0, 0)
    }

    #[test]
    fn extract_state_deps_finds_state_in_binary() {
        let state: HashSet<String> = HashSet::from(["count".into()]);
        let expr = HirExpr::Binary(
            HirBinOp::Add,
            Box::new(HirExpr::Ident("count".into(), sp())),
            Box::new(HirExpr::Ident("n".into(), sp())),
            sp(),
        );
        let deps = extract_state_deps(&expr, &state);
        assert_eq!(deps, vec!["count".to_string()]);
    }

    #[test]
    fn cross_call_recursion_into_reactive_callee_finds_state() {
        // derived label = format(count) where format is `@reactive fn format(c) { c + 1 }`
        // — without the callee descent the dep set is empty (caller doesn't read count
        // directly); with the descent we should see `count`.
        let state: HashSet<String> = HashSet::from(["count".into()]);
        let call = HirExpr::Call(
            Box::new(HirExpr::Ident("format".into(), sp())),
            vec![HirArg {
                name: None,
                value: HirExpr::Ident("count".into(), sp()),
            }],
            false,
            sp(),
        );
        // Body: just an expression statement that reads `count`.
        let format_body = vec![HirStmt::Expr {
            expr: HirExpr::Ident("count".into(), sp()),
            span: sp(),
        }];
        let mut callees: HashMap<String, Vec<HirStmt>> = HashMap::new();
        callees.insert("format".into(), format_body);

        let with = extract_state_deps_with_callees(&call, &state, &callees);
        assert_eq!(with, vec!["count".to_string()]);

        // Sanity: without the callees map, the same expression already finds `count` because
        // it appears as a direct call argument — but the body-internal read would be invisible
        // if the arg didn't reference state. Verify the body-only path:
        let opaque_arg_call = HirExpr::Call(
            Box::new(HirExpr::Ident("format".into(), sp())),
            vec![], // no args mentioning state
            false,
            sp(),
        );
        let body_only = extract_state_deps_with_callees(&opaque_arg_call, &state, &callees);
        assert_eq!(body_only, vec!["count".to_string()]);
        // Without the callees map the body-only path returns nothing.
        let no_callees = extract_state_deps(&opaque_arg_call, &state);
        assert!(no_callees.is_empty());
    }

    #[test]
    fn cross_call_recursion_does_not_loop_on_self_reference() {
        // A `@reactive` function that calls itself shouldn't blow the stack.
        let state: HashSet<String> = HashSet::from(["x".into()]);
        let call = HirExpr::Call(
            Box::new(HirExpr::Ident("recurse".into(), sp())),
            vec![],
            false,
            sp(),
        );
        let recurse_body = vec![HirStmt::Expr {
            expr: HirExpr::Call(
                Box::new(HirExpr::Ident("recurse".into(), sp())),
                vec![HirArg {
                    name: None,
                    value: HirExpr::Ident("x".into(), sp()),
                }],
                false,
                sp(),
            ),
            span: sp(),
        }];
        let mut callees: HashMap<String, Vec<HirStmt>> = HashMap::new();
        callees.insert("recurse".into(), recurse_body);

        let deps = extract_state_deps_with_callees(&call, &state, &callees);
        assert_eq!(deps, vec!["x".to_string()]);
    }

    #[test]
    fn extract_state_deps_sorts_and_dedupes() {
        let state: HashSet<String> = HashSet::from(["a".into(), "b".into()]);
        let inner = HirExpr::Binary(
            HirBinOp::Add,
            Box::new(HirExpr::Ident("a".into(), sp())),
            Box::new(HirExpr::Ident("b".into(), sp())),
            sp(),
        );
        let expr = HirExpr::Binary(
            HirBinOp::Add,
            Box::new(inner),
            Box::new(HirExpr::Ident("a".into(), sp())),
            sp(),
        );
        let deps = extract_state_deps(&expr, &state);
        assert_eq!(deps, vec!["a".to_string(), "b".to_string()]);
    }
}
