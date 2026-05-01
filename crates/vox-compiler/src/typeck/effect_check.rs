//! Structural checks for function effect annotations (TASK-4.2).
//!
//! Current scope (structural only — call-graph propagation deferred):
//! - `E_EFFECT_PURE_CONFLICT` — function declares both `@pure` and a `uses` clause.
//! - `E_EFFECT_DUPLICATE` — same effect kind appears twice in one `uses` clause.
//!
//! Call-graph propagation (`caller.effects ⊇ callee.effects`) is a Phase 5 item
//! that requires a resolved call graph.

use crate::hir::nodes::{HirEndpointFn, HirFn};
use crate::hir::nodes::effect::HirEffectKind;
use crate::typeck::diagnostics::{DiagnosticCategory, TypeckSeverity};
use crate::typeck::Diagnostic;

/// Run structural effect checks over a slice of HIR functions.
pub fn check_fn_effects(fns: &[HirFn]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for f in fns {
        check_one_fn(f, &mut diags);
    }
    diags
}

/// Run structural effect checks over a slice of endpoint functions.
///
/// Endpoint functions carry the same `is_pure`/`effects` contract as regular
/// functions; this enforces the same `E_EFFECT_PURE_CONFLICT` and
/// `E_EFFECT_DUPLICATE` rules across the endpoint surface.
pub fn check_endpoint_fn_effects(fns: &[HirEndpointFn]) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    for f in fns {
        check_one_endpoint_fn(f, &mut diags);
    }
    diags
}

fn check_one_fn(f: &HirFn, diags: &mut Vec<Diagnostic>) {
    // E_EFFECT_PURE_CONFLICT: @pure + uses clause is contradictory.
    if f.is_pure && !f.effects.is_empty() {
        let labels: Vec<String> = f.effects.iter().map(|e: &HirEffectKind| e.label()).collect();
        diags.push(Diagnostic {
            severity: TypeckSeverity::Error,
            message: format!(
                "function `{}` is marked `@pure` but also declares effects: {}. \
                 Remove `@pure` or remove the `uses` clause.",
                f.name,
                labels.join(", ")
            ),
            span: f.span,
            expected_type: None,
            found_type: None,
            context: Some("@pure means no side effects; `uses` declares side effects".to_string()),
            suggestions: vec![],
            category: DiagnosticCategory::Typecheck,
            code: Some("E_EFFECT_PURE_CONFLICT".to_string()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: Some("FnDecl".to_string()),
        });
    }

    // E_EFFECT_DUPLICATE: same effect listed more than once.
    let mut seen: Vec<&HirEffectKind> = Vec::new();
    for eff in &f.effects {
        if seen.iter().any(|s| *s == eff) {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: format!(
                    "function `{}` declares effect `{}` more than once in its `uses` clause.",
                    f.name,
                    eff.label()
                ),
                span: f.span,
                expected_type: None,
                found_type: None,
                context: Some("each effect kind should appear at most once".to_string()),
                suggestions: vec![],
                category: DiagnosticCategory::Typecheck,
                code: Some("E_EFFECT_DUPLICATE".to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: Some("FnDecl".to_string()),
            });
        } else {
            seen.push(eff);
        }
    }
}

