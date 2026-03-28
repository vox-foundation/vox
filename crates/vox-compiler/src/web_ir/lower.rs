//! Lower [`crate::hir::HirModule`] into [`super::WebIrModule`] (ADR 012 Phase 1).
//!
//! **Lowering stages (OP-0065):**
//! - **R (routes)** — `hir.client_routes` → [`super::RouteNode`] via [`lower_routes`]; HIR HTTP routes
//!   and RPC endpoints → [`super::RouteNode::LoaderContract`] / [`super::ServerFnContract`] /
//!   [`super::MutationContract`] (OP-0072).
//! - **S (style)** — `@component` `style { }` blocks on [`crate::hir::HirComponent`] → [`super::StyleNode::Rule`]
//!   with [`super::StyleSelector::Unparsed`] selectors (OP-0070).
//! - **B (behavior)** — reactive state/derived/effect/mount/cleanup → [`super::BehaviorNode`].
//! - **D (DOM)** — reactive `view:` [`HirExpr`] → [`super::DomNode`] arena + [`super::WebIrModule::view_roots`].
//!
//! **JSX attributes (OP-0068):** element attributes use [`crate::codegen_ts::hir_emit::map_jsx_attr_name`]
//! so Vox spellings (`on_click`, `on:click`, `class`) match TS emit (`onClick`, `className`).
//!
//! **Islands (OP-0066):** [`DomArena::lower_island`] follows the same `data-prop-*` naming as
//! [`crate::codegen_ts::island_emit`] / `hir_emit` so mounts stay consistent across TS and WebIR.
//!
//! **AST `HirComponent` (OP-0179):** JSX-shaped classic `@component fn` bodies lower into
//! [`WebIrModule::view_roots`] using [`crate::hir::lower_classic_component_view`]. Components that
//! do not end in a supported JSX tail remain counted in [`WebIrLowerSummary::classic_components_deferred`].
//!
//! ## Blueprint batch OP-S057 / S085 / S133 / S155 / S189 / S052-route-style (supplemental)
//! Keep style-only TODOs inside [`lower_styles_from_classic_components`] and HTTP/client route splits inside
//! [`lower_http_routes`] / [`lower_routes`] — do not add parallel style string emit here. Interop hatch lowering
//! stays denormalized into [`crate::web_ir::InteropNode`] when introduced; route contract ids must match
//! validate-stage uniqueness.

use std::collections::HashSet;

use serde_json::json;

use crate::codegen_ts::hir_emit::{
    emit_hir_expr, emit_hir_expr_attr_value, map_hir_type_to_ts, map_jsx_attr_name,
};
use crate::codegen_ts::island_emit::island_data_prop_attr;
use crate::hir::{
    HirComponent, HirExpr, HirJsxAttr, HirJsxElement, HirJsxSelfClosing, HirModule, HirParam,
    HirReactiveMember, HirRoutes, HirServerFn,
};
use crate::web_ir::{
    BehaviorNode, DomNode, DomNodeId, FieldOptionality, MutationContract, RouteContract, RouteNode,
    ServerFnContract, StyleDeclarationValue, StyleNode, StyleSelector, WebIrDiagnostic,
    WebIrLowerSummary, WebIrModule, WebIrVersion,
};

struct DomArena {
    nodes: Vec<DomNode>,
    expr_fallback_count: usize,
}

impl DomArena {
    fn new() -> Self {
        Self {
            nodes: Vec::new(),
            expr_fallback_count: 0,
        }
    }

    fn push(&mut self, node: DomNode) -> DomNodeId {
        let id = DomNodeId(self.nodes.len() as u32);
        // Patch Element id field to match arena index.
        let node = match node {
            DomNode::Element {
                tag,
                attrs,
                children,
                span,
                ..
            } => DomNode::Element {
                id,
                tag,
                attrs,
                children,
                span,
            },
            _ => node,
        };
        self.nodes.push(node);
        id
    }

    fn lower_expr(
        &mut self,
        expr: &HirExpr,
        state_names: &HashSet<String>,
        island_names: &HashSet<String>,
    ) -> DomNodeId {
        match expr {
            HirExpr::Jsx(el) => self.lower_jsx_el(el, state_names, island_names),
            HirExpr::JsxSelfClosing(el) => self.lower_jsx_self(el, state_names, island_names),
            HirExpr::StringLit(s, _) => self.push(DomNode::Text {
                content: s.clone(),
                span: None,
            }),
            _ => {
                self.expr_fallback_count += 1;
                let ts = emit_hir_expr(expr, state_names, island_names);
                self.push(DomNode::Expr { ts, span: None })
            }
        }
    }

