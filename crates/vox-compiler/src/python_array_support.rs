//! `std.array` signature table generated from `contracts/python/ecosystem-support.yaml`.

/// `plan` builds another plan. `materializer` is the worker boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PythonArrayOpKind {
    Plan,
    Materializer,
}

/// One allowlisted `std.array` op. Later plans read only [`python_array_ops`].
#[derive(Debug, Clone, Copy)]
pub struct PythonArrayOp {
    pub name: &'static str,
    pub kind: PythonArrayOpKind,
    pub signature: &'static str,
    pub handler: &'static str,
    pub expected_yaml: &'static str,
    pub required_named: &'static [&'static str],
}

include!(concat!(env!("OUT_DIR"), "/python_array_ops.rs"));

/// The generated op table. This is the only reader of `python_array_ops.rs`.
#[must_use]
pub fn python_array_ops() -> &'static [PythonArrayOp] {
    PYTHON_ARRAY_OPS
}
