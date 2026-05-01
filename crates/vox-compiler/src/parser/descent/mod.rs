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
use crate::parser::error::{ParseError, ParseErrorClass, ParseSeverity};

/// Strict parse: returns [`Module`] or **all** accumulated [`ParseError`] values.
pub fn parse(tokens: Vec<Spanned>) -> Result<Module, Vec<ParseError>> {
    let mut p = Parser::new(tokens);
    p.parse_module()
}

/// Script-mode parse (audit item A.1 — `vox run --mode script`).
///
/// Wraps any top-level *statements* (let bindings, expressions, assignments,
/// control flow) in a synthetic `fn main() { ... }` so that script files
/// like `scripts/foo.vox` work without requiring a hand-written `fn main`.
///
/// Top-level *declarations* (`fn`, `import`, `type`, `@table`, …) are kept
/// as-is and placed before the synthetic main.  A mixed file is valid.
///
/// Returns `Err(errors)` if any parse errors were accumulated.
pub fn parse_script(tokens: Vec<Spanned>) -> Result<Module, Vec<ParseError>> {
    let mut p = Parser::new(tokens);
    p.parse_module_script()
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

    pub(crate) fn eat_return_arrow(&mut self) -> bool {
        if self.eat(&Token::Arrow) {
            let mut err = ParseError::classified(
                self.span(),
                "The '->' syntax is deprecated for return types. Use 'to'.",
                vec![],
                None,
                ParseErrorClass::Expression,
            );
            err.severity = ParseSeverity::Warning;
            self.errors.push(err);
            true
        } else {
            self.eat(&Token::To)
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
        if self
            .errors
            .iter()
            .any(|e| e.severity == ParseSeverity::Error)
        {
            Err(self.errors.clone())
        } else {
            Ok(Module {
                declarations: decls,
                span: start.merge(self.span()),
            })
        }
    }

    /// Script-mode module parse.
    ///
    /// Tokens that look like top-level declarations (`fn`, `import`, `type`,
    /// `@…`, `actor`, `workflow`, `component`, `routes`, `http`, `let`,
    /// `async`) are parsed as declarations.  Everything else at the top
    /// level is parsed as a *statement* and accumulated into the body of a
    /// synthetic `fn main() {}` appended at the end of the module.
    pub(crate) fn parse_module_script(&mut self) -> Result<Module, Vec<ParseError>> {
        use crate::ast::decl::fundecl::VerifyMode;
        use crate::ast::decl::{Decl, FnDecl};

        let start = self.span();
        let mut decls: Vec<Decl> = Vec::new();
        let mut script_stmts: Vec<crate::ast::stmt::Stmt> = Vec::new();
        let script_start = self.span();

        self.skip_newlines();
        while !matches!(self.peek(), Token::Eof) {
            // Tokens that unambiguously begin a declaration at the top level.
            let is_decl_position = matches!(
                self.peek(),
                Token::Import
                    | Token::AtComponent
                    | Token::Component
                    | Token::AtIsland
                    | Token::AtLoading
                    | Token::AtTest
                    | Token::AtV0
                    | Token::AtServer
                    | Token::AtQuery
                    | Token::AtMutation
                    | Token::AtEndpoint
                    | Token::AtForall
                    | Token::AtScheduled
                    | Token::AtTool
                    | Token::AtMcpTool
                    | Token::AtResource
                    | Token::AtMcpResource
                    | Token::Fn
                    | Token::Pub
                    | Token::TypeKw
                    | Token::Actor
                    | Token::Agent
                    | Token::Env
                    | Token::Workflow
                    | Token::Activity
                    | Token::Http
                    | Token::AtTable
                    | Token::AtIndex
                    | Token::Async
            ) || matches!(self.peek(), Token::Ident(n) if n == "routes" || n == "url" || n == "state_machine");

            let is_tombstoned = matches!(
                self.peek(),
                Token::Http | Token::AtComponent | Token::Agent | Token::Env
            );

            if is_tombstoned {
                let tok = self.peek().clone();
                self.errors.push(ParseError::classified(
                    self.span(),
                    format!("The `{tok}` construct is tombstoned and no longer supported. Use standard functions and MCP skills instead."),
                    vec![],
                    Some(tok.to_string()),
                    ParseErrorClass::Tombstoned,
                ));
                self.advance();
                self.recover_to_top_level();
            } else if is_decl_position {
                match self.parse_decl() {
                    Ok(d) => decls.push(d),
                    Err(_) => self.recover_to_top_level(),
                }
            } else {
                // Statement position — parse as script body stmt.
                match self.parse_stmt() {
                    Ok(s) => script_stmts.push(s),
                    Err(()) => {
                        // Recovery: skip past newline or EOF.
                        while !matches!(self.peek(), Token::Newline | Token::Eof) {
                            self.advance();
                        }
                    }
                }
            }
            self.skip_newlines();
        }

        // Wrap accumulated script statements in a synthetic fn main().
        if !script_stmts.is_empty() {
            let script_end = self.span();
            let main_fn = FnDecl {
                name: "main".to_string(),
                generics: vec![],
                params: vec![],
                return_type: None,
                body: script_stmts,
                is_async: false,
                is_deprecated: false,
                is_pure: false,
                is_traced: false,
                is_llm: false,
                llm_model: None,
                is_pub: false,
                auth_provider: None,
                roles: vec![],
                cors: None,
                preconditions: vec![],
                postconditions: vec![],
                invariants: vec![],
                verify_mode: VerifyMode::Off,
                test_strategy: None,
                is_mobile_native: false,
                effects: vec![],
                span: script_start.merge(script_end),
            };
            decls.push(Decl::Function(main_fn));
        }

        if self
            .errors
            .iter()
            .any(|e| e.severity == ParseSeverity::Error)
        {
            Err(self.errors.clone())
        } else {
            Ok(Module {
                declarations: decls,
                span: start.merge(self.span()),
            })
        }
    }

    pub(crate) fn recover_to_top_level(&mut self) {
        let mut brace_depth = 0;
        loop {
            match self.peek() {
                Token::Eof => break,
                Token::LBrace => {
                    brace_depth += 1;
                    self.advance();
                }
                Token::RBrace => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                        self.advance();
                        if brace_depth == 0 {
                            break;
                        }
                    } else {
                        self.advance();
                        break;
                    }
                }
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
                | Token::AtEndpoint
                | Token::AtTable
                | Token::TypeKw
                | Token::Agent
                | Token::Env
                | Token::Component
                | Token::AtForall
                | Token::AtScheduled
                | Token::AtRequire
                | Token::AtEnsure
                | Token::AtInvariant
                | Token::AtFuzz
                | Token::AtPure
                | Token::AtAi
                | Token::AtDeprecated
                | Token::AtLoading
                | Token::Let
                | Token::Agent
                | Token::Env
                | Token::Async
                    if brace_depth == 0 =>
                {
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
            Token::Component => self.parse_reactive_component(),
            Token::AtIsland => self.parse_island(),
            Token::AtV0 => self.parse_v0_component(),
            Token::AtLoading => self.parse_loading(),
            Token::AtTest => self.parse_test(),
            Token::AtServer => self.parse_server_fn(),
            Token::AtQuery => self.parse_query_fn(),
            Token::AtMutation => self.parse_mutation_fn(),
            Token::AtEndpoint => self.parse_endpoint(),
            Token::AtForall => self.parse_forall(),
            Token::AtScheduled => self.parse_scheduled(),
            Token::AtTool | Token::AtMcpTool => self.parse_mcp_tool(),
            Token::AtResource | Token::AtMcpResource => self.parse_mcp_resource(),
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
                    | Token::AtPure
                    | Token::AtAi
                    | Token::AtDeprecated
                    | Token::AtNative => {
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
            Token::Fn
            | Token::AtRequire
            | Token::AtEnsure
            | Token::AtInvariant
            | Token::AtFuzz
            | Token::AtPure
            | Token::AtAi
            | Token::AtDeprecated
            | Token::AtNative => {
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
                    | Token::AtPure
                    | Token::AtAi
                    | Token::AtDeprecated
                    | Token::AtNative => {
                        let f = self.parse_fn_decl(true)?;
                        Ok(Decl::Function(f))
                    }
                    Token::TypeKw => self.parse_typedef(true),
                    Token::Ident(ref name) if name == "url" => self.parse_url_decl(true),
                    Token::Ident(ref name) if name == "state_machine" => {
                        self.parse_state_machine_decl(true, false)
                    }
                    Token::Ident(ref name) if name == "partial" => {
                        self.advance(); // eat `partial`
                        match self.peek().clone() {
                            Token::Ident(ref n) if n == "state_machine" => {
                                self.parse_state_machine_decl(true, true)
                            }
                            _ => {
                                self.errors.push(ParseError::classified(
                                    self.span(),
                                    "Expected `state_machine` after `partial`",
                                    vec!["state_machine".into()],
                                    Some(self.peek().to_string()),
                                    ParseErrorClass::Declaration,
                                ));
                                Err(())
                            }
                        }
                    }
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
            Token::AtIndex => self.parse_index(),
            Token::Workflow | Token::Activity | Token::Actor | Token::Http | Token::AtComponent | Token::Agent | Token::Env => {
                let tok = self.peek().clone();
                self.errors.push(ParseError::classified(
                    self.span(),
                    format!("The `{tok}` construct is tombstoned and no longer supported. Use standard functions and MCP skills instead."),
                    vec![],
                    Some(tok.to_string()),
                    ParseErrorClass::Tombstoned,
                ));
                self.advance();
                Err(())
            }
            Token::TypeKw => self.parse_typedef(false),
            Token::Ident(ref name) if name == "url" => self.parse_url_decl(false),
            Token::Ident(ref name) if name == "state_machine" => {
                self.parse_state_machine_decl(false, false)
            }
            Token::Ident(ref name) if name == "partial" => {
                // `partial state_machine Name { … }`
                self.advance(); // eat `partial`
                match self.peek().clone() {
                    Token::Ident(ref n) if n == "state_machine" => {
                        self.parse_state_machine_decl(false, true)
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected `state_machine` after `partial`",
                            vec!["state_machine".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        Err(())
                    }
                }
            }
            Token::AtTable => self.parse_table(),
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