    fn lower_jsx_el(
        &mut self,
        el: &HirJsxElement,
        state_names: &HashSet<String>,
        island_names: &HashSet<String>,
    ) -> DomNodeId {
        if island_names.contains(&el.tag) {
            return self.lower_island(
                &el.tag,
                &el.attributes,
                el.children.len(),
                state_names,
                island_names,
            );
        }
        let mut attrs = Vec::new();
        for attr in &el.attributes {
            attrs.push(lower_jsx_attr_pair(attr, state_names, island_names));
        }
        let child_ids: Vec<DomNodeId> = el
            .children
            .iter()
            .map(|c| self.lower_expr(c, state_names, island_names))
            .collect();
        self.push(DomNode::Element {
            id: DomNodeId(0),
            tag: el.tag.clone(),
            attrs,
            children: child_ids,
            span: None,
        })
    }

    fn lower_jsx_self(
        &mut self,
        el: &HirJsxSelfClosing,
        state_names: &HashSet<String>,
        island_names: &HashSet<String>,
    ) -> DomNodeId {
        if island_names.contains(&el.tag) {
            return self.lower_island(&el.tag, &el.attributes, 0, state_names, island_names);
        }
        let mut attrs = Vec::new();
        for attr in &el.attributes {
            attrs.push(lower_jsx_attr_pair(attr, state_names, island_names));
        }
        self.push(DomNode::Element {
            id: DomNodeId(0),
            tag: el.tag.clone(),
            attrs,
            children: vec![],
            span: None,
        })
    }

    /// **Island branch (OP-S013):** when JSX `tag` is in `island_names`, skip normal [`DomNode::Element`]
    /// emission and produce [`DomNode::IslandMount`]. Non-`bind` attrs map through [`island_data_prop_attr`]
    /// so lowered keys match runtime `data-prop-*`; `ignored_child_count` records stripped children for hydration parity.
    fn lower_island(
        &mut self,
        tag: &str,
        attributes: &[HirJsxAttr],
        child_count: usize,
        state_names: &HashSet<String>,
        island_names: &HashSet<String>,
    ) -> DomNodeId {
        let mut props = Vec::new();
        for attr in attributes {
            if attr.name == "bind" {
                continue;
            }
            let dname = island_data_prop_attr(&attr.name);
            let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &dname);
            props.push((dname, val));
        }
        self.push(DomNode::IslandMount {
            island_name: tag.to_string(),
            props,
            ignored_child_count: child_count as u32,
            span: None,
        })
    }
}

/// Map JSX attribute name + value the same way as TS `hir_emit` (OP-S015).
///
/// Event spellings (`on_click`, `on:click`) become React-style `onClick` names on [`DomNode::Element`];
/// handler bodies stay as stringified TS expressions. Dedicated [`BehaviorNode::EventHandler`] rows are
/// reserved for future binding tables — Phase 1 keeps behavior on the DOM edge for parity with `hir_emit`.
fn lower_jsx_attr_pair(
    attr: &HirJsxAttr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> (String, String) {
    let name = map_jsx_attr_name(&attr.name).to_string();
    let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &name);
    (name, val)
}

fn lower_routes(routes: &HirRoutes) -> RouteNode {
    let contracts: Vec<RouteContract> = routes
        .0
        .entries
        .iter()
        .enumerate()
        .map(|(i, e)| RouteContract {
            id: format!("route_{i}"),
            pattern: e.path.clone(),
            meta: json!({ "component": e.component_name }),
        })
        .collect();
    RouteNode::RouteTree {
        routes: contracts,
        span: None,
    }
}

fn qualify(component: &str, name: &str) -> String {
    format!("{component}::{name}")
}

fn fn_signature_for_contract(sf: &HirServerFn) -> String {
    let params: Vec<String> = sf.params.iter().map(param_type_annotation).collect();
    let ret = sf
        .return_type
        .as_ref()
        .map(map_hir_type_to_ts)
        .unwrap_or_else(|| "void".to_string());
    format!("({}) -> {}", params.join(", "), ret)
}

