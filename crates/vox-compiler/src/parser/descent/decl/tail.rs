// Table, index, v0, reactive member, and routes parsing.

use super::super::Parser;
use crate::ast::decl::*;
use crate::ast::expr::Expr;
use crate::lexer::token::Token;
use crate::parser::error::{ParseError, ParseErrorClass};

impl Parser {
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

    /// Parse `@v0 "chat-id" Name { … }` or `@v0 from "design.png" Name { … }` (v0 island stub body).
    #[allow(dead_code)]
    pub(crate) fn parse_v0_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @v0

        let (v0_id, image_path) = match self.peek().clone() {
            Token::Ident(ref w) if w == "from" => {
                self.advance();
                let path = match self.peek().clone() {
                    Token::StringLit(s) => {
                        self.advance();
                        s
                    }
                    _ => {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected image path string after `@v0 from`",
                            vec!["\"design.png\"".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                };
                (String::new(), Some(path))
            }
            Token::StringLit(s) => {
                self.advance();
                (s, None)
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected v0 chat id string or `from \"path\"` after @v0",
                    vec!["\"chat-id\"".into(), "from \"file.png\"".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
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
        Ok(Decl::V0Component(V0ComponentDecl {
            v0_id,
            image_path,
            name,
            props,
            span: start.merge(self.span()),
        }))
    }

    /// Parse optional `style { .selector { property: "value" } }` blocks.
    pub(crate) fn parse_reactive_component(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // component
        let name = self.parse_ident_name()?;
        let mut inner = self.finish_reactive_component_after_name(start, name)?;
        inner.styles = self.parse_style_blocks();
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
    /// Optional `with loader: name` / `with pending: Name` / `with (loader: a, pending: b)` on a route line.
    fn parse_optional_route_with_clause(&mut self) -> Result<(Option<String>, Option<String>), ()> {
        if !self.eat(&Token::With) {
            return Ok((None, None));
        }
        let mut loader_name = None;
        let mut pending_component_name = None;
        if self.eat(&Token::LParen) {
            if !matches!(self.peek(), Token::RParen) {
                loop {
                    let key = self.parse_ident_name()?;
                    self.expect(&Token::Colon)?;
                    let _val = self.parse_ident_name()?;
                    match key.as_str() {
                        "loader" => loader_name = Some(_val),
                        "pending" => pending_component_name = Some(_val),
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "In `routes { ... }`, `with (...)` only supports `loader:` and `pending:` keys",
                                vec!["loader".into(), "pending".into()],
                                Some(key),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }
                    if self.eat(&Token::Comma) {
                        if matches!(self.peek(), Token::RParen) {
                            break;
                        }
                        continue;
                    }
                    break;
                }
            }
            self.expect(&Token::RParen)?;
        } else {
            let key = self.parse_ident_name()?;
            self.expect(&Token::Colon)?;
            let val = self.parse_ident_name()?;
            match key.as_str() {
                "loader" => loader_name = Some(val),
                "pending" => pending_component_name = Some(val),
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "In `routes { ... }`, use `with loader: fnName` or `with pending: Spinner` (or `with (loader: a, pending: b)`)",
                        vec!["loader".into(), "pending".into()],
                        Some(key),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            }
        }
        Ok((loader_name, pending_component_name))
    }

    /// Parse child entries until `}` (consumes the closing brace).
    fn parse_nested_route_entries(&mut self) -> Result<Vec<RouteEntry>, ()> {
        let mut children = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace) {
                self.advance();
                return Ok(children);
            }
            match self.peek().clone() {
                Token::StringLit(_) => children.push(self.parse_route_entry_from_path_literal()?),
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "In nested route `{ ... }` blocks, each entry must start with a string path (\"...\")",
                        vec!["\"/child\"".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            }
        }
    }

    /// `"path" to Component [with ...] [ { child routes } ]`
    pub(crate) fn parse_route_entry_from_path_literal(&mut self) -> Result<RouteEntry, ()> {
        let entry_start = self.span();
        let path = match self.peek().clone() {
            Token::StringLit(p) => {
                self.advance();
                p
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected string literal route path",
                    vec!["\"/\"".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };
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
        let (loader_name, pending_component_name) = self.parse_optional_route_with_clause()?;
        let children = if matches!(self.peek(), Token::LBrace) {
            self.advance();
            self.parse_nested_route_entries()?
        } else {
            vec![]
        };
        Ok(RouteEntry {
            path,
            component_name,
            children,
            redirect: None,
            is_wildcard: false,
            loader_name,
            pending_component_name,
            span: entry_start.merge(self.span()),
        })
    }

    /// Parse `url TypeName { Variant, Variant(args), ... }` (TASK-4.3).
    /// NOTE: This function is superseded by `parse_url_decl` in `mid.rs`. Kept for reference.
    #[allow(dead_code)]
    pub(crate) fn parse_url_block(&mut self) -> Result<Decl, ()> {
        use crate::ast::decl::{UrlArg, UrlDecl, UrlVariant};
        let start = self.span();
        self.advance(); // eat 'url'
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        self.skip_newlines();
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::Ident(ref vname) | Token::TypeIdent(ref vname)
                    if !vname.is_empty() =>
                {
                    let var_start = self.span();
                    let vname = vname.clone();
                    self.advance();
                    let args = if self.peek() == &Token::LParen {
                        self.advance(); // eat '('
                        let mut args = Vec::new();
                        loop {
                            self.skip_newlines();
                            if self.peek() == &Token::RParen {
                                break;
                            }
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                                continue;
                            }
                            let arg_start = self.span();
                            let optional = if matches!(self.peek(), Token::Question) {
                                self.advance();
                                true
                            } else {
                                false
                            };
                            let arg_name = self.parse_ident_name()?;
                            self.expect(&Token::Colon)?;
                            let type_ann = self.parse_type_expr()?;
                            args.push(UrlArg {
                                name: arg_name,
                                optional,
                                type_ann,
                                span: arg_start.merge(self.span()),
                            });
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RParen)?;
                        args
                    } else {
                        vec![]
                    };
                    variants.push(UrlVariant {
                        name: vname,
                        args,
                        span: var_start.merge(self.span()),
                    });
                    if matches!(self.peek(), Token::Comma) {
                        self.advance();
                    }
                }
                Token::RBrace => break,
                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Unexpected token in `url` body; expected a variant name or `}`",
                        vec!["}".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    self.expect(&Token::RBrace)?;
                    return Err(());
                }
            }
        }
        self.expect(&Token::RBrace)?;
        Ok(Decl::Url(UrlDecl {
            name,
            variants,
            is_pub: false,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `state_machine Name { state ..., on ... }` declaration (TASK-4.1).
    pub(crate) fn parse_state_machine(&mut self) -> Result<Decl, ()> {
        use crate::ast::decl::state_machine::{
            SmFromPattern, SmState, SmTransition, StateMachineDecl,
        };
        let start = self.span();
        self.advance(); // eat 'state_machine'

        // Machine name (PascalCase → TypeIdent, but accept Ident too)
        let name = match self.peek().clone() {
            Token::TypeIdent(n) | Token::Ident(n) => {
                self.advance();
                n
            }
            _ => {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Expected state machine name after `state_machine`",
                    vec!["AgentLifecycle".into()],
                    Some(self.peek().to_string()),
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
        };

        self.expect(&Token::LBrace)?;

        let mut states: Vec<SmState> = Vec::new();
        let mut transitions: Vec<SmTransition> = Vec::new();

        loop {
            self.skip_newlines();
            match self.peek().clone() {
                Token::RBrace | Token::Eof => break,

                // `terminal state Name ...`
                Token::Ident(ref kw) if kw == "terminal" => {
                    let item_start = self.span();
                    self.advance(); // eat 'terminal'
                    if !matches!(self.peek(), Token::State) {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected `state` keyword after `terminal`",
                            vec!["state".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                    self.advance(); // eat 'state'
                    let sname = match self.peek().clone() {
                        Token::TypeIdent(n) | Token::Ident(n) => {
                            self.advance();
                            n
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected state name",
                                vec!["Idle".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };
                    let fields = self.parse_sm_state_fields()?;
                    states.push(SmState {
                        name: sname,
                        fields,
                        is_terminal: true,
                        span: item_start.merge(self.span()),
                    });
                }

                // `state Name ...`
                Token::State => {
                    let item_start = self.span();
                    self.advance(); // eat 'state'
                    let sname = match self.peek().clone() {
                        Token::TypeIdent(n) | Token::Ident(n) => {
                            self.advance();
                            n
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected state name",
                                vec!["Idle".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };
                    let fields = self.parse_sm_state_fields()?;
                    states.push(SmState {
                        name: sname,
                        fields,
                        is_terminal: false,
                        span: item_start.merge(self.span()),
                    });
                }

                // `on EventName(...) from ... -> ...`
                Token::On => {
                    let item_start = self.span();
                    self.advance(); // eat 'on'

                    // Event name (PascalCase)
                    let event_name = match self.peek().clone() {
                        Token::TypeIdent(n) | Token::Ident(n) => {
                            self.advance();
                            n
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected event name after `on`",
                                vec!["Assign".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };

                    // Optional event params: `(name, ...)` — just names, stored as Vec<String>
                    let event_params = if matches!(self.peek(), Token::LParen) {
                        self.advance(); // eat '('
                        let mut params: Vec<String> = Vec::new();
                        loop {
                            self.skip_newlines();
                            if matches!(self.peek(), Token::RParen | Token::Eof) {
                                break;
                            }
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                                continue;
                            }
                            let p_name = self.parse_ident_name()?;
                            // Consume optional `: Type` annotation (ignored at parse time)
                            if self.eat(&Token::Colon) {
                                let _ = self.parse_type_expr()?;
                            }
                            params.push(p_name);
                            if matches!(self.peek(), Token::Comma) {
                                self.advance();
                            }
                        }
                        self.expect(&Token::RParen)?;
                        params
                    } else {
                        vec![]
                    };

                    // `from` keyword
                    match self.peek().clone() {
                        Token::Ident(ref kw) if kw == "from" => {
                            self.advance();
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected `from` keyword in transition",
                                vec!["from".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    }

                    // Source pattern: `any` or `StateName` or `StateName(_)`
                    let from = match self.peek().clone() {
                        Token::Ident(ref kw) if kw == "any" => {
                            self.advance();
                            SmFromPattern::Any
                        }
                        Token::TypeIdent(n) | Token::Ident(n) => {
                            self.advance();
                            // Optionally consume `(_)` pattern
                            if matches!(self.peek(), Token::LParen) {
                                self.advance(); // eat '('
                                while !matches!(self.peek(), Token::RParen | Token::Eof) {
                                    self.advance();
                                }
                                self.eat(&Token::RParen);
                            }
                            SmFromPattern::Named(n)
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected source state name or `any` in `from` clause",
                                vec!["Idle".into(), "any".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };

                    // `->`
                    if !matches!(self.peek(), Token::Arrow) {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected `->` in transition",
                            vec!["->".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                    self.advance(); // eat '->'

                    // Target state name
                    let to_state = match self.peek().clone() {
                        Token::TypeIdent(n) | Token::Ident(n) => {
                            self.advance();
                            n
                        }
                        _ => {
                            self.errors.push(ParseError::classified(
                                self.span(),
                                "Expected target state name after `->`",
                                vec!["Working".into()],
                                Some(self.peek().to_string()),
                                ParseErrorClass::Declaration,
                            ));
                            return Err(());
                        }
                    };

                    // Optionally consume constructor args on target `(arg, ...)`
                    if matches!(self.peek(), Token::LParen) {
                        self.advance(); // eat '('
                        while !matches!(self.peek(), Token::RParen | Token::Eof) {
                            self.advance();
                        }
                        self.eat(&Token::RParen);
                    }

                    transitions.push(SmTransition {
                        event_name,
                        event_params,
                        from,
                        to_state,
                        span: item_start.merge(self.span()),
                    });
                }

                _ => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        "Unexpected token in `state_machine` body; expected `state`, `on`, or `}`",
                        vec!["state".into(), "on".into(), "}".into()],
                        Some(self.peek().to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    self.expect(&Token::RBrace)?;
                    return Err(());
                }
            }
        }

        self.expect(&Token::RBrace)?;

        Ok(Decl::StateMachine(StateMachineDecl {
            name,
            states,
            transitions,
            is_partial: false,
            is_pub: false,
            span: start.merge(self.span()),
        }))
    }

    /// Helper: parse optional `(name: Type, ...)` field list for a state declaration.
    fn parse_sm_state_fields(&mut self) -> Result<Vec<crate::ast::decl::state_machine::SmField>, ()> {
        use crate::ast::decl::state_machine::SmField;
        if !matches!(self.peek(), Token::LParen) {
            return Ok(vec![]);
        }
        self.advance(); // eat '('
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RParen | Token::Eof) {
                break;
            }
            if matches!(self.peek(), Token::Comma) {
                self.advance();
                continue;
            }
            let f_start = self.span();
            let f_name = self.parse_ident_name()?;
            let f_ty = if self.eat(&Token::Colon) {
                Some(self.parse_type_expr()?)
            } else {
                None
            };
            fields.push(SmField {
                name: f_name,
                type_ann: f_ty,
                span: f_start.merge(self.span()),
            });
            if matches!(self.peek(), Token::Comma) {
                self.advance();
            }
        }
        self.expect(&Token::RParen)?;
        Ok(fields)
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
        let mut not_found_component: Option<String> = None;
        let mut error_component: Option<String> = None;
        loop {
            self.maybe_parser_trace("routes.entry");
            self.skip_newlines();
            match self.peek().clone() {
                Token::StringLit(_) => {
                    entries.push(self.parse_route_entry_from_path_literal()?);
                }
                Token::Ident(ref key) if key == "not_found" => {
                    let bind = self.span();
                    self.advance();
                    self.expect(&Token::Colon)?;
                    let comp = self.parse_ident_name()?;
                    not_found_component = Some(comp);
                    let _ = bind.merge(self.span());
                }
                Token::Ident(ref key) if key == "error" => {
                    let bind = self.span();
                    self.advance();
                    self.expect(&Token::Colon)?;
                    let comp = self.parse_ident_name()?;
                    error_component = Some(comp);
                    let _ = bind.merge(self.span());
                }
                _ => break,
            }
        }
        self.eat(&Token::RBrace);
        Ok(Decl::Routes(RoutesDecl {
            entries,
            not_found_component,
            error_component,
            span: start.merge(self.span()),
        }))
    }
}
