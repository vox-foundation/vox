//! WebIR validation pass (ADR 012) — structural checks before target emitters.
//!
//! ## Stages (OP-0081)
//! - **DOM** — view root ids, arena bounds, edge walks (existing).
//! - **Routes** — duplicate contract ids, loader / RPC shape (OP-0084, OP-0086).
//! - **Behavior** — [`crate::web_ir::FieldOptionality`] vs initial value (OP-0082).
//! - **Style** — empty rules and declarations (OP-0088).
//! - **Islands** — prop key sanity on [`crate::web_ir::DomNode::IslandMount`] (OP-0090).
//! - **Interop** — non-empty fields on [`crate::web_ir::InteropNode`] (ADR 012).
//!
//! Diagnostic **codes** use dotted prefixes (`web_ir_validate.dom.*`, `web_ir_validate.route.*`, …)
//! for dashboards (OP-0092).
//!
//! **Serializability + interop enforcement (OP-S063 / S109 / S135 / S157 / S187):** every
//! `web_ir_validate.*` code should be stable for CI dashboards; [`super::WebIrModule`] JSON round-trips
//! are tested in `web_ir_lower_emit.rs`. Interop rows use `web_ir_validate.interop.*` for escape hatch
//! and import policy.

use std::collections::HashSet;

use super::{
    BehaviorNode, DomNode, DomNodeId, FieldOptionality, InteropNode, RouteContract, RouteNode,
    StyleNode, StyleSelector, WebIrDiagnostic, WebIrDiagnosticSeverity, WebIrModule,
    WebIrValidateMetrics,
};

fn walk_route_contract_ids(
    nodes: &[RouteContract],
    seen: &mut HashSet<String>,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    for r in nodes {
        metrics.route_contract_ids_checked += 1;
        if !seen.insert(r.id.clone()) {
            out.push(WebIrDiagnostic {
                code: "web_ir_validate.route.duplicate_contract_id".to_string(),
                message: format!("duplicate RouteContract.id {:?}", r.id),
                span: None,
                category: Some("route".to_string()),
                ..Default::default()
            });
        }
        if !r.children.is_empty() {
            walk_route_contract_ids(&r.children, seen, out, metrics);
        }
    }
}

fn check_dom_id(out: &mut Vec<WebIrDiagnostic>, len: usize, id: DomNodeId, ctx: &str) -> bool {
    if (id.0 as usize) >= len {
        out.push(WebIrDiagnostic {
            code: "web_ir_validate.dom.id_oob".to_string(),
            message: format!("{ctx}: DomNodeId({}) out of range (len {len})", id.0),
            span: None,
            category: Some("dom".to_string()),
            ..Default::default()
        });
        return false;
    }
    true
}

fn walk_dom_edges(
    out: &mut Vec<WebIrDiagnostic>,
    module: &WebIrModule,
    id: DomNodeId,
    metrics: &mut WebIrValidateMetrics,
) {
    let len = module.dom_nodes.len();
    metrics.dom_nodes_traversed += 1;
    if !check_dom_id(out, len, id, "walk") {
        return;
    }
    let Some(node) = module.dom_nodes.get(id.0 as usize) else {
        return;
    };
    if let DomNode::IslandMount { props, .. } = node {
        metrics.island_mounts_checked += 1;
        for (k, _) in props {
            if k.is_empty() {
                out.push(WebIrDiagnostic {
                    code: "web_ir_validate.island.empty_prop_key".to_string(),
                    message: "IslandMount prop key must not be empty".to_string(),
                    span: None,
                    category: Some("island".to_string()),
                    ..Default::default()
                });
            }
        }
    }
    let child_ids: Vec<DomNodeId> = match node {
        DomNode::Element { children, .. } | DomNode::Fragment { children, .. } => children.clone(),
        DomNode::Conditional {
            then_children,
            else_children,
            ..
        } => {
            let mut v = then_children.clone();
            v.extend(else_children.iter().copied());
            v
        }
        DomNode::Loop { body, .. } => body.clone(),
        DomNode::IslandMount { .. }
        | DomNode::Text { .. }
        | DomNode::Slot { .. }
        | DomNode::Expr { .. } => vec![],
    };
    for c in child_ids {
        walk_dom_edges(out, module, c, metrics);
    }
}

