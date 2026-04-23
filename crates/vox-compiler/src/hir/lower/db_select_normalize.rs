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

    crate::hir::db_op_walk::for_each_hir_expr_in_module_mut(hir, &mut |expr| {
        normalize_select_at_expr(expr, &field_order);
    });
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

fn normalize_select_at_expr(expr: &mut HirExpr, field_order: &HashMap<String, Vec<String>>) {
    if let HirExpr::DbTableOp {
        table,
        select_cols,
        args,
        limit,
        plan,
        ..
    } = expr
    {
        if let Some(cols) = select_cols
            && let Some(order) = field_order.get(table)
        {
            reorder_db_select_cols(order, cols);
        }
        if let Some(p) = plan
            && let Some(cols) = p.projection.as_mut()
            && let Some(order) = field_order.get(table)
        {
            reorder_db_select_cols(order, cols);
        }
        let _ = args;
        let _ = limit;
    }
}
