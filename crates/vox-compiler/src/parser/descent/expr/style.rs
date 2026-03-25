// style { } blocks on components.

use super::super::Parser;
use crate::ast::decl::StyleBlock;
use crate::lexer::token::Token;

impl Parser {
    pub(crate) fn parse_style_blocks(&mut self) -> Vec<StyleBlock> {
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
}
