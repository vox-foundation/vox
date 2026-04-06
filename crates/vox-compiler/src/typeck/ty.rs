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
    Decimal,
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
    /// Opaque import symbol while external bindings remain unresolved.
    ImportPlaceholder(String),
    Error,
    // Database / domain
    Database,
    Table(String, Vec<(String, Ty)>),
    Collection(String, Vec<(String, Ty)>),
    /// Handle returned by `spawn(ActorName)`; supports `.handler(...)` per registered actor.
    ActorRef(String),
}

impl Ty {
    /// Deterministic, transport-friendly signature used in contracts and diagnostics.
    #[must_use]
    pub fn signature(&self) -> String {
        match self {
            Ty::Int => "int".to_string(),
            Ty::Float => "float".to_string(),
            Ty::Str => "str".to_string(),
            Ty::Bool => "bool".to_string(),
            Ty::Char => "char".to_string(),
            Ty::Unit => "Unit".to_string(),
            Ty::Decimal => "dec".to_string(),
            Ty::Never => "never".to_string(),
            Ty::List(inner) => format!("List[{}]", inner.signature()),
            Ty::Option(inner) => format!("Option[{}]", inner.signature()),
            Ty::Result(inner) => format!("Result[{}]", inner.signature()),
            Ty::Stream(inner) => format!("Stream[{}]", inner.signature()),
            Ty::Map(k, v) => format!("Map[{}, {}]", k.signature(), v.signature()),
            Ty::Set(inner) => format!("Set[{}]", inner.signature()),
            Ty::Fn(params, ret) => {
                let params = params
                    .iter()
                    .map(Ty::signature)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("fn({params}) -> {}", ret.signature())
            }
            Ty::Tuple(elements) => {
                let elems = elements
                    .iter()
                    .map(Ty::signature)
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("({elems})")
            }
            Ty::Record(fields) => {
                let fields = fields
                    .iter()
                    .map(|(n, t)| format!("{n}: {}", t.signature()))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{fields}}}")
            }
            Ty::Element => "Element".to_string(),
            Ty::TypeVar(id) => format!("t{id}"),
            Ty::GenericParam(id) => format!("g{id}"),
            Ty::Named(name) => name.clone(),
            Ty::ImportPlaceholder(name) => format!("Import<{name}>"),
            Ty::Error => "Error".to_string(),
            Ty::Database => "Database".to_string(),
            Ty::Table(name, _) => format!("Table<{name}>"),
            Ty::Collection(name, _) => format!("Collection<{name}>"),
            Ty::ActorRef(name) => format!("ActorRef<{name}>"),
        }
    }
}

/// Debug-oriented display for diagnostics (not a full pretty-printer).
#[must_use]
pub fn ty_display(ty: &Ty) -> String {
    ty.signature()
}
