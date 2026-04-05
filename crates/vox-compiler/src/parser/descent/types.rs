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
        } else if self.eat(&Token::Underscore) {
            Ok(TypeExpr::Infer { span: start })
        } else {
            Ok(TypeExpr::Named {
                name,
                span: start.merge(self.span()),
            })
        }
    }
}
