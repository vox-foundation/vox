// Primary expressions, postfix, control/lambda forms.

use super::super::Parser;
use crate::ast::expr::{
    Arg, Expr, JsxAttribute, JsxElement, JsxSelfClosingElement, MatchArm, UnOp,
};
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

impl Parser {
    pub(crate) fn parse_primary(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        let mut expr = match self.peek().clone() {
            Token::IntLit(v) => {
                self.advance();
                Expr::IntLit {
                    value: v,
                    span: start,
                }
            }
            Token::FloatLit(v) => {
                self.advance();
                Expr::FloatLit {
                    value: v,
                    span: start,
                }
            }
            Token::StringLit(s) | Token::SingleStringLit(s) => {
                self.advance();
                Expr::StringLit {
                    value: s,
                    span: start,
                }
            }
            Token::DecLit(s) => {
                self.advance();
                Expr::DecimalLit {
                    value: s,
                    span: start,
                }
            }

            Token::True => {
                self.advance();
                Expr::BoolLit {
                    value: true,
                    span: start,
                }
            }
            Token::False => {
                self.advance();
                Expr::BoolLit {
                    value: false,
                    span: start,
                }
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_primary()?;
                Expr::Unary {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                    span: start.merge(self.span()),
                }
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_primary()?;
                Expr::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                    span: start.merge(self.span()),
                }
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                let paren_expr = if self.eat(&Token::Comma) {
                    let mut elems = vec![e];
                    loop {
                        self.skip_newlines();
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.skip_newlines();
                    if self.expect(&Token::RParen).is_err() {
                        return Err(());
                    }
                    Expr::TupleLit {
                        elements: elems,
                        span: start.merge(self.span()),
                    }
                } else {
                    self.skip_newlines();
                    if self.expect(&Token::RParen).is_err() {
                        return Err(());
                    }
                    e
                };

                if self.eat(&Token::FatArrow) {
                    let mut params = Vec::new();
                    match paren_expr {
                        Expr::Ident { name, span } => {
                            params.push(crate::ast::expr::Param {
                                name,
                                type_ann: None,
                                default: None,
                                span,
                            });
                        }
                        Expr::TupleLit { elements, .. } => {
                            for elem in elements {
                                match elem {
                                    Expr::Ident { name, span } => {
                                        params.push(crate::ast::expr::Param {
                                            name,
                                            type_ann: None,
                                            default: None,
                                            span,
                                        });
                                    }
                                    _ => {
                                        self.errors.push(ParseError::classified(
                                            elem.span(),
                                            "Expected identifier in lambda parameters",
                                            vec![],
                                            None,
                                            ParseErrorClass::Expression,
                                        ));
                                        return Err(());
                                    }
                                }
                            }
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                paren_expr.span(),
                                "Expected identifier or tuple in lambda parameters",
                                vec![],
                                None,
                                ParseErrorClass::Expression,
                            ));
                            return Err(());
                        }
                    }
                    let body = self.parse_expr()?;
                    Expr::Lambda {
                        params,
                        return_type: None,
                        body: Box::new(body),
                        span: start.merge(self.span()),
                    }
                } else {
                    paren_expr
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                    self.skip_newlines();
                    if matches!(self.peek(), Token::RBracket | Token::Eof) {
                        break;
                    }
                    elems.push(self.parse_expr()?);
                    self.skip_newlines();
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.skip_newlines();
                self.expect(&Token::RBracket)?;
                Expr::ListLit {
                    elements: elems,
                    span: start.merge(self.span()),
                }
            }
            Token::LBrace => self.parse_brace_expr()?,
            Token::Match => self.parse_match()?,
            Token::If => self.parse_if()?,
            Token::For => self.parse_for()?,
            Token::Fn => self.parse_lambda()?,
            Token::Spawn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let target = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Expr::Spawn {
                    target: Box::new(target),
                    span: start.merge(self.span()),
                }
            }
            // VUV: angle-bracket JSX (`<tag attr=...>`) was retired as a parser entry point.
            // View calls are now `Ident(kwargs) { children }`. Hitting `<` here is a real
            // less-than usage in expression context — fall through to the error path so we
            // don't silently consume HTML-shaped source.
            Token::Ident(name) | Token::TypeIdent(name) => {
                self.advance();
                if self.eat(&Token::FatArrow) {
                    let body = self.parse_expr()?;
                    Expr::Lambda {
                        params: vec![crate::ast::expr::Param {
                            name: name.clone(),
                            type_ann: None,
                            default: None,
                            span: start,
                        }],
                        return_type: None,
                        body: Box::new(body),
                        span: start.merge(self.span()),
                    }
                } else {
                    Expr::Ident { name, span: start }
                }
            }
            Token::Env => {
                self.advance();
                Expr::Ident {
                    name: "env".to_string(),
                    span: start,
                }
            }
            Token::To => {
                self.advance();
                Expr::Ident {
                    name: "to".to_string(),
                    span: start,
                }
            }
            Token::Http => {
                self.advance();
                Expr::Ident {
                    name: "http".to_string(),
                    span: start,
                }
            }
            _ => {
                self.errors.push(ParseError::classified(
                    start,
                    format!("Unexpected token in expression: {}", self.peek()),
                    vec![],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Expression,
                ));
                return Err(());
            }
        };
        // Postfix: calls, field access, method calls
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen)?;
                    // VUV view-call form: `Ident(args) { children }` parses as JSX.
                    // Trigger requires (a) callee is a bare Ident (no method chains, no field access)
                    // and (b) next non-newline token is `{`. Sugars to Expr::Jsx so HIR / web_ir /
                    // codegen are untouched. Positional args are rejected — view calls are kw-only.
                    //
                    // Capitalized-Ident calls without a trailing block are also sugared into
                    // Expr::JsxSelfClosing — `Component()` ≡ `Component() {}` ≡ `<Component/>`.
                    // This matches React's tag-naming convention and avoids forcing every
                    // self-closing component invocation to write `() {}`.
                    if let Expr::Ident { name: tag, .. } = &expr {
                        let mut peek_pos = self.pos;
                        while peek_pos < self.tokens.len()
                            && matches!(self.tokens[peek_pos].token, Token::Newline)
                        {
                            peek_pos += 1;
                        }
                        let is_brace_next = matches!(
                            self.tokens.get(peek_pos).map(|t| &t.token),
                            Some(Token::LBrace)
                        );
                        let starts_uppercase =
                            tag.chars().next().is_some_and(|c| c.is_ascii_uppercase());
                        let is_view_callee =
                            starts_uppercase || crate::web_ir::primitives::is_primitive(tag);
                        let all_named = args.iter().all(|a| a.name.is_some());
                        // Trailing-block-as-children sugar fires only when the call shape is
                        // unambiguously a view-call: callee is a recognized view-callee
                        // (capitalized component or primitive), all args are named, AND a `{`
                        // follows. Otherwise the `{` belongs to an outer construct (e.g.
                        // `if !has_capability(cap) {`), and we must NOT consume it as children.
                        if is_brace_next && is_view_callee && all_named {
                            let tag = tag.clone();
                            let attributes = self.view_args_to_attrs(args)?;
                            self.skip_newlines();
                            self.expect(&Token::LBrace)?;
                            let children = self.parse_view_children()?;
                            self.expect(&Token::RBrace)?;
                            expr = Expr::Jsx(JsxElement {
                                tag,
                                attributes,
                                children,
                                span: start.merge(self.span()),
                            });
                            continue;
                        } else if (starts_uppercase
                            || crate::web_ir::primitives::is_primitive(tag)
                            || is_known_html_view_tag(tag))
                            && args.iter().all(|a| a.name.is_some())
                        {
                            // Three view-call self-closing triggers, all requiring all-named args:
                            //   1. Capitalized callee (`Component(...)`) — React component shape.
                            //   2. Recognized UI primitive (`row(...)`, `panel(...)`).
                            //   3. Recognized raw HTML element from a curated allowlist
                            //      (`input(attr_type="checkbox")`, `select(...)`, etc.). The
                            //      allowlist guards against ordinary lowercase function calls
                            //      like `fetch(timeout=5)` accidentally lowering to JSX.
                            // Positional args (enum/type constructors like `Some(x)`) always fall
                            // through to a regular Expr::Call below.
                            let tag = tag.clone();
                            let attributes = self.view_args_to_attrs(args)?;
                            expr = Expr::JsxSelfClosing(JsxSelfClosingElement {
                                tag,
                                attributes,
                                span: start.merge(self.span()),
                            });
                            continue;
                        }
                    }
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span: start.merge(self.span()),
                    };
                }
                Token::Dot => {
                    self.advance();
                    let field = self.parse_ident_name()?;
                    if self.eat(&Token::LParen) {
                        let args = self.parse_args()?;
                        self.expect(&Token::RParen)?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span: start.merge(self.span()),
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span: start.merge(self.span()),
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    pub(crate) fn parse_args(&mut self) -> Result<Vec<Arg>, ()> {
        let mut args = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            // Check for named arg: name=value
            if let Token::Ident(name) = self.peek().clone() {
                let saved = self.pos;
                self.advance();
                if self.eat(&Token::Eq) {
                    let value = self.parse_expr()?;
                    args.push(Arg {
                        name: Some(name),
                        value,
                    });
                    self.skip_newlines();
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                    self.skip_newlines();
                    continue;
                }
                self.pos = saved; // backtrack
            }
            let value = self.parse_expr()?;
            args.push(Arg { name: None, value });
            self.skip_newlines();
            if !self.eat(&Token::Comma) {
                break;
            }
            self.skip_newlines();
        }
        Ok(args)
    }

    pub(crate) fn parse_brace_expr(&mut self) -> Result<Expr, ()> {
        let start = self.span();

        // Peek past { and newlines
        let mut i = self.pos + 1;
        while i < self.tokens.len() && matches!(self.tokens[i].token, Token::Newline) {
            i += 1;
        }

        if matches!(self.tokens.get(i).map(|t| &t.token), Some(Token::RBrace)) {
            self.advance(); // {
            self.skip_newlines();
            self.advance(); // }
            return Ok(Expr::ObjectLit {
                fields: Vec::new(),
                span: start.merge(self.span()),
            });
        }

        let is_object = if let Some(
            Token::Ident(_) | Token::TypeIdent(_) | Token::StringLit(_) | Token::SingleStringLit(_),
        ) = self.tokens.get(i).map(|t| &t.token)
        {
            let mut j = i + 1;
            while j < self.tokens.len() && matches!(self.tokens[j].token, Token::Newline) {
                j += 1;
            }
            matches!(self.tokens.get(j).map(|t| &t.token), Some(Token::Colon))
        } else {
            false
        };

        if is_object {
            self.parse_object_lit()
        } else {
            self.advance(); // {
            let stmts = self.parse_block()?;
            Ok(Expr::Block {
                stmts,
                span: start.merge(self.span()),
            })
        }
    }

    pub(crate) fn parse_object_lit(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // {
        let mut fields = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let key = match self.peek().clone() {
                Token::Ident(n) | Token::TypeIdent(n) => {
                    self.advance();
                    n
                }
                Token::StringLit(s) | Token::SingleStringLit(s) => {
                    self.advance();
                    s
                }
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Expected identifier or string as object key",
                        vec![],
                        None,
                        ParseErrorClass::Expression,
                    ));
                    return Err(());
                }
            };
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push((key, value));
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.skip_newlines();
        self.expect(&Token::RBrace)?;
        Ok(Expr::ObjectLit {
            fields,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_match(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'match'
        let subject = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let arm_start = self.span();
            let pattern = self.parse_pattern()?;
            if self.eat(&Token::Arrow) {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "The '->' syntax is deprecated for match arms. Use '=>'.",
                    vec![],
                    None,
                    ParseErrorClass::Expression,
                ));
            } else {
                self.expect(&Token::FatArrow)?;
            }
            let body = self.parse_expr()?;
            arms.push(MatchArm {
                pattern,
                guard: None,
                body: Box::new(body),
                span: arm_start.merge(self.span()),
            });
            self.skip_newlines();
        }
        self.eat(&Token::RBrace);
        Ok(Expr::Match {
            subject: Box::new(subject),
            arms,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_if(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'if'
        let condition = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.skip_newlines();
        let else_body = if self.eat(&Token::Else) {
            self.expect(&Token::LBrace)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Expr::If {
            condition: Box::new(condition),
            then_body,
            else_body,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_for(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'for'
        let binding = self.parse_ident_name()?;
        self.expect(&Token::In)?;
        let iterable = self.parse_expr()?;
        let body = self.parse_expr()?;
        Ok(Expr::For {
            binding,
            iterable: Box::new(iterable),
            body: Box::new(body),
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_lambda(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'fn'
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat_return_arrow() {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            params,
            return_type,
            body: Box::new(body),
            span: start.merge(self.span()),
        })
    }

    // Curated allowlist of raw HTML elements that may appear as view-calls outside the primitive
    // set. Only tags that genuinely belong in a view tree are listed; ordinary function names
    // like `fetch`, `query`, `request` — even when called with named args only — must NOT be
    // sugared into JSX, because that breaks them at the type-check layer.
}

#[must_use]
fn is_known_html_view_tag(tag: &str) -> bool {
    matches!(
        tag,
        // Form / interactive
        "input" | "label" | "textarea" | "select" | "option" | "form" | "fieldset" | "legend"
        // Tabular
        | "table" | "thead" | "tbody" | "tfoot" | "tr" | "th" | "td" | "caption"
        // Media
        | "audio" | "video" | "source" | "track" | "canvas" | "svg" | "img"
        // Inline structural
        | "span" | "br" | "hr" | "i" | "b" | "em" | "strong" | "small" | "code" | "pre"
        | "kbd" | "abbr" | "cite" | "blockquote" | "q"
        // Document
        | "head" | "meta" | "title" | "link" | "script" | "style" | "noscript" | "main" | "section"
        | "article" | "aside" | "header" | "footer" | "nav" | "details" | "summary" | "dialog"
        | "figure" | "figcaption"
        // Embedded
        | "iframe" | "embed" | "object" | "param"
    )
}

impl super::super::Parser {
    /// VUV: convert positional/named call args into JSX attributes. Positional args are rejected
    /// because view calls are keyword-only by design (props need names like HTML attributes).
    ///
    /// Reserved-keyword escape: kwarg names beginning with `attr_` have the prefix stripped so
    /// HTML attributes whose names collide with Vox keywords can still be expressed. Example:
    /// `attr_type="checkbox"` → JSX `type="checkbox"`. The escape is needed because `type`,
    /// `for`, and similar HTML attribute names are reserved Vox identifiers and cannot appear
    /// as bare kwarg names without a parse error.
    pub(crate) fn view_args_to_attrs(&mut self, args: Vec<Arg>) -> Result<Vec<JsxAttribute>, ()> {
        let mut attrs = Vec::with_capacity(args.len());
        for arg in args {
            match arg.name {
                Some(mut name) => {
                    if let Some(rest) = name.strip_prefix("attr_") {
                        name = rest.to_string();
                    }
                    attrs.push(JsxAttribute {
                        name,
                        value: arg.value,
                    });
                }
                None => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Positional argument in view-call form. View calls are keyword-only — give every argument a name.",
                        vec![],
                        None,
                        ParseErrorClass::Expression,
                    ));
                    return Err(());
                }
            }
        }
        Ok(attrs)
    }

    /// VUV: parse the trailing `{ … }` children block of a view call. Each statement-position
    /// expression is one child; separators (newline, comma, semicolon) are all accepted.
    /// Caller has already consumed the opening `{`; this stops at (but does not consume) `}`.
    pub(crate) fn parse_view_children(&mut self) -> Result<Vec<Expr>, ()> {
        let mut children = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            let child = self.parse_expr()?;
            children.push(child);
            self.skip_newlines();
            self.eat(&Token::Comma);
            self.skip_newlines();
        }
        Ok(children)
    }
}
