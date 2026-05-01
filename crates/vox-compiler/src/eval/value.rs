use crate::hir::nodes::HirStmt;

#[derive(Debug, Clone)]
pub enum VoxValue {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    List(Vec<VoxValue>),
    Object(Vec<(String, VoxValue)>),
    Tuple(Vec<VoxValue>),
    Null,
    Fn {
        params: Vec<String>,
        body: Vec<HirStmt>,
        env: crate::eval::env::Scope,
    },
    Option(core::option::Option<Box<VoxValue>>),
    Result(core::result::Result<Box<VoxValue>, String>),
    /// An ADT variant constructor callable (not yet applied). Created by `run_module`.
    Constructor(String),
    /// An applied ADT variant value, e.g. `Applied(10, 0)`.
    Tagged { name: String, fields: Vec<VoxValue> },
    // Sentinel for control flow
    _Return(Box<VoxValue>),
    _Break,
    _Continue,
}

impl PartialEq for VoxValue {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Int(a), Self::Int(b)) => a == b,
            (Self::Float(a), Self::Float(b)) => a == b,
            (Self::Str(a), Self::Str(b)) => a == b,
            (Self::Bool(a), Self::Bool(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Object(a), Self::Object(b)) => a == b,
            (Self::Tuple(a), Self::Tuple(b)) => a == b,
            (Self::Null, Self::Null) => true,
            (Self::Option(a), Self::Option(b)) => a == b,
            (Self::Result(a), Self::Result(b)) => a == b,
            (Self::Constructor(a), Self::Constructor(b)) => a == b,
            (
                Self::Tagged { name: na, fields: fa },
                Self::Tagged { name: nb, fields: fb },
            ) => na == nb && fa == fb,
            _ => false,
        }
    }
}
