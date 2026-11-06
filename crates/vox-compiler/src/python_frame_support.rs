//! `std.frame` signature table generated from `contracts/python/frame-support.yaml`.

/// `plan` builds another plan. `materializer` is the worker boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonFrameOpKind {
    Plan,
    Materializer,
}

/// One allowlisted `std.frame` op. Later plans read only [`python_frame_ops`].
#[derive(Debug, Clone, Copy)]
pub struct PythonFrameOp {
    pub name: &'static str,
    pub kind: PythonFrameOpKind,
    pub signature: &'static str,
    pub handler: &'static str,
    pub expected_yaml: &'static str,
    pub required_named: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/python_frame_ops.rs"));

/// The generated op table. This is the only reader of `python_frame_ops.rs`.
#[must_use]
pub fn python_frame_ops() -> &'static [PythonFrameOp] {
    PYTHON_FRAME_OPS
}

#[cfg(test)]
mod tests {
    use super::{PythonFrameOpKind, python_frame_ops};

    const FRAME_OPS: &[&str] = &[
        "scan_csv", "col", "filter", "group_by", "agg", "sort", "sum", "count", "mean", "min",
        "max", "eq", "neq", "gt", "gte", "lt", "lte", "and", "or", "collect",
    ];

    #[test]
    fn frame_support_collect_is_only_materializer() {
        let table = python_frame_ops();
        let names: Vec<&str> = table.iter().map(|op| op.name).collect();
        assert_eq!(names, FRAME_OPS);
        for op in table {
            assert_eq!(op.handler, op.name);
            assert!(
                op.signature.contains("->"),
                "{} signature must return a type",
                op.name
            );
            assert!(!op.expected_yaml.trim().is_empty());
            assert_ne!(op.expected_yaml.trim(), "null");
            let kind = if op.name == "collect" {
                PythonFrameOpKind::Materializer
            } else {
                PythonFrameOpKind::Plan
            };
            assert_eq!(op.kind, kind, "{}", op.name);
            match op.name {
                "sort" => assert_eq!(op.required_named, &["nulls"]),
                "sum" | "count" | "mean" | "min" | "max" => {
                    assert_eq!(op.required_named, &["alias"]);
                }
                _ => assert!(op.required_named.is_empty(), "{}", op.name),
            }
        }
    }
}
