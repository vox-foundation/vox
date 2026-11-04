//! Pure `std.frame` plans. Building a plan stores a path and op names.
//! It does not call the Python worker.

use std::sync::atomic::{AtomicU32, Ordering};

/// Worker calls recorded while frame plans were built. Plan constructors
/// never increment this.
static PLAN_WORKER_CALLS: AtomicU32 = AtomicU32::new(0);

/// How many worker calls frame-plan construction has recorded.
#[must_use]
pub fn plan_worker_calls() -> u32 {
    PLAN_WORKER_CALLS.load(Ordering::SeqCst)
}

/// One expression inside a filter or aggregation. Constructors store the
/// op name. They do not call the worker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameExpr {
    op: &'static str,
    column: Option<String>,
    literal: Option<String>,
    alias_name: Option<String>,
    child: Option<Box<FrameExpr>>,
}

impl FrameExpr {
    /// `col("name")`. Does not call the worker.
    #[must_use]
    pub fn col(name: &str) -> Self {
        Self {
            op: "col",
            column: Some(name.to_string()),
            literal: None,
            alias_name: None,
            child: None,
        }
    }

    /// Comparison `eq`. The receiver is the child expression.
    #[must_use]
    pub fn eq(&self, value: &str) -> Self {
        self.with_literal("eq", value)
    }

    /// Aggregation `sum`. The receiver is the child expression.
    #[must_use]
    pub fn sum(&self) -> Self {
        self.wrap("sum")
    }

    /// Record `.alias("name")` on this expression. Not a separate plan step.
    #[must_use]
    pub fn alias(&self, name: &str) -> Self {
        let mut next = self.clone();
        next.alias_name = Some(name.to_string());
        next
    }

    #[must_use]
    pub fn op(&self) -> &'static str {
        self.op
    }

    #[must_use]
    pub fn column(&self) -> Option<&str> {
        self.column.as_deref()
    }

    #[must_use]
    pub fn literal(&self) -> Option<&str> {
        self.literal.as_deref()
    }

    #[must_use]
    pub fn alias_name(&self) -> Option<&str> {
        self.alias_name.as_deref()
    }

    #[must_use]
    pub fn child(&self) -> Option<&FrameExpr> {
        self.child.as_deref()
    }

    fn wrap(&self, op: &'static str) -> Self {
        Self {
            op,
            column: None,
            literal: None,
            alias_name: None,
            child: Some(Box::new(self.clone())),
        }
    }

    fn with_literal(&self, op: &'static str, value: &str) -> Self {
        Self {
            op,
            column: None,
            literal: Some(value.to_string()),
            alias_name: None,
            child: Some(Box::new(self.clone())),
        }
    }
}

/// One step on a [`FramePlan`], ready for `frame_plan_params`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameStep {
    pub op: &'static str,
    pub column: Option<String>,
    pub nulls: Option<&'static str>,
    pub keys: Vec<String>,
    pub exprs: Vec<FrameExpr>,
}

/// A lazy polars plan. Constructors copy the worker counter and never
/// increment it. `collect` is not a step on this plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FramePlan {
    path: String,
    steps: Vec<FrameStep>,
    worker_calls: u32,
}

impl FramePlan {
    /// Store `path` on the plan. The caller supplies the canonical path.
    /// This does not touch the filesystem and does not call the worker.
    #[must_use]
    pub fn scan_csv(path: &str) -> Self {
        Self {
            path: path.to_string(),
            steps: vec![FrameStep {
                op: "scan_csv",
                column: None,
                nulls: None,
                keys: Vec::new(),
                exprs: Vec::new(),
            }],
            worker_calls: plan_worker_calls(),
        }
    }

    /// Append `filter`. Does not call the worker.
    #[must_use]
    pub fn filter(&self, expr: &FrameExpr) -> Self {
        self.push("filter", None, None, Vec::new(), vec![expr.clone()])
    }

    /// Append `group_by`. Does not call the worker.
    #[must_use]
    pub fn group_by(&self, keys: &[&str]) -> Self {
        self.push(
            "group_by",
            None,
            None,
            keys.iter().copied().map(str::to_string).collect(),
            Vec::new(),
        )
    }

    /// Append `agg`. Does not call the worker.
    #[must_use]
    pub fn agg(&self, exprs: &[FrameExpr]) -> Self {
        self.push("agg", None, None, Vec::new(), exprs.to_vec())
    }

