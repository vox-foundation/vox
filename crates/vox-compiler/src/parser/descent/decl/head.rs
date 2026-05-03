// Top-level and declaration parsing.

use super::super::Parser;
use crate::ast::decl::{
    Decl, EffectDecl,
    EndpointDecl, EndpointKind, FnDecl, ForallDecl, ImportDecl, ImportPath, ImportPathKind,
    IslandDecl, IslandProp, LoadingDecl, McpResourceDecl, McpToolDecl, MutationDecl, OnCleanupDecl,
    OnMountDecl, PostCondition, QueryDecl, ReactiveComponentDecl, ReactiveMemberDecl,
    RustCrateImport, ScheduledDecl, ServerFnDecl, TestDecl,
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
            Token::Env => {
                self.advance();
                "env".to_string()
            }
            Token::Http => {
                self.advance();
                "http".to_string()
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
                Token::Env => {
                    self.advance();
                    segments.push("env".to_string());
                }
                // `http` is a dedicated keyword for route headers, but it must still parse as a
                // path segment after `.` (e.g. `import std.http`, `std.http.get_text(...)`).
                Token::Http => {
                    self.advance();
                    segments.push("http".to_string());
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

    #[allow(dead_code)]
    pub(crate) fn parse_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @component
        self.skip_newlines();
        match self.peek().clone() {
            Token::Fn => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Retired classic `@component fn`. Use Path C `component Name() { ... }` (or prefix: `@component Name() { ... }`).",
                    vec!["component Counter() { state n: int = 0; view: <span>{n}</span> }".into()],
                    Some("fn".into()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
            Token::Ident(_) | Token::TypeIdent(_) => {
                let name = self.parse_ident_name()?;
                let mut inner = self.finish_reactive_component_after_name(start, name)?;
                inner.styles = self.parse_style_blocks();
                Ok(Decl::ReactiveComponent(inner))
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Unsupported head after `@component`: use an identifier for Path C (`@component Name(...)`). Classic `@component fn` is retired.",
                    vec!["ComponentName".into()],
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
                Token::On => {
                    let on_start = self.span();
                    self.advance();
                    match self.peek().clone() {
                        Token::Mount => {
                            let body = self.parse_reactive_block()?;
                            members.push(ReactiveMemberDecl::OnMount(OnMountDecl {
                                body,
                                span: on_start.merge(self.span()),
                            }));
                        }
                        Token::Cleanup => {
                            let body = self.parse_reactive_block()?;
                            members.push(ReactiveMemberDecl::OnCleanup(OnCleanupDecl {
                                body,
                                span: on_start.merge(self.span()),
                            }));
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected `mount` or `cleanup` after `on` in reactive component block.",
                                vec!["mount".into(), "cleanup".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                }
                Token::View => {
                    self.advance();
                    self.expect(&Token::Colon)?;
                    view = Some(self.parse_expr()?);
                }
                _ => {
                    let stmt = self.parse_stmt()?;
                    members.push(ReactiveMemberDecl::Stmt(stmt));
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
            styles: vec![],
            span: start.merge(self.span()),
        })
    }

    /// ADR-033: parse a `fragment Name(args) { <markup> }` declaration into a
    /// [`crate::ast::decl::FragmentDecl`]. The body is parsed as a single expression
    /// (matches the `view:` shape inside reactive components). Codegen for fragments
    /// is gated on Phase 6 typed-primitive stabilization per the ADR; for now the
    /// parser accepts the syntax and the AST node carries it through to whatever
    /// future codegen / lowering wants to consume it.
    pub(crate) fn parse_fragment_decl(&mut self) -> Result<crate::ast::decl::Decl, ()> {
        use crate::ast::decl::FragmentDecl;
        use crate::lexer::token::Token;

        let start = self.span();
        self.expect(&Token::Fragment)?;
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let body = self.parse_expr()?;
        self.skip_newlines();
        self.expect(&Token::RBrace)?;
        Ok(crate::ast::decl::Decl::Fragment(FragmentDecl {
            name,
            params,
            body,
            span: start.merge(self.span()),
        }))
    }

    /// ADR-032: parse module-scope reactive members in a `.vox.ui` file into a single
    /// synthetic [`ReactiveModuleDecl`]. Consumes consecutive `state` / `derived` /
    /// `effect` / `on mount` / `on cleanup` declarations until it hits a token that
    /// isn't one of those, then returns. Subsequent module-scope reactive members in
    /// the same file would be picked up by another `parse_decl` call and produce a
    /// second `ReactiveModuleDecl` — that's intentional; per-module name disambiguation
    /// is the file's responsibility.
    ///
    /// Caller (`parse_decl`) only invokes this when `self.file_kind ==
    /// FileKind::ReactiveModule` and the next token is a reactive member.
    pub(crate) fn parse_reactive_module_decl(&mut self) -> Result<crate::ast::decl::Decl, ()> {
        use crate::ast::decl::{
            EffectDecl, OnCleanupDecl, OnMountDecl, ReactiveMemberDecl, ReactiveModuleDecl,
        };

        let start = self.span();
        let mut members: Vec<ReactiveMemberDecl> = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::State => {
                    members.push(ReactiveMemberDecl::State(self.parse_state_decl()?))
                }
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
                Token::On => {
                    let on_start = self.span();
                    self.advance();
                    match self.peek().clone() {
                        Token::Mount => {
                            let body = self.parse_reactive_block()?;
                            members.push(ReactiveMemberDecl::OnMount(OnMountDecl {
                                body,
                                span: on_start.merge(self.span()),
                            }));
                        }
                        Token::Cleanup => {
                            let body = self.parse_reactive_block()?;
                            members.push(ReactiveMemberDecl::OnCleanup(OnCleanupDecl {
                                body,
                                span: on_start.merge(self.span()),
                            }));
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected `mount` or `cleanup` after `on` at module scope in a `.vox.ui` file.",
                                vec!["mount".into(), "cleanup".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                }
                _ => break,
            }
        }

        Ok(crate::ast::decl::Decl::ReactiveModule(ReactiveModuleDecl {
            // Module name is filled in later by codegen from the source file basename;
            // the parser doesn't know the path. Empty for now.
            name: String::new(),
            members,
            span: start.merge(self.span()),
        }))
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
        self.skip_newlines();
        if let Token::StringLit(_) = self.peek().clone() {
            self.advance();
        }
        self.skip_newlines();
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

    /// `@scheduled("1h") fn name(...) { ... }` — interval string is retained on [`ScheduledDecl`].
    pub(crate) fn parse_scheduled(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @scheduled
        self.skip_newlines();
        let interval = if self.eat(&Token::LParen) {
            let s = match self.peek().clone() {
                Token::StringLit(s) => {
                    self.advance();
                    s
                }
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Expected string literal schedule in @scheduled(\"...\")",
                        vec!["@scheduled(\"1h\") fn tick() -> Unit { return Unit }".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };
            self.expect(&Token::RParen)?;
            s
        } else {
            self.errors.push(ParseError::classified(
                self.span(),
                "Expected `(` after @scheduled",
                vec!["@scheduled(\"1h\") fn tick() -> Unit { return Unit }".into()],
                Some(self.peek().to_string()),
                ParseErrorClass::Declaration,
            ));
            return Err(());
        };
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Scheduled(ScheduledDecl { interval, func: f }))
    }

    pub(crate) fn parse_server_fn(&mut self) -> Result<Decl, ()> {
        let span = self.span();
        self.advance(); // eat @server
        self.errors.push(ParseError::warning(
            span,
            "The `@server` decorator is deprecated. Use `@endpoint(kind: server)` instead.",
            ParseErrorClass::Tombstoned,
        ));
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::ServerFn(ServerFnDecl { func: f }))
    }

    pub(crate) fn parse_query_fn(&mut self) -> Result<Decl, ()> {
        let span = self.span();
        self.advance(); // eat @query
        self.errors.push(ParseError::warning(
            span,
            "The `@query` decorator is deprecated. Use `@endpoint(kind: query)` instead.",
            ParseErrorClass::Tombstoned,
        ));
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Query(QueryDecl { func: f }))
    }

    pub(crate) fn parse_mutation_fn(&mut self) -> Result<Decl, ()> {
        let span = self.span();
        self.advance(); // eat @mutation
        self.errors.push(ParseError::warning(
            span,
            "The `@mutation` decorator is deprecated. Use `@endpoint(kind: mutation)` instead.",
            ParseErrorClass::Tombstoned,
        ));
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Mutation(MutationDecl { func: f }))
    }

    pub(crate) fn parse_endpoint(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @endpoint
        self.expect(&Token::LParen)?;
        let mut kind = None;
        if let Token::Ident(k) = self.peek().clone() {
            if k == "kind" {
                self.advance();
                self.expect(&Token::Colon)?;
                if let Token::Ident(v) = self.peek().clone() {
                    match v.as_str() {
                        "query" => kind = Some(EndpointKind::Query),
                        "mutation" => kind = Some(EndpointKind::Mutation),
                        "server" => kind = Some(EndpointKind::Server),
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Unknown endpoint kind. Expected query, mutation, or server.",
                                vec!["query".into(), "mutation".into(), "server".into()],
                                Some(v),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    self.advance();
                }
            }
        }
        self.expect(&Token::RParen)?;
        if kind.is_none() {
            self.errors.push(ParseError::classified(
                self.span(),
                "Expected `kind: query`, `kind: mutation`, or `kind: server` inside `@endpoint(...)`.",
                vec!["kind: query".into()],
                Some(self.peek().to_string()),
                ParseErrorClass::Declaration,
            ));
            return Err(());
        }
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Endpoint(EndpointDecl { kind: kind.unwrap(), func: f }))
    }

    pub(crate) fn parse_fn_decl(&mut self, is_pub: bool) -> Result<FnDecl, ()> {
        let start = self.span();
        let mut preconditions = Vec::new();
        let mut postconditions = Vec::new();
        let mut invariants = Vec::new();
        let mut is_mobile_native = false;
        let mut is_pure = false;
        let mut is_reactive = false;
        let mut is_deprecated = false;
        let mut is_llm = false;
        let mut llm_model = None;

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
                    let condition = self.parse_expr()?;
                    let mut fallback = None;
                    if self.eat(&Token::Comma) {
                        if let Token::Ident(k) = self.peek().clone() {
                            if k == "fallback" {
                                self.advance();
                                self.expect(&Token::Colon)?;
                                fallback = Some(self.parse_ident_name()?);
                            }
                        }
                    }
                    postconditions.push(PostCondition {
                        condition,
                        fallback,
                    });
                    self.expect(&Token::RParen)?;
                }
                Token::AtInvariant => {
                    self.advance();
                    self.expect(&Token::LParen)?;
                    invariants.push(self.parse_expr()?);
                    self.expect(&Token::RParen)?;
                }
                Token::AtPure => {
                    self.advance();
                    is_pure = true;
                }
                Token::AtReactive => {
                    self.advance();
                    is_reactive = true;
                }
                Token::AtDeprecated => {
                    self.advance();
                    is_deprecated = true;
                }
                Token::AtFuzz | Token::AtNative => {
                    self.advance();
                    is_mobile_native = true;
                }
                Token::AtAi => {
                    self.advance();
                    is_llm = true;
                    if self.eat(&Token::LParen) {
                        if let Token::Ident(key) = self.peek().clone() {
                            if key == "model" {
                                self.advance();
                                self.expect(&Token::Eq)?;
                                if let Token::StringLit(m) = self.peek().clone() {
                                    self.advance();
                                    llm_model = Some(m);
                                }
                            }
                        }
                        self.expect(&Token::RParen)?;
                    }
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
        let effects = self.parse_uses_clause();
        let return_type = if self.eat_return_arrow() {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let body = if is_llm && !matches!(self.peek(), Token::LBrace) {
            vec![]
        } else {
            self.expect(&Token::LBrace)?;
            self.parse_block()?
        };
        Ok(FnDecl {
            name,
            generics,
            params,
            return_type,
            body,
            is_async: false,
            is_deprecated,
            is_pure,
            is_reactive,
            is_llm,
            llm_model,
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
            effects,
            span: start.merge(self.span()),
        })
    }

    pub(crate) fn parse_ident_name(&mut self) -> Result<String, ()> {
        match self.peek().clone() {
            Token::Ident(n) | Token::TypeIdent(n) => {
                self.advance();
                Ok(n)
            }
            Token::TypeKw => {
                self.advance();
                Ok("type".to_string())
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
            Token::Http => {
                self.advance();
                Ok("http".to_string())
            }
            Token::Env => {
                self.advance();
                Ok("env".to_string())
            }
            Token::To => {
                self.advance();
                Ok("to".to_string())
            }
            Token::In => {
                self.advance();
                Ok("in".to_string())
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

    /// Parse an optional `uses <effect-list>` clause after `)` in a function signature.
    ///
    /// Grammar: `uses (<effect-name> | mcp(<tool-name>)) (',' (<effect-name> | mcp(<tool-name>)))*`
    /// where `effect-name` ∈ {net, db, fs, env, clock, random, spawn, nothing}.
    ///
    /// Returns an empty vec when no `uses` keyword is present (unannotated = unconstrained).
    pub(crate) fn parse_uses_clause(&mut self) -> Vec<crate::ast::decl::EffectAnnotation> {
        // `uses` is a contextual keyword — check by value, not token type.
        let is_uses = matches!(self.peek(), Token::Ident(n) if n == "uses");
        if !is_uses {
            return Vec::new();
        }
        self.advance(); // eat `uses`

        let mut effects = Vec::new();
        loop {
            let eff = match self.peek().clone() {
                Token::Ident(ref name) => {
                    let name = name.clone();
                    if name == "mcp" {
                        self.advance(); // eat `mcp`
                        if self.eat(&Token::LParen) {
                            let tool = match self.peek().clone() {
                                Token::Ident(t) | Token::TypeIdent(t) => {
                                    self.advance();
                                    t
                                }
                                _ => {
                                    self.errors.push(ParseError::classified(
                                        self.span(),
                                        "Expected MCP tool name inside `mcp(...)`",
                                        vec!["tool_name".into()],
                                        Some(self.peek().to_string()),
                                        ParseErrorClass::Declaration,
                                    ));
                                    return effects;
                                }
                            };
                            let _ = self.expect(&Token::RParen);
                            crate::ast::decl::EffectAnnotation::Mcp(tool)
                        } else {
                            crate::ast::decl::EffectAnnotation::Mcp(String::new())
                        }
                    } else if let Some(eff) = crate::ast::decl::EffectAnnotation::from_keyword(&name) {
                        self.advance();
                        eff
                    } else {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            format!("Unknown effect `{name}`; expected one of: net, db, fs, env, clock, random, spawn, mcp(…), nothing"),
                            vec!["net".into(), "db".into(), "fs".into(), "env".into(), "clock".into(), "random".into(), "spawn".into(), "mcp(…)".into(), "nothing".into()],
                            Some(name),
                            ParseErrorClass::Declaration,
                        ));
                        return effects;
                    }
                }
                // `env` is a keyword token, allow it here.
                Token::Env => {
                    self.advance();
                    crate::ast::decl::EffectAnnotation::Env
                }
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Expected effect name after `uses`",
                        vec!["net".into(), "db".into(), "nothing".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return effects;
                }
            };
            effects.push(eff);
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        effects
    }
}
