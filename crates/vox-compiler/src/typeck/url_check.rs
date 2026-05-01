//! Type checking for `url` declarations (TASK-4.3).
//!
//! Validates:
//! - No duplicate variant names within a `url` block.
//! - No duplicate arg names within a variant.
//! - No duplicate `url` type names within a module.
use crate::hir::nodes::url::HirUrlDecl;
use crate::typeck::diagnostics::{DiagnosticCategory, TypeckSeverity};
use crate::typeck::Diagnostic;
use std::collections::HashSet;

/// Run all url-declaration checks. Returns a list of diagnostics.
pub fn check_url_decls(url_decls: &[HirUrlDecl]) -> Vec<Diagnostic> {
    let mut out = Vec::new();
    let mut seen_type_names: HashSet<&str> = HashSet::new();

    for decl in url_decls {
        // Duplicate type names
        if !seen_type_names.insert(decl.name.as_str()) {
            out.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: format!("duplicate url type name `{}`", decl.name),
                span: decl.span,
                expected_type: None,
                found_type: None,
                context: Some(
                    "url declarations must have unique names within a module".to_string(),
                ),
                suggestions: vec![],
                category: DiagnosticCategory::Typecheck,
                code: Some("E200".to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: Some("UrlDecl".to_string()),
            });
        }

        let mut seen_variants: HashSet<&str> = HashSet::new();
        for variant in &decl.variants {
            if !seen_variants.insert(variant.name.as_str()) {
                out.push(Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: format!(
                        "duplicate variant `{}` in url type `{}`",
                        variant.name, decl.name
                    ),
                    span: variant.span,
                    expected_type: None,
                    found_type: None,
                    context: Some(format!(
                        "url `{}` has two variants named `{}`",
                        decl.name, variant.name
                    )),
                    suggestions: vec![],
                    category: DiagnosticCategory::Typecheck,
                    code: Some("E201".to_string()),
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    ast_node_kind: Some("UrlVariant".to_string()),
                });
            }

            let mut seen_args: HashSet<&str> = HashSet::new();
            for arg in &variant.args {
                if !seen_args.insert(arg.name.as_str()) {
                    out.push(Diagnostic {
                        severity: TypeckSeverity::Error,
                        message: format!(
                            "duplicate argument `{}` in variant `{}::{}`",
                            arg.name, decl.name, variant.name
                        ),
                        span: arg.span,
                        expected_type: None,
                        found_type: None,
                        context: None,
                        suggestions: vec![],
                        category: DiagnosticCategory::Typecheck,
                        code: Some("E202".to_string()),
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        ast_node_kind: Some("UrlArg".to_string()),
                    });
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::ast::types::TypeExpr;
    use crate::hir::nodes::url::{HirUrlArg, HirUrlDecl, HirUrlVariant};

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }
    fn dummy_type() -> TypeExpr {
        TypeExpr::Named {
            name: "str".to_string(),
            span: dummy_span(),
        }
    }

    #[test]
    fn clean_url_decl_no_diagnostics() {
        let decls = vec![HirUrlDecl {
            name: "Path".to_string(),
            variants: vec![
                HirUrlVariant {
                    name: "Home".to_string(),
                    args: vec![],
                    span: dummy_span(),
                },
                HirUrlVariant {
                    name: "Task".to_string(),
                    args: vec![HirUrlArg {
                        name: "id".to_string(),
                        optional: false,
                        ty: dummy_type(),
                        span: dummy_span(),
                    }],
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        }];
        assert!(check_url_decls(&decls).is_empty());
    }

    #[test]
    fn duplicate_variant_name_is_error() {
        let decls = vec![HirUrlDecl {
            name: "Path".to_string(),
            variants: vec![
                HirUrlVariant {
                    name: "Home".to_string(),
                    args: vec![],
                    span: dummy_span(),
                },
                HirUrlVariant {
                    name: "Home".to_string(),
                    args: vec![],
                    span: dummy_span(),
                },
            ],
            span: dummy_span(),
        }];
        let diags = check_url_decls(&decls);
        assert!(diags.iter().any(|d| d.code.as_deref() == Some("E201")));
    }
}
