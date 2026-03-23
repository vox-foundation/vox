//! Single-module recursive-descent parser implementation.
//!
//! **This is the only parser implementation** for `vox-parser`. There is no
//! secondary parser, no multi-module rewrite, and no separate LSP tree-sitter
//! layer in this crate. The public entry point is [`parse`].
//!
//! See `crate` (lib.rs) for the scope table — what constructs are in/out of scope.

use crate::error::ParseError;
use vox_ast::decl::*;
use vox_ast::expr::*;
use vox_ast::pattern::Pattern;
use vox_ast::span::Span;
use vox_ast::stmt::Stmt;
use vox_ast::types::TypeExpr;
use vox_lexer::cursor::Spanned;
use vox_lexer::token::Token;

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
    fn new(tokens: Vec<Spanned>) -> Self {
        Self {
            tokens,
            pos: 0,
            errors: vec![],
        }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.pos)
            .map(|s| &s.token)
            .unwrap_or(&Token::Eof)
    }

    fn span(&self) -> Span {
        self.tokens
            .get(self.pos)
            .map(|s| Span::new(s.span.start, s.span.end))
            .unwrap_or(Span::new(0, 0))
    }

    fn advance(&mut self) -> Token {
        let t = self
            .tokens
            .get(self.pos)
            .map(|s| s.token.clone())
            .unwrap_or(Token::Eof);
        self.pos += 1;
        t
    }

    fn expect(&mut self, expected: &Token) -> Result<Span, ()> {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            let sp = self.span();
            self.advance();
            Ok(sp)
        } else {
            self.errors.push(ParseError::new(
                self.span(),
                format!("Expected {expected}, found {}", self.peek()),
                vec![expected.to_string()],
                Some(self.peek().to_string()),
            ));
            Err(())
        }
    }

    fn eat(&mut self, expected: &Token) -> bool {
        if std::mem::discriminant(self.peek()) == std::mem::discriminant(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn skip_newlines(&mut self) {
        while matches!(self.peek(), Token::Newline) {
            self.advance();
        }
    }

    fn parse_module(&mut self) -> Result<Module, Vec<ParseError>> {
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

    fn recover_to_top_level(&mut self) {
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
                | Token::AtV0 => break,
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

    fn parse_decl(&mut self) -> Result<Decl, ()> {
        self.skip_newlines();
        match self.peek().clone() {
            Token::Import => self.parse_import(),
            Token::AtComponent => self.parse_component(),
            Token::AtIsland => self.parse_island(),
            Token::AtTest => self.parse_test(),
            Token::AtServer => self.parse_server_fn(),
            Token::AtV0 => self.parse_v0_component(),
            Token::AtMcpTool => self.parse_mcp_tool(),
            Token::Fn => {
                let f = self.parse_fn_decl(false)?;
                Ok(Decl::Function(f))
            }
            Token::Pub => {
                self.advance();
                match self.peek().clone() {
                    Token::Fn => {
                        let f = self.parse_fn_decl(true)?;
                        Ok(Decl::Function(f))
                    }
                    Token::TypeKw => self.parse_typedef(true),
                    _ => {
                        self.errors.push(ParseError::new(
                            self.span(),
                            "Expected fn or type after pub",
                            vec!["fn".into(), "type".into()],
                            Some(self.peek().to_string()),
                        ));
                        Err(())
                    }
                }
            }
            Token::TypeKw => self.parse_typedef(false),
            Token::Actor => self.parse_actor(),
            Token::Workflow => self.parse_workflow(),
            Token::Activity => self.parse_activity(),
            Token::Http => self.parse_http_route(),
            Token::AtTable => self.parse_table(false),
            Token::AtIndex => self.parse_index(),
            Token::Ident(ref name) if name == "routes" => self.parse_routes(),
            _ => {
                self.errors.push(ParseError::new(
                    self.span(),
                    format!("Unexpected token at top level: {}", self.peek()),
                    vec!["fn".into(), "import".into(), "type".into()],
                    Some(self.peek().to_string()),
                ));
                Err(())
            }
        }
    }

    fn parse_import(&mut self) -> Result<Decl, ()> {
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

    fn parse_import_path(&mut self) -> Result<ImportPath, ()> {
        let start = self.span();
        let mut segments = Vec::new();
        match self.peek().clone() {
            Token::Ident(name) | Token::TypeIdent(name) => {
                segments.push(name);
                self.advance();
            }
            _ => {
                self.errors.push(ParseError::new(
                    self.span(),
                    "Expected identifier in import path",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
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

    fn parse_component(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @component
        let f = self.parse_fn_decl(false)?;
        // Check for optional style: block after the function body
        let styles = self.parse_style_blocks();
        Ok(Decl::Component(ComponentDecl { func: f, styles }))
    }

    /// `@island Name { prop: Type, prop?: Type }` — brace-delimited prop block.
    fn parse_island(&mut self) -> Result<Decl, ()> {
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

    fn parse_mcp_tool(&mut self) -> Result<Decl, ()> {
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

    fn parse_test(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @test
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::Test(TestDecl { func: f }))
    }

    fn parse_server_fn(&mut self) -> Result<Decl, ()> {
        self.advance(); // eat @server
        let f = self.parse_fn_decl(false)?;
        Ok(Decl::ServerFn(ServerFnDecl { func: f }))
    }

    fn parse_fn_decl(&mut self, is_pub: bool) -> Result<FnDecl, ()> {
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

    fn parse_ident_name(&mut self) -> Result<String, ()> {
        match self.peek().clone() {
            Token::Ident(n) | Token::TypeIdent(n) => {
                self.advance();
                Ok(n)
            }
            _ => {
                self.errors.push(ParseError::new(
                    self.span(),
                    "Expected identifier",
                    vec!["identifier".into()],
                    Some(self.peek().to_string()),
                ));
                Err(())
            }
        }
    }

    fn parse_params(&mut self) -> Result<Vec<Param>, ()> {
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

    fn parse_type_expr(&mut self) -> Result<TypeExpr, ()> {
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
        } else {
            Ok(TypeExpr::Named {
                name,
                span: start.merge(self.span()),
            })
        }
    }

    fn parse_block(&mut self) -> Result<Vec<Stmt>, ()> {
        self.skip_newlines();
        let mut stmts = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            stmts.push(self.parse_stmt()?);
            self.skip_newlines();
        }
        self.eat(&Token::RBrace);
        Ok(stmts)
    }

    fn parse_stmt(&mut self) -> Result<Stmt, ()> {
        let start = self.span();
        match self.peek().clone() {
            Token::Let => self.parse_let_stmt(),
            Token::Ret => {
                self.advance();
                let value = if matches!(self.peek(), Token::Newline | Token::RBrace | Token::Eof) {
                    None
                } else {
                    Some(self.parse_expr()?)
                };
                Ok(Stmt::Return {
                    value,
                    span: start.merge(self.span()),
                })
            }
            _ => {
                let expr = self.parse_expr()?;
                if self.eat(&Token::Eq) {
                    let value = self.parse_expr()?;
                    Ok(Stmt::Assign {
                        target: expr,
                        value,
                        span: start.merge(self.span()),
                    })
                } else {
                    Ok(Stmt::Expr {
                        expr: expr.clone(),
                        span: expr.span(),
                    })
                }
            }
        }
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ()> {
        let start = self.span();
        self.advance(); // eat 'let'
        let mutable = self.eat(&Token::Mut);
        let pattern = self.parse_pattern()?;
        let type_ann = if self.eat(&Token::Colon) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        self.expect(&Token::Eq)?;
        let value = self.parse_expr()?;
        Ok(Stmt::Let {
            pattern,
            type_ann,
            value,
            mutable,
            span: start.merge(self.span()),
        })
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ()> {
        let start = self.span();
        match self.peek().clone() {
            Token::Underscore => {
                self.advance();
                Ok(Pattern::Wildcard { span: start })
            }
            Token::LParen => {
                self.advance();
                let mut elems = Vec::new();
                loop {
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    elems.push(self.parse_pattern()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
                Ok(Pattern::Tuple {
                    elements: elems,
                    span: start.merge(self.span()),
                })
            }
            Token::TypeIdent(name) => {
                self.advance();
                if self.eat(&Token::LParen) {
                    let mut fields = Vec::new();
                    loop {
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        fields.push(self.parse_pattern()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Ok(Pattern::Constructor {
                        name,
                        fields,
                        span: start.merge(self.span()),
                    })
                } else {
                    Ok(Pattern::Ident {
                        name,
                        span: start.merge(self.span()),
                    })
                }
            }
            Token::Ident(name) => {
                self.advance();
                Ok(Pattern::Ident {
                    name,
                    span: start.merge(self.span()),
                })
            }
            Token::IntLit(v) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: Box::new(Expr::IntLit {
                        value: v,
                        span: start,
                    }),
                    span: start,
                })
            }
            Token::StringLit(s) => {
                self.advance();
                Ok(Pattern::Literal {
                    value: Box::new(Expr::StringLit {
                        value: s,
                        span: start,
                    }),
                    span: start,
                })
            }
            _ => {
                self.errors.push(ParseError::new(
                    start,
                    "Expected pattern",
                    vec!["identifier".into(), "_".into()],
                    Some(self.peek().to_string()),
                ));
                Err(())
            }
        }
    }

    // Pratt parser for expressions
    fn parse_expr(&mut self) -> Result<Expr, ()> {
        self.parse_expr_bp(0)
    }

    fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ()> {
        let mut lhs = self.parse_primary()?;
        loop {
            if matches!(self.peek(), Token::With) {
                let (l_bp, r_bp) = (5, 6);
                if l_bp < min_bp {
                    break;
                }
                self.advance();
                let rhs = self.parse_expr_bp(r_bp)?;
                let span = lhs.span().merge(rhs.span());
                lhs = Expr::With {
                    operand: Box::new(lhs),
                    options: Box::new(rhs),
                    span,
                };
                continue;
            }

            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Lte => BinOp::Lte,
                Token::Gte => BinOp::Gte,
                Token::And => BinOp::And,
                Token::Or => BinOp::Or,
                Token::Is => BinOp::Is,
                Token::Isnt => BinOp::Isnt,
                Token::PipeOp => BinOp::Pipe,
                _ => break,
            };
            let (l_bp, r_bp) = infix_bp(op);
            if l_bp < min_bp {
                break;
            }
            self.advance();
            let rhs = self.parse_expr_bp(r_bp)?;
            let span = lhs.span().merge(rhs.span());
            lhs = Expr::Binary {
                op,
                left: Box::new(lhs),
                right: Box::new(rhs),
                span,
            };
        }
        Ok(lhs)
    }

    fn parse_primary(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        let mut expr = match self.peek().clone() {
            Token::IntLit(v) => {
                self.advance();
                Expr::IntLit {
                    value: v,
                    span: start,
                }
            }
            Token::FloatLit(v) => {
                self.advance();
                Expr::FloatLit {
                    value: v,
                    span: start,
                }
            }
            Token::StringLit(s) => {
                self.advance();
                Expr::StringLit {
                    value: s,
                    span: start,
                }
            }
            Token::SingleQuoteStringLit(s) => {
                self.advance();
                Expr::StringLit {
                    value: s,
                    span: start,
                }
            }
            Token::True => {
                self.advance();
                Expr::BoolLit {
                    value: true,
                    span: start,
                }
            }
            Token::False => {
                self.advance();
                Expr::BoolLit {
                    value: false,
                    span: start,
                }
            }
            Token::Not => {
                self.advance();
                let operand = self.parse_primary()?;
                Expr::Unary {
                    op: UnOp::Not,
                    operand: Box::new(operand),
                    span: start.merge(self.span()),
                }
            }
            Token::Minus => {
                self.advance();
                let operand = self.parse_primary()?;
                Expr::Unary {
                    op: UnOp::Neg,
                    operand: Box::new(operand),
                    span: start.merge(self.span()),
                }
            }
            Token::LParen => {
                self.advance();
                let e = self.parse_expr()?;
                if self.eat(&Token::Comma) {
                    let mut elems = vec![e];
                    loop {
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        elems.push(self.parse_expr()?);
                        if !self.eat(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect(&Token::RParen)?;
                    Expr::TupleLit {
                        elements: elems,
                        span: start.merge(self.span()),
                    }
                } else {
                    self.expect(&Token::RParen)?;
                    e
                }
            }
            Token::LBracket => {
                self.advance();
                let mut elems = Vec::new();
                while !matches!(self.peek(), Token::RBracket | Token::Eof) {
                    elems.push(self.parse_expr()?);
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RBracket)?;
                Expr::ListLit {
                    elements: elems,
                    span: start.merge(self.span()),
                }
            }
            Token::LBrace => self.parse_object_lit()?,
            Token::Match => self.parse_match()?,
            Token::If => self.parse_if()?,
            Token::For => self.parse_for()?,
            Token::Fn => self.parse_lambda()?,
            Token::Spawn => {
                self.advance();
                self.expect(&Token::LParen)?;
                let target = self.parse_expr()?;
                self.expect(&Token::RParen)?;
                Expr::Spawn {
                    target: Box::new(target),
                    span: start.merge(self.span()),
                }
            }
            Token::Lt => self.parse_jsx()?,
            Token::Ident(name) => {
                self.advance();
                Expr::Ident { name, span: start }
            }
            Token::TypeIdent(name) => {
                self.advance();
                Expr::Ident { name, span: start }
            }
            _ => {
                self.errors.push(ParseError::new(
                    start,
                    format!("Unexpected token in expression: {}", self.peek()),
                    vec![],
                    Some(self.peek().to_string()),
                ));
                return Err(());
            }
        };
        // Postfix: calls, field access, method calls
        loop {
            match self.peek() {
                Token::LParen => {
                    self.advance();
                    let args = self.parse_args()?;
                    self.expect(&Token::RParen)?;
                    expr = Expr::Call {
                        callee: Box::new(expr),
                        args,
                        span: start.merge(self.span()),
                    };
                }
                Token::Dot => {
                    self.advance();
                    let field = self.parse_ident_name()?;
                    if self.eat(&Token::LParen) {
                        let args = self.parse_args()?;
                        self.expect(&Token::RParen)?;
                        expr = Expr::MethodCall {
                            object: Box::new(expr),
                            method: field,
                            args,
                            span: start.merge(self.span()),
                        };
                    } else {
                        expr = Expr::FieldAccess {
                            object: Box::new(expr),
                            field,
                            span: start.merge(self.span()),
                        };
                    }
                }
                _ => break,
            }
        }
        Ok(expr)
    }

    fn parse_args(&mut self) -> Result<Vec<Arg>, ()> {
        let mut args = Vec::new();
        while !matches!(self.peek(), Token::RParen | Token::Eof) {
            // Check for named arg: name=value
            if let Token::Ident(name) = self.peek().clone() {
                let saved = self.pos;
                self.advance();
                if self.eat(&Token::Eq) {
                    let value = self.parse_expr()?;
                    args.push(Arg {
                        name: Some(name),
                        value,
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                    continue;
                }
                self.pos = saved; // backtrack
            }
            let value = self.parse_expr()?;
            args.push(Arg { name: None, value });
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        Ok(args)
    }

    fn parse_object_lit(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat '{'
        let mut fields = Vec::new();
        while !matches!(self.peek(), Token::RBrace | Token::Eof) {
            let key = self.parse_ident_name()?;
            self.expect(&Token::Colon)?;
            let value = self.parse_expr()?;
            fields.push((key, value));
            if !self.eat(&Token::Comma) {
                break;
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Expr::ObjectLit {
            fields,
            span: start.merge(self.span()),
        })
    }

    fn parse_match(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'match'
        let subject = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut arms = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }
            let arm_start = self.span();
            let pattern = self.parse_pattern()?;
            self.expect(&Token::Arrow)?;
            let body = self.parse_expr()?;
            arms.push(MatchArm {
                pattern,
                guard: None,
                body: Box::new(body),
                span: arm_start.merge(self.span()),
            });
            self.skip_newlines();
        }
        self.eat(&Token::RBrace);
        Ok(Expr::Match {
            subject: Box::new(subject),
            arms,
            span: start.merge(self.span()),
        })
    }

    fn parse_if(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'if'
        let condition = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        let then_body = self.parse_block()?;
        self.skip_newlines();
        let else_body = if self.eat(&Token::Else) {
            self.expect(&Token::LBrace)?;
            Some(self.parse_block()?)
        } else {
            None
        };
        Ok(Expr::If {
            condition: Box::new(condition),
            then_body,
            else_body,
            span: start.merge(self.span()),
        })
    }

    fn parse_for(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'for'
        let binding = self.parse_ident_name()?;
        self.expect(&Token::In)?;
        let iterable = self.parse_expr()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let body = self.parse_expr()?;
        self.skip_newlines();
        self.eat(&Token::RBrace);
        Ok(Expr::For {
            binding,
            iterable: Box::new(iterable),
            body: Box::new(body),
            span: start.merge(self.span()),
        })
    }

    fn parse_lambda(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat 'fn'
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat(&Token::To) {
            Some(self.parse_type_expr()?)
        } else {
            None
        };
        let body = self.parse_expr()?;
        Ok(Expr::Lambda {
            params,
            return_type,
            body: Box::new(body),
            span: start.merge(self.span()),
        })
    }

    fn parse_jsx(&mut self) -> Result<Expr, ()> {
        let start = self.span();
        self.advance(); // eat '<'
        let tag = self.parse_ident_name()?;
        let mut attrs = Vec::new();
        // Parse attributes until '>' or '/>'
        loop {
            match self.peek() {
                Token::Gt | Token::JsxSelfClose | Token::Eof => break,
                _ => {
                    let attr_name = self.parse_ident_name()?;
                    self.expect(&Token::Eq)?;
                    let value = if self.eat(&Token::LBrace) {
                        let e = self.parse_expr()?;
                        self.expect(&Token::RBrace)?;
                        e
                    } else if let Token::StringLit(s) = self.peek().clone() {
                        self.advance();
                        Expr::StringLit {
                            value: s,
                            span: self.span(),
                        }
                    } else {
                        self.parse_expr()?
                    };
                    attrs.push(JsxAttribute {
                        name: attr_name,
                        value,
                    });
                }
            }
        }
        if self.eat(&Token::JsxSelfClose) {
            return Ok(Expr::JsxSelfClosing(JsxSelfClosingElement {
                tag,
                attributes: attrs,
                span: start.merge(self.span()),
            }));
        }
        self.expect(&Token::Gt)?;
        // Children
        let mut children = Vec::new();
        self.skip_newlines();
        loop {
            self.skip_newlines();
            match self.peek() {
                Token::JsxCloseStart | Token::Eof => break,
                Token::Lt => {
                    children.push(self.parse_jsx()?);
                }
                Token::LBrace => {
                    self.advance();
                    let e = self.parse_expr()?;
                    self.expect(&Token::RBrace)?;
                    children.push(e);
                }
                Token::For => {
                    children.push(self.parse_for()?);
                }
                Token::StringLit(s) => {
                    let s = s.clone();
                    let sp = self.span();
                    self.advance();
                    children.push(Expr::StringLit { value: s, span: sp });
                }
                _ => {
                    children.push(self.parse_expr()?);
                }
            }
            self.skip_newlines();
        }
        // Close tag: </tag>
        self.skip_newlines();
        if self.eat(&Token::JsxCloseStart) {
            let _ = self.parse_ident_name(); // tag name
            self.expect(&Token::Gt)?;
        }
        Ok(Expr::Jsx(JsxElement {
            tag,
            attributes: attrs,
            children,
            span: start.merge(self.span()),
        }))
    }

    fn parse_typedef(&mut self, is_pub: bool) -> Result<Decl, ()> {
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

    fn parse_actor(&mut self) -> Result<Decl, ()> {
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
                let ret = if self.eat(&Token::To) {
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

    fn parse_workflow(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'workflow'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret = if self.eat(&Token::To) {
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

    fn parse_activity(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'activity'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        let params = self.parse_params()?;
        self.expect(&Token::RParen)?;
        let ret = if self.eat(&Token::To) {
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

    fn parse_http_route(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'http'
        let method = match self.peek() {
            Token::Get => {
                self.advance();
                HttpMethod::Get
            }
            Token::Post => {
                self.advance();
                HttpMethod::Post
            }
            Token::Put => {
                self.advance();
                HttpMethod::Put
            }
            Token::Delete => {
                self.advance();
                HttpMethod::Delete
            }
            _ => {
                self.errors.push(ParseError::new(
                    self.span(),
                    "Expected HTTP method",
                    vec!["get".into(), "post".into()],
                    Some(self.peek().to_string()),
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
                self.errors.push(ParseError::new(
                    self.span(),
                    "Expected route path string",
                    vec!["\"path\"".into()],
                    Some(self.peek().to_string()),
                ));
                return Err(());
            }
        };
        let ret = if self.eat(&Token::To) {
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

    /// Parse `@table type Name { field: Type }` — brace-delimited field block.
    fn parse_table(&mut self, is_pub: bool) -> Result<Decl, ()> {
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
    fn parse_index(&mut self) -> Result<Decl, ()> {
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
    fn parse_v0_component(&mut self) -> Result<Decl, ()> {
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
                        self.errors.push(ParseError::new(
                            self.span(),
                            "Expected image path string after 'from'",
                            vec!["\"path\"".into()],
                            Some(self.peek().to_string()),
                        ));
                        return Err(());
                    }
                }
            }
            _ => {
                self.errors.push(ParseError::new(
                    self.span(),
                    "Expected prompt string or 'from' after @v0",
                    vec!["\"prompt\"".into(), "from".into()],
                    Some(self.peek().to_string()),
                ));
                return Err(());
            }
        };
        self.expect(&Token::Fn)?;
        let name = self.parse_ident_name()?;
        self.expect(&Token::LParen)?;
        self.expect(&Token::RParen)?;
        let return_type = if self.eat(&Token::To) {
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
    fn parse_style_blocks(&mut self) -> Vec<StyleBlock> {
        let mut styles = Vec::new();
        self.skip_newlines();
        while let Token::Ident(ref name) = self.peek().clone() {
            if name != "style" {
                break;
            }
            let _start = self.span();
            self.advance(); // eat 'style'
            if !self.eat(&Token::LBrace) {
                break;
            }
            self.skip_newlines();
            loop {
                self.skip_newlines();
                match self.peek().clone() {
                    Token::Dot => {
                        let sel_start = self.span();
                        self.advance(); // eat '.'
                        let class_name = match self.parse_ident_name() {
                            Ok(n) => n,
                            Err(_) => break,
                        };
                        let selector = format!(".{}", class_name);
                        if !self.eat(&Token::LBrace) {
                            break;
                        }
                        self.skip_newlines();
                        let mut properties = Vec::new();
                        loop {
                            self.skip_newlines();
                            match self.peek().clone() {
                                Token::Ident(prop_name) => {
                                    self.advance();
                                    if !self.eat(&Token::Colon) {
                                        break;
                                    }
                                    match self.peek().clone() {
                                        Token::StringLit(val) => {
                                            self.advance();
                                            properties.push((prop_name, val));
                                        }
                                        _ => break,
                                    }
                                }
                                _ => break,
                            }
                        }
                        self.eat(&Token::RBrace); // close .selector {
                        styles.push(StyleBlock {
                            selector,
                            properties,
                            span: sel_start.merge(self.span()),
                        });
                    }
                    _ => break,
                }
            }
            self.eat(&Token::RBrace); // close style {
        }
        styles
    }

    /// Parse `routes { "path" to ComponentName }` declaration.
    fn parse_routes(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat 'routes'
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut entries = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::StringLit(path) => {
                    let entry_start = self.span();
                    self.advance();
                    self.expect(&Token::To)?;
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

fn infix_bp(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Pipe => (1, 2),
        BinOp::Or => (3, 4),
        BinOp::And => (5, 6),
        BinOp::Is | BinOp::Isnt => (7, 8),
        BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte => (9, 10),
        BinOp::Add | BinOp::Sub => (11, 12),
        BinOp::Mul | BinOp::Div => (13, 14),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vox_lexer::cursor::lex;

    fn parse_str(source: &str) -> Module {
        let tokens = lex(source);
        parse(tokens).unwrap_or_else(|e| panic!("Parse errors: {e:?}"))
    }

    #[test]
    fn test_parse_simple_fn() {
        let m = parse_str("fn add(a, b) to int { ret a + b }");
        assert_eq!(m.declarations.len(), 1);
        assert!(matches!(&m.declarations[0], Decl::Function(f) if f.name == "add"));
    }

    #[test]
    fn test_parse_import() {
        let m = parse_str("import react.use_state, network.HTTP");
        assert!(matches!(&m.declarations[0], Decl::Import(i) if i.paths.len() == 2));
    }

    #[test]
    fn test_parse_let() {
        let m = parse_str("fn main() { let x = 42\n ret x }");
        if let Decl::Function(f) = &m.declarations[0] {
            assert_eq!(f.body.len(), 2);
            assert!(matches!(&f.body[0], Stmt::Let { .. }));
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_component() {
        let m = parse_str("@component fn Chat() to Element { ret 0 }");
        assert!(matches!(&m.declarations[0], Decl::Component(_)));
    }

    #[test]
    fn test_parse_http_route() {
        let m = parse_str("http post \"/api/chat\" to Result { ret 0 }");
        assert!(matches!(&m.declarations[0], Decl::HttpRoute(r) if r.path == "/api/chat"));
    }

    #[test]
    fn test_parse_match() {
        let m = parse_str("fn f() { match x { Ok(r) -> r\n Error(e) -> e\n } }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Expr {
                expr: Expr::Match { arms, .. },
                ..
            } = &f.body[0]
            {
                assert_eq!(arms.len(), 2);
            } else {
                panic!("Expected match");
            }
        }
    }

    #[test]
    fn test_parse_type_def() {
        let m = parse_str("type Shape =\n    | Circle(r: float)\n    | Point");
        if let Decl::TypeDef(t) = &m.declarations[0] {
            assert_eq!(t.name, "Shape");
            assert_eq!(t.variants.len(), 2);
        } else {
            panic!("Expected type def");
        }
    }

    #[test]
    fn test_parse_operator_precedence() {
        let m = parse_str("fn f() { ret 1 + 2 * 3 }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Return {
                value:
                    Some(Expr::Binary {
                        op: BinOp::Add,
                        right,
                        ..
                    }),
                ..
            } = &f.body[0]
            {
                assert!(matches!(
                    right.as_ref(),
                    Expr::Binary { op: BinOp::Mul, .. }
                ));
            } else {
                panic!("Expected add(1, mul(2,3))");
            }
        }
    }

    #[test]
    fn test_parse_pipe() {
        let m = parse_str("fn f() { ret x |> transform |> render }");
        assert!(matches!(&m.declarations[0], Decl::Function(_)));
    }

    #[test]
    fn test_parse_actor() {
        let m = parse_str("actor Worker { on receive(msg) to str { ret msg } }");
        if let Decl::Actor(a) = &m.declarations[0] {
            assert_eq!(a.name, "Worker");
            assert_eq!(a.handlers.len(), 1);
            assert_eq!(a.handlers[0].event_name, "receive");
        } else {
            panic!("Expected actor");
        }
    }

    #[test]
    fn test_parse_workflow() {
        let m = parse_str("workflow process(file: str) to str { ret file }");
        if let Decl::Workflow(w) = &m.declarations[0] {
            assert_eq!(w.name, "process");
            assert_eq!(w.params.len(), 1);
        } else {
            panic!("Expected workflow");
        }
    }

    #[test]
    fn test_parse_lambda() {
        let m = parse_str("fn f() { let add = fn(a, b) a + b\n ret add(1, 2) }");
        if let Decl::Function(f) = &m.declarations[0] {
            assert_eq!(f.body.len(), 2);
            if let Stmt::Let {
                value: Expr::Lambda { params, .. },
                ..
            } = &f.body[0]
            {
                assert_eq!(params.len(), 2);
            } else {
                panic!("Expected lambda let");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_if_else() {
        let m = parse_str("fn f(x) { if x { ret 1\n} else { ret 0\n} }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Expr {
                expr:
                    Expr::If {
                        then_body,
                        else_body,
                        ..
                    },
                ..
            } = &f.body[0]
            {
                assert_eq!(then_body.len(), 1);
                assert!(else_body.is_some());
            } else {
                panic!("Expected if/else");
            }
        }
    }

    #[test]
    fn test_parse_mutable_let() {
        let m = parse_str("fn f() { let mut x = 0\n x = 1\n ret x }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Let { mutable, .. } = &f.body[0] {
                assert!(mutable, "Should be mutable");
            } else {
                panic!("Expected mutable let");
            }
        }
    }

    #[test]
    fn test_parse_method_chain() {
        let m = parse_str("fn f() { ret list.map(fn(x) x).filter(fn(x) x) }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Return {
                value: Some(Expr::MethodCall { method, .. }),
                ..
            } = &f.body[0]
            {
                assert_eq!(method, "filter");
            } else {
                panic!("Expected method chain");
            }
        }
    }

    #[test]
    fn test_parse_jsx_self_closing() {
        let m = parse_str("@component fn App() to Element { <input value=\"test\" /> }");
        if let Decl::Component(c) = &m.declarations[0] {
            if let Stmt::Expr {
                expr: Expr::JsxSelfClosing(_),
                ..
            } = &c.func.body[0]
            {
                // ok
            } else {
                panic!("Expected self-closing JSX");
            }
        }
    }

    #[test]
    fn test_parse_jsx_with_children() {
        let m = parse_str(
            "@component fn A() to Element { <div><span>hello</span></div> }",
        );
        if let Decl::Component(c) = &m.declarations[0] {
            if let Stmt::Expr {
                expr: Expr::Jsx(el),
                ..
            } = &c.func.body[0]
            {
                assert_eq!(el.tag, "div");
                assert_eq!(el.children.len(), 1);
            } else {
                panic!("Expected JSX element");
            }
        }
    }

    #[test]
    fn test_parse_spawn() {
        let m = parse_str("fn f() { ret spawn(Worker) }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Return {
                value: Some(Expr::Spawn { .. }),
                ..
            } = &f.body[0]
            {
                // ok
            } else {
                panic!("Expected spawn");
            }
        }
    }

    #[test]
    fn test_parse_for_loop() {
        let m = parse_str("fn f() { for x in items { x } }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Expr {
                expr: Expr::For { binding, .. },
                ..
            } = &f.body[0]
            {
                assert_eq!(binding, "x");
            } else {
                panic!("Expected for loop");
            }
        }
    }

    #[test]
    fn test_parse_pub_fn() {
        let m = parse_str("pub fn helper() to int { ret 42 }");
        if let Decl::Function(f) = &m.declarations[0] {
            assert!(f.is_pub);
            assert_eq!(f.name, "helper");
        } else {
            panic!("Expected pub fn");
        }
    }

    #[test]
    fn test_parse_multiple_decls() {
        let src = "import std\n\nfn a() { ret 1 }\n\nfn b() { ret 2 }";
        let m = parse_str(src);
        assert_eq!(m.declarations.len(), 3, "import + 2 functions");
    }

    #[test]
    fn test_parse_activity() {
        let m = parse_str("activity send_email(recipient: str) to str { ret recipient }");
        if let Decl::Activity(a) = &m.declarations[0] {
            assert_eq!(a.name, "send_email");
            assert_eq!(a.params.len(), 1);
            assert_eq!(a.params[0].name, "recipient");
            assert!(a.return_type.is_some());
        } else {
            panic!("Expected activity declaration, got {:?}", m.declarations[0]);
        }
    }

    #[test]
    fn test_parse_with_expression() {
        let m = parse_str("fn f() { ret call() with { timeout: 5 } }");
        if let Decl::Function(f) = &m.declarations[0] {
            if let Stmt::Return {
                value: Some(Expr::With {
                    operand, options, ..
                }),
                ..
            } = &f.body[0]
            {
                assert!(matches!(operand.as_ref(), Expr::Call { .. }));
                assert!(matches!(options.as_ref(), Expr::ObjectLit { .. }));
            } else {
                panic!("Expected With expression in return");
            }
        } else {
            panic!("Expected function");
        }
    }

    #[test]
    fn test_parse_table() {
        let m = parse_str("@table type Task { title: str\n done: bool\n priority: int }");
        if let Decl::Table(t) = &m.declarations[0] {
            assert_eq!(t.name, "Task");
            assert_eq!(t.fields.len(), 3);
            assert_eq!(t.fields[0].name, "title");
            assert_eq!(t.fields[1].name, "done");
            assert_eq!(t.fields[2].name, "priority");
        } else {
            panic!("Expected table declaration, got {:?}", m.declarations[0]);
        }
    }

    #[test]
    fn test_parse_index() {
        let m = parse_str("@index Task.by_done on (done, priority)");
        if let Decl::Index(idx) = &m.declarations[0] {
            assert_eq!(idx.table_name, "Task");
            assert_eq!(idx.index_name, "by_done");
            assert_eq!(idx.columns, vec!["done", "priority"]);
        } else {
            panic!("Expected index declaration, got {:?}", m.declarations[0]);
        }
    }
    #[test]
    fn test_parse_v0_prompt() {
        let m = parse_str("@v0 \"A dashboard with charts\" fn Dashboard() to Element");
        if let Decl::V0Component(v) = &m.declarations[0] {
            assert_eq!(v.name, "Dashboard");
            assert_eq!(v.prompt, "A dashboard with charts");
            assert!(v.image_path.is_none());
        } else {
            panic!("Expected V0Component, got {:?}", m.declarations[0]);
        }
    }

    #[test]
    fn test_parse_v0_from_image() {
        let m = parse_str("@v0 from \"design.png\" fn Dashboard() to Element");
        if let Decl::V0Component(v) = &m.declarations[0] {
            assert_eq!(v.name, "Dashboard");
            assert!(v.prompt.is_empty());
            assert_eq!(v.image_path.as_deref(), Some("design.png"));
        } else {
            panic!("Expected V0Component, got {:?}", m.declarations[0]);
        }
    }
}
