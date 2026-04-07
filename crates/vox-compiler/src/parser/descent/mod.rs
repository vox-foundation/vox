//! Single-module recursive-descent parser implementation.
//!
//! **This is the only parser implementation** for `vox-parser`. There is no
//! secondary parser, no multi-module rewrite, and no separate LSP tree-sitter
//! layer in this crate. The public entry point is [`parse`].
//!
//! See `crate` (lib.rs) for the scope table — what constructs are in/out of scope.

use crate::ast::decl::*;
use crate::ast::span::Span;
use crate::lexer::cursor::Spanned;
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

/// Strict parse: returns [`Module`] or **all** accumulated [`ParseError`] values.
pub fn parse(tokens: Vec<Spanned>) -> Result<Module, Vec<ParseError>> {
    let mut p = Parser::new(tokens);
    p.parse_module()
}

struct Parser {
    tokens: Vec<Spanned>,
    pos: usize,
    errors: Vec<ParseError>,
}

impl Parser {
    pub(crate) fn new(tokens: Vec<Spanned>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: vec![],
        }
    }

    pub(crate) fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    pub(crate) fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| Span::new(s.span.start, s.span.end))
            .unwrap_or(Span::new(0, 0))
    }

    pub(crate) fn advance(&mut self) -> &Token {
        let t = &self.tokens[self.pos].token;
        if self.pos < self.tokens.len() - 1 {
            self.pos += 1;
        }
        t
    }

    pub(crate) fn expect(&mut self, expected: &Token) -> Result<Span, ()> {
        if self.peek() == expected {
            let sp = self.span();
            self.advance();
            Ok(sp)
        } else {
            self.errors.push(ParseError::classified(
                self.span(),
                format!("Expected {expected}, found {}", self.peek()),
                vec![expected.to_string()],
                Some(self.peek().to_string()),
                ParseErrorClass::ExpectToken,
            ));
            Err(())
        }
    }

    pub(crate) fn eat(&mut self, expected: &Token) -> bool {
        if self.peek() == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    pub(crate) fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    /// Debug-only trace when `VOX_PARSER_DEBUG` is set in the environment (OP-0008 / OP-0031).
    pub(crate) fn maybe_parser_trace(&self, label: &'static str) {
        if std::env::var_os("VOX_PARSER_DEBUG").is_some() {
            eprintln!("[vox-parser:{label}] {:?}", self.peek());
        }
    }

    pub(crate) fn parse_module(&mut self) -> Result<Module, Vec<ParseError>> {
        let start = self.span();
        let mut decls = Vec::new();
        self.skip_newlines();
        while !matches!(self.peek(), Token::Eof) {
            match self.parse_decl() {
                Ok(d) => decls.push(d),
                Err(_) => {
                    self.recover_to_top_level();
                }
            }
            self.skip_newlines();
        }
        if self.errors.is_empty() {
            Ok(Module {
                declarations: decls,
                span: start.merge(self.span()),
            })
        } else {
            Err(self.errors.clone())
        }
    }

    pub(crate) fn recover_to_top_level(&mut self) {
        loop {
            match self.peek() {
                Token::Eof => break,
                Token::Fn
                | Token::AtComponent
                | Token::AtIsland
                | Token::Import
                | Token::TypeKw
                | Token::Actor
                | Token::Workflow
                | Token::Http
                | Token::AtTest
                | Token::AtServer
                | Token::AtQuery
                | Token::AtMutation
                | Token::Component
                | Token::AtV0
                | Token::AtForall
                | Token::AtRequire
                | Token::AtEnsure
                | Token::AtInvariant
                | Token::AtFuzz
                | Token::AtLoading
                | Token::Let
                | Token::Agent
                | Token::Environment
                | Token::Async => break,
                Token::RBrace => {
                    self.advance();
                    break;
                }
                Token::Newline => {
                    self.advance();
                    break;
                }
                _ => {
                    self.advance();
                }
            }
        }
    }

    pub(crate) fn parse_decl(&mut self) -> Result<Decl, ()> {
        self.skip_newlines();
        match self.peek().clone() {
            Token::Import => self.parse_import(),
            Token::AtComponent => self.parse_component(),
            Token::Component => self.parse_reactive_component(),
            Token::AtIsland => self.parse_island(),
            Token::AtLoading => self.parse_loading(),
            Token::AtTest => self.parse_test(),
            Token::AtServer => self.parse_server_fn(),
            Token::AtQuery => self.parse_query_fn(),
            Token::AtMutation => self.parse_mutation_fn(),
            Token::AtV0 => self.parse_v0_component(),
            Token::AtForall => self.parse_forall(),
            Token::AtMcpTool => self.parse_mcp_tool(),
            Token::AtMcpResource => self.parse_mcp_resource(),
            Token::Let => {
                let start = self.span();
                self.advance(); // eat 'let'
                let _mutable = self.eat(&Token::Mut);
                let name = self.parse_ident_name()?;
                let type_ann = if self.eat(&Token::Colon) {
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                self.expect(&Token::Eq)?;
                let value = self.parse_expr()?;
                Ok(Decl::Const(crate::ast::decl::ConstDecl {
                    name,
                    value,
                    type_ann,
                    is_pub: false,
                    is_deprecated: false,
                    is_build_const: false,
                    span: start.merge(self.span()),
                }))
            }
            Token::Async => {
                self.advance(); // eat 'async'
                match self.peek().clone() {
                    Token::Fn
                    | Token::AtRequire
                    | Token::AtEnsure
                    | Token::AtInvariant
                    | Token::AtFuzz
                    | Token::AtMobileNative => {
                        let mut f = self.parse_fn_decl(false)?;
                        f.is_async = true;
                        Ok(Decl::Function(f))
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected fn after async",
                            vec!["fn".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        Err(())
                    }
                }
            }
            Token::Fn | Token::AtRequire | Token::AtEnsure | Token::AtInvariant | Token::AtFuzz | Token::AtMobileNative => {
                let f = self.parse_fn_decl(false)?;
                Ok(Decl::Function(f))
            }
            Token::Pub => {
                self.advance();
                match self.peek().clone() {
                    Token::Fn
                    | Token::AtRequire
                    | Token::AtEnsure
                    | Token::AtInvariant
                    | Token::AtFuzz
                    | Token::AtMobileNative => {
                        let f = self.parse_fn_decl(true)?;
                        Ok(Decl::Function(f))
                    }
                    Token::TypeKw => self.parse_typedef(true),
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected fn or type after pub",
                            vec!["fn".into(), "type".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        Err(())
                    }
                }
            }
            Token::TypeKw => self.parse_typedef(false),
            Token::Actor => self.parse_actor(),
            Token::Agent => self.parse_agent(),
            Token::Environment => self.parse_environment(),
            Token::Workflow => self.parse_workflow(),
            Token::Activity => self.parse_activity(),
            Token::Http => self.parse_http_route(),
            Token::AtTable => self.parse_table(false),
            Token::AtIndex => self.parse_index(),
            Token::Ident(ref name) if name == "routes" => self.parse_routes(),
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    format!("Unexpected token at top level: {}", self.peek()),
                    vec!["fn".into(), "import".into(), "type".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::TopLevel,
                ));
                Err(())
            }
        }
    }
}

mod decl;
mod expr;
mod stmt;
mod types;

#[cfg(test)]
mod tests;