fn validate_dom_roots(
    module: &WebIrModule,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    if module.dom_nodes.len() > 1_000_000 {
        out.push(WebIrDiagnostic {
            code: "web_ir_validate.dom.arena_too_large".to_string(),
            message: "dom node arena exceeds implementation limit".to_string(),
            span: None,
            category: Some("dom".to_string()),
                    ..Default::default()
        });
    }

    for (name, root) in &module.view_roots {
        metrics.view_roots_walked += 1;
        if !check_dom_id(
            out,
            module.dom_nodes.len(),
            *root,
            &format!("view root '{name}'"),
        ) {
            continue;
        }
        walk_dom_edges(out, module, *root, metrics);
    }
}

/// **Stage Routes (OP-S019):** enforces unique [`RouteContract::id`] per module, loader id/contract non-empty,
/// and non-empty server/mutation contract fields — router-facing summaries only; HTTP semantics live on HIR.
fn validate_route_families(
    module: &WebIrModule,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    let mut seen_route_contract_ids = HashSet::<String>::new();
    let mut seen_loader_ids = HashSet::<String>::new();

    for node in &module.route_nodes {
        match node {
            RouteNode::RouteTree { routes, .. } => {
                walk_route_contract_ids(routes, &mut seen_route_contract_ids, out, metrics);
            }
            RouteNode::LoaderContract {
                route_id, contract, ..
            } => {
                if !seen_loader_ids.insert(route_id.clone()) {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.duplicate_loader_id".to_string(),
                        message: format!("duplicate LoaderContract.route_id {:?}", route_id),
                        span: None,
                        category: Some("route".to_string()),
                        ..Default::default()
                    });
                }
                if route_id.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_loader_id".to_string(),
                        message: "LoaderContract.route_id must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
                if contract.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_loader_contract".to_string(),
                        message: "LoaderContract.contract must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
            }
            RouteNode::ServerFnContract(s) => {
                if s.name.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_server_fn_name".to_string(),
                        message: "ServerFnContract.name must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
                if s.export_path.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_server_export_path".to_string(),
                        message: "ServerFnContract.export_path must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
                if s.signature.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_server_signature".to_string(),
                        message: "ServerFnContract.signature must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
            }
            RouteNode::MutationContract(m) => {
                if m.name.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_mutation_name".to_string(),
                        message: "MutationContract.name must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
                if m.payload_type.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_mutation_payload_type".to_string(),
                        message: "MutationContract.payload_type must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    ..Default::default()
                    });
                }
            }
        }
    }
}

/// **Behavior / optionality (OP-S017):** currently guards `StateDecl` — [`FieldOptionality::Required`] must
/// ship a lowered `initial`; extend here for `Optional`/`Defaulted` invariants on islands and props.
fn validate_behaviors(
    module: &WebIrModule,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    for b in &module.behavior_nodes {
        metrics.behavior_nodes_checked += 1;
        if let BehaviorNode::StateDecl {
            optionality,
            initial,
            name,
            ..
        } = b
            && matches!(optionality, FieldOptionality::Required)
            && initial.is_none()
        {
            out.push(WebIrDiagnostic {
                code: "web_ir_validate.behavior.required_state_without_initial".to_string(),
                message: format!(
                    "StateDecl '{name}' is Required but has no lowered initial expression"
                ),
                span: None,
                category: Some("behavior".to_string()),
                ..Default::default()
            });
        }
    }
}

