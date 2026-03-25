use crate::ast::pattern::Pattern;
use crate::ast::stmt::Stmt;
use crate::hir::*;

use super::LowerCtx;

impl LowerCtx {
    pub(crate) fn lower_stmt(&mut self, s: &Stmt) -> HirStmt {
        match s {
            Stmt::Let {
                pattern,
                type_ann,
                value,
                mutable,
                span,
            } => HirStmt::Let {
                pattern: self.lower_pattern(pattern),
                type_ann: type_ann.as_ref().map(|t| self.lower_type(t)),
                value: self.lower_expr(value),
                mutable: *mutable,
                span: *span,
            },
            Stmt::Assign {
                target,
                value,
                span,
            } => HirStmt::Assign {
                target: self.lower_expr(target),
                value: self.lower_expr(value),
                span: *span,
            },
            Stmt::Return { value, span } => HirStmt::Return {
                value: value.as_ref().map(|v| self.lower_expr(v)),
                span: *span,
            },
            Stmt::Expr { expr, span } => HirStmt::Expr {
                expr: self.lower_expr(expr),
                span: *span,
            },
        }
    }

    pub(crate) fn lower_pattern(&mut self, p: &Pattern) -> HirPattern {
        match p {
            Pattern::Ident { name, span } => {
                self.def_map.define(name.clone());
                HirPattern::Ident(name.clone(), *span)
            }
            Pattern::Tuple { elements, span } => HirPattern::Tuple(
                elements.iter().map(|e| self.lower_pattern(e)).collect(),
                *span,
            ),
            Pattern::Constructor { name, fields, span } => HirPattern::Constructor(
                name.clone(),
                fields.iter().map(|f| self.lower_pattern(f)).collect(),
                *span,
            ),
            Pattern::Wildcard { span } => HirPattern::Wildcard(*span),
            Pattern::Literal { value, span } => {
                HirPattern::Literal(Box::new(self.lower_expr(value)), *span)
            }
        }
    }
}