fn param_type_annotation(p: &HirParam) -> String {
    let ty = p
        .type_ann
        .as_ref()
        .map(map_hir_type_to_ts)
        .unwrap_or_else(|| "unknown".to_string());
    format!("{}: {}", p.name, ty)
}

fn mutation_payload_type(sf: &HirServerFn) -> String {
    sf.params
        .first()
        .map(param_type_annotation)
        .unwrap_or_else(|| "void".to_string())
}

fn slug_path_segment(p: &str) -> String {
    let t = p.trim_matches('/');
    if t.is_empty() {
        "root".to_string()
    } else {
        t.chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect()
    }
}

fn lower_styles_from_classic_components(
    hir: &HirModule,
    m: &mut WebIrModule,
    summary: &mut WebIrLowerSummary,
) {
    for HirComponent(decl) in &hir.components {
        for block in &decl.styles {
            let declarations: Vec<(String, StyleDeclarationValue)> = block
                .properties
                .iter()
                .map(|(prop, val)| (prop.clone(), StyleDeclarationValue::Raw(val.clone())))
                .collect();
            m.style_nodes.push(StyleNode::Rule {
                selector: StyleSelector::Unparsed(block.selector.clone()),
                declarations,
                span: None,
            });
            summary.style_rules_lowered += 1;
        }
    }
}

fn lower_http_routes(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    for (i, r) in hir.routes.iter().enumerate() {
        let slug = slug_path_segment(&r.path);
        let route_id = format!("http_{i}_{slug}");
        let return_ty = r
            .return_type
            .as_ref()
            .map(map_hir_type_to_ts)
            .unwrap_or_else(|| "void".to_string());
        let contract = json!({
            "kind": "http",
            "method": r.method.as_str(),
            "path": r.path,
            "route_contract": r.route_contract,
            "return_type": return_ty,
        })
        .to_string();
        m.route_nodes.push(RouteNode::LoaderContract {
            route_id,
            contract,
            span: None,
        });
        summary.http_loader_contracts += 1;
    }
}

fn lower_server_fn_contracts(
    hir: &HirModule,
    m: &mut WebIrModule,
    summary: &mut WebIrLowerSummary,
) {
    for sf in &hir.server_fns {
        m.route_nodes
            .push(RouteNode::ServerFnContract(ServerFnContract {
                name: sf.name.clone(),
                export_path: sf.route_path.clone(),
                signature: fn_signature_for_contract(sf),
                span: None,
            }));
        summary.server_fn_contracts += 1;
    }
}

fn lower_query_fn_contracts(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    for qf in &hir.query_fns {
        m.route_nodes
            .push(RouteNode::ServerFnContract(ServerFnContract {
                name: qf.name.clone(),
                export_path: qf.route_path.clone(),
                signature: fn_signature_for_contract(qf),
                span: None,
            }));
        summary.query_fn_contracts += 1;
    }
}

fn lower_mutation_contracts(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    for mf in &hir.mutation_fns {
        m.route_nodes
            .push(RouteNode::MutationContract(MutationContract {
                name: mf.name.clone(),
                payload_type: mutation_payload_type(mf),
                span: None,
            }));
        summary.mutation_contracts += 1;
    }
}

fn note_lowering_gaps(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    summary.classic_components_deferred = hir
        .components
        .len()
        .saturating_sub(summary.classic_component_views_lowered);
    if !hir.legacy_ast_nodes.is_empty() {
        m.diagnostic_nodes.push(WebIrDiagnostic {
            code: "web_ir.lower.unlowered_ast_decls".to_string(),
            message: format!(
                "{} declaration(s) remain in HIR legacy_ast_nodes (not represented in typed WebIR vectors)",
                hir.legacy_ast_nodes.len()
            ),
            span: None,
            category: Some("lower".to_string()),
        });
        summary.lowering_diagnostics += 1;
    }
}

