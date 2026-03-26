use std::collections::HashMap;

use crate::hir::*;

pub(super) fn normalize_db_select_projections(hir: &mut HirModule) {
    let field_order: HashMap<String, Vec<String>> = hir
        .tables
        .iter()
        .map(|t| {
            (
                t.name.clone(),
                t.fields.iter().map(|f| f.name.clone()).collect(),
            )
        })
        .collect();

    for f in &mut hir.functions {
        normalize_stmts_select(&mut f.body, &field_order);
    }
    for f in &mut hir.tests {
        normalize_stmts_select(&mut f.body, &field_order);
    }
    for r in &mut hir.routes {
        normalize_stmts_select(&mut r.body, &field_order);
    }
    for w in &mut hir.workflows {
        normalize_stmts_select(&mut w.body, &field_order);
    }
    for a in &mut hir.activities {
        normalize_stmts_select(&mut a.body, &field_order);
    }
    for sf in &mut hir.server_fns {
        normalize_stmts_select(&mut sf.body, &field_order);
    }
    for qf in &mut hir.query_fns {
        normalize_stmts_select(&mut qf.body, &field_order);
    }
    for mf in &mut hir.mutation_fns {
        normalize_stmts_select(&mut mf.body, &field_order);
    }
    for actor in &mut hir.actors {
        for h in &mut actor.handlers {
            normalize_stmts_select(&mut h.body, &field_order);
        }
    }
    for tool in &mut hir.mcp_tools {
        normalize_stmts_select(&mut tool.func.body, &field_order);
    }
}

fn reorder_db_select_cols(field_names: &[String], cols: &mut Vec<String>) {
    let ordered: Vec<String> = field_names
        .iter()
        .filter(|n| cols.iter().any(|c| c == *n))
        .cloned()
        .collect();
    if ordered.len() == cols.len() {
        *cols = ordered;
    }
}

fn normalize_stmts_select(stmts: &mut [HirStmt], field_order: &HashMap<String, Vec<String>>) {
    for s in stmts.iter_mut() {
        normalize_stmt_select(s, field_order);
    }
}

fn normalize_stmt_select(stmt: &mut HirStmt, field_order: &HashMap<String, Vec<String>>) {
    match stmt {
        HirStmt::Let { value, .. } => normalize_expr_select(value, field_order),
        HirStmt::Assign { target, value, .. } => {
            normalize_expr_select(target, field_order);
            normalize_expr_select(value, field_order);
        }
        HirStmt::Return { value: Some(v), .. } => normalize_expr_select(v, field_order),
        HirStmt::Return { value: None, .. } => {}
        HirStmt::Expr { expr, .. } => normalize_expr_select(expr, field_order),
    }
}

fn normalize_expr_select(expr: &mut HirExpr, field_order: &HashMap<String, Vec<String>>) {
    match expr {
        HirExpr::DbTableOp {
            table,
            select_cols,
            args,
            limit,
            plan,
            ..
        } => {
            if let Some(cols) = select_cols {
                if let Some(order) = field_order.get(table) {
                    reorder_db_select_cols(order, cols);
                }
            }
            if let Some(p) = plan
                && let Some(cols) = p.projection.as_mut()
                && let Some(order) = field_order.get(table)
            {
                reorder_db_select_cols(order, cols);
            }
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
            if let Some(l) = limit.as_mut() {
                normalize_expr_select(l.as_mut(), field_order);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields.iter_mut() {
                normalize_expr_select(v, field_order);
            }
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for it in items.iter_mut() {
                normalize_expr_select(it, field_order);
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            normalize_expr_select(l.as_mut(), field_order);
            normalize_expr_select(r.as_mut(), field_order);
        }
        HirExpr::Unary(_, o, _) => normalize_expr_select(o.as_mut(), field_order),
        HirExpr::Call(callee, args, _, _) => {
            normalize_expr_select(callee.as_mut(), field_order);
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            normalize_expr_select(obj.as_mut(), field_order);
            for a in args.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::FieldAccess(o, _, _) => normalize_expr_select(o.as_mut(), field_order),
        HirExpr::Match(subj, arms, _) => {
            normalize_expr_select(subj.as_mut(), field_order);
            for arm in arms.iter_mut() {
                if let Some(g) = arm.guard.as_mut() {
                    normalize_expr_select(g.as_mut(), field_order);
                }
                normalize_expr_select(arm.body.as_mut(), field_order);
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            normalize_expr_select(cond.as_mut(), field_order);
            normalize_stmts_select(then_b, field_order);
            if let Some(else_stmts) = else_b.as_mut() {
                normalize_stmts_select(else_stmts, field_order);
            }
        }
        HirExpr::For(_, it, body, _) => {
            normalize_expr_select(it.as_mut(), field_order);
            normalize_expr_select(body.as_mut(), field_order);
        }
        HirExpr::Lambda(_, _, body, _) => normalize_expr_select(body.as_mut(), field_order),
        HirExpr::Pipe(l, r, _) => {
            normalize_expr_select(l.as_mut(), field_order);
            normalize_expr_select(r.as_mut(), field_order);
        }
        HirExpr::Spawn(t, _) => normalize_expr_select(t.as_mut(), field_order),
        HirExpr::With(b, o, _) => {
            normalize_expr_select(b.as_mut(), field_order);
            normalize_expr_select(o.as_mut(), field_order);
        }
        HirExpr::Jsx(el) => {
            for a in el.attributes.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
            for c in el.children.iter_mut() {
                normalize_expr_select(c, field_order);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for a in el.attributes.iter_mut() {
                normalize_expr_select(&mut a.value, field_order);
            }
        }
        HirExpr::Block(stmts, _) => normalize_stmts_select(stmts, field_order),
        HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}
