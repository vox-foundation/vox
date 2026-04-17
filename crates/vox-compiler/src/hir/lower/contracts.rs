use super::LowerCtx;
use crate::ast::decl::fundecl::{FnDecl, VerifyMode};
use crate::hir::{HirExpr, HirStmt};

impl LowerCtx {
    /// Lowers contracts into the function body based on VerifyMode.
    pub(crate) fn inject_contracts(&mut self, f: &FnDecl, body: Vec<HirStmt>) -> Vec<HirStmt> {
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
                new_body.extend(body);
                new_body
            }
            VerifyMode::Full => {
                let pre_stmts: Vec<HirStmt> = f
                    .preconditions
                    .iter()
                    .map(|p| {
                        let expr = self.lower_expr(p);
                        HirStmt::Expr {
                            expr,
                            span: crate::ast::span::Span::new(0, 0),
                        }
                    })
                    .collect();

                let post_stmts: Vec<HirStmt> = f
                    .postconditions
                    .iter()
                    .map(|p| {
                        let expr = self.lower_expr(p);
                        HirStmt::Expr {
                            expr,
                            span: crate::ast::span::Span::new(0, 0),
                        }
                    })
                    .collect();

                let mut new_body = pre_stmts;
                let mut body_to_process = body;

                // Inject postconditions before every explicit return statement
                self.inject_postconditions_before_returns(&mut body_to_process, &post_stmts);

                // Check if the body ends in a return statement to avoid double-injecting at the end
                let ends_in_return =
                    matches!(body_to_process.last(), Some(HirStmt::Return { .. }));

                new_body.extend(body_to_process);

                if !ends_in_return {
                    new_body.extend(post_stmts);
                }

                new_body
            }
        }
    }

    fn inject_postconditions_before_returns(
        &self,
        body: &mut Vec<HirStmt>,
        post_stmts: &[HirStmt],
    ) {
        let mut i = 0;
        while i < body.len() {
            match &mut body[i] {
                HirStmt::Return { value, span } => {
                    let s = *span;
                    if let Some(v) = value.take() {
                        // Replace Return { value: Some(v) } with:
                        // let __result__ = v
                        // <postconditions>
                        // return __result__
                        body[i] = HirStmt::Let {
                            pattern: crate::hir::HirPattern::Ident("__result__".into(), s),
                            type_ann: None,
                            value: v,
                            mutable: false,
                            span: s,
                        };
                        i += 1;
                        for post in post_stmts {
                            body.insert(i, post.clone());
                            i += 1;
                        }
                        body.insert(
                            i,
                            HirStmt::Return {
                                value: Some(crate::hir::HirExpr::Ident("__result__".into(), s)),
                                span: s,
                            },
                        );
                        i += 1;
                    } else {
                        // Replace Return { value: None } with:
                        // <postconditions>
                        // return
                        for post in post_stmts {
                            body.insert(i, post.clone());
                            i += 1;
                        }
                        i += 1; // Skip the original return (now at i)
                    }
                }
                HirStmt::While { body: b, .. } | HirStmt::Loop { body: b, .. } => {
                    self.inject_postconditions_before_returns(b, post_stmts);
                    i += 1;
                }
                HirStmt::Expr { expr, .. } => {
                    self.inject_postconditions_into_expr(expr, post_stmts);
                    i += 1;
                }
                _ => i += 1,
            }
        }
    }

    fn inject_postconditions_into_expr(&self, expr: &mut HirExpr, post_stmts: &[HirStmt]) {
        match expr {
            HirExpr::Block(stmts, _) => {
                self.inject_postconditions_before_returns(stmts, post_stmts);
            }
            HirExpr::If(_, then_body, else_body, _) => {
                self.inject_postconditions_before_returns(then_body, post_stmts);
                if let Some(eb) = else_body {
                    self.inject_postconditions_before_returns(eb, post_stmts);
                }
            }
            HirExpr::Match(_, arms, _) => {
                for arm in arms {
                    self.inject_postconditions_into_expr(&mut arm.body, post_stmts);
                }
            }
            HirExpr::For(_, _, body, _) => {
                self.inject_postconditions_into_expr(body, post_stmts);
            }
            // Lambdas create a new return context, so we do NOT inject postconditions into them.
            _ => {}
        }
    }
}
