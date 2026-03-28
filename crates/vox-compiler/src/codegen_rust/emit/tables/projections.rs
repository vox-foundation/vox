use std::collections::{HashMap, HashSet};

use crate::hir::{HirDbTableOp, HirExpr, HirModule};

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

    crate::hir::db_op_walk::for_each_hir_expr_in_module(module, &mut |expr| {
        if let HirExpr::DbTableOp {
            table,
            op,
            select_cols,
            plan,
            ..
        } = expr
            && matches!(op, HirDbTableOp::All | HirDbTableOp::FilterRecord)
        {
            if let Some(cols) = select_cols {
                record(table, cols);
            }
            if let Some(p) = plan
                && let Some(cols) = p.projection.as_ref()
            {
                record(table, cols);
            }
        }
    });

    sets.into_iter()
        .map(|(k, v)| {
            let mut projections: Vec<Vec<String>> = v.into_iter().collect();
            projections.sort();
            (k, projections)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::collect_table_select_projections;
    use crate::ast::span::Span;
    use crate::hir::{
        DefId, HirDbPlanCapabilities, HirDbQueryPlan, HirDbTableOp, HirDerived, HirExpr, HirModule,
        HirReactiveComponent, HirReactiveMember,
    };

    #[test]
    fn collects_select_projections_from_reactive_members() {
        let span = Span::new(0, 0);
        let expr = HirExpr::DbTableOp {
            table: "Task".to_string(),
            op: HirDbTableOp::FilterRecord,
            args: vec![],
            select_cols: Some(vec!["done".to_string(), "title".to_string()]),
            order_by: None,
            limit: None,
            plan: Some(HirDbQueryPlan {
                table: "Task".to_string(),
                op: HirDbTableOp::FilterRecord,
                predicate: None,
                projection: Some(vec!["done".to_string(), "title".to_string()]),
                order_by: None,
                has_limit: false,
                capabilities: HirDbPlanCapabilities::default(),
            }),
            span,
        };
        let mut module = HirModule::default();
        module.reactive_components.push(HirReactiveComponent {
            id: DefId(1),
            name: "TaskList".to_string(),
            params: vec![],
            members: vec![HirReactiveMember::Derived(HirDerived {
                id: DefId(2),
                name: "rows".to_string(),
                ty: None,
                expr,
                span,
            })],
            view: None,
            span,
        });

        let got = collect_table_select_projections(&module);
        let task = got.get("Task").expect("Task projections");
        assert_eq!(task.len(), 1);
        assert_eq!(
            task[0],
            vec!["done".to_string(), "title".to_string()],
            "reactive member projection should be collected"
        );
    }
}
