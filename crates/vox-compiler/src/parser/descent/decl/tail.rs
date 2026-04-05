// Table, index, v0, reactive member, and routes parsing.

use super::super::Parser;
use crate::ast::decl::*;
use crate::ast::expr::Expr;
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

impl Parser {
    /// Parse `@table type Name { field: Type }` — brace-delimited field block.
    pub(crate) fn parse_table(&mut self, is_pub: bool) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @table
        self.expect(&Token::TypeKw)?; // eat 'type'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::Ident(_) => {
                    let fstart = self.span();
                    let fname = self.parse_ident_name()?;
                    self.expect(&Token::Colon)?;
                    let ftype = self.parse_type_expr()?;
                    fields.push(TableField {
                        name: fname,
                        type_ann: ftype,
                        description: None,
                        span: fstart.merge(self.span()),
                    });
                }
                _ => break,
            }
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Table(TableDecl {
            name,
            fields,
            description: None,
            json_layout: None,
            auth_provider: None,
            roles: vec![],
            cors: None,
            is_pub,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `@index Table.index_name on (col1, col2, ...)`
    pub(crate) fn parse_index(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @index
        let table_name = self.parse_ident_name()?;
        self.expect(&Token::Dot)?;
        let index_name = self.parse_ident_name()?;
        self.expect(&Token::On)?;
        self.expect(&Token::LParen)?;
        let mut columns = Vec::new();
        loop {
            if matches!(self.peek(), Token::RParen) {
                break;
            }
            columns.push(self.parse_ident_name()?);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RParen)?;
        Ok(Decl::Index(IndexDecl {
            table_name,
            index_name,
            columns,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `@v0 "prompt" fn Name() to Element` or `@v0 from "path" fn Name() to Element`
    pub(crate) fn parse_v0_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @v0
        // Determine if this is a prompt string or `from "image.png"`
        let (prompt, image_path) = match self.peek().clone() {
            Token::StringLit(s) => {
                self.advance();
                (s, None)
            }
            Token::Ident(kw) if kw == "from" => {
                self.advance(); // eat 'from'
                match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        (String::new(), Some(s))
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected image path string after 'from'",
                            vec!["\"path\"".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                }
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected prompt string or 'from' after @v0",
                    vec!["\"prompt\"".into(), "from".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
        self.expect(&Token::Fn)?;
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat(&Token::To) || self.eat(&Token::Arrow) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        Ok(Decl::V0Component(V0ComponentDecl {
            prompt,
            image_path,
            name,
            return_type,
            span: start.merge(self.span()),
        }))
    }

    /// Parse optional `style { .selector { property: "value" } }` blocks.
    pub(crate) fn parse_reactive_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // component
        let name = self.parse_ident_name()?;
        let inner = self.finish_reactive_component_after_name(start, name)?;
        Ok(Decl::ReactiveComponent(inner))
    }

    pub(crate) fn parse_state_decl(&mut self) -> Result<StateDecl, ()> {
        let start = self.span();
        self.advance(); // state
        let name = self.parse_ident_name()?;
        let ty = if self.eat(&Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let init = self.parse_expr()?;
        Ok(StateDecl {
            name,
            ty,
            init,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_derived_decl(&mut self) -> Result<DerivedDecl, ()> {
        let start = self.span();
        self.advance(); // derived
        let name = self.parse_ident_name()?;
        let ty = if self.eat(&Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let expr = self.parse_expr()?;
        Ok(DerivedDecl {
            name,
            ty,
            expr,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_reactive_block(&mut self) -> Result<Expr, ()> {
        let _start = self.span();
        self.advance(); // keyword
        self.expect(&Token::Colon)?;

        if matches!(self.peek(), Token::LBrace) {
            let b_start = self.span();
            self.advance(); // {
            let stmts = self.parse_block()?;
            Ok(Expr::Block {
                stmts,
                span: b_start.merge(self.span()),
            })
        } else {
            self.parse_expr()
        }
    }
    /// Parse `routes { "path" to ComponentName ... }` declaration.
    ///
    /// Grammar (descent): repeated entries, each `StringLit`, `to`, then component identifier; `K-metric` appendix branch `G04`.
    /// Braces are authoritative: `{` must follow `routes` with only newlines between (OP-0025).
    /// Tooling: [`RoutesDecl::parse_summary`](crate::ast::decl::RoutesDecl::parse_summary); surface inventory [`crate::parser::WEB_SURFACE_SYNTAX_INVENTORY`] (OP-S003).
    pub(crate) fn parse_routes(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'routes'
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut entries = Vec::new();
        loop {
            self.maybe_parser_trace("routes.entry");
            self.skip_newlines();
            match self.peek().clone() {
                Token::StringLit(path) => {
                    let entry_start = self.span();
                    self.advance();
                    if self.peek() != &Token::To {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "In `routes { ... }`, each entry must place the keyword `to` between the path string and the component name (for example: `\"/\" to Home`)",
                            vec!["to".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                    self.advance();
                    let component_name = self.parse_ident_name()?;
                    entries.push(RouteEntry {
                        path,
                        component_name,
                        children: vec![],
                        redirect: None,
                        is_wildcard: false,
                        span: entry_start.merge(self.span()),
                    });
                }
                _ => break,
            }
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Routes(RoutesDecl {
            entries,
            span: start.merge(self.span()),
        }))
    }
}
