//! Expression, type, and pattern printing (OP-0205).

use crate::ast::expr::{BinOp, Expr};
use crate::ast::pattern::Pattern;
use crate::ast::types::TypeExpr;

use super::printer::Printer;

impl Printer {
    pub(crate) fn print_expr(&mut self, e: &Expr) {
        match e {
            Expr::IntLit { value, .. } => self.out.push_str(&value.to_string()),
            Expr::FloatLit { value, .. } => self.out.push_str(&value.to_string()),
            Expr::StringLit { value, .. } => self.out.push_str(&format!("\"{}\"", value)),
            Expr::BoolLit { value, .. } => self.out.push_str(if *value { "true" } else { "false" }),
            Expr::Ident { name, .. } => self.out.push_str(name),
            Expr::Call { callee, args, .. } => {
                self.print_expr(callee);
                self.out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    if let Some(ref n) = arg.name {
                        self.out.push_str(n);
                        self.out.push_str(": ");
                    }
                    self.print_expr(&arg.value);
                }
                self.out.push(')');
            }
            Expr::Binary {
                op, left, right, ..
            } => {
                self.print_expr(left);
                self.out.push(' ');
                self.print_binop(*op);
                self.out.push(' ');
                self.print_expr(right);
            }
            Expr::If {
                condition,
                then_body,
                else_body,
                ..
            } => {
                self.out.push_str("if ");
                self.print_expr(condition);
                self.out.push_str(
                    " {
",
                );
                self.indent();
                for s in then_body {
                    self.print_stmt(s);
                }
                self.dedent();
                self.write_indent();
                self.out.push('}');
                if let Some(else_stmts) = else_body {
                    self.out.push_str(
                        " else {
",
                    );
                    self.indent();
                    for s in else_stmts {
                        self.print_stmt(s);
                    }
                    self.dedent();
                    self.write_indent();
                    self.out.push('}');
                }
            }
            Expr::Match { subject, arms, .. } => {
                self.out.push_str("match ");
                self.print_expr(subject);
                self.out.push_str(
                    " {
",
                );
                self.indent();
                for arm in arms {
                    self.write_indent();
                    self.print_pattern(&arm.pattern);
                    self.out.push_str(" -> ");
                    self.print_expr(&arm.body);
                    self.out.push('\n');
                }
                self.dedent();
                self.write_indent();
                self.out.push('}');
            }
            _ => self.out.push_str("..."),
        }
    }

    fn print_binop(&mut self, op: BinOp) {
        self.out.push_str(match op {
            BinOp::Add => "+",
            BinOp::Sub => "-",
            BinOp::Mul => "*",
            BinOp::Div => "/",
            BinOp::Is => "is",
            BinOp::Isnt => "isnt",
            _ => "??",
        });
    }

    pub(crate) fn print_type(&mut self, t: &TypeExpr) {
        match t {
            TypeExpr::Named { name, .. } => self.out.push_str(name),
            TypeExpr::Generic { name, args, .. } => {
                self.out.push_str(name);
                self.out.push('[');
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        self.out.push_str(", ");
                    }
                    self.print_type(a);
                }
                self.out.push(']');
            }
            _ => self.out.push_str("Any"),
        }
    }

    pub(crate) fn print_pattern(&mut self, p: &Pattern) {
        match p {
            Pattern::Ident { name, .. } => self.out.push_str(name),
            Pattern::Wildcard { .. } => self.out.push('_'),
            _ => self.out.push_str("pat"),
        }
    }
}
