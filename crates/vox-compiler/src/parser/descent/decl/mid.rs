// ADT typedefs and actor / workflow / HTTP declarations.

use super::super::Parser;
use crate::ast::decl::*;
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

impl Parser {
    pub(crate) fn parse_typedef(&mut self, is_pub: bool) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'type'
        let name = self.parse_ident_name()?;
        self.expect(&Token::Eq)?;
        self.skip_newlines();
        // Variants may appear inline (| A | B) or on separate lines
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if !self.eat(&Token::Bar) {
                break;
            }
            let vstart = self.span();
            let vname = self.parse_ident_name()?;
            let mut fields = Vec::new();
            if self.eat(&Token::LParen) {
                loop {
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    let fname = self.parse_ident_name()?;
                    self.expect(&Token::Colon)?;
                    let ftype = self.parse_type_expr()?;
                    fields.push(VariantField {
                        name: fname,
                        type_ann: ftype,
                        span: vstart.merge(self.span()),
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
            }
            variants.push(Variant {
                name: vname,
                fields,
                literal_value: None,
                span: vstart.merge(self.span()),
            });
        }
        Ok(Decl::TypeDef(TypeDefDecl {
            name,
            generics: vec![],
            variants,
            fields: vec![],
            type_alias: None,
            json_layout: None,
            is_pub,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_actor(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'actor'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut handlers = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            if self.eat(&Token::On) {
                let hstart = self.span();
                let event = self.parse_ident_name()?;
                self.expect(&Token::LParen)?;
                let params = self.parse_params()?;
                self.expect(&Token::RParen)?;
                let ret = if self.eat(&Token::Arrow) {
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                self.expect(&Token::LBrace)?;
                let body = self.parse_block()?;
                handlers.push(ActorHandler {
                    event_name: event,
                    params,
                    return_type: ret,
                    body,
                    is_traced: false,
                    span: hstart.merge(self.span()),
                });
            } else {
                break;
            }
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Actor(ActorDecl {
            name,
            state_fields: vec![],
            handlers,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_workflow(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'workflow'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret = if self.eat(&Token::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Decl::Workflow(WorkflowDecl {
            name,
            params,
            return_type: ret,
            body,
            is_traced: false,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_activity(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'activity'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret = if self.eat(&Token::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Decl::Activity(ActivityDecl {
            name,
            params,
            return_type: ret,
            body,
            options: None,
            prompt: None,
            is_traced: false,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_http_route(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'http'
        let method = match self.peek().clone() {
            Token::Ident(m) if m == "get" => {
                self.advance();
                HttpMethod::Get
            }
            Token::Ident(m) if m == "post" => {
                self.advance();
                HttpMethod::Post
            }
            Token::Ident(m) if m == "put" => {
                self.advance();
                HttpMethod::Put
            }
            Token::Ident(m) if m == "delete" => {
                self.advance();
                HttpMethod::Delete
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected HTTP method",
                    vec!["get".into(), "post".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
        let path = match self.peek().clone() {
            Token::StringLit(s) => {
                self.advance();
                s
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected route path string",
                    vec!["\"path\"".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
        let ret = if self.eat(&Token::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(Decl::HttpRoute(HttpRouteDecl {
            method,
            path,
            params: vec![],
            return_type: ret,
            body,
            auth_provider: None,
            roles: vec![],
            cors: None,
            is_traced: false,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }
}
