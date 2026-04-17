// Primary expressions, postfix, control/lambda forms.

use super::super::Parser;
use crate::ast::expr::{Arg, Expr, MatchArm, UnOp};
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
            Token::StringLit(s) => {
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
            Token::Lt => self.parse_jsx()?,
            Token::Ident(name) => {
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
            Token::TypeIdent(name) => {
                self.advance();
                Expr::Ident { name, span: start }
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
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                    continue;
                }
                self.pos = saved; // backtrack
            }
            let value = self.parse_expr()?;
            args.push(Arg { name: None, value });
            if !self.eat(&Token::Comma) {
                break;
            }
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

        let is_object = if let Some(Token::Ident(_) | Token::TypeIdent(_)) =
            self.tokens.get(i).map(|t| &t.token)
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
            let key = self.parse_ident_name()?;
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
            self.expect(&Token::Arrow)?;
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
        let return_type = if self.eat(&Token::Arrow) || self.eat(&Token::To) {
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
}
