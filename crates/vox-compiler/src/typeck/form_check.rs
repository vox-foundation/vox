//! Form validation: cross-checks `@form` declarations against the `@endpoint` they reference.

use crate::hir::HirModule;
use crate::typeck::diagnostics::{Diagnostic, DiagnosticCategory, TypeckSeverity};

/// Check all `@form` declarations in the HIR module.
///
/// Emits diagnostics for:
/// - `lint.form.unknown_endpoint` — `on_submit` names a function that is not an `@endpoint`.
/// - `lint.form.field_unmatched` — a form field has no matching parameter in the endpoint.
/// - `lint.form.field_type_mismatch` — a form field's type differs from the endpoint parameter type.
pub fn check_forms(hir: &HirModule, _source: &str) -> Vec<Diagnostic> {
    use crate::ast::span::Span;

    let dummy_span = Span::new(0, 0);

    let mut diags = vec![];
    for form in &hir.forms {
        if let Some(submit) = &form.on_submit {
            let endpoint = hir.endpoint_fns.iter().find(|e| &e.name == submit);
            if endpoint.is_none() {
                diags.push(Diagnostic {
                    severity: TypeckSeverity::Error,
                    message: format!(
                        "@form `{}` references on_submit `{}` but no @endpoint with that name exists.",
                        form.name, submit
                    ),
                    span: form.span,
                    code: Some("lint.form.unknown_endpoint".into()),
                    category: DiagnosticCategory::Lint,
                    suggestions: vec![],
                    fixes: vec![],
                    line_col: None,
                    missing_cases: vec![],
                    expected_type: None,
                    found_type: None,
                    context: None,
                    ast_node_kind: None,
                });
                continue;
            }
            let ep = endpoint.unwrap();
            for vf in form.fields.iter().filter(|f| !f.hidden || f.default.is_none()) {
                let param = ep.params.iter().find(|p| p.name == vf.name);
                match param {
                    None => diags.push(Diagnostic {
                        severity: TypeckSeverity::Error,
                        message: format!(
                            "@form `{}` field `{}` has no matching parameter in @endpoint `{}`.",
                            form.name, vf.name, submit
                        ),
                        span: vf.span,
                        code: Some("lint.form.field_unmatched".into()),
                        category: DiagnosticCategory::Lint,
                        suggestions: vec![format!(
                            "Add `{}: {:?}` to @endpoint `{}` or remove the field.",
                            vf.name, vf.ty, submit
                        )],
                        fixes: vec![],
                        line_col: None,
                        missing_cases: vec![],
                        expected_type: None,
                        found_type: None,
                        context: None,
                        ast_node_kind: None,
                    }),
                    Some(p) => {
                        // Compare field type against endpoint param type annotation.
                        // If the param has no type annotation we cannot verify — skip.
                        if let Some(param_ty) = &p.type_ann {
                            if param_ty != &vf.ty {
                                diags.push(Diagnostic {
                                    severity: TypeckSeverity::Error,
                                    message: format!(
                                        "@form `{}` field `{}` has type `{:?}` but @endpoint `{}` expects `{:?}`.",
                                        form.name, vf.name, vf.ty, submit, param_ty
                                    ),
                                    span: vf.span,
                                    code: Some("lint.form.field_type_mismatch".into()),
                                    category: DiagnosticCategory::Lint,
                                    suggestions: vec![],
                                    fixes: vec![],
                                    line_col: None,
                                    missing_cases: vec![],
                                    expected_type: Some(format!("{:?}", param_ty)),
                                    found_type: Some(format!("{:?}", vf.ty)),
                                    context: None,
                                    ast_node_kind: None,
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    let _ = dummy_span; // suppress unused warning
    diags
}
