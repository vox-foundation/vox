use crate::ast::span::Span;

/// Unique identifier for definitions within a module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DefId(
    /// Opaque numeric id assigned during HIR lowering.
    pub u32,
);
/// A function parameter in HIR.
#[derive(Debug, Clone)]
pub struct HirParam {
    /// Parameter definition id.
    pub id: DefId,
    /// Binding name.
    pub name: String,
    /// Optional type annotation.
    pub type_ann: Option<HirType>,
    /// Default value expression.
    pub default: Option<HirExpr>,
    /// Span covering this parameter.
    pub span: Span,
}

/// Type representation in HIR (resolved from [`crate::ast::types::TypeExpr`]).
#[derive(Debug, Clone, PartialEq)]
pub enum HirType {
    /// Named scalar or path type (`int`, `User`).
    Named(String),
    /// Generic application (`list[T]`).
    Generic(String, Vec<HirType>),
    /// Function type (`fn(a, b) -> c`).
    Function(Vec<HirType>, Box<HirType>),
    /// Tuple type.
    Tuple(Vec<HirType>),
    /// Unit / void.
    Unit,
}

/// Expression in HIR (mirrors AST but with resolved names).
#[derive(Debug, Clone)]
pub enum HirExpr {
    /// Integer literal.
    IntLit(i64, Span),
    /// Floating-point literal.
    FloatLit(f64, Span),
    /// String literal.
    StringLit(String, Span),
    /// Boolean literal.
    BoolLit(bool, Span),
    /// Local or global identifier.
    Ident(String, Span),
    /// Object literal `{ k: v }`.
    ObjectLit(Vec<(String, HirExpr)>, Span),
    /// List literal.
    ListLit(Vec<HirExpr>, Span),
    /// Tuple literal.
    TupleLit(Vec<HirExpr>, Span),
    /// Binary operator application.
    Binary(HirBinOp, Box<HirExpr>, Box<HirExpr>, Span),
    /// Unary operator application.
    Unary(HirUnOp, Box<HirExpr>, Span),
    /// Call; `bool` indicates tail-call hint when used by backend.
    Call(Box<HirExpr>, Vec<HirArg>, bool, Span),
    /// Method call `obj.method(args)`.
    MethodCall(Box<HirExpr>, String, Vec<HirArg>, Span),
    /// Field projection.
    FieldAccess(Box<HirExpr>, String, Span),
    /// `match` expression.
    Match(Box<HirExpr>, Vec<HirMatchArm>, Span),
    /// `if` with optional else.
    If(Box<HirExpr>, Vec<HirStmt>, Option<Vec<HirStmt>>, Span),
    /// `for` loop expression (often JSX).
    For(String, Box<HirExpr>, Box<HirExpr>, Span),
    /// Lambda / closure.
    Lambda(Vec<HirParam>, Option<HirType>, Box<HirExpr>, Span),
    /// Pipeline `a |> b`.
    Pipe(Box<HirExpr>, Box<HirExpr>, Span),
    /// `spawn` actor constructor.
    Spawn(Box<HirExpr>, Span),
    /// `with` options wrapper.
    With(Box<HirExpr>, Box<HirExpr>, Span),
    /// JSX element with children.
    Jsx(HirJsxElement),
    /// Self-closing JSX.
    JsxSelfClosing(HirJsxSelfClosing),
    /// Statement block used as expression.
    Block(Vec<HirStmt>, Span),
}

/// Named or positional call argument.
#[derive(Debug, Clone)]
pub struct HirArg {
    /// Label for named args (`json=`); `None` for positional.
    pub name: Option<String>,
    /// Argument value.
    pub value: HirExpr,
}

/// Binary operators in HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirBinOp {
    /// `+`
    Add,
    /// `-`
    Sub,
    /// `*`
    Mul,
    /// `/`
    Div,
    /// `<`
    Lt,
    /// `>`
    Gt,
    /// `<=`
    Lte,
    /// `>=`
    Gte,
    /// `and`
    And,
    /// `or`
    Or,
    /// `is`
    Is,
    /// `isnt`
    Isnt,
    /// `|>`
    Pipe,
}

/// Unary operators in HIR.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HirUnOp {
    /// `not`
    Not,
    /// `-`
    Neg,
}

/// Single `match` arm.
#[derive(Debug, Clone)]
pub struct HirMatchArm {
    /// Matched pattern.
    pub pattern: HirPattern,
    /// Optional guard.
    pub guard: Option<Box<HirExpr>>,
    /// Arm body.
    pub body: Box<HirExpr>,
    /// Span covering the arm.
    pub span: Span,
}

/// Pattern after lowering (names resolved where applicable).
#[derive(Debug, Clone)]
pub enum HirPattern {
    /// Bind identifier.
    Ident(String, Span),
    /// Tuple destructuring.
    Tuple(Vec<HirPattern>, Span),
    /// ADT constructor pattern.
    Constructor(String, Vec<HirPattern>, Span),
    /// `_` wildcard.
    Wildcard(Span),
    /// Literal pattern.
    Literal(Box<HirExpr>, Span),
}

/// Statement in HIR.
#[derive(Debug, Clone)]
pub enum HirStmt {
    /// `let` / `let mut` binding.
    Let {
        /// Bound pattern.
        pattern: HirPattern,
        /// Optional type ascription.
        type_ann: Option<HirType>,
        /// Initializer.
        value: HirExpr,
        /// `mut` flag.
        mutable: bool,
        /// Span.
        span: Span,
    },
    /// Assignment.
    Assign {
        /// Target l-value.
        target: HirExpr,
        /// New value.
        value: HirExpr,
        /// Span covering the assignment.
        span: Span,
    },
    /// `ret` statement.
    Return {
        /// Return value, if any.
        value: Option<HirExpr>,
        /// Span covering `ret` / value.
        span: Span,
    },
    /// Expression statement.
    Expr {
        /// Evaluated expression.
        expr: HirExpr,
        /// Span covering the statement.
        span: Span,
    },
}

/// JSX element with children.
#[derive(Debug, Clone)]
pub struct HirJsxElement {
    /// Tag name.
    pub tag: String,
    /// Attributes.
    pub attributes: Vec<HirJsxAttr>,
    /// Child expressions.
    pub children: Vec<HirExpr>,
    /// Span covering the element.
    pub span: Span,
}

/// Self-closing JSX element.
#[derive(Debug, Clone)]
pub struct HirJsxSelfClosing {
    /// Tag name.
    pub tag: String,
    /// Attributes.
    pub attributes: Vec<HirJsxAttr>,
    /// Span covering `<tag .../>`.
    pub span: Span,
}

/// JSX attribute.
#[derive(Debug, Clone)]
pub struct HirJsxAttr {
    /// Attribute name.
    pub name: String,
    /// Attribute value expression.
    pub value: HirExpr,
}
