use super::LowerCtx;
use crate::ast::decl::fundecl::{FnDecl, VerifyMode};
use crate::hir::HirStmt;

impl LowerCtx {
    /// Lowers contracts into the function body based on VerifyMode.
    pub(crate) fn inject_contracts(&mut self, f: &FnDecl, mut body: Vec<HirStmt>) -> Vec<HirStmt> {
        match f.verify_mode {
            VerifyMode::Off => body, // Strip all contracts
            VerifyMode::RequireOnly => {
                let mut new_body = Vec::new();
                for pre in &f.preconditions {
                    let expr = self.lower_expr(pre);
                    new_body.push(HirStmt::Expr {
                        expr,
                        span: crate::ast::span::Span::new(0, 0),
                    });
                }
                new_body.append(&mut body);
                new_body
            }
            VerifyMode::Full => {
                let mut new_body = Vec::new();
                for pre in &f.preconditions {
                    let expr = self.lower_expr(pre);
                    new_body.push(HirStmt::Expr {
                        expr,
                        span: crate::ast::span::Span::new(0, 0),
                    });
                }
                new_body.append(&mut body);
                for post in &f.postconditions {
                    let expr = self.lower_expr(post);
                    new_body.push(HirStmt::Expr {
                        expr,
                        span: crate::ast::span::Span::new(0, 0),
                    });
                }
                new_body
            }
        }
    }
}
