//! Imperative statements inside function, component, and handler bodies.
//!
//! Vox uses **expression-oriented** blocks: a [`crate::ast::expr::Expr::Block`] contains statements,
//! and the block’s value is the last expression (not a separate `return` type in the AST). `ret`
//! is only modeled here as `Stmt::Return`.

use crate::ast::expr::Expr;
use crate::ast::pattern::Pattern;
use crate::ast::span::Span;
use crate::ast::types::TypeExpr;

/// Statement: binding, assignment, return, or effectful expression.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum Stmt {
    /// Let binding: `let x = 5` or `let mut x = 0`
    Let {
        /// Bound pattern (`x`, `(a, b)`, …).
        pattern: Pattern,
        /// Optional type ascription.
        type_ann: Option<TypeExpr>,
        /// Initializer expression.
        value: Expr,
        /// Whether the binding is `mut`.
        mutable: bool,
        /// Span covering the `let` statement.
        span: Span,
    },
    /// Assignment: `x = x + 1` (only valid for mut bindings)
    Assign {
        /// Assignment target l-value.
        target: Expr,
        /// New value expression.
        value: Expr,
        /// Span covering the assignment.
        span: Span,
    },
    /// Return statement: `ret value`
    Return {
        /// Returned expression, if any.
        value: Option<Expr>,
        /// Span covering `ret` / expression.
        span: Span,
    },
    /// Expression statement (an expression evaluated for its side effects)
    Expr {
        /// Evaluated expression.
        expr: Expr,
        /// Span covering the statement.
        span: Span,
    },
}

impl Stmt {
    /// Span covering this statement for diagnostics.
    pub fn span(&self) -> Span {
        match self {
            Stmt::Let { span, .. }
            | Stmt::Assign { span, .. }
            | Stmt::Return { span, .. }
            | Stmt::Expr { span, .. } => *span,
        }
    }
}