fn validate_styles(
    module: &WebIrModule,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    let mut seen_selectors: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for node in &module.style_nodes {
        metrics.style_nodes_checked += 1;
        if let StyleNode::Rule {
            selector,
            declarations,
            ..
        } = node
        {
            let sel_key = match selector {
                StyleSelector::Class(c) => format!(".{}", c),
                StyleSelector::Id(i) => format!("#{}", i),
                StyleSelector::Element(e) => e.clone(),
                StyleSelector::Unparsed(u) => u.clone(),
                StyleSelector::Compound(_) => "compound".to_string(), // Simplified for now
                StyleSelector::Pseudo { pseudo, .. } => format!("pseudo:{}", pseudo),
            };

            if declarations.is_empty() {
                out.push(WebIrDiagnostic {
                    code: "web_ir_validate.style.empty_declarations".to_string(),
                    message: "StyleNode::Rule has no declarations".to_string(),
                    span: None,
                    category: Some("style".to_string()),
                    ..Default::default()
                });
            }

            let mut props_in_rule = std::collections::HashSet::new();
            for (prop, _) in declarations {
                if prop.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.style.empty_property".to_string(),
                        message: "style declaration property name must not be empty".to_string(),
                        span: None,
                        category: Some("style".to_string()),
                    ..Default::default()
                    });
                } else {
                    let css_prop = prop.chars().fold(String::new(), |mut acc, c| {
                        if c.is_uppercase() {
                            acc.push('-');
                            acc.push(c.to_ascii_lowercase());
                        } else {
                            acc.push(c);
                        }
                        acc
                    });

                    if !props_in_rule.insert(css_prop.clone()) {
                        out.push(WebIrDiagnostic {
                            code: "web_ir_validate.style.duplicate_property_in_rule".to_string(),
                            message: format!("Duplicate property '{}' in the same rule", prop),
                            span: None,
                            category: Some("style".to_string()),
                            ..Default::default()
                        });
                    }

                    if !crate::codegen_shared::css_property_allowlist::is_allowed_css_property(
                        &css_prop,
                    ) {
                        out.push(WebIrDiagnostic {
                            code: "web_ir_validate.style.unknown_property".to_string(),
                            message: format!(
                                "Unknown CSS property '{}' (normalized to '{}')",
                                prop, css_prop
                            ),
                            span: None,
                            category: Some("style".to_string()),
                            ..Default::default()
                        });
                    }

                    if let Some(existing_props) = seen_selectors.get(&sel_key) {
                        if existing_props.contains(&css_prop) {
                            out.push(WebIrDiagnostic {
                                code: "web_ir_validate.style.specificity_conflict".to_string(),
                                message: format!("Property '{}' redefined for selector '{}' at same specificity level", prop, sel_key),
                                span: None,
                                category: Some("style".to_string()),
                                ..Default::default()
                            });
                        }
                    }
                }
            }

            let normalized_props: Vec<String> = declarations
                .iter()
                .map(|(p, _)| {
                    p.chars().fold(String::new(), |mut acc, c| {
                        if c.is_uppercase() {
                            acc.push('-');
                            acc.push(c.to_ascii_lowercase());
                        } else {
                            acc.push(c);
                        }
                        acc
                    })
                })
                .collect();

            seen_selectors
                .entry(sel_key)
                .or_default()
                .extend(normalized_props);
        }
    }
}

fn validate_scheduled_jobs(
    module: &WebIrModule,
    out: &mut Vec<WebIrDiagnostic>,
    metrics: &mut WebIrValidateMetrics,
) {
    for job in &module.scheduled_jobs {
        metrics.scheduled_jobs_checked += 1;
        if job.name.trim().is_empty() {
            out.push(WebIrDiagnostic {
                code: "web_ir_validate.scheduled.empty_name".to_string(),
                message: "ScheduledJobSpec.name must not be empty".to_string(),
                span: None,
                category: Some("scheduled".to_string()),
                    ..Default::default()
            });
        }
        if job.interval.trim().is_empty() {
            out.push(WebIrDiagnostic {
                code: "web_ir_validate.scheduled.empty_interval".to_string(),
                message: "ScheduledJobSpec.interval must not be empty".to_string(),
                span: None,
                category: Some("scheduled".to_string()),
                    ..Default::default()
            });
        }
    }
}

