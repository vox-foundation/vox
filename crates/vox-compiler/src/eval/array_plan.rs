//! Pure `std.array` plans. Building a plan stores host data and op names.
//! It does not call the Python worker.

use std::sync::atomic::{AtomicU32, Ordering};

/// Worker calls recorded while array plans were built. Plan constructors
/// never increment this.
static PLAN_WORKER_CALLS: AtomicU32 = AtomicU32::new(0);

/// How many worker calls plan construction has recorded.
#[must_use]
pub fn plan_worker_calls() -> u32 {
    PLAN_WORKER_CALLS.load(Ordering::SeqCst)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanStep {
    pub op: &'static str,
    pub correction: Option<i64>,
    /// `sort` records `first` or `last`. Other ops leave this empty.
    pub nulls: Option<&'static str>,
    /// Host column `from_list` reads. Other ops leave this empty.
    pub column: Option<u32>,
}

/// Host numbers stored on a plan. Floats and ints stay in separate columns
/// so `i64::MAX` is not rounded through `f64`.
#[derive(Debug, Clone, PartialEq)]
enum HostColumn {
    F64(Vec<f64>),
    I64(Vec<i64>),
}

/// One host column ready to encode. Owned so the plan can be borrowed.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum HostPayload {
    F64(Vec<f64>),
    I64(Vec<i64>),
}

/// A lazy numpy plan. `mean` and `std` stay rank 0. Nothing here calls
/// `Bridge::call`.
#[derive(Debug, Clone, PartialEq)]
pub struct ArrayPlan {
    steps: Vec<PlanStep>,
    hosts: Vec<HostColumn>,
    rank: u8,
    worker_calls: u32,
}

impl ArrayPlan {
    /// Store `values` on the plan. Does not send them to the worker.
    #[must_use]
    pub fn from_list(values: Vec<f64>) -> Self {
        Self {
            steps: vec![PlanStep {
                op: "from_list",
                correction: None,
                nulls: None,
                column: Some(0),
            }],
            hosts: vec![HostColumn::F64(values)],
            rank: 1,
            worker_calls: plan_worker_calls(),
        }
    }

    /// Store host ints. Does not send them to the worker. `i64::MIN` is not
    /// constructed here; the golden only spells `i64::MAX`.
    #[must_use]
    pub(crate) fn from_ints(values: Vec<i64>) -> Self {
        Self {
            steps: vec![PlanStep {
                op: "from_list",
                correction: None,
                nulls: None,
                column: Some(0),
            }],
            hosts: vec![HostColumn::I64(values)],
            rank: 1,
            worker_calls: plan_worker_calls(),
        }
    }

    /// Append `astype`. This plan only records the op; it does not cast.
    #[must_use]
    pub fn astype(&self, _dtype: &str) -> Self {
        self.push("astype", None, None, self.rank)
    }

    /// Elementwise add. A rank-0 right-hand side broadcasts.
    #[must_use]
    pub fn add(&self, other: &Self) -> Self {
        self.combine(other, "add")
    }

    /// Elementwise subtract. A rank-0 right-hand side broadcasts.
    #[must_use]
    pub fn sub(&self, other: &Self) -> Self {
        self.combine(other, "sub")
    }

    /// Elementwise multiply. A rank-0 right-hand side broadcasts.
    #[must_use]
    pub fn mul(&self, other: &Self) -> Self {
        self.combine(other, "mul")
    }

    /// Elementwise divide. A rank-0 right-hand side broadcasts.
    #[must_use]
    pub fn div(&self, other: &Self) -> Self {
        self.combine(other, "div")
    }

    /// Append `sort`. `nulls` is stored on the step. This does not call the worker
    /// and does not pass `nulls` to `np.sort`.
    #[must_use]
    pub fn sort(&self, nulls: &str) -> Self {
        let recorded = match nulls {
            "first" => Some("first"),
            "last" => Some("last"),
            _ => None,
        };
        self.push("sort", None, recorded, self.rank)
    }

    /// 0-d mean plan. Does not call the worker.
    #[must_use]
    pub fn mean(&self) -> Self {
        self.push("mean", None, None, 0)
    }