/// Build a [`WebIrModule`] from lowered HIR (reactive views + `routes:` contracts + behaviors)
/// and return structural counts for gates (OP-0078).
#[must_use]
pub fn lower_hir_to_web_ir_with_summary(hir: &HirModule) -> (WebIrModule, WebIrLowerSummary) {
    let island_names = crate::codegen_ts::island_emit::collect_island_names(hir);

    let mut m = WebIrModule {
        version: WebIrVersion::V0_1,
        ..Default::default()
    };

    let mut summary = WebIrLowerSummary::default();

    // Stage R — TanStack client route trees
    for r in &hir.client_routes {
        m.route_nodes.push(lower_routes(r));
        summary.client_route_trees += 1;
    }

    // Stage R — HTTP handlers and RPC-shaped endpoints from HIR
    lower_http_routes(hir, &mut m, &mut summary);
    lower_server_fn_contracts(hir, &mut m, &mut summary);
    lower_query_fn_contracts(hir, &mut m, &mut summary);
    lower_mutation_contracts(hir, &mut m, &mut summary);

    // Stage S — classic `@component` scoped CSS (AST-retained)
    lower_styles_from_classic_components(hir, &mut m, &mut summary);

    let mut arena = DomArena::new();

    // Stage B + D — Path C reactive components
    summary.reactive_components = hir.reactive_components.len();
    for rc in &hir.reactive_components {
        let state_names: HashSet<String> = rc
            .members
            .iter()
            .filter_map(|mem| match mem {
                HirReactiveMember::State(s) => Some(s.name.clone()),
                _ => None,
            })
            .collect();

        for mem in &rc.members {
            match mem {
                HirReactiveMember::State(s) => {
                    let initial = emit_hir_expr(&s.init, &state_names, &island_names);
                    m.behavior_nodes.push(BehaviorNode::StateDecl {
                        name: qualify(&rc.name, &s.name),
                        initial: Some(initial),
                        optionality: FieldOptionality::Required,
                        span: None,
                    });
                }
                HirReactiveMember::Derived(d) => {
                    let expr = emit_hir_expr(&d.expr, &state_names, &island_names);
                    m.behavior_nodes.push(BehaviorNode::DerivedDecl {
                        name: qualify(&rc.name, &d.name),
                        expr,
                        span: None,
                    });
                }
                HirReactiveMember::Effect(e) => {
                    let body = emit_hir_expr(&e.body, &state_names, &island_names);
                    m.behavior_nodes.push(BehaviorNode::EffectDecl {
                        deps: vec![],
                        body,
                        span: None,
                    });
                }
                HirReactiveMember::OnMount(om) => {
                    let body = emit_hir_expr(&om.body, &state_names, &island_names);
                    m.behavior_nodes.push(BehaviorNode::EffectDecl {
                        deps: vec![qualify(&rc.name, "mount")],
                        body,
                        span: None,
                    });
                }
                HirReactiveMember::OnCleanup(oc) => {
                    let body = emit_hir_expr(&oc.body, &state_names, &island_names);
                    m.behavior_nodes.push(BehaviorNode::EffectDecl {
                        deps: vec![qualify(&rc.name, "cleanup")],
                        body,
                        span: None,
                    });
                }
            }
        }

        if let Some(view) = &rc.view {
            let root = arena.lower_expr(view, &state_names, &island_names);
            m.view_roots.push((rc.name.clone(), root));
        }
    }

    for comp in &hir.components {
        if let Some((view, state_names)) = crate::hir::lower_classic_component_view(comp) {
            let root = arena.lower_expr(&view, &state_names, &island_names);
            m.view_roots.push((comp.0.func.name.clone(), root));
            summary.classic_component_views_lowered += 1;
        }
    }

    summary.dom_expr_fallbacks = arena.expr_fallback_count;
    m.dom_nodes = arena.nodes;

    note_lowering_gaps(hir, &mut m, &mut summary);

    (m, summary)
}

/// Build a [`WebIrModule`] from lowered HIR (reactive views + `routes:` contracts + behaviors).
#[must_use]
pub fn lower_hir_to_web_ir(hir: &HirModule) -> WebIrModule {
    lower_hir_to_web_ir_with_summary(hir).0
}

/// Project web IR from typed core ([`crate::hir::TypedCoreIR_v2`]) — alias for [`lower_hir_to_web_ir`].
#[must_use]
pub fn project_web_from_core(hir: &crate::hir::TypedCoreIR_v2) -> super::WebProjectionIR {
    lower_hir_to_web_ir(hir)
}

/// Lower only the `view:` expression of a single reactive component (tests / tools).
#[must_use]
pub fn lower_hir_view_expr(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> (Vec<DomNode>, DomNodeId) {
    let mut arena = DomArena::new();
    let root = arena.lower_expr(expr, state_names, island_names);
    (arena.nodes, root)
}
