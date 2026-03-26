use crate::hir::{HirExpr, HirStmt};

pub(crate) fn has_async_stmts(stmts: &[HirStmt]) -> bool {
    stmts.iter().any(has_async_stmt)
}

fn has_async_stmt(s: &HirStmt) -> bool {
    match s {
        HirStmt::Let { value, .. } => has_async_expr(value),
        HirStmt::Assign { value, .. } => has_async_expr(value),
        HirStmt::Return { value, .. } => value.as_ref().is_some_and(has_async_expr),
        HirStmt::Expr { expr, .. } => has_async_expr(expr),
    }
}

fn has_async_expr(e: &HirExpr) -> bool {
    match e {
        HirExpr::IntLit(..)
        | HirExpr::FloatLit(..)
        | HirExpr::StringLit(..)
        | HirExpr::BoolLit(..)
        | HirExpr::Ident(..)
        | HirExpr::Spawn(..)
        | HirExpr::Jsx(..)
        | HirExpr::JsxSelfClosing(..) => false,
        HirExpr::ListLit(elements, _) | HirExpr::TupleLit(elements, _) => {
            elements.iter().any(has_async_expr)
        }
        HirExpr::ObjectLit(fields, _) => fields.iter().map(|(_, v)| v).any(has_async_expr),
        HirExpr::Binary(_, l, r, _) => has_async_expr(l) || has_async_expr(r),
        HirExpr::Unary(_, e, _) => has_async_expr(e),
        HirExpr::Call(callee, args, is_await, _) => {
            *is_await || has_async_expr(callee) || args.iter().map(|a| &a.value).any(has_async_expr)
        }
        HirExpr::MethodCall(obj, m, args, _) => {
            if m == "send" {
                return true;
            }
            has_async_expr(obj) || args.iter().map(|a| &a.value).any(has_async_expr)
        }
        // All Codex table ops are lowered to `.await` in Rust codegen.
        HirExpr::DbTableOp { .. } => true,
        HirExpr::FieldAccess(obj, _, _) => has_async_expr(obj),
        HirExpr::Match(subj, arms, _) => {
            has_async_expr(subj)
                || arms.iter().any(|arm| {
                    has_async_expr(&arm.body)
                        || arm.guard.as_ref().is_some_and(|g| has_async_expr(g))
                })
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            has_async_expr(cond)
                || has_async_stmts(then_b)
                || else_b.as_ref().is_some_and(|b| has_async_stmts(b))
        }
        HirExpr::For(_, iter, body, _) => has_async_expr(iter) || has_async_expr(body),
        HirExpr::Lambda(..) => false,
        HirExpr::Pipe(l, r, _) => has_async_expr(l) || has_async_expr(r),
        HirExpr::With(l, r, _) => has_async_expr(l) || has_async_expr(r),
        HirExpr::Block(stmts, _) => has_async_stmts(stmts),
    }
}
