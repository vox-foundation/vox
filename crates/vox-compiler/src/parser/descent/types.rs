// Type and parameter parsing for Parser.

use super::Parser;
use crate::ast::expr::Param;
use crate::ast::types::TypeExpr;
use crate::lexer::token::Token;

impl Parser {
    pub(crate) fn parse_params(&mut self) -> Result<Vec<Param>, ()> {
        let mut params = Vec::new();
        if matches!(self.peek(), Token::RParen) {
            return Ok(params);
        }
        loop {
            let start = self.span();
            let name = self.parse_ident_name()?;
            let type_ann = if self.eat(&Token::Colon) {
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            let default = if self.eat(&Token::Eq) {
                Some(self.parse_expr()?)
            } else {
                None
            };
            params.push(Param {
                name,
                type_ann,
                default,
                span: start.merge(self.span()),
            });
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(params)
    }

    pub(crate) fn parse_type_expr(&mut self) -> Result<TypeExpr, ()> {
        let start = self.span();
        if self.eat(&Token::Underscore) {
            return Ok(TypeExpr::Infer { span: start });
        }
        if self.eat(&Token::Dec) {
            return Ok(TypeExpr::Decimal { span: start });
        }
        if let Token::IntLit(v) = self.peek().clone() {
            self.advance();
            return Ok(TypeExpr::Named {
                name: v.to_string(),
                span: start.merge(self.span()),
            });
        }
        if self.eat(&Token::Fn) {
            self.expect(&Token::LParen)?;
            let mut params = Vec::new();
            if !matches!(self.peek(), Token::RParen) {
                loop {
                    params.push(self.parse_type_expr()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
            }
            self.expect(&Token::RParen)?;
            let return_type = if self.eat_return_arrow() {
                self.parse_type_expr()?
            } else {
                TypeExpr::Unit { span: self.span() }
            };
            return Ok(TypeExpr::Function {
                params,
                return_type: Box::new(return_type),
                span: start.merge(self.span()),
            });
        }
        let name = self.parse_ident_name()?;
        if self.eat(&Token::LBracket) {
            let mut args = Vec::new();
            loop {
                args.push(self.parse_type_expr()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RBracket)?;
            Ok(TypeExpr::Generic {
                name,
                args,
                span: start.merge(self.span()),
            })
        } else {
            Ok(TypeExpr::Named {
                name,
                span: start.merge(self.span()),
            })
        }
    }
}
