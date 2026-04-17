// Infix / binding-power expression parsing.

use super::super::Parser;
use crate::ast::expr::{BinOp, Expr};
use crate::lexer::token::Token;

impl Parser {
    pub(crate) fn parse_expr(&mut self) -> Result<Expr, ()> {
        self.skip_newlines();
        self.parse_expr_bp(0)
    }

    pub(crate) fn parse_expr_bp(&mut self, min_bp: u8) -> Result<Expr, ()> {
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

            if matches!(self.peek(), Token::Question) {
                let l_bp = 100; // tightly bind
                if l_bp < min_bp {
                    break;
                }
                let span = lhs.span().merge(self.span());
                self.advance();
                lhs = Expr::Try {
                    target: Box::new(lhs),
                    span,
                };
                continue;
            }

            let op = match self.peek() {
                Token::Plus => BinOp::Add,
                Token::Minus => BinOp::Sub,
                Token::Star => BinOp::Mul,
                Token::Slash => BinOp::Div,
                Token::Percent => BinOp::Mod,
                Token::Lt => BinOp::Lt,
                Token::Gt => BinOp::Gt,
                Token::Lte => BinOp::Lte,
                Token::Gte => BinOp::Gte,
                Token::And => BinOp::And,
                Token::Or => BinOp::Or,
                Token::Is => BinOp::Is,
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
}

fn infix_bp(op: BinOp) -> (u8, u8) {
    match op {
        BinOp::Pipe => (1, 2),
        BinOp::Or => (3, 4),
        BinOp::And => (5, 6),
        BinOp::Is | BinOp::Isnt => (7, 8),
        BinOp::Lt | BinOp::Gt | BinOp::Lte | BinOp::Gte => (9, 10),
        BinOp::Add | BinOp::Sub => (11, 12),
        BinOp::Mul | BinOp::Div | BinOp::Mod => (13, 14),
    }
}
