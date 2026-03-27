use std::collections::{HashMap, HashSet};

use crate::hir::{HirDbTableOp, HirExpr, HirModule, HirStmt};

/// Stable Rust method suffix for `SELECT _id, col1, …` projections (columns in declaration order).
pub(crate) fn db_projection_method_suffix(cols: &[String]) -> String {
    cols.join("_")
}

/// Fail codegen when two different column projections collapse to the same Rust helper suffix
/// (`join("_")`), which would emit duplicate/inconsistent `from_row_sel_*` methods.
pub fn validate_db_projection_suffixes_unique(
    table_name: &str,
    projections: &[Vec<String>],
) -> Result<(), miette::Error> {
    let mut by_suffix: HashMap<String, Vec<String>> = HashMap::new();
    for cols in projections {
        let sfx = db_projection_method_suffix(cols);
        if let Some(prev) = by_suffix.get(&sfx) {
            if prev != cols {
                return Err(miette::miette!(
                    "table '{}': `.select([…])` projections {:?} and {:?} both codegen to suffix '{}'; disambiguate column lists (suffix is columns joined with '_')",
                    table_name,
                    prev,
                    cols,
                    sfx
                ));
            }
        } else {
            by_suffix.insert(sfx, cols.clone());
        }
    }
    Ok(())
}

fn walk_select_projection_stmts(stmts: &[HirStmt], record: &mut impl FnMut(&str, &[String])) {
    for s in stmts {
        walk_select_projection_stmt(s, record);
    }
}

fn walk_select_projection_stmt(stmt: &HirStmt, record: &mut impl FnMut(&str, &[String])) {
    match stmt {
        HirStmt::Let { value, .. } => walk_select_projection_expr(value, record),
        HirStmt::Assign { target, value, .. } => {
            walk_select_projection_expr(target, record);
            walk_select_projection_expr(value, record);
        }
        HirStmt::Return { value: Some(v), .. } => walk_select_projection_expr(v, record),
        HirStmt::Return { value: None, .. } => {}
        HirStmt::Expr { expr, .. } => walk_select_projection_expr(expr, record),
    }
}

fn walk_select_projection_expr(expr: &HirExpr, record: &mut impl FnMut(&str, &[String])) {
    match expr {
        HirExpr::DbTableOp {
            table,
            op,
            args,
            select_cols,
            plan,
            limit,
            ..
        } => {
            if matches!(op, HirDbTableOp::All | HirDbTableOp::FilterRecord) {
                if let Some(cols) = select_cols {
                    record(table, cols);
                }
                if let Some(p) = plan
                    && let Some(cols) = p.projection.as_ref()
                {
                    record(table, cols);
                }
            }
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
            if let Some(l) = limit {
                walk_select_projection_expr(l.as_ref(), record);
            }
        }
        HirExpr::ObjectLit(fields, _) => {
            for (_, v) in fields {
                walk_select_projection_expr(v, record);
            }
        }
        HirExpr::ListLit(items, _) | HirExpr::TupleLit(items, _) => {
            for it in items {
                walk_select_projection_expr(it, record);
            }
        }
        HirExpr::Binary(_, l, r, _) => {
            walk_select_projection_expr(l.as_ref(), record);
            walk_select_projection_expr(r.as_ref(), record);
        }
        HirExpr::Unary(_, o, _) => walk_select_projection_expr(o.as_ref(), record),
        HirExpr::Call(callee, args, _, _) => {
            walk_select_projection_expr(callee.as_ref(), record);
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::MethodCall(obj, _, args, _) => {
            walk_select_projection_expr(obj.as_ref(), record);
            for a in args {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::FieldAccess(o, _, _) => walk_select_projection_expr(o.as_ref(), record),
        HirExpr::Match(subj, arms, _) => {
            walk_select_projection_expr(subj.as_ref(), record);
            for arm in arms {
                if let Some(g) = &arm.guard {
                    walk_select_projection_expr(g.as_ref(), record);
                }
                walk_select_projection_expr(arm.body.as_ref(), record);
            }
        }
        HirExpr::If(cond, then_b, else_b, _) => {
            walk_select_projection_expr(cond.as_ref(), record);
            walk_select_projection_stmts(then_b, record);
            if let Some(else_stmts) = else_b {
                walk_select_projection_stmts(else_stmts, record);
            }
        }
        HirExpr::For(_, it, body, _) => {
            walk_select_projection_expr(it.as_ref(), record);
            walk_select_projection_expr(body.as_ref(), record);
        }
        HirExpr::Lambda(_, _, body, _) => walk_select_projection_expr(body.as_ref(), record),
        HirExpr::Pipe(l, r, _) => {
            walk_select_projection_expr(l.as_ref(), record);
            walk_select_projection_expr(r.as_ref(), record);
        }
        HirExpr::Spawn(t, _) => walk_select_projection_expr(t.as_ref(), record),
        HirExpr::With(b, o, _) => {
            walk_select_projection_expr(b.as_ref(), record);
            walk_select_projection_expr(o.as_ref(), record);
        }
        HirExpr::Jsx(el) => {
            for a in &el.attributes {
                walk_select_projection_expr(&a.value, record);
            }
            for c in &el.children {
                walk_select_projection_expr(c, record);
            }
        }
        HirExpr::JsxSelfClosing(el) => {
            for a in &el.attributes {
                walk_select_projection_expr(&a.value, record);
            }
        }
        HirExpr::Block(stmts, _) => walk_select_projection_stmts(stmts, record),
        HirExpr::IntLit(_, _)
        | HirExpr::FloatLit(_, _)
        | HirExpr::StringLit(_, _)
        | HirExpr::BoolLit(_, _)
        | HirExpr::Ident(_, _) => {}
    }
}

/// Distinct column projections per table (post-HIR normalization — declaration order).
pub fn collect_table_select_projections(module: &HirModule) -> HashMap<String, Vec<Vec<String>>> {
    let mut sets: HashMap<String, HashSet<Vec<String>>> = HashMap::new();
    let mut record = |table_name: &str, cols: &[String]| {
        if cols.is_empty() {
            return;
        }
        sets.entry(table_name.to_string())
            .or_default()
            .insert(cols.to_vec());
    };

    for f in &module.functions {
        walk_select_projection_stmts(&f.body, &mut record);
    }
    for f in &module.tests {
        walk_select_projection_stmts(&f.body, &mut record);
    }
    for r in &module.routes {
        walk_select_projection_stmts(&r.body, &mut record);
    }
    for w in &module.workflows {
        walk_select_projection_stmts(&w.body, &mut record);
    }
    for a in &module.activities {
        walk_select_projection_stmts(&a.body, &mut record);
    }
    for sf in &module.server_fns {
        walk_select_projection_stmts(&sf.body, &mut record);
    }
    for qf in &module.query_fns {
        walk_select_projection_stmts(&qf.body, &mut record);
    }
    for mf in &module.mutation_fns {
        walk_select_projection_stmts(&mf.body, &mut record);
    }
    for actor in &module.actors {
        for h in &actor.handlers {
            walk_select_projection_stmts(&h.body, &mut record);
        }
    }
    for tool in &module.mcp_tools {
        walk_select_projection_stmts(&tool.func.body, &mut record);
    }

    sets.into_iter()
        .map(|(k, v)| {
            let mut projections: Vec<Vec<String>> = v.into_iter().collect();
            projections.sort();
            (k, projections)
        })
        .collect()
}
