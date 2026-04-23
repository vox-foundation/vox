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
    StyleNode, StyleSelector, WebIrDiagnostic, WebIrModule, WebIrValidateMetrics,
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
                    });
                }
                if route_id.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_loader_id".to_string(),
                        message: "LoaderContract.route_id must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    });
                }
                if contract.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_loader_contract".to_string(),
                        message: "LoaderContract.contract must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
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
                    });
                }
                if s.export_path.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_server_export_path".to_string(),
                        message: "ServerFnContract.export_path must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
                    });
                }
                if s.signature.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_server_signature".to_string(),
                        message: "ServerFnContract.signature must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
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
                    });
                }
                if m.payload_type.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.route.empty_mutation_payload_type".to_string(),
                        message: "MutationContract.payload_type must not be empty".to_string(),
                        span: None,
                        category: Some("route".to_string()),
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
                        });
                    }

                    if let Some(existing_props) = seen_selectors.get(&sel_key) {
                        if existing_props.contains(&css_prop) {
                            out.push(WebIrDiagnostic {
                                code: "web_ir_validate.style.specificity_conflict".to_string(),
                                message: format!("Property '{}' redefined for selector '{}' at same specificity level", prop, sel_key),
                                span: None,
                                category: Some("style".to_string()),
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
            });
        }
        if job.interval.trim().is_empty() {
            out.push(WebIrDiagnostic {
                code: "web_ir_validate.scheduled.empty_interval".to_string(),
                message: "ScheduledJobSpec.interval must not be empty".to_string(),
                span: None,
                category: Some("scheduled".to_string()),
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
                    });
                }
                if import_source.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_import_source".to_string(),
                        message: "ReactComponentRef.import_source must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
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
                    });
                }
                if reason.is_empty() {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.interop.empty_escape_reason".to_string(),
                        message: "EscapeHatchExpr.reason must not be empty".to_string(),
                        span: None,
                        category: Some("interop".to_string()),
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
