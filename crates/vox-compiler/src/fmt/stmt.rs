//! Statement printing (OP-0205).

use crate::ast::stmt::Stmt;

use super::printer::Printer;

impl Printer {
    pub(crate) fn print_stmt(&mut self, s: &Stmt) {
        match s {
            Stmt::Let {
                pattern,
                type_ann,
                value,
                mutable,
                ..
            } => {
                self.write_indent();
                self.out
                    .push_str(if *mutable { "let mut " } else { "let " });
                self.print_pattern(pattern);
                if let Some(ty) = type_ann.as_ref() {
                    self.out.push_str(": ");
                    self.print_type(ty);
                }
                self.out.push_str(" = ");
                self.print_expr(value);
                self.out.push('\n');
            }
            Stmt::Assign { target, value, .. } => {
                self.write_indent();
                self.print_expr(target);
                self.out.push_str(" = ");
                self.print_expr(value);
                self.out.push('\n');
            }
            Stmt::Return { value, .. } => {
                self.write_indent();
                self.out.push_str("ret");
                if let Some(e) = value.as_ref() {
                    self.out.push(' ');
                    self.print_expr(e);
                }
                self.out.push('\n');
            }
            Stmt::Expr { expr, .. } => {
                self.write_indent();
                self.print_expr(expr);
                self.out.push('\n');
            }
            Stmt::While {
                condition, body, ..
            } => {
                self.write_indent();
                self.out.push_str("while ");
                self.print_expr(condition);
                self.out.push_str(" {\n");
                self.indent();
                for s in body {
                    self.print_stmt(s);
                }
                self.dedent();
                self.write_indent();
                self.out.push_str("}\n");
            }
            Stmt::Loop { body, .. } => {
                self.write_indent();
                self.out.push_str("loop {\n");
                self.indent();
                for s in body {
                    self.print_stmt(s);
                }
                self.dedent();
                self.write_indent();
                self.out.push_str("}\n");
            }
            Stmt::Break { .. } => {
                self.write_indent();
                self.out.push_str("break\n");
            }
            Stmt::Continue { .. } => {
                self.write_indent();
                self.out.push_str("continue\n");
            }
        }
    }
}