    /// Append `sort`. The column string and the nulls string are stored
    /// separately. `nulls` is recorded only for `first` and `last`.
    /// This does not call the worker.
    #[must_use]
    pub fn sort(&self, column: &str, nulls: &str) -> Self {
        let recorded = match nulls {
            "first" => Some("first"),
            "last" => Some("last"),
            _ => None,
        };
        self.push(
            "sort",
            Some(column.to_string()),
            recorded,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Path stored by `scan_csv`.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    #[must_use]
    pub fn op_names(&self) -> Vec<&'static str> {
        self.steps.iter().map(|step| step.op).collect()
    }

    /// Worker calls recorded on this plan. Plan builders copy the global
    /// counter and never increment it.
    #[must_use]
    pub fn worker_calls(&self) -> u32 {
        self.worker_calls
    }

    /// Expression stored on the last `filter` step.
    #[must_use]
    pub fn filter_expr(&self) -> Option<&FrameExpr> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "filter")
            .and_then(|step| step.exprs.first())
    }

    /// Keys stored on the last `group_by` step.
    #[must_use]
    pub fn group_keys(&self) -> &[String] {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "group_by")
            .map(|step| step.keys.as_slice())
            .unwrap_or(&[])
    }

    /// Expressions stored on the last `agg` step.
    #[must_use]
    pub fn agg_exprs(&self) -> &[FrameExpr] {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "agg")
            .map(|step| step.exprs.as_slice())
            .unwrap_or(&[])
    }

    /// Column string stored on the last `sort` step.
    #[must_use]
    pub fn sort_column(&self) -> Option<&str> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "sort")
            .and_then(|step| step.column.as_deref())
    }

    /// `nulls` stored on the last `sort` step. Only `first` and `last`.
    #[must_use]
    pub fn sort_nulls(&self) -> Option<&'static str> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "sort")
            .and_then(|step| step.nulls)
    }

    /// Steps in plan order for the worker JSON.
    #[must_use]
    pub(crate) fn param_steps(&self) -> Vec<FrameStep> {
        self.steps.clone()
    }

    fn push(
        &self,
        op: &'static str,
        column: Option<String>,
        nulls: Option<&'static str>,
        keys: Vec<String>,
        exprs: Vec<FrameExpr>,
    ) -> Self {
        let mut next = self.clone();
        next.steps.push(FrameStep {
            op,
            column,
            nulls,
            keys,
            exprs,
        });
        next.worker_calls = plan_worker_calls();
        next
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameExpr, FramePlan, plan_worker_calls};
    use crate::python_frame_support::{PythonFrameOpKind, python_frame_ops};

    #[test]
    fn frame_plan_collect_is_one_round_trip() {
        let before = plan_worker_calls();
        let pred = FrameExpr::col("region").eq("west");
        let revenue = FrameExpr::col("amount").sum().alias("revenue");
        let plan = FramePlan::scan_csv("fixtures/py_frame_report.csv")
            .filter(&pred)
            .group_by(&["month"])
            .agg(&[revenue])
            .sort("month", "last");

        assert_eq!(plan.worker_calls(), 0);
        assert_eq!(plan_worker_calls(), before);
        assert_eq!(
            plan_worker_calls(),
            0,
            "plan constructors record zero worker calls"
        );
        assert_eq!(
            plan.op_names(),
            ["scan_csv", "filter", "group_by", "agg", "sort"]
        );
        assert!(
            !plan.op_names().contains(&"collect"),
            "collect is not stored on the plan"
        );
        assert_eq!(plan.path(), "fixtures/py_frame_report.csv");

        let filter = plan.filter_expr().expect("filter step");
        assert_eq!(filter.op(), "eq");
        assert_eq!(filter.literal(), Some("west"));
        let column = filter.child().expect("eq child");
        assert_eq!(column.op(), "col");
        assert_eq!(column.column(), Some("region"));

        assert_eq!(plan.group_keys(), &["month".to_string()]);
        let agg = plan.agg_exprs();
        assert_eq!(agg.len(), 1);
        assert_eq!(agg[0].op(), "sum");
        assert_eq!(agg[0].alias_name(), Some("revenue"));
        assert_eq!(agg[0].child().and_then(FrameExpr::column), Some("amount"));

        assert_eq!(plan.sort_column(), Some("month"));
        assert_eq!(plan.sort_nulls(), Some("last"));

        let first = plan.sort("month", "first");
        assert_eq!(first.sort_column(), Some("month"));
        assert_eq!(first.sort_nulls(), Some("first"));
        assert_eq!(first.worker_calls(), 0);
        let dropped = plan.sort("month", "middle");
        assert_eq!(dropped.sort_column(), Some("month"));
        assert_eq!(dropped.sort_nulls(), None);
        assert_eq!(dropped.worker_calls(), 0);

        let table = python_frame_ops();
        let materializers: Vec<&str> = table
            .iter()
            .filter(|op| op.kind == PythonFrameOpKind::Materializer)
            .map(|op| op.name)
            .collect();
        assert_eq!(materializers, ["collect"]);
    }
}
