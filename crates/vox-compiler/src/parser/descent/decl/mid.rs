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
}
