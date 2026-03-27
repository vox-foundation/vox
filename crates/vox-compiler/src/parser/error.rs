use crate::ast::span::Span;

/// High-level parse failure category (stable for tooling; see `docs/src/reference/parser-ambiguity-inventory.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ParseErrorClass {
    /// Generic / uncategorized until call sites adopt a finer class.
    #[default]
    Other,
    /// Token mismatch in `Parser::expect`.
    ExpectToken,
    /// Unknown or misplaced top-level construct.
    TopLevel,
    /// Declaration / attribute head or tail.
    Declaration,
    /// Misplaced or unknown token inside a Path C / `@component` reactive body (`state`, `view:`, …).
    ReactiveComponentMember,
    Expression,
    Statement,
    TypeExpr,
}

/// A parse error with detailed context.
#[derive(Debug, Clone)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub expected: Vec<String>,
    pub found: Option<String>,
    pub class: ParseErrorClass,
}

impl ParseError {
    /// Build a parse diagnostic (span + message + optional expected/found hints).
    #[must_use]
    pub fn new(
        span: Span,
        message: impl Into<String>,
        expected: Vec<String>,
        found: Option<String>,
    ) -> Self {
        Self::classified(span, message, expected, found, ParseErrorClass::Other)
    }

    /// Same as [`ParseError::new`] with an explicit [`ParseErrorClass`].
    #[must_use]
    pub fn classified(
        span: Span,
        message: impl Into<String>,
        expected: Vec<String>,
        found: Option<String>,
        class: ParseErrorClass,
    ) -> Self {
        Self {
            message: message.into(),
            span,
            expected,
            found,
            class,
        }
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)?;
        if !self.expected.is_empty() {
            write!(f, " (expected: {})", self.expected.join(", "))?;
        }
        if let Some(ref found) = self.found {
            write!(f, " (found: {found})")?;
        }
        Ok(())
    }
}
