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
            _ => false,
        }
    }
}