fn check_one_endpoint_fn(f: &HirEndpointFn, diags: &mut Vec<Diagnostic>) {
    // E_EFFECT_PURE_CONFLICT: @pure + uses clause is contradictory.
    if f.is_pure && !f.effects.is_empty() {
        let labels: Vec<String> = f.effects.iter().map(|e: &HirEffectKind| e.label()).collect();
        diags.push(Diagnostic {
            severity: TypeckSeverity::Error,
            message: format!(
                "endpoint `{}` is marked `@pure` but also declares effects: {}. \
                 Remove `@pure` or remove the `uses` clause.",
                f.name,
                labels.join(", ")
            ),
            span: f.span,
            expected_type: None,
            found_type: None,
            context: Some("@pure means no side effects; `uses` declares side effects".to_string()),
            suggestions: vec![],
            category: DiagnosticCategory::Typecheck,
            code: Some("E_EFFECT_PURE_CONFLICT".to_string()),
            fixes: vec![],
            line_col: None,
            missing_cases: vec![],
            ast_node_kind: Some("EndpointFnDecl".to_string()),
        });
    }

    // E_EFFECT_DUPLICATE: same effect listed more than once.
    let mut seen: Vec<&HirEffectKind> = Vec::new();
    for eff in &f.effects {
        if seen.iter().any(|s| *s == eff) {
            diags.push(Diagnostic {
                severity: TypeckSeverity::Error,
                message: format!(
                    "endpoint `{}` declares effect `{}` more than once in its `uses` clause.",
                    f.name,
                    eff.label()
                ),
                span: f.span,
                expected_type: None,
                found_type: None,
                context: Some("each effect kind should appear at most once".to_string()),
                suggestions: vec![],
                category: DiagnosticCategory::Typecheck,
                code: Some("E_EFFECT_DUPLICATE".to_string()),
                fixes: vec![],
                line_col: None,
                missing_cases: vec![],
                ast_node_kind: Some("EndpointFnDecl".to_string()),
            });
        } else {
            seen.push(eff);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::span::Span;
    use crate::hir::nodes::effect::HirEffectKind;
    use crate::hir::nodes::DefId;

    fn dummy_span() -> Span {
        Span::new(0, 0)
    }

    fn make_fn(name: &str, is_pure: bool, effects: Vec<HirEffectKind>) -> HirFn {
        HirFn {
            id: DefId(0),
            name: name.to_string(),
            generics: vec![],
            params: vec![],
            return_type: None,
            body: vec![],
            is_component: false,
            is_async: false,
            is_pub: false,
            is_mobile_native: false,
            is_pure,
            effects,
            is_llm: false,
            llm_model: None,
            is_deprecated: false,
            schedule_interval: None,
            durability: None,
            actor_state_fields: vec![],
            postconditions: vec![],
            span: dummy_span(),
        }
    }

    #[test]
    fn test_clean_fn_no_effects() {
        let f = make_fn("total", false, vec![]);
        let diags = check_fn_effects(&[f]);
        assert!(diags.is_empty(), "fn with no effects should produce no diagnostics");
    }

    #[test]
    fn test_clean_fn_with_effects() {
        let f = make_fn("fetch_tasks", false, vec![HirEffectKind::Net]);
        let diags = check_fn_effects(&[f]);
        assert!(diags.is_empty(), "fn with declared net effect should pass structural check");
    }

    #[test]
    fn test_pure_conflict() {
        let f = make_fn("bad", true, vec![HirEffectKind::Db]);
        let diags = check_fn_effects(&[f]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Some("E_EFFECT_PURE_CONFLICT".to_string()));
        assert_eq!(diags[0].severity, TypeckSeverity::Error);
    }

    #[test]
    fn test_duplicate_effect() {
        let f = make_fn("dup", false, vec![HirEffectKind::Net, HirEffectKind::Net]);
        let diags = check_fn_effects(&[f]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Some("E_EFFECT_DUPLICATE".to_string()));
    }

    #[test]
    fn test_mcp_tool_variant() {
        let f = make_fn(
            "save_task",
            false,
            vec![HirEffectKind::Db, HirEffectKind::Mcp("vox_notify_ludus".into())],
        );
        let diags = check_fn_effects(&[f]);
        assert!(diags.is_empty(), "fn with db + mcp effects should pass structural check");
    }

    // ── endpoint fn checks ──────────────────────────────────────────────────

    fn make_endpoint_fn(name: &str, is_pure: bool, effects: Vec<HirEffectKind>) -> HirEndpointFn {
        use crate::hir::nodes::{HirEndpointKind};
        HirEndpointFn {
            kind: HirEndpointKind::Query,
            id: DefId(0),
            name: name.to_string(),
            params: vec![],
            return_type: None,
            body: vec![],
            route_path: format!("/api/query/{name}"),
            is_pure,
            effects,
            span: dummy_span(),
        }
    }

    #[test]
    fn endpoint_pure_conflict_is_caught() {
        let f = make_endpoint_fn("bad_endpoint", true, vec![HirEffectKind::Db]);
        let diags = check_endpoint_fn_effects(&[f]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Some("E_EFFECT_PURE_CONFLICT".to_string()));
        assert_eq!(diags[0].severity, TypeckSeverity::Error);
        assert!(diags[0].ast_node_kind.as_deref() == Some("EndpointFnDecl"));
    }

    #[test]
    fn endpoint_duplicate_effect_is_caught() {
        let f = make_endpoint_fn("dup_endpoint", false, vec![HirEffectKind::Net, HirEffectKind::Net]);
        let diags = check_endpoint_fn_effects(&[f]);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, Some("E_EFFECT_DUPLICATE".to_string()));
    }

    #[test]
    fn endpoint_clean_effects_pass() {
        let f = make_endpoint_fn("list_tasks", false, vec![HirEffectKind::Db]);
        let diags = check_endpoint_fn_effects(&[f]);
        assert!(diags.is_empty(), "endpoint with single declared effect should pass");
    }
}
