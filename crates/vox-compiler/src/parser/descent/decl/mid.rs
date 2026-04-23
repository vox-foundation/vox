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
}