    /// 0-d std plan. `correction` is stored on the step and is not defaulted
    /// by a worker call.
    #[must_use]
    pub fn std(&self, correction: i64) -> Self {
        self.push("std", Some(correction), None, 0)
    }

    /// `from_list`, `astype` f64, subtract `mean`, divide `std(correction)`.
    #[must_use]
    pub fn zscore(values: &[f64], correction: i64) -> Self {
        let base = Self::from_list(values.to_vec()).astype("f64");
        base.sub(&base.mean()).div(&base.std(correction))
    }

    #[must_use]
    pub fn op_names(&self) -> Vec<&'static str> {
        self.steps.iter().map(|step| step.op).collect()
    }

    #[must_use]
    pub fn host(&self) -> &[f64] {
        self.hosts
            .iter()
            .find_map(|column| match column {
                HostColumn::F64(values) => Some(values.as_slice()),
                HostColumn::I64(_) => None,
            })
            .unwrap_or(&[])
    }

    /// Host ints, when `from_list` was given a list of int.
    #[cfg(test)]
    #[must_use]
    pub(crate) fn host_ints(&self) -> Option<&[i64]> {
        self.hosts.iter().find_map(|column| match column {
            HostColumn::I64(values) => Some(values.as_slice()),
            HostColumn::F64(_) => None,
        })
    }

    /// Every host column, in `from_list` index order.
    #[must_use]
    pub(crate) fn host_columns(&self) -> Vec<HostPayload> {
        self.hosts
            .iter()
            .map(|column| match column {
                HostColumn::F64(values) => HostPayload::F64(values.clone()),
                HostColumn::I64(values) => HostPayload::I64(values.clone()),
            })
            .collect()
    }

    #[must_use]
    pub fn rank(&self) -> u8 {
        self.rank
    }

    /// Worker calls recorded on this plan. Plan builders copy the global
    /// counter and never increment it.
    #[must_use]
    pub fn worker_calls(&self) -> u32 {
        self.worker_calls
    }

    /// `nulls` stored on the `sort` step, if this plan has one.
    #[must_use]
    pub fn sort_nulls(&self) -> Option<&'static str> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "sort")
            .and_then(|step| step.nulls)
    }

    /// Append a materializer op (`item` or `shape`) without calling the worker.
    #[must_use]
    pub(crate) fn appending(&self, op: &'static str) -> Self {
        self.push(op, None, None, self.rank)
    }

    /// Steps in plan order, including `correction` and `nulls`.
    #[must_use]
    pub(crate) fn param_steps(&self) -> Vec<PlanStep> {
        self.steps.clone()
    }

    /// `correction` stored on the `std` step, if this plan has one.
    #[must_use]
    pub fn correction(&self) -> Option<i64> {
        self.steps
            .iter()
            .rev()
            .find(|step| step.op == "std")
            .and_then(|step| step.correction)
    }

    fn push(
        &self,
        op: &'static str,
        correction: Option<i64>,
        nulls: Option<&'static str>,
        rank: u8,
    ) -> Self {
        let mut next = self.clone();
        next.steps.push(PlanStep {
            op,
            correction,
            nulls,
            column: None,
        });
        next.rank = rank;
        next.worker_calls = plan_worker_calls();
        next
    }

    fn combine(&self, other: &Self, op: &'static str) -> Self {
        let mut next = self.clone();
        let offset = u32::try_from(next.hosts.len()).unwrap_or(u32::MAX);
        next.hosts.extend(other.hosts.iter().cloned());
        for step in &other.steps {
            let mut copied = *step;
            if copied.op == "from_list" {
                copied.column = Some(copied.column.unwrap_or(0).saturating_add(offset));
            }
            next.steps.push(copied);
        }
        next.steps.push(PlanStep {
            op,
            correction: None,
            nulls: None,
            column: None,
        });
        next.rank = if self.rank == 0 {
            other.rank
        } else {
            self.rank
        };
        next.worker_calls = plan_worker_calls();
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::value::VoxValue;
    use crate::python_array_support::{PythonArrayOpKind, python_array_ops};

    fn repo_yaml() -> String {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../contracts/python/ecosystem-support.yaml");
        std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
    }

    #[test]
    fn array_plan_zscore_is_pure() {
        let before = plan_worker_calls();
        let base = ArrayPlan::from_list(vec![1.0, 3.0]).astype("f64");
        let mean = base.mean();
        let std = base.std(0);
        assert_eq!(mean.rank(), 0, "mean stays 0-d");
        assert_eq!(std.rank(), 0, "std stays 0-d");
        assert_eq!(mean.worker_calls(), 0);
        assert_eq!(std.worker_calls(), 0);
        assert_eq!(std.correction(), Some(0));
        assert!(mean.op_names().contains(&"mean"));
        assert!(std.op_names().contains(&"std"));

        let plan = ArrayPlan::zscore(&[1.0, 3.0], 0);
        let names = plan.op_names();
        for op in ["from_list", "astype", "sub", "div", "mean", "std"] {
            assert!(names.contains(&op), "missing {op} in {names:?}");
        }
        assert_eq!(plan.host(), &[1.0, 3.0]);
        assert_eq!(plan.rank(), 1);
        assert_eq!(plan.worker_calls(), 0);
        assert_eq!(plan_worker_calls(), before);
        assert_eq!(
            plan_worker_calls(),
            0,
            "mean and std record zero worker calls"
        );

        match VoxValue::ArrayPlan(plan) {
            VoxValue::ArrayPlan(inner) => assert_eq!(inner.worker_calls(), 0),
            other => panic!("expected ArrayPlan, got {other:?}"),
        }

        // Expected values come from the yaml rows. A golden call does not fill them.
        let raw = repo_yaml();
        assert!(
            !raw.contains("py_array_stats"),
            "the registry must not be filled from the golden"
        );
        assert_eq!(raw.matches("empty: .nan").count(), 2);
        let doc: serde_yaml::Value = serde_yaml::from_str(&raw).expect("yaml");
        let ops = doc
            .get("ops")
            .and_then(serde_yaml::Value::as_sequence)
            .expect("ops");
        let yaml_names: Vec<&str> = ops
            .iter()
            .map(|op| op.get("name").and_then(serde_yaml::Value::as_str).unwrap())
            .collect();
        assert_eq!(
            yaml_names,
            [
                "from_list",
                "astype",
                "add",
                "sub",
                "mul",
                "div",
                "mean",
                "std",
                "sort",
                "to_list",
                "item",
                "shape",
            ]
        );
        for op in ops {
            let name = op.get("name").and_then(serde_yaml::Value::as_str).unwrap();
            let expected = op
                .get("expected")
                .unwrap_or_else(|| panic!("{name} missing expected"));
            assert!(
                expected.is_mapping(),
                "{name} expected must be a non-null object, got {expected:?}"
            );
            let handler = op.get("handler").and_then(serde_yaml::Value::as_str);
            assert_eq!(handler, Some(name));
            assert!(
                op.get("signature")
                    .and_then(serde_yaml::Value::as_str)
                    .is_some()
            );
        }

        let table = python_array_ops();
        let table_names: Vec<&str> = table.iter().map(|op| op.name).collect();
        assert_eq!(table_names, yaml_names);
        for op in table {
            assert!(!op.expected_yaml.is_empty());
            assert_ne!(op.expected_yaml.trim(), "null");
            assert!(!op.signature.is_empty());
            assert_eq!(op.handler, op.name);
            match op.name {
                "std" => assert_eq!(op.required_named, &["correction"]),
                "sort" => assert_eq!(op.required_named, &["nulls"]),
                _ => assert!(op.required_named.is_empty(), "{}", op.name),
            }
            let kind = if matches!(op.name, "to_list" | "item" | "shape") {
                PythonArrayOpKind::Materializer
            } else {
                PythonArrayOpKind::Plan
            };
            assert_eq!(op.kind, kind);
        }
        let to_list = table
            .iter()
            .find(|op| op.name == "to_list")
            .expect("to_list");
        let to_list_expected: serde_yaml::Value =
            serde_yaml::from_str(to_list.expected_yaml).expect("to_list expected");
        assert_eq!(
            to_list_expected
                .get("host")
                .and_then(serde_yaml::Value::as_str),
            Some("list_f64")
        );
        assert_eq!(
            f64_list(to_list_expected.get("identity").expect("identity")),
            vec![1.0, 3.0]
        );
        assert_eq!(
            f64_list(to_list_expected.get("zscore").expect("zscore")),
            vec![-1.0, 1.0]
        );
        assert!(to_list.signature.contains("Result[list[float]]"));
        assert!(raw.contains("identity: [1.0, 3.0]"));
        assert!(raw.contains("zscore: [-1.0, 1.0]"));

        let mean_row = table.iter().find(|op| op.name == "mean").expect("mean");
        let mean_expected: serde_yaml::Value =
            serde_yaml::from_str(mean_row.expected_yaml).expect("mean expected");
        assert_eq!(
            f64_list(mean_expected.get("values").expect("mean values")),
            vec![2.0]
        );
        assert!(
            mean_expected
                .get("empty")
                .and_then(serde_yaml::Value::as_f64)
                .is_some_and(f64::is_nan)
        );
        let std_row = table.iter().find(|op| op.name == "std").expect("std");
        let std_expected: serde_yaml::Value =
            serde_yaml::from_str(std_row.expected_yaml).expect("std expected");
        assert_eq!(
            f64_list(std_expected.get("values").expect("std values")),
            vec![1.0]
        );
        assert!(
            std_expected
                .get("empty")
                .and_then(serde_yaml::Value::as_f64)
                .is_some_and(f64::is_nan)
        );
    }

    #[test]
    fn sort_is_a_plan() {
        let before = plan_worker_calls();
        let plan = ArrayPlan::from_list(vec![3.0, f64::NAN, 1.0]).sort("first");
        match VoxValue::ArrayPlan(plan.clone()) {
            VoxValue::ArrayPlan(inner) => {
                assert_eq!(inner.worker_calls(), 0);
                assert!(inner.op_names().contains(&"sort"), "{:?}", inner.op_names());
                assert_eq!(inner.sort_nulls(), Some("first"));
                assert_eq!(inner.host().len(), 3);
                assert_eq!(inner.host()[0], 3.0);
                assert!(
                    inner.host()[1].is_nan(),
                    "sort stores the input; it does not reorder"
                );
                assert_eq!(inner.host()[2], 1.0);
            }
            other => panic!("expected ArrayPlan, got {other:?}"),
        }
        let last = ArrayPlan::from_list(vec![3.0, f64::NAN, 1.0]).sort("last");
        assert_eq!(last.sort_nulls(), Some("last"));
        assert_eq!(last.worker_calls(), 0);
        assert!(matches!(VoxValue::ArrayPlan(last), VoxValue::ArrayPlan(_)));
        assert_eq!(plan_worker_calls(), before);
        assert_eq!(plan.worker_calls(), 0);
    }

    #[test]
    fn add_mul_are_plans() {
        let before = plan_worker_calls();
        let left = ArrayPlan::from_list(vec![1.0, 3.0]);
        let sum = left.add(&ArrayPlan::from_list(vec![1.0, 1.0]));
        let prod = ArrayPlan::from_list(vec![2.0, 3.0]).mul(&ArrayPlan::from_list(vec![4.0, 5.0]));
        let broadcast = left.add(&left.mean());
        assert!(sum.op_names().contains(&"add"), "{:?}", sum.op_names());
        assert!(prod.op_names().contains(&"mul"), "{:?}", prod.op_names());
        assert_eq!(broadcast.rank(), 1, "a 0-d addend broadcasts");
        assert_eq!(sum.worker_calls(), 0);
        assert_eq!(prod.worker_calls(), 0);
        assert_eq!(broadcast.worker_calls(), 0);
        assert_eq!(plan_worker_calls(), before);
        assert!(matches!(VoxValue::ArrayPlan(sum), VoxValue::ArrayPlan(_)));
    }

    fn f64_list(value: &serde_yaml::Value) -> Vec<f64> {
        value
            .as_sequence()
            .unwrap_or_else(|| panic!("expected a list, got {value:?}"))
            .iter()
            .map(|item| {
                item.as_f64()
                    .unwrap_or_else(|| panic!("expected a float, got {item:?}"))
            })
            .collect()
    }
}
