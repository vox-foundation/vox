// JSX expression parsing.

use super::super::Parser;
use crate::ast::expr::{Expr, JsxAttribute, JsxElement, JsxSelfClosingElement};
use crate::lexer::token::Token;

impl Parser {
    pub(crate) fn parse_jsx(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat '<'
        let tag = self.parse_ident_name()?;
        let mut attrs = Vec::new();
        // Parse attributes until '>' or '/>'
        loop {
            self.skip_newlines();
            match self.peek() {
                Token::Gt | Token::JsxSelfClose | Token::Eof => break,
                _ => {
                    let mut attr_name = self.parse_ident_name()?;
                    if self.eat(&Token::Colon) {
                        attr_name.push(':');
                        attr_name.push_str(&self.parse_ident_name()?);
                    } else if self.eat(&Token::Minus) {
                        attr_name.push('-');
                        attr_name.push_str(&self.parse_ident_name()?);
                    }
                    self.expect(&Token::Eq)?;
                    let value = if matches!(self.peek(), Token::LBrace) {
                        self.parse_brace_expr()?
                    } else if let Token::StringLit(s) = self.peek().clone() {
                        self.advance();
                        Expr::StringLit {
                            value: s,
                            span: self.span(),
                        }
                    } else {
                        self.parse_expr()?
                    };
                    attrs.push(JsxAttribute {
                        name: attr_name,
                        value,
                    });
                }
            }
        }
        if self.eat(&Token::JsxSelfClose) {
            return Ok(Expr::JsxSelfClosing(JsxSelfClosingElement {
                tag,
                attributes: attrs,
                span: start.merge(self.span()),
            }));
        }
        self.expect(&Token::Gt)?;
        // Children
        let mut children = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::JsxCloseStart | Token::Eof) {
                break;
            }

            match self.peek().clone() {
                Token::Lt => {
                    children.push(self.parse_jsx()?);
                }
                Token::LBrace => {
                    children.push(self.parse_brace_expr()?);
                }
                Token::For => {
                    children.push(self.parse_for()?);
                }
                Token::StringLit(s) => {
                    let sp = self.span();
                    self.advance();
                    children.push(Expr::StringLit { value: s, span: sp });
                }
                _ => {
                    children.push(self.parse_expr()?);
                }
            }
        }

        if self.eat(&Token::JsxCloseStart) {
            let _ = self.parse_ident_name()?;
            self.expect(&Token::Gt)?;
        }

        Ok(Expr::Jsx(JsxElement {
            tag,
            attributes: attrs,
            children,
            span: start.merge(self.span()),
        }))
    }
}
