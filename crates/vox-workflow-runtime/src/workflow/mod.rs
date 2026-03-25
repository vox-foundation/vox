//! Interpreted workflow planning and execution (internal).

pub mod plan;
pub mod populi;
pub mod run;
pub mod tracker;
pub mod types;

pub use plan::plan_workflow_activities;
#[cfg(feature = "mens")]
pub use populi::execute_populi_step;
pub use run::{interpret_workflow, interpret_workflow_durable};
pub use tracker::{DefaultTracker, WorkflowTracker};
pub use types::{PlannedActivity, PopuliActivity, PopuliHttpOp};

#[cfg(test)]
mod tests {
    use super::*;
    use vox_compiler::ast::span::Span;
    use vox_compiler::hir::{DefId, HirExpr, HirModule, HirStmt, HirWorkflow};

    fn span() -> Span {
        Span { start: 0, end: 0 }
    }

    fn minimal_wf(body: Vec<HirStmt>) -> HirModule {
        let mut module = HirModule::default();
        module.workflows.push(HirWorkflow {
            id: DefId(0),
            name: "demo".to_string(),
            params: vec![],
            return_type: None,
            body,
            span: span(),
        });
        module
    }

    #[test]
    fn plan_finds_mesh_and_local_calls() {
        let body = vec![
            HirStmt::Expr {
                expr: HirExpr::Call(
                    Box::new(HirExpr::Ident("local_step".to_string(), span())),
                    vec![],
                    false,
                    span(),
                ),
                span: span(),
            },
            HirStmt::Expr {
                expr: HirExpr::With(
                    Box::new(HirExpr::Call(
                        Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                        vec![],
                        false,
                        span(),
                    )),
                    Box::new(HirExpr::ObjectLit(vec![], span())),
                    span(),
                ),
                span: span(),
            },
        ];
        let hir = minimal_wf(body);
        let p = plan_workflow_activities(&hir, "demo").expect("plan");
        assert_eq!(p.len(), 2);
        assert!(!p[0].mens);
        assert_eq!(p[0].populi_op, PopuliHttpOp::Heartbeat);
        assert!(p[1].mens);
        assert_eq!(p[1].populi_op, PopuliHttpOp::Heartbeat);
    }

    #[test]
    fn plan_mesh_snapshot_from_with_block() {
        let body = vec![HirStmt::Expr {
            expr: HirExpr::With(
                Box::new(HirExpr::Call(
                    Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                    vec![],
                    false,
                    span(),
                )),
                Box::new(HirExpr::ObjectLit(
                    vec![(
                        "mens".to_string(),
                        HirExpr::StringLit("snapshot".to_string(), span()),
                    )],
                    span(),
                )),
                span(),
            ),
            span: span(),
        }];
        let hir = minimal_wf(body);
        let p = plan_workflow_activities(&hir, "demo").expect("plan");
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].populi_op, PopuliHttpOp::Snapshot);
    }

    #[tokio::test]
    async fn interpret_workflow_journal_local_and_mesh() {
        let body = vec![
            HirStmt::Expr {
                expr: HirExpr::Call(
                    Box::new(HirExpr::Ident("local_step".to_string(), span())),
                    vec![],
                    false,
                    span(),
                ),
                span: span(),
            },
            HirStmt::Expr {
                expr: HirExpr::With(
                    Box::new(HirExpr::Call(
                        Box::new(HirExpr::Ident("mesh_ping".to_string(), span())),
                        vec![],
                        false,
                        span(),
                    )),
                    Box::new(HirExpr::ObjectLit(vec![], span())),
                    span(),
                ),
                span: span(),
            },
        ];
        let hir = minimal_wf(body);
        let journal = interpret_workflow(&hir, "demo").await.expect("interpret");
        let events: Vec<_> = journal
            .iter()
            .filter_map(|v| v.get("event").and_then(|e| e.as_str()))
            .collect();
        assert!(events.contains(&"WorkflowStarted"));
        assert!(events.contains(&"ActivityStarted"));
        assert!(events.contains(&"ActivityCompleted"));
        assert!(events.contains(&"LocalActivity"));
        assert!(events.contains(&"MeshActivity"));
        assert!(events.contains(&"WorkflowCompleted"));
    }
}
