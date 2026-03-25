use crate::ast::span::Span;
use crate::hir::{HirMatchArm, HirPattern};
use crate::typeck::diagnostics::Diagnostic;
use crate::typeck::env::TypeEnv;
use crate::typeck::ty::Ty;

/// ADT / option-style exhaustiveness (mirrors `check::patterns::check_match_exhaustiveness` for HIR).
pub(crate) fn check_hir_match_exhaustiveness(
    env: &TypeEnv,
    diags: &mut Vec<Diagnostic>,
    subject_ty: &Ty,
    arms: &[HirMatchArm],
    span: Span,
    source: &str,
) {
    let type_name = match subject_ty {
        Ty::Named(name) => name.as_str(),
        _ => return,
    };

    let adt = match env.lookup_adt(type_name) {
        Some(adt) => adt,
        None => return,
    };

    let mut covered_variants: Vec<String> = Vec::new();
    let mut has_wildcard = false;

    for arm in arms {
        match &arm.pattern {
            HirPattern::Wildcard(_) => {
                has_wildcard = true;
            }
            HirPattern::Ident(name, _) => {
                if adt.variants.iter().any(|v| v.name == *name) {
                    covered_variants.push(name.clone());
                } else {
                    has_wildcard = true;
                }
            }
            HirPattern::Constructor(name, _, _) => {
                covered_variants.push(name.clone());
            }
            HirPattern::Tuple(_, _) | HirPattern::Literal(_, _) => {}
        }
    }

    if has_wildcard {
        return;
    }

    let missing: Vec<&str> = adt
        .variants
        .iter()
        .filter(|v| !covered_variants.contains(&v.name))
        .map(|v| v.name.as_str())
        .collect();

    if !missing.is_empty() {
        diags.push(Diagnostic::error(
            format!(
                "Non-exhaustive match on type '{}'. Missing variant(s): {}",
                type_name,
                missing.join(", ")
            ),
            span,
            source,
        ));
    }
}
