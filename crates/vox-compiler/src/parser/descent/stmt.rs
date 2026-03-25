// Statement, block, and pattern parsing.

use super::Parser;
use crate::ast::expr::Expr;
use crate::ast::pattern::Pattern;
use crate::ast::stmt::Stmt;
use crate::lexer::token::Token;
use crate::parser::error::ParseError;

impl Parser {
    pub(crate) fn parse_block(&mut self) -> Result<Vec<Stmt>, ()> {
        self.skip_newlines();
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.skip_newlines();
        self.expect(&Token::RBrace)?;
        Ok(stmts)
    }

    pub(crate) fn parse_stmt(&mut self) -> Result<Stmt, ()> {
        let start = self.span();
        match self.peek().clone() {
            Token::Let => self.parse_let_stmt(),
            Token::Ret => {
                self.advance();
                let value = if matches!(self.peek(), Token::Newline | Token::RBrace | Token::Eof) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                Ok(Stmt::Return {
                    value,
                    span: start.merge(self.span()),
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.eat(&Token::Eq) {
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign {
                        target: expr,
                        value,
                        span: start.merge(self.span()),
                    })
                } else {
                    Ok(Stmt::Expr {
                        expr: expr.clone(),
                        span: expr.span(),
                    })
                }
            }
        }
    }

    pub(crate) fn parse_let_stmt(&mut self) -> Result<Stmt, ()> {
        let start = self.span();
        self.advance(); // eat 'let'
        let mutable = self.eat(&Token::Mut);
        let pattern = self.parse_pattern()?;
        let type_ann = if self.eat(&Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let {
            pattern,
            type_ann,
            value,
            mutable,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        let start = self.span();
        match self.peek().clone() {
            Token::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard { span: start })
            }
            Token::LParen => {
                self.advance();
                let mut elems = Vec::new();
                loop {
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    elems.push(self.parse_pattern()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
                Ok(Pattern::Tuple {
                    elements: elems,
                    span: start.merge(self.span()),
                })
            }
            Token::TypeIdent(name) => {
                self.advance();
                if self.eat(&Token::LParen) {
                    let mut fields = Vec::new();
                    loop {
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        fields.push(self.parse_pattern()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Pattern::Constructor {
                        name,
                        fields,
                        span: start.merge(self.span()),
                    })
                } else {
                    Ok(Pattern::Ident {
                        name,
                        span: start.merge(self.span()),
                    })
                }
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Pattern::Ident {
                    name,
                    span: start.merge(self.span()),
                })
            }
            Token::IntLit(v) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: Box::new(Expr::IntLit {
                        value: v,
                        span: start,
                    }),
                    span: start,
                })
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: Box::new(Expr::StringLit {
                        value: s,
                        span: start,
                    }),
                    span: start,
                })
            }
            _ => {
                self.errors.push(ParseError::new(
                    start,
                    "Expected pattern",
                    vec!["identifier".into(), "_".into()],
                    Some(self.peek().to_string()),
                ));
                Err(())
            }
        }
    }
}
