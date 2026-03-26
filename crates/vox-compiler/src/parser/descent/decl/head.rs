// Top-level and declaration parsing.

use super::super::Parser;
use crate::ast::decl::{
    ComponentDecl, Decl, EffectDecl, FnDecl, ImportDecl, ImportPath, IslandDecl, IslandProp,
    LoadingDecl, McpToolDecl, MutationDecl, OnCleanupDecl, OnMountDecl, QueryDecl,
    ReactiveComponentDecl, ReactiveMemberDecl, ServerFnDecl, TestDecl,
};
use crate::ast::span::Span;
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

impl Parser {
    pub(crate) fn parse_import(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'import'
        let mut paths = Vec::new();
        loop {
            let path = self.parse_import_path()?;
            paths.push(path);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(Decl::Import(ImportDecl {
            paths,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_import_path(&mut self) -> Result<ImportPath, ()> {
        let start = self.span();
        let mut segments = Vec::new();
        match self.peek().clone() {
            Token::Ident(name) | Token::TypeIdent(name) => {
                segments.push(name);
                self.advance();
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected identifier in import path",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        }
        while self.eat(&Token::Dot) {
            match self.peek().clone() {
                Token::Ident(name) | Token::TypeIdent(name) => {
                    segments.push(name);
                    self.advance();
                }
                _ => break,
            }
        }
        Ok(ImportPath {
            segments,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @component
        self.skip_newlines();
        match self.peek().clone() {
            Token::Fn => {
                let f = self.parse_fn_decl(false)?;
                let styles = self.parse_style_blocks();
                Ok(Decl::Component(ComponentDecl { func: f, styles }))
            }
            Token::Ident(_) | Token::TypeIdent(_) => {
                let name = self.parse_ident_name()?;
                let inner = self.finish_reactive_component_after_name(start, name)?;
                Ok(Decl::ReactiveComponent(inner))
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "After @component, expected `fn` or a reactive component name (Path C)",
                    vec!["fn".into(), "ComponentName".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                Err(())
            }
        }
    }

    /// `Name(params) { state ... }` — shared by `component` and `@component` reactive forms.
    pub(crate) fn finish_reactive_component_after_name(
        &mut self,
        start: Span,
        name: String,
    ) -> Result<ReactiveComponentDecl, ()> {
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;

        self.expect(&Token::LBrace)?;
        let mut members = Vec::new();
        let mut view = None;

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::RBrace | Token::Eof => break,
                Token::State => members.push(ReactiveMemberDecl::State(self.parse_state_decl()?)),
                Token::Derived => {
                    members.push(ReactiveMemberDecl::Derived(self.parse_derived_decl()?))
                }
                Token::Effect => {
                    let eff_start = self.span();
                    let body = self.parse_reactive_block()?;
                    members.push(ReactiveMemberDecl::Effect(EffectDecl {
                        body,
                        span: eff_start.merge(self.span()),
                    }));
                }
                Token::Mount => {
                    let m_start = self.span();
                    let body = self.parse_reactive_block()?;
                    members.push(ReactiveMemberDecl::OnMount(OnMountDecl {
                        body,
                        span: m_start.merge(self.span()),
                    }));
                }
                Token::Cleanup => {
                    let c_start = self.span();
                    let body = self.parse_reactive_block()?;
                    members.push(ReactiveMemberDecl::OnCleanup(OnCleanupDecl {
                        body,
                        span: c_start.merge(self.span()),
                    }));
                }
                Token::View => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    view = Some(self.parse_expr()?);
                }
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Unexpected token in component block: {}", self.peek()),
                        vec![],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Statement,
                    ));
                    return Err(());
                }
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;

        Ok(ReactiveComponentDecl {
            name,
            params,
            members,
            view,
            span: start.merge(self.span()),
        })
    }

    /// `@island Name { prop: Type, prop?: Type }` — brace-delimited prop block.
    /// `@loading fn Name() to Element { ... }` — TanStack Router `pendingComponent` / suspense UI.
    pub(crate) fn parse_loading(&mut self) -> Result<Decl, ()> {
        self.advance(); // @loading
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Loading(LoadingDecl { func: f }))
    }

    pub(crate) fn parse_island(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // @island
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut props = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let pname = self.parse_ident_name()?;
            let is_optional = self.eat(&Token::Question);
            self.expect(&Token::Colon)?;
            let ty = self.parse_type_expr()?;
            props.push(IslandProp {
                name: pname,
                ty,
                is_optional,
            });
            self.skip_newlines();
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Island(IslandDecl {
            name,
            props,
            span: start.merge(self.span()),
        }))
    }

    pub(crate) fn parse_mcp_tool(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @mcp.tool
        let desc = if let Token::StringLit(s) = self.peek().clone() {
            self.advance();
            s
        } else {
            String::new()
        };
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::McpTool(McpToolDecl {
            description: desc,
            func: f,
        }))
    }

    pub(crate) fn parse_test(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @test
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Test(TestDecl { func: f }))
    }

    pub(crate) fn parse_server_fn(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @server
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::ServerFn(ServerFnDecl { func: f }))
    }

    pub(crate) fn parse_query_fn(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @query
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Query(QueryDecl { func: f }))
    }

    pub(crate) fn parse_mutation_fn(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @mutation
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Mutation(MutationDecl { func: f }))
    }

    pub(crate) fn parse_fn_decl(&mut self, is_pub: bool) -> Result<FnDecl, ()> {
        let start = self.span();
        self.expect(&Token::Fn)?;
        let name = self.parse_ident_name()?;

        let generics = if self.eat(&Token::Lt) {
            let mut gs = Vec::new();
            loop {
                gs.push(self.parse_ident_name()?);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::Gt)?;
            gs
        } else {
            Vec::new()
        };

        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat(&Token::To) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::LBrace)?;
        let body = self.parse_block()?;
        Ok(FnDecl {
            name,
            generics,
            params,
            return_type,
            body,
            is_async: false,
            is_deprecated: false,
            is_pure: false,
            is_traced: false,
            is_llm: false,
            llm_model: None,
            is_layout: false,
            is_pub,
            is_metric: false,
            metric_name: None,
            is_health: false,
            auth_provider: None,
            roles: vec![],
            cors: None,
            preconditions: vec![],
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_ident_name(&mut self) -> Result<String, ()> {
        match self.peek().clone() {
            Token::Ident(n) | Token::TypeIdent(n) => {
                self.advance();
                Ok(n)
            }
            Token::On => {
                self.advance();
                Ok("on".to_string())
            }
            Token::State => {
                self.advance();
                Ok("state".to_string())
            }
            Token::Derived => {
                self.advance();
                Ok("derived".to_string())
            }
            Token::Effect => {
                self.advance();
                Ok("effect".to_string())
            }
            Token::Mount => {
                self.advance();
                Ok("mount".to_string())
            }
            Token::Cleanup => {
                self.advance();
                Ok("cleanup".to_string())
            }
            Token::View => {
                self.advance();
                Ok("view".to_string())
            }
            Token::Component => {
                self.advance();
                Ok("component".to_string())
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected identifier",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                Err(())
            }
        }
    }
}
