//! Internal type representation for the type Checker.

/// Internal type representation for the type Checker.
#[derive(Debug, Clone, PartialEq)]
pub enum Ty {
    Int,
    Float,
    Str,
    Bool,
    Char,
    Unit,
    /// Bottom type for early return / break.
    Never,
    List(Box<Ty>),
    Option(Box<Ty>),
    Result(Box<Ty>),
    Stream(Box<Ty>),
    Map(Box<Ty>, Box<Ty>),
    Set(Box<Ty>),
    Fn(Vec<Ty>, Box<Ty>),
    Tuple(Vec<Ty>),
    Record(Vec<(String, Ty)>),
    Element,
    TypeVar(u32),
    GenericParam(u32),
    Named(String),
    Error,
    // Database / domain
    Database,
    Table(String, Vec<(String, Ty)>),
    Collection(String, Vec<(String, Ty)>),
    /// Handle returned by `spawn(ActorName)`; supports `.handler(...)` per registered actor.
    ActorRef(String),
}

/// Debug-oriented display for diagnostics (not a full pretty-printer).
#[must_use]
pub fn ty_display(ty: &Ty) -> String {
    format!("{ty:?}")
}
