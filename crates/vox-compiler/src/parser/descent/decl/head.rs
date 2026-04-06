// Top-level and declaration parsing.

use super::super::Parser;
use crate::ast::decl::{
    ComponentDecl, Decl, EffectDecl, FnDecl, ForallDecl, ImportDecl, ImportPath, ImportPathKind,
    IslandDecl, IslandProp, LoadingDecl, McpResourceDecl, McpToolDecl, MutationDecl, OnCleanupDecl,
    OnMountDecl, QueryDecl, ReactiveComponentDecl, ReactiveMemberDecl, RustCrateImport,
    ServerFnDecl, TestDecl,
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
        let mut alias = None;
        let first = match self.peek().clone() {
            Token::Ident(name) | Token::TypeIdent(name) => {
                self.advance();
                name
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Import path must begin with an identifier (for example `react.use_state` or `rust:serde_json`).",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };

        if first == "rust" && self.eat(&Token::Colon) {
            let crate_name = match self.peek().clone() {
                Token::Ident(name) | Token::TypeIdent(name) => {
                    self.advance();
                    name
                }
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Rust import must include a crate name after `rust:` (for example `import rust:serde_json`).",
                        vec!["crate-name".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };

            let mut rust_meta = RustCrateImport {
                crate_name,
                version: None,
                path: None,
                git: None,
                rev: None,
            };

            if self.eat(&Token::LParen) {
                loop {
                    match self.peek().clone() {
                        Token::RParen => {
                            self.advance();
                            break;
                        }
                        Token::Ident(key) => {
                            self.advance();
                            self.expect(&Token::Colon)?;
                            let value = match self.peek().clone() {
                                Token::StringLit(v) => {
                                    self.advance();
                                    v
                                }
                                Token::Ident(v) | Token::TypeIdent(v) => {
                                    self.advance();
                                    v
                                }
                                _ => {
                                    self.errors.push(ParseError::classified(
                                        self.span(),
                                        "Rust import metadata values must be string or identifier.",
                                        vec!["string".into(), "identifier".into()],
                                        Some(self.peek().to_string()),
                                        ParseErrorClass::Declaration,
                                    ));
                                    return Err(());
                                }
                            };
                            match key.as_str() {
                                "version" => rust_meta.version = Some(value),
                                "path" => rust_meta.path = Some(value),
                                "git" => rust_meta.git = Some(value),
                                "rev" | "branch" => rust_meta.rev = Some(value),
                                _ => {
                                    self.errors.push(ParseError::classified(
                                        self.span(),
                                        format!(
                                            "Unknown rust import metadata key `{key}`; expected one of version/path/git/rev."
                                        ),
                                        vec![
                                            "version".into(),
                                            "path".into(),
                                            "git".into(),
                                            "rev".into(),
                                        ],
                                        Some(key),
                                        ParseErrorClass::Declaration,
                                    ));
                                    return Err(());
                                }
                            }
                            if self.eat(&Token::Comma) {
                                continue;
                            }
                            if self.eat(&Token::RParen) {
                                break;
                            }
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected `,` or `)` after rust import metadata item.",
                                vec![",".into(), ")".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected metadata key or `)` in rust import metadata list.",
                                vec!["identifier".into(), ")".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                }
            }

            if let Token::Ident(word) = self.peek().clone()
                && word == "as"
            {
                self.advance();
                alias = Some(self.parse_ident_name()?);
            }

            return Ok(ImportPath {
                kind: ImportPathKind::RustCrate(rust_meta),
                alias,
                span: start.merge(self.span()),
            });
        }

        let mut segments = vec![first];
        while self.eat(&Token::Dot) {
            match self.peek().clone() {
                Token::Ident(name) | Token::TypeIdent(name) => {
                    segments.push(name);
                    self.advance();
                }
                _ => break,
            }
        }
        if let Token::Ident(word) = self.peek().clone()
            && word == "as"
        {
            self.advance();
            alias = Some(self.parse_ident_name()?);
        }

        Ok(ImportPath {
            kind: ImportPathKind::SymbolPath { segments },
            alias,
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
                    "Unsupported head after `@component`: use `fn` for the classic component (`@component fn Name(...)`) or an identifier for Path C (`@component Name(...)`). Nothing else may follow `@component` here.",
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
                        "Parse (reactive body): unexpected token; expected a member keyword (`state`, `derived`, `effect`, `mount`, `cleanup`) or `view:` (parse-stage; see diagnostic taxonomy `parse` row)".to_string(),
                        vec![
                            "state".into(),
                            "derived".into(),
                            "effect".into(),
                            "mount".into(),
                            "cleanup".into(),
                            "view:".into(),
                        ],
                        Some(self.peek().to_string()),
                        ParseErrorClass::ReactiveComponentMember,
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

    /// Parser truth for WebIR docs: only `{ prop: Ty` / `prop?: Ty }` forms; no comma-required between props.
    /// Braces are authoritative: `{` must follow the island name immediately (non-speculative; OP-0013).
    pub(crate) fn parse_island(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // @island
        self.maybe_parser_trace("island.after_kw");
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut props = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            props.push(self.parse_island_prop_line()?);
            self.skip_newlines();
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Island(IslandDecl {
            name,
            props,
            span: start.merge(self.span()),
        }))
    }

    /// One `@island` prop line: `name`, optional `?`, `:`, type (OP-0006).
    pub(crate) fn parse_island_prop_line(&mut self) -> Result<IslandProp, ()> {
        let pname = self.parse_ident_name()?;
        if std::env::var_os("VOX_PARSER_DEBUG").is_some() {
            eprintln!(
                "[vox-parser:island.prop] name={pname:?} next={:?}",
                self.peek()
            );
        }
        let is_optional = self.eat(&Token::Question);
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;
        Ok(IslandProp {
            name: pname,
            ty,
            is_optional,
        })
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

    /// `@mcp.resource ("uri", "desc") fn ...` or `@mcp.resource "uri" "desc" fn ...`.
    pub(crate) fn parse_mcp_resource(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @mcp.resource
        let (uri, description) = match self.peek().clone() {
            Token::LParen => {
                self.advance();
                let u = match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected string literal for resource URI",
                            vec!["\"...\"".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                };
                self.expect(&Token::Comma)?;
                let d = match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected string literal for resource description",
                            vec!["\"...\"".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                };
                self.expect(&Token::RParen)?;
                (u, d)
            }
            Token::StringLit(_) => {
                let u = match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => unreachable!(),
                };
                let d = match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected second string literal (description) after resource URI",
                            vec!["\"...\"".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                };
                (u, d)
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected `(` or string literal after @mcp.resource",
                    vec!["(\"uri\", \"desc\")".into(), "\"uri\"".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::McpResource(McpResourceDecl {
            uri,
            description,
            func: f,
        }))
    }

    pub(crate) fn parse_test(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @test
        let mut label = String::new();
        if self.eat(&Token::LParen) {
            match self.peek().clone() {
                Token::StringLit(s) => {
                    self.advance();
                    label = s;
                }
                _ => {}
            }
            let _ = self.eat(&Token::RParen);
        }
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Test(TestDecl { label, func: f }))
    }

    pub(crate) fn parse_forall(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @forall
        let mut label = String::new();
        if self.eat(&Token::LParen) {
            match self.peek().clone() {
                Token::StringLit(s) => {
                    self.advance();
                    label = s;
                }
                _ => {
                    while !self.eat(&Token::RParen) && !matches!(self.peek(), Token::Eof) {
                        self.advance();
                    }
                }
            }
            let _ = self.eat(&Token::RParen);
        }
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Forall(ForallDecl {
            label,
            iterations: 1000,
            func: f,
        }))
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
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();
        let mut invariants = Vec::new();
        let mut is_mobile_native = false;

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::AtRequire => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    preconditions.push(self.parse_expr()?);
                    self.expect(&Token::RParen)?;
                }
                Token::AtEnsure => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    postconditions.push(self.parse_expr()?);
                    self.expect(&Token::RParen)?;
                }
                Token::AtInvariant => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    invariants.push(self.parse_expr()?);
                    self.expect(&Token::RParen)?;
                }
                Token::AtFuzz | Token::AtMobileNative => {
                    self.advance();
                    is_mobile_native = true;
                }
                _ => break,
            }
        }

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
        let return_type = if self.eat(&Token::Arrow) || self.eat(&Token::To) {
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
            is_pub,
            auth_provider: None,
            roles: vec![],
            cors: None,
            preconditions,
            postconditions,
            invariants,
            verify_mode: crate::ast::decl::fundecl::VerifyMode::Off,
            test_strategy: None,
            is_mobile_native,
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
