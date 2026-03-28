//! Walk [`HirExpr`] trees to find every [`HirExpr::DbTableOp`] site (shared by runtime projection).

use crate::hir::{
    HirArg, HirDbQueryPlan, HirDbTableOp, HirExpr, HirModule, HirReactiveMember, HirStmt,
};

/// One occurrence of a lowered `db.Table.op(...)` expression.
pub(crate) struct DbTableOpSite<'a> {
    pub table: &'a str,
    pub op: HirDbTableOp,
    pub plan: Option<&'a HirDbQueryPlan>,
}

pub(crate) fn for_each_db_table_op_in_module(
    module: &HirModule,
    f: &mut impl FnMut(DbTableOpSite<'_>),
) {
    for fd in &module.functions {
        walk_stmts(&fd.body, f);
    }
    for fd in &module.tests {
        walk_stmts(&fd.body, f);
    }
    for r in &module.routes {
        walk_stmts(&r.body, f);
    }
    for w in &module.workflows {
        walk_stmts(&w.body, f);
    }
    for a in &module.activities {
        walk_stmts(&a.body, f);
    }
    for sf in &module.server_fns {
        walk_stmts(&sf.body, f);
    }
    for qf in &module.query_fns {
        walk_stmts(&qf.body, f);
    }
    for mf in &module.mutation_fns {
        walk_stmts(&mf.body, f);
    }
    for actor in &module.actors {
        for h in &actor.handlers {
            walk_stmts(&h.body, f);
        }
    }
    for tool in &module.mcp_tools {
        walk_stmts(&tool.func.body, f);
    }
    for rc in &module.reactive_components {
        for m in &rc.members {
            match m {
                HirReactiveMember::State(s) => walk_expr(&s.init, f),
                HirReactiveMember::Derived(d) => walk_expr(&d.expr, f),
                HirReactiveMember::Effect(e) => walk_expr(&e.body, f),
                HirReactiveMember::OnMount(e) => walk_expr(&e.body, f),
                HirReactiveMember::OnCleanup(e) => walk_expr(&e.body, f),
            }
        }
        if let Some(view) = &rc.view {
            walk_expr(view, f);
        }
    }
}

fn walk_stmts(stmts: &[HirStmt], f: &mut impl FnMut(DbTableOpSite<'_>)) {
    for s in stmts {
        walk_stmt(s, f);
    }
}

fn walk_stmt(stmt: &HirStmt, f: &mut impl FnMut(DbTableOpSite<'_>)) {
    match stmt {
        HirStmt::Let { value, .. } => walk_expr(value, f),
        HirStmt::Assign { target, value, .. } => {
            walk_expr(target, f);
            walk_expr(value, f);
        }
        HirStmt::Return { value: Some(v), .. } => walk_expr(v, f),
        HirStmt::Return { value: None, .. } => {}
        HirStmt::Expr { expr, .. } => walk_expr(expr, f),
    }
}

fn walk_expr(expr: &HirExpr, f: &mut impl FnMut(DbTableOpSite<'_>)) {
    match expr {
        HirExpr::DbTableOp {
            table,
            op,
            args,
            limit,
            plan,
            ..
        } => {
            f(DbTableOpSite {
                table: table.as_str(),
                op: *op,
                plan: plan.as_ref(),
            });
            for a in args {
                walk_arg(a, f);
            }
            if let Some(l) = limit {
                walk_expr(l.as_ref(), f);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                walk_expr(v, f);
            }
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for it in items {
                walk_expr(it, f);
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            walk_expr(l.as_ref(), f);
            walk_expr(r.as_ref(), f);
        }
        HirExpr::Unary(_, o, _) => walk_expr(o.as_ref(), f),
        HirExpr::Call(callee, args, _, _) => {
            walk_expr(callee.as_ref(), f);
            for a in args {
                walk_arg(a, f);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            walk_expr(obj.as_ref(), f);
            for a in args {
                walk_arg(a, f);
            }
        }
        HirExpr::FieldAccess(o, _, _) => walk_expr(o.as_ref(), f),
        HirExpr::Match(subj, arms, _) => {
            walk_expr(subj.as_ref(), f);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    walk_expr(g.as_ref(), f);
                }
                walk_expr(arm.body.as_ref(), f);
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            walk_expr(cond.as_ref(), f);
            walk_stmts(then_b, f);
            if let Some(else_stmts) = else_b {
                walk_stmts(else_stmts, f);
            }
        }
        HirExpr::For(_, it, body, _) => {
            walk_expr(it.as_ref(), f);
            walk_expr(body.as_ref(), f);
        }
        HirExpr::Lambda(_, _, body, _) => walk_expr(body.as_ref(), f),
        HirExpr::Pipe(l, r, _) => {
            walk_expr(l.as_ref(), f);
            walk_expr(r.as_ref(), f);
        }
        HirExpr::Spawn(t, _) => walk_expr(t.as_ref(), f),
        HirExpr::With(b, o, _) => {
            walk_expr(b.as_ref(), f);
            walk_expr(o.as_ref(), f);
        }
        HirExpr::Jsx(el) => {
            for a in &el.attributes {
                walk_expr(&a.value, f);
            }
            for c in &el.children {
                walk_expr(c, f);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for a in &el.attributes {
                walk_expr(&a.value, f);
            }
        }
        HirExpr::Block(stmts, _) => walk_stmts(stmts, f),
        HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}

fn walk_arg(arg: &HirArg, f: &mut impl FnMut(DbTableOpSite<'_>)) {
    walk_expr(&arg.value, f);
}
