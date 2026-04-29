// ADT typedefs and actor / workflow / HTTP declarations.

use super::super::Parser;
use crate::ast::decl::*;
use crate::lexer::token::Token;

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
    pub(crate) fn parse_table(&mut self) -> Result<Decl, ()> {
        let start = self.span();
        self.advance(); // eat @table
        self.expect(&Token::TypeKw)?;
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        let mut fields = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Token::RBrace) {
                break;
            }
            if matches!(self.peek(), Token::Eof) {
                break;
            }
            let fstart = self.span();
            let fname = self.parse_ident_name()?;
            self.expect(&Token::Colon)?;
            let ftype = self.parse_type_expr()?;
            fields.push(crate::ast::decl::TableField {
                name: fname,
                type_ann: ftype,
                description: None,
                span: fstart.merge(self.span()),
            });
            self.eat(&Token::Comma);
        }
        Ok(Decl::Table(crate::ast::decl::TableDecl {
            name,
            fields,
            description: None,
            json_layout: None,
            auth_provider: None,
            roles: vec![],
            cors: None,
            is_pub: false,
            is_deprecated: false,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `url Name { Variant; Variant(arg: Type); Variant(?opt: Type) }`.
    pub(crate) fn parse_url_decl(&mut self, is_pub: bool) -> Result<Decl, ()> {
        use crate::parser::error::{ParseError, ParseErrorClass};
        let start = self.span();
        self.advance(); // eat `url`
        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;
        let mut variants = Vec::new();
        loop {
            self.skip_newlines();
            if self.eat(&Token::RBrace) {
                break;
            }
            if matches!(self.peek(), Token::Eof) {
                self.errors.push(ParseError::classified(
                    self.span(),
                    "Unexpected EOF inside `url` block",
                    vec!["}".into()],
                    None,
                    ParseErrorClass::Declaration,
                ));
                return Err(());
            }
            let vstart = self.span();
            let vname = self.parse_ident_name()?;
            let mut args = Vec::new();
            if self.eat(&Token::LParen) {
                loop {
                    self.skip_newlines();
                    if matches!(self.peek(), Token::RParen) {
                        break;
                    }
                    let astart = self.span();
                    let optional = self.eat(&Token::Question);
                    let aname = self.parse_ident_name()?;
                    self.expect(&Token::Colon)?;
                    let atype = self.parse_type_expr()?;
                    args.push(crate::ast::decl::UrlArg {
                        name: aname,
                        optional,
                        type_ann: atype,
                        span: astart.merge(self.span()),
                    });
                    if !self.eat(&Token::Comma) {
                        break;
                    }
                }
                self.expect(&Token::RParen)?;
            }
            variants.push(crate::ast::decl::UrlVariant {
                name: vname,
                args,
                span: vstart.merge(self.span()),
            });
            // Allow an optional comma between variants; newlines are skipped at loop top
            self.eat(&Token::Comma);
        }
        Ok(Decl::Url(crate::ast::decl::UrlDecl {
            name,
            variants,
            is_pub,
            span: start.merge(self.span()),
        }))
    }

    /// Parse `[partial] state_machine Name { state … on … }`.
    ///
    /// Called with the cursor on the `state_machine` ident token (or on `partial`
    /// if `is_partial` was set by the caller after consuming `partial`).
    pub(crate) fn parse_state_machine_decl(
        &mut self,
        is_pub: bool,
        is_partial: bool,
    ) -> Result<Decl, ()> {
        use crate::ast::decl::{SmState, SmTransition, StateMachineDecl};
        use crate::parser::error::{ParseError, ParseErrorClass};

        let start = self.span();
        self.advance(); // eat `state_machine` ident

        let name = self.parse_ident_name()?;
        self.expect(&Token::LBrace)?;

        let mut states: Vec<SmState> = Vec::new();
        let mut transitions: Vec<SmTransition> = Vec::new();

        loop {
            self.skip_newlines();
            if matches!(self.peek(), Token::RBrace | Token::Eof) {
                break;
            }

            match self.peek().clone() {
                // `state Name` or `state Name(field: Type, …)`
                Token::State => {
                    let ss = self.parse_sm_state(false)?;
                    states.push(ss);
                }
                // `terminal state Name(…)`
                Token::Ident(ref kw) if kw == "terminal" => {
                    self.advance(); // eat `terminal`
                    if !matches!(self.peek(), Token::State) {
                        self.errors.push(ParseError::classified(
                            self.span(),
                            "Expected `state` after `terminal`",
                            vec!["state".into()],
                            Some(self.peek().to_string()),
                            ParseErrorClass::Declaration,
                        ));
                        return Err(());
                    }
                    let ss = self.parse_sm_state(true)?;
                    states.push(ss);
                }
                // `on Event(params) from State -> Target`
                Token::On => {
                    let tr = self.parse_sm_transition()?;
                    transitions.push(tr);
                }
                other => {
                    self.errors.push(ParseError::classified(
                        self.span(),
                        format!(
                            "Expected `state`, `terminal state`, or `on` inside state_machine block; got {other}"
                        ),
                        vec!["state".into(), "terminal".into(), "on".into()],
                        Some(other.to_string()),
                        ParseErrorClass::Declaration,
                    ));
                    return Err(());
                }
            }

            // Allow optional commas or newlines as separators.
            self.eat(&Token::Comma);
        }

        self.expect(&Token::RBrace)?;

        Ok(Decl::StateMachine(StateMachineDecl {
            name,
            states,
            transitions,
            is_partial,
            is_pub,
            span: start.merge(self.span()),
        }))
    }

    fn parse_sm_state(&mut self, is_terminal: bool) -> Result<crate::ast::decl::SmState, ()> {
        use crate::ast::decl::{SmField, SmState};

        let start = self.span();
        self.advance(); // eat `state`
        let name = self.parse_ident_name()?;

        let fields = if self.eat(&Token::LParen) {
            let mut fs = Vec::new();
            loop {
                self.skip_newlines();
                if matches!(self.peek(), Token::RParen | Token::Eof) {
                    break;
                }
                let fstart = self.span();
                let fname = self.parse_ident_name()?;
                let type_ann = if self.eat(&Token::Colon) {
                    Some(self.parse_type_expr()?)
                } else {
                    None
                };
                fs.push(SmField {
                    name: fname,
                    type_ann,
                    span: fstart.merge(self.span()),
                });
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
            fs
        } else {
            Vec::new()
        };

        Ok(SmState {
            name,
            fields,
            is_terminal,
            span: start.merge(self.span()),
        })
    }

    fn parse_sm_transition(&mut self) -> Result<crate::ast::decl::SmTransition, ()> {
        use crate::ast::decl::{SmFromPattern, SmTransition};
        use crate::parser::error::{ParseError, ParseErrorClass};

        let start = self.span();
        self.advance(); // eat `on`

        let event_name = self.parse_ident_name()?;

        // Optional event params: `on Assign(t, r)` or `on Resume` (no parens).
        let event_params = if self.eat(&Token::LParen) {
            let mut params = Vec::new();
            loop {
                self.skip_newlines();
                if matches!(self.peek(), Token::RParen | Token::Eof) {
                    break;
                }
                // Accept `_` as wildcard or any ident name.
                let pname = self.parse_ident_name()?;
                params.push(pname);
                if !self.eat(&Token::Comma) {
                    break;
                }
            }
            self.expect(&Token::RParen)?;
            params
        } else {
            Vec::new()
        };

        // `from State` or `from any`
        let from_kw_ok = matches!(self.peek(), Token::Ident(n) if n == "from");
        if !from_kw_ok {
            self.errors.push(ParseError::classified(
                self.span(),
                "Expected `from` after event in transition",
                vec!["from".into()],
                Some(self.peek().to_string()),
                ParseErrorClass::Declaration,
            ));
            return Err(());
        }
        self.advance(); // eat `from`

        let from = match self.peek().clone() {
            Token::Ident(ref n) if n == "any" => {
                self.advance();
                SmFromPattern::Any
            }
            // `from Working(_)` — consume the state name and skip any parens/wildcards.
            _ => {
                let state_name = self.parse_ident_name()?;
                // Consume optional `(_)` wildcard pattern (e.g. `from Working(_)`).
                if self.eat(&Token::LParen) {
                    let mut depth = 1usize;
                    loop {
                        match self.peek() {
                            Token::LParen => {
                                depth += 1;
                                self.advance();
                            }
                            Token::RParen => {
                                depth -= 1;
                                self.advance();
                                if depth == 0 {
                                    break;
                                }
                            }
                            Token::Eof => break,
                            _ => {
                                self.advance();
                            }
                        }
                    }
                }
                SmFromPattern::Named(state_name)
            }
        };

        // `->`
        self.expect(&Token::Arrow)?;

        let to_state = self.parse_ident_name()?;

        // Consume optional target args `(t)` without deep parsing.
        if self.eat(&Token::LParen) {
            let mut depth = 1usize;
            loop {
                match self.peek() {
                    Token::LParen => {
                        depth += 1;
                        self.advance();
                    }
                    Token::RParen => {
                        depth -= 1;
                        self.advance();
                        if depth == 0 {
                            break;
                        }
                    }
                    Token::Eof => break,
                    _ => {
                        self.advance();
                    }
                }
            }
        }

        Ok(SmTransition {
            event_name,
            event_params,
            from,
            to_state,
            span: start.merge(self.span()),
        })
    }
}
