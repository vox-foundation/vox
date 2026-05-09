// Top-level and declaration parsing.

use super::super::Parser;
use crate::ast::decl::{
    BackButtonDecl, Decl, DeepLinkDecl, EffectDecl, EndpointDecl, EndpointKind, FieldConstraint,
    FnDecl, ForallDecl, FormDecl, FormField, ImportDecl, ImportPath, ImportPathKind, LoadingDecl,
    McpResourceDecl, McpToolDecl, OnCleanupDecl, OnMountDecl, PostCondition, PushDecl,
    ReactiveComponentDecl, ReactiveMemberDecl, RustCrateImport, ScheduledDecl, TestDecl,
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
            // `rust:` imports use the full parse_import_path handler.
            // All symbol imports go through parse_symbol_import which also accepts
            // `/` as a path separator and `as { name1, name2 }` destructuring.
            let first_is_rust =
                matches!(self.peek(), Token::Ident(n) if n == "rust");
            if first_is_rust {
                let path = self.parse_import_path()?;
                paths.push(path);
            } else {
                self.parse_symbol_import(&mut paths)?;
            }
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(Decl::Import(ImportDecl {
            paths,
            span: start.merge(self.span()),
        }))
    }

    /// Parse one symbol import declaration, appending zero or more `ImportPath`s to `paths`.
    ///
    /// Handles three forms:
    ///   `import lib.chrome.StateChip`          — dotted single-item
    ///   `import lib/chrome.StateChip`          — slash-separated (equivalent)
    ///   `import lib/chrome as { A, B, C }`     — destructured multi-item (ES6-style)
    ///   `import lib.chrome as Alias`           — whole-module alias
    fn parse_symbol_import(&mut self, paths: &mut Vec<ImportPath>) -> Result<(), ()> {
        let seg_start = self.span();

        // ── collect path segments (separated by '.' or '/') ──────────────────
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
                    "Import path must begin with an identifier (for example `lib.chrome.StateChip` or `lib/chrome as { StateChip }`).",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };

        let mut segments = vec![first];
        loop {
            // Accept both '.' and '/' as path separators.
            if !matches!(self.peek(), Token::Dot | Token::Slash) {
                break;
            }
            self.advance(); // eat '.' or '/'
            match self.peek().clone() {
                Token::Ident(name) | Token::TypeIdent(name) => {
                    segments.push(name);
                    self.advance();
                }
                Token::Env => {
                    self.advance();
                    segments.push("env".to_string());
                }
                Token::Http => {
                    self.advance();
                    segments.push("http".to_string());
                }
                _ => break,
            }
        }

        // ── check for `as …` ──────────────────────────────────────────────────
        let has_as = matches!(self.peek(), Token::Ident(w) if w == "as");
        if has_as {
            self.advance(); // eat 'as'
            if matches!(self.peek(), Token::LBrace) {
                self.advance(); // eat '{'
                // Destructured form: `import lib/chrome as { StateChip, TopBar }`
                // Expand into one ImportPath per item (item appended to segments).
                loop {
                    if matches!(self.peek(), Token::RBrace) {
                        break;
                    }
                    let item = match self.peek().clone() {
                        Token::Ident(name) | Token::TypeIdent(name) => {
                            self.advance();
                            name
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected identifier inside destructured import `as { ... }`.",
                                vec!["identifier".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };
                    // Optional `as item_alias` inside the braces.
                    let item_alias =
                        if matches!(self.peek(), Token::Ident(w) if w == "as") {
                            self.advance();
                            Some(self.parse_ident_name()?)
                        } else {
                            None
                        };
                    let mut full = segments.clone();
                    full.push(item.clone());
                    paths.push(ImportPath {
                        kind: ImportPathKind::SymbolPath { segments: full },
                        alias: item_alias,
                        span: seg_start.merge(self.span()),
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RBrace)?;
            } else {
                // Single alias: `import lib.chrome as chrome`
                let alias_name = self.parse_ident_name()?;
                paths.push(ImportPath {
                    kind: ImportPathKind::SymbolPath { segments },
                    alias: Some(alias_name),
                    span: seg_start.merge(self.span()),
                });
            }
        } else {
            // No `as` — last segment is the item name.
            paths.push(ImportPath {
                kind: ImportPathKind::SymbolPath { segments },
                alias: None,
                span: seg_start.merge(self.span()),
            });
        }
        Ok(())
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
                Err(())
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
                    self.advance(); // eat `effect`
                    // Optional `depends_on (a, b)` clause.
                    let explicit_deps =
                        if matches!(self.peek(), Token::Ident(n) if n == "depends_on") {
                            self.advance(); // eat `depends_on`
                            self.expect(&Token::LParen)?;
                            let mut deps = Vec::new();
                            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                                deps.push(self.parse_ident_name()?);
                                if !self.eat(&Token::Comma) {
                                    break;
                                }
                            }
                            self.expect(&Token::RParen)?;
                            Some(deps)
                        } else {
                            None
                        };
                    self.expect(&Token::Colon)?;
                    let body = if matches!(self.peek(), Token::LBrace) {
                        let b_start = self.span();
                        self.advance(); // eat `{`
                        let stmts = self.parse_block()?;
                        crate::ast::expr::Expr::Block {
                            stmts,
                            span: b_start.merge(self.span()),
                        }
                    } else {
                        self.parse_expr()?
                    };
                    members.push(ReactiveMemberDecl::Effect(EffectDecl {
                        body,
                        explicit_deps,
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
                Token::State => members.push(ReactiveMemberDecl::State(self.parse_state_decl()?)),
                Token::Derived => {
                    members.push(ReactiveMemberDecl::Derived(self.parse_derived_decl()?))
                }
                Token::Effect => {
                    let eff_start = self.span();
                    self.advance(); // eat `effect`
                    // Optional `depends_on (a, b)` clause.
                    let explicit_deps =
                        if matches!(self.peek(), Token::Ident(n) if n == "depends_on") {
                            self.advance(); // eat `depends_on`
                            self.expect(&Token::LParen)?;
                            let mut deps = Vec::new();
                            while !matches!(self.peek(), Token::RParen | Token::Eof) {
                                deps.push(self.parse_ident_name()?);
                                if !self.eat(&Token::Comma) {
                                    break;
                                }
                            }
                            self.expect(&Token::RParen)?;
                            Some(deps)
                        } else {
                            None
                        };
                    self.expect(&Token::Colon)?;
                    let body = if matches!(self.peek(), Token::LBrace) {
                        let b_start = self.span();
                        self.advance(); // eat `{`
                        let stmts = self.parse_block()?;
                        crate::ast::expr::Expr::Block {
                            stmts,
                            span: b_start.merge(self.span()),
                        }
                    } else {
                        self.parse_expr()?
                    };
                    members.push(ReactiveMemberDecl::Effect(EffectDecl {
                        body,
                        explicit_deps,
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

    /// `@loading fn Name() to Element { ... }` — TanStack Router `pendingComponent` / suspense UI.
    pub(crate) fn parse_loading(&mut self) -> Result<Decl, ()> {
        self.advance(); // @loading
        self.skip_newlines();
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Loading(LoadingDecl { func: f }))
    }

    /// One v0 component prop line: `name`, optional `?`, `:`, type (OP-0006).
    pub(crate) fn parse_v0_prop_line(&mut self) -> Result<crate::ast::decl::V0Prop, ()> {
        let pname = self.parse_ident_name()?;
        if std::env::var_os("VOX_PARSER_DEBUG").is_some() {
            eprintln!("[vox-parser:v0.prop] name={pname:?} next={:?}", self.peek());
        }
        let is_optional = self.eat(&Token::Question);
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;
        Ok(crate::ast::decl::V0Prop {
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
            if let Token::StringLit(s) = self.peek().clone() {
                self.advance();
                label = s;
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

    pub(crate) fn parse_endpoint(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @endpoint
        self.expect(&Token::LParen)?;
        let mut kind = None;
        if let Token::Ident(k) = self.peek().clone()
            && k == "kind"
        {
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
        Ok(Decl::Endpoint(EndpointDecl {
            kind: kind.unwrap(),
            func: f,
        }))
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
                    if self.eat(&Token::Comma)
                        && let Token::Ident(k) = self.peek().clone()
                        && k == "fallback"
                    {
                        self.advance();
                        self.expect(&Token::Colon)?;
                        fallback = Some(self.parse_ident_name()?);
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
                        if let Token::Ident(key) = self.peek().clone()
                            && key == "model"
                        {
                            self.advance();
                            self.expect(&Token::Eq)?;
                            if let Token::StringLit(m) = self.peek().clone() {
                                self.advance();
                                llm_model = Some(m);
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
            ts_extern_module: None,
            effects,
            span: start.merge(self.span()),
        })
    }

    /// Parse `extern fn name(args) to T = "./module"` (TS-source FFI, plan 6).
    /// The body is empty; codegen-TS emits `import { name } from "./module"`.
    pub(crate) fn parse_extern_fn(&mut self) -> Result<crate::ast::decl::Decl, ()> {
        use crate::parser::error::{ParseError, ParseErrorClass};
        let start = self.span();
        self.advance(); // eat `extern`
        self.expect(&Token::Fn)?;
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat_return_arrow() {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let module = match self.peek().clone() {
            Token::StringLit(s) | Token::SingleStringLit(s) => {
                self.advance();
                s
            }
            other => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected string literal module path after `=` in extern fn",
                    vec!["\"./module\"".into()],
                    Some(other.to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
        Ok(crate::ast::decl::Decl::Function(FnDecl {
            name,
            generics: vec![],
            params,
            return_type,
            body: vec![],
            is_async: false,
            is_deprecated: false,
            is_pure: false,
            is_reactive: false,
            effects: vec![],
            is_traced: false,
            is_llm: false,
            llm_model: None,
            is_pub: true,
            auth_provider: None,
            roles: vec![],
            cors: None,
            preconditions: vec![],
            postconditions: vec![],
            invariants: vec![],
            verify_mode: crate::ast::decl::fundecl::VerifyMode::Off,
            test_strategy: None,
            is_mobile_native: false,
            ts_extern_module: Some(module),
            span: start.merge(self.span()),
        }))
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

    /// Parse a `@form Name { field ... on_submit: ... }` declaration.
    pub(crate) fn parse_form_decl(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @form
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;

        let mut fields: Vec<FormField> = Vec::new();
        let mut on_submit: Option<String> = None;
        let mut success_redirect: Option<String> = None;
        let mut error_message: Option<String> = None;

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::RBrace | Token::Eof => break,
                Token::Ident(ref kw) if kw == "field" => {
                    self.advance(); // eat `field`
                    fields.push(self.parse_form_field()?);
                }
                Token::Ident(ref kw) if kw == "on_submit" => {
                    self.advance(); // eat `on_submit`
                    self.expect(&Token::Colon)?;
                    on_submit = Some(self.parse_ident_name()?);
                }
                Token::Ident(ref kw) if kw == "success_redirect" => {
                    self.advance(); // eat `success_redirect`
                    self.expect(&Token::Colon)?;
                    match self.peek().clone() {
                        Token::StringLit(s) => {
                            self.advance();
                            success_redirect = Some(s);
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected string literal for success_redirect",
                                vec!["\"...\"".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                }
                Token::Ident(ref kw) if kw == "error_message" => {
                    self.advance(); // eat `error_message`
                    self.expect(&Token::Colon)?;
                    match self.peek().clone() {
                        Token::StringLit(s) => {
                            self.advance();
                            error_message = Some(s);
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected string literal for error_message",
                                vec!["\"...\"".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                }
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Unexpected token inside @form block: {other}; expected `field`, `on_submit`, `success_redirect`, or `}}`"),
                        vec!["field".into(), "on_submit".into(), "success_redirect".into(), "}".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;

        Ok(Decl::Form(FormDecl {
            name,
            fields,
            on_submit,
            success_redirect,
            error_message,
            span: start.merge(self.span()),
        }))
    }

    /// Parse a single `field name: Type [constraints...] [required|optional] [hidden]` line
    /// inside a `@form` block. Cursor is positioned after the `field` keyword.
    fn parse_form_field(&mut self) -> Result<FormField, ()> {
        let start = self.span();
        let name = self.parse_ident_name()?;
        self.expect(&Token::Colon)?;
        let ty = self.parse_type_expr()?;

        let mut constraints: Vec<FieldConstraint> = Vec::new();
        let mut required = false;
        let mut hidden = false;
        let mut label: Option<String> = None;
        let mut default: Option<crate::ast::expr::Expr> = None;

        // Parse optional modifiers on the same line until newline or `}`
        loop {
            match self.peek().clone() {
                Token::Newline | Token::RBrace | Token::Eof => break,
                Token::Ident(ref kw) if kw == "required" => {
                    self.advance();
                    required = true;
                }
                Token::Ident(ref kw) if kw == "optional" => {
                    self.advance();
                    required = false;
                }
                Token::Ident(ref kw) if kw == "hidden" => {
                    self.advance();
                    hidden = true;
                }
                Token::Ident(ref kw) if kw == "label" => {
                    self.advance(); // eat `label`
                    self.expect(&Token::LParen)?;
                    match self.peek().clone() {
                        Token::StringLit(s) => {
                            self.advance();
                            label = Some(s);
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected string literal inside label(...)",
                                vec!["\"label text\"".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                Token::Ident(ref kw) if kw == "range" => {
                    self.advance(); // eat `range`
                    self.expect(&Token::LParen)?;
                    // Parse `lo..hi` — lexed as IntLit, Dot, Dot, IntLit
                    let lo_span = self.span();
                    let lo = match self.peek().clone() {
                        Token::IntLit(v) => {
                            let span = self.span();
                            self.advance();
                            crate::ast::expr::Expr::IntLit { value: v, span }
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected integer literal for range lower bound",
                                vec!["1".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };
                    // Consume two dots: `..`
                    self.expect(&Token::Dot)?;
                    self.expect(&Token::Dot)?;
                    let hi = match self.peek().clone() {
                        Token::IntLit(v) => {
                            let span = self.span();
                            self.advance();
                            crate::ast::expr::Expr::IntLit { value: v, span }
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected integer literal for range upper bound",
                                vec!["10".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };
                    self.expect(&Token::RParen)?;
                    let _ = lo_span;
                    constraints.push(FieldConstraint::Range(lo, hi));
                }
                Token::Ident(ref kw) if kw == "max_len" => {
                    self.advance(); // eat `max_len`
                    self.expect(&Token::LParen)?;
                    match self.peek().clone() {
                        Token::IntLit(v) => {
                            self.advance();
                            constraints.push(FieldConstraint::MaxLen(v as usize));
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected integer literal inside max_len(...)",
                                vec!["280".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                Token::Ident(ref kw) if kw == "min_len" => {
                    self.advance(); // eat `min_len`
                    self.expect(&Token::LParen)?;
                    match self.peek().clone() {
                        Token::IntLit(v) => {
                            self.advance();
                            constraints.push(FieldConstraint::MinLen(v as usize));
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected integer literal inside min_len(...)",
                                vec!["1".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                Token::Ident(ref kw) if kw == "pattern" => {
                    self.advance(); // eat `pattern`
                    self.expect(&Token::LParen)?;
                    match self.peek().clone() {
                        Token::StringLit(s) => {
                            self.advance();
                            constraints.push(FieldConstraint::Pattern(s));
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected string literal inside pattern(...)",
                                vec!["\"^[a-z]+$\"".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    self.expect(&Token::RParen)?;
                }
                Token::Ident(ref kw) if kw == "default" => {
                    self.advance(); // eat `default`
                    self.expect(&Token::LParen)?;
                    default = Some(self.parse_expr()?);
                    self.expect(&Token::RParen)?;
                }
                _ => break,
            }
        }

        Ok(FormField {
            name,
            ty,
            label,
            required,
            hidden,
            default,
            constraints,
            span: start.merge(self.span()),
        })
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
                    } else if let Some(eff) =
                        crate::ast::decl::EffectAnnotation::from_keyword(&name)
                    {
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

    // ── Mobile Capacitor primitives (Tasks D2-D4) ─────────────────────────

    /// Parse `@back_button { on_press: handler [fallback: handler] }`.
    pub(crate) fn parse_back_button_decl(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @back_button
        self.expect(&Token::LBrace)?;
        let mut on_press = String::new();
        let mut fallback: Option<String> = None;
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let key = match self.peek().clone() {
                Token::Ident(k) => { self.advance(); k }
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Expected field name inside @back_button block, got `{other}`"),
                        vec!["on_press".into(), "fallback".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };
            self.expect(&Token::Colon)?;
            let val = self.parse_ident_name()?;
            match key.as_str() {
                "on_press" => on_press = val,
                "fallback" => fallback = Some(val),
                _ => {}
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        Ok(Decl::BackButton(BackButtonDecl {
            on_press,
            fallback,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `@deep_link { scheme: "…" on_link: handler [universal_link: "…"] }`.
    pub(crate) fn parse_deep_link_decl(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @deep_link
        self.expect(&Token::LBrace)?;
        let mut scheme = String::new();
        let mut universal_link: Option<String> = None;
        let mut on_link = String::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let key = match self.peek().clone() {
                Token::Ident(k) => { self.advance(); k }
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Expected field name inside @deep_link block, got `{other}`"),
                        vec!["scheme".into(), "on_link".into(), "universal_link".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };
            self.expect(&Token::Colon)?;
            // Values are either string literals or identifiers.
            let val = match self.peek().clone() {
                Token::StringLit(s) => { self.advance(); s }
                Token::Ident(_) | Token::TypeIdent(_) => self.parse_ident_name()?,
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Expected string or identifier as value in @deep_link block, got `{other}`"),
                        vec!["\"…\"".into(), "identifier".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };
            match key.as_str() {
                "scheme" => scheme = val,
                "universal_link" => universal_link = Some(val),
                "on_link" => on_link = val,
                _ => {}
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        Ok(Decl::DeepLink(DeepLinkDecl {
            scheme,
            universal_link,
            on_link,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `@push { [on_register: handler] [on_notification: handler] [on_action: handler] }`.
    pub(crate) fn parse_push_decl(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @push
        self.expect(&Token::LBrace)?;
        let mut on_register: Option<String> = None;
        let mut on_notification: Option<String> = None;
        let mut on_action: Option<String> = None;
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let key = match self.peek().clone() {
                Token::Ident(k) => { self.advance(); k }
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!("Expected field name inside @push block, got `{other}`"),
                        vec!["on_register".into(), "on_notification".into(), "on_action".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            };
            self.expect(&Token::Colon)?;
            let val = self.parse_ident_name()?;
            match key.as_str() {
                "on_register" => on_register = Some(val),
                "on_notification" => on_notification = Some(val),
                "on_action" => on_action = Some(val),
                _ => {}
            }
            self.skip_newlines();
        }
        self.expect(&Token::RBrace)?;
        Ok(Decl::Push(PushDecl {
            on_register,
            on_notification,
            on_action,
            span: start.merge(self.span()),
        }))
    }
}