fn validate_interop(module: &WebIrModule, out: &mut Vec<WebIrDiagnostic>) {
    for node in &module.interop_nodes {
        match node {
            InteropNode::ReactComponentRef {
                component,
                import_source,
                ..
            } => {
                if component.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_component".to_string(),
                        message: "ReactComponentRef.component must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
                    ..Default::default()
                    });
                }
                if import_source.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_import_source".to_string(),
                        message: "ReactComponentRef.import_source must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
                    ..Default::default()
                    });
                }
            }
            InteropNode::ExternalModuleRef { specifier, .. } => {
                if specifier.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_external_specifier".to_string(),
                        message: "ExternalModuleRef.specifier must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
                    ..Default::default()
                    });
                }
            }
            InteropNode::EscapeHatchExpr { expr, reason, .. } => {
                if expr.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_escape_expr".to_string(),
                        message: "EscapeHatchExpr.expr must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
                    ..Default::default()
                    });
                }
                if reason.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_escape_reason".to_string(),
                        message: "EscapeHatchExpr.reason must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
                    ..Default::default()
                    });
                }
            }
        }
    }
}

/// Semicolon-joined diagnostic lines for `VOX_WEBIR_VALIDATE` gates (OP-0287); shared with codegen.
#[must_use]
pub fn format_web_ir_validate_failure(diags: &[WebIrDiagnostic]) -> String {
    diags
        .iter()
        .map(|d| {
            format!(
                "{} [{}]: {}",
                d.code,
                d.category.as_deref().unwrap_or("-"),
                d.message
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// Run structural checks that should hold before any target emitter, with counters for gates (OP-0094).
#[must_use]
pub fn validate_web_ir_with_metrics(
    module: &WebIrModule,
) -> (Vec<WebIrDiagnostic>, WebIrValidateMetrics) {
    let mut out = Vec::new();
    let mut metrics = WebIrValidateMetrics::default();

    validate_dom_roots(module, &mut out, &mut metrics);
    validate_route_families(module, &mut out, &mut metrics);
    validate_behaviors(module, &mut out, &mut metrics);
    validate_styles(module, &mut out, &mut metrics);
    validate_scheduled_jobs(module, &mut out, &mut metrics);
    validate_interop(module, &mut out);

    (out, metrics)
}

/// Run structural checks that should hold before any target emitter.
#[must_use]
pub fn validate_web_ir(module: &WebIrModule) -> Vec<WebIrDiagnostic> {
    validate_web_ir_with_metrics(module).0
}

/// Run structural checks with an optional token registry for TokenRef validation.
///
/// Extends [`validate_web_ir`] with:
/// - `web_ir_validate.style.unknown_token_ref` — a `StyleDeclarationValue::TokenRef` value
///   is not present in the registry.
/// - `web_ir_validate.style.raw_literal_color` — a `StyleDeclarationValue::Raw` value that
///   looks like a literal color (`#rrggbb`, `rgb(…)`, `hsl(…)`) should use a token instead.
///
/// When `token_registry` is `None`, this is identical to [`validate_web_ir`].
#[must_use]
pub fn validate_web_ir_with_tokens(
    module: &WebIrModule,
    token_registry: Option<&crate::tokens::TokenRegistry>,
) -> Vec<WebIrDiagnostic> {
    let mut out = validate_web_ir(module);
    let Some(registry) = token_registry else {
        return out;
    };
    validate_token_refs(module, registry, &mut out);
    out
}

/// Common CSS named colors that should use design tokens instead of literals.
/// Non-exhaustive: covers the most frequent offenders; extend as needed.
const CSS_NAMED_COLORS: &[&str] = &[
    "red", "blue", "green", "yellow", "orange", "purple", "pink", "brown",
    "black", "white", "gray", "grey", "cyan", "magenta", "lime", "indigo",
    "violet", "gold", "silver", "teal", "navy", "maroon", "olive", "aqua",
    "transparent", "currentcolor", "inherit", "initial", "unset",
];

/// CSS dimension suffixes that indicate a hard-coded unit value.
const CSS_DIMENSION_SUFFIXES: &[&str] = &[
    "px", "rem", "em", "vh", "vw", "vmin", "vmax", "%", "pt", "cm", "mm",
    "ex", "ch", "fr", "dvh", "dvw", "svh", "svw",
];

/// Returns `true` if `s` is a literal style value that should use a design token:
/// - Hex color (`#RGB`, `#RRGGBB`, `#RGBA`, `#RRGGBBAA`)
/// - Functional color (`rgb(...)`, `rgba(...)`, `hsl(...)`, `hsla(...)`, `oklch(...)`)
/// - CSS named color (`red`, `transparent`, …)
/// - Dimensional literal (`12px`, `1.5rem`, `50%`, …)
///
/// Note: single bare keywords used as CSS values (e.g., `bold`, `auto`, `none`) are
/// intentionally *not* flagged — they are not design-token candidates.
fn is_literal_style_value(s: &str) -> bool {
    let s = s.trim();
    if s.is_empty() {
        return false;
    }

    // Hex color: #RGB, #RGBA, #RRGGBB, #RRGGBBAA (3–8 hex digits after #).
    if s.starts_with('#') {
        let rest = &s[1..];
        if matches!(rest.len(), 3 | 4 | 6 | 8)
            && rest.chars().all(|c| c.is_ascii_hexdigit())
        {
            return true;
        }
    }

    // Functional color notations.
    for prefix in &["rgb(", "rgba(", "hsl(", "hsla(", "oklch(", "oklab(", "lch(", "lab(", "color("] {
        if s.to_ascii_lowercase().starts_with(prefix) {
            return true;
        }
    }

    // Named CSS colors (case-insensitive).
    let s_lower = s.to_ascii_lowercase();
    if CSS_NAMED_COLORS.contains(&s_lower.as_str()) {
        return true;
    }

    // Dimensional literals: optional sign, digits, optional decimal, then a known suffix.
    // Strip leading sign if present.
    let s_num = s.strip_prefix(['+', '-']).unwrap_or(s);
    // Find where the numeric part ends.
    let digit_end = s_num
        .find(|c: char| !c.is_ascii_digit() && c != '.')
        .unwrap_or(s_num.len());
    if digit_end > 0 {
        let suffix = &s_num[digit_end..];
        let suffix_lower = suffix.to_ascii_lowercase();
        if CSS_DIMENSION_SUFFIXES.contains(&suffix_lower.as_str()) {
            return true;
        }
    }

    false
}

fn check_declaration_value(
    name: &str,
    value: &super::StyleDeclarationValue,
    registry: &crate::tokens::TokenRegistry,
    out: &mut Vec<WebIrDiagnostic>,
) {
    use super::StyleDeclarationValue;
    match value {
        StyleDeclarationValue::TokenRef(token_name) => {
            if !registry.contains(token_name) {
                let suggestions = registry.suggest(token_name);
                let hint = if suggestions.is_empty() {
                    String::new()
                } else {
                    format!("; did you mean: {}?", suggestions.join(", "))
                };
                out.push(WebIrDiagnostic {
                    code: "web_ir_validate.style.unknown_token_ref".to_string(),
                    message: format!(
                        "unknown token '{token_name}' on property '{name}'{hint}"
                    ),
                    span: None,
                    category: Some("style".to_string()),
                    severity: WebIrDiagnosticSeverity::Warning,
                });
            }
        }
        StyleDeclarationValue::Raw(raw) => {
            if is_literal_style_value(raw) {
                // Phase 5: warning (not yet error) because the `raw_css { }` escape hatch
                // that would let users opt out is not added until Phase 6. Once that escape
                // hatch lands, this will be promoted to `Error`.
                out.push(WebIrDiagnostic {
                    code: "web_ir_validate.style.literal_value".to_string(),
                    message: format!(
                        "literal style value {raw:?} on property '{name}' should use a design token \
                         (e.g. token(\"color.primary\")). Use `raw_css {{ }}` to suppress once available."
                    ),
                    span: None,
                    category: Some("style".to_string()),
                    severity: WebIrDiagnosticSeverity::Warning,
                });
            }
        }
        _ => {}
    }
}

fn validate_token_refs(
    module: &WebIrModule,
    registry: &crate::tokens::TokenRegistry,
    out: &mut Vec<WebIrDiagnostic>,
) {
    for node in &module.style_nodes {
        match node {
            StyleNode::Rule { declarations, .. } => {
                for (prop, value) in declarations {
                    check_declaration_value(prop, value, registry, out);
                }
            }
            StyleNode::TokenRef { name, .. } => {
                // A top-level TokenRef node — validate its name against the registry.
                if !registry.contains(name) {
                    let suggestions = registry.suggest(name);
                    let hint = if suggestions.is_empty() {
                        String::new()
                    } else {
                        format!("; did you mean: {}?", suggestions.join(", "))
                    };
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.style.unknown_token_ref".to_string(),
                        message: format!("unknown token '{name}' in TokenRef node{hint}"),
                        span: None,
                        category: Some("style".to_string()),
                        severity: WebIrDiagnosticSeverity::Warning,
                    });
                }
            }
            StyleNode::Declaration { property, value, .. } => {
                check_declaration_value(property, value, registry, out);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validate_with_tokens_catches_unknown_ref() {
        use crate::tokens::TokenRegistry;
        use crate::web_ir::{StyleDeclarationValue, StyleNode, StyleSelector, WebIrModule};
        let registry =
            TokenRegistry::load_from_str(r##"{"color":{"primary":"#3a86ff"}}"##).unwrap();
        let mut m = WebIrModule::default();
        m.style_nodes.push(StyleNode::Rule {
            selector: StyleSelector::Class("x".to_string()),
            declarations: vec![(
                "color".to_string(),
                StyleDeclarationValue::TokenRef("color.nonexistent".to_string()),
            )],
            specificity: (0, 1, 0),
            span: None,
        });
        let diags = validate_web_ir_with_tokens(&m, Some(&registry));
        assert!(
            diags.iter().any(|d| d.code == "web_ir_validate.style.unknown_token_ref"),
            "expected unknown_token_ref diag, got: {diags:?}"
        );
    }

    // ── TASK-5.1: is_literal_style_value coverage ────────────────────────────

    #[test]
    fn literal_hex_colors_detected() {
        // All valid hex lengths.
        assert!(is_literal_style_value("#f00"),    "#RGB detected");
        assert!(is_literal_style_value("#ff0000"), "#RRGGBB detected");
        assert!(is_literal_style_value("#f00f"),   "#RGBA detected");
        assert!(is_literal_style_value("#ff0000ff"), "#RRGGBBAA detected");
        // Invalid hex (wrong length) — should not trigger.
        assert!(!is_literal_style_value("#ff"),    "too short — not a color");
        assert!(!is_literal_style_value("#fffff"),  "5 chars — not standard");
    }

    #[test]
    fn functional_color_notations_detected() {
        assert!(is_literal_style_value("rgb(255, 0, 0)"));
        assert!(is_literal_style_value("rgba(255, 0, 0, 0.5)"));
        assert!(is_literal_style_value("hsl(0, 100%, 50%)"));
        assert!(is_literal_style_value("hsla(0, 100%, 50%, 1)"));
        assert!(is_literal_style_value("oklch(0.7 0.15 50)"));
        assert!(is_literal_style_value("color(display-p3 1 0 0)"));
        // Case-insensitive.
        assert!(is_literal_style_value("RGB(0,0,0)"));
    }

    #[test]
    fn named_css_colors_detected() {
        assert!(is_literal_style_value("red"));
        assert!(is_literal_style_value("blue"));
        assert!(is_literal_style_value("transparent"));
        assert!(is_literal_style_value("WHITE"),    "case-insensitive");
        assert!(is_literal_style_value("currentColor"), "camelCase normalized");
        // Not a named color.
        assert!(!is_literal_style_value("bold"),    "font-weight keyword, not a color");
        assert!(!is_literal_style_value("auto"),    "CSS keyword, not a literal");
        assert!(!is_literal_style_value("none"));
    }

    #[test]
    fn dimensional_literals_detected() {
        assert!(is_literal_style_value("12px"),   "px unit");
        assert!(is_literal_style_value("1.5rem"), "rem unit");
        assert!(is_literal_style_value("50%"),    "percent");
        assert!(is_literal_style_value("2em"),    "em unit");
        assert!(is_literal_style_value("100vh"),  "viewport height");
        assert!(is_literal_style_value("4fr"),    "grid fraction");
        // Non-dimension bare numbers are not flagged (no suffix).
        assert!(!is_literal_style_value("0"),     "zero without unit — valid CSS");
        assert!(!is_literal_style_value("1"),     "bare integer");
    }

    #[test]
    fn token_ref_passes_through() {
        // TokenRef values should not trigger literal_value warning.
        assert!(!is_literal_style_value("token(\"color.primary\")"),
            "token() call should not be flagged");
    }

    #[test]
    fn literal_value_diag_emitted_for_hex_raw() {
        use crate::tokens::TokenRegistry;
        use crate::web_ir::{StyleDeclarationValue, StyleNode, StyleSelector, WebIrModule};
        let registry =
            TokenRegistry::load_from_str(r##"{"color":{"primary":"#3a86ff"}}"##).unwrap();
        let mut m = WebIrModule::default();
        m.style_nodes.push(StyleNode::Rule {
            selector: StyleSelector::Class("card".to_string()),
            declarations: vec![
                ("color".to_string(), StyleDeclarationValue::Raw("#ff0000".to_string())),
            ],
            specificity: (0, 1, 0),
            span: None,
        });
        let diags = validate_web_ir_with_tokens(&m, Some(&registry));
        let lit_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.code == "web_ir_validate.style.literal_value")
            .collect();
        assert!(!lit_diags.is_empty(), "expected literal_value diag for #ff0000");
        // Should be a warning, not an error (no escape hatch yet).
        assert_eq!(
            lit_diags[0].severity,
            WebIrDiagnosticSeverity::Warning,
            "literal_value should be Warning until raw_css{{}} escape hatch is available"
        );
    }

    #[test]
    fn dimensional_literal_diag_emitted() {
        use crate::tokens::TokenRegistry;
        use crate::web_ir::{StyleDeclarationValue, StyleNode, StyleSelector, WebIrModule};
        let registry =
            TokenRegistry::load_from_str(r##"{"spacing":{"md":"16px"}}"##).unwrap();
        let mut m = WebIrModule::default();
        m.style_nodes.push(StyleNode::Rule {
            selector: StyleSelector::Class("gap".to_string()),
            declarations: vec![
                ("gap".to_string(), StyleDeclarationValue::Raw("16px".to_string())),
            ],
            specificity: (0, 1, 0),
            span: None,
        });
        let diags = validate_web_ir_with_tokens(&m, Some(&registry));
        assert!(
            diags.iter().any(|d| d.code == "web_ir_validate.style.literal_value"),
            "expected literal_value diag for 16px"
        );
    }

    #[test]
    fn old_raw_literal_color_code_is_retired() {
        // Regression: ensure the old code name is not emitted (renamed in TASK-5.1).
        use crate::tokens::TokenRegistry;
        use crate::web_ir::{StyleDeclarationValue, StyleNode, StyleSelector, WebIrModule};
        let registry = TokenRegistry::load_from_str(r##"{"color":{}}"##).unwrap();
        let mut m = WebIrModule::default();
        m.style_nodes.push(StyleNode::Rule {
            selector: StyleSelector::Class("x".to_string()),
            declarations: vec![
                ("color".to_string(), StyleDeclarationValue::Raw("red".to_string())),
            ],
            specificity: (0, 1, 0),
            span: None,
        });
        let diags = validate_web_ir_with_tokens(&m, Some(&registry));
        assert!(
            !diags.iter().any(|d| d.code == "web_ir_validate.style.raw_literal_color"),
            "old code raw_literal_color must not appear; renamed to literal_value"
        );
        assert!(
            diags.iter().any(|d| d.code == "web_ir_validate.style.literal_value"),
            "new code literal_value must appear for named color 'red'"
        );
    }
}
