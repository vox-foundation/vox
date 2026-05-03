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
    emit_hir_expr, emit_hir_expr_attr_value, expand_bind_hir_attribute, map_hir_type_to_ts,
    map_jsx_attr_name, map_jsx_tag,
};
use crate::codegen_ts::island_emit::island_data_prop_attr;
use crate::hir::{
    HirPattern, HirReactiveMember, HirEndpointFn, HirEndpointKind, HirStmt, HirModule, HirExpr,
    HirJsxElement, HirJsxSelfClosing, HirJsxAttr, HirParam,
};
use crate::web_ir::{
    BehaviorNode, DomNode, DomNodeId, FieldOptionality, MutationContract, RouteContract, RouteNode,
    ScheduledJobSpec, ServerFnContract, StyleDeclarationValue, StyleNode, StyleSelector,
    WebIrDiagnostic, WebIrLowerSummary, WebIrModule, WebIrVersion,
};

fn hir_pattern_binding_names(pat: &HirPattern, out: &mut HashSet<String>) {
    match pat {
        HirPattern::Ident(n, _) => {
            out.insert(n.clone());
        }
        HirPattern::Tuple(items, _) => {
            for p in items {
                hir_pattern_binding_names(p, out);
            }
        }
        HirPattern::Constructor(_, items, _) => {
            for p in items {
                hir_pattern_binding_names(p, out);
            }
        }
        HirPattern::Wildcard(_) | HirPattern::Literal(_, _) => {}
    }
}

fn collect_hir_stmt_binding_names(s: &HirStmt, out: &mut HashSet<String>) {
    match s {
        HirStmt::Let { pattern, .. } => hir_pattern_binding_names(pattern, out),
        HirStmt::While { body, .. } | HirStmt::Loop { body, .. } => {
            for x in body {
                collect_hir_stmt_binding_names(x, out);
            }
        }
        _ => {}
    }
}

fn reactive_component_name_set_for_web_ir(
    rc: &crate::hir::HirReactiveComponent,
) -> HashSet<String> {
    let mut names = HashSet::new();
    for mem in &rc.members {
        match mem {
            HirReactiveMember::State(s) => {
                names.insert(s.name.clone());
            }
            HirReactiveMember::Stmt(st) => collect_hir_stmt_binding_names(st, &mut names),
            _ => {}
        }
    }
    names
}

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
            HirExpr::JsxFragment(children, _) => {
                let child_ids: Vec<DomNodeId> = children
                    .iter()
                    .map(|c| self.lower_expr(c, state_names, island_names))
                    .collect();
                self.push(DomNode::Fragment {
                    children: child_ids,
                    span: None,
                })
            }
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
        let mut attrs: Vec<(String, String)> = Vec::new();
        for attr in &el.attributes {
            attrs.extend(lower_jsx_attr_pair(attr, state_names, island_names));
        }
        // TASK-6.1: resolve primitive tags → canonical HTML tag + Tailwind class list.
        let mapped_tag = map_jsx_tag(&el.tag);
        let (tag, attrs) = apply_primitive_emission(mapped_tag, attrs);
        let child_ids: Vec<DomNodeId> = el
            .children
            .iter()
            .map(|c| self.lower_expr(c, state_names, island_names))
            .collect();
        self.push(DomNode::Element {
            id: DomNodeId(0),
            tag,
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
        let mut attrs: Vec<(String, String)> = Vec::new();
        for attr in &el.attributes {
            attrs.extend(lower_jsx_attr_pair(attr, state_names, island_names));
        }
        // TASK-6.1: resolve primitive tags.
        let mapped_tag = map_jsx_tag(&el.tag);
        let (tag, attrs) = apply_primitive_emission(mapped_tag, attrs);
        self.push(DomNode::Element {
            id: DomNodeId(0),
            tag,
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
/// Props that are consumed by the primitive resolver and must not appear on the final HTML element.
const PRIMITIVE_CONSUMED_PROPS: &[&str] = &[
    "gap", "size", "weight", "align", "wrap", "variant", "level", "surface",
    "z", "position",
];

/// TASK-6.1: if `tag` is a known primitive, replace it with the canonical HTML tag and inject
/// the primitive's Tailwind class list into the `className` attribute (merging with any existing
/// `class` / `className` attr from the author).  Returns `(html_tag, final_attrs)`.
fn apply_primitive_emission(
    tag: &str,
    mut attrs: Vec<(String, String)>,
) -> (String, Vec<(String, String)>) {
    // JSX string literals arrive with surrounding quotes ("\"value\""); strip them for prop resolution.
    let unquoted: Vec<(String, String)> = attrs
        .iter()
        .map(|(k, v)| {
            let v = v.trim_matches('"').trim_matches('\'');
            (k.clone(), v.to_string())
        })
        .collect();
    let Some(emission) = super::primitives::resolve(tag, &unquoted) else {
        return (tag.to_string(), attrs);
    };
    // Remove primitive-specific props that are consumed by the resolver.
    attrs.retain(|(k, _)| !PRIMITIVE_CONSUMED_PROPS.contains(&k.as_str()));
    let base = emission.class_string();
    if !base.is_empty() {
        // Merge with any existing class / className attr from the author.
        if let Some(pos) = attrs
            .iter()
            .position(|(k, _)| k == "className" || k == "class")
        {
            let existing = attrs[pos].1.clone();
            attrs[pos].1 = format!("{base} {existing}");
        } else {
            attrs.push(("className".to_string(), base));
        }
    }
    if let Some(role) = emission.aria_role {
        if !attrs.iter().any(|(k, _)| k == "role") {
            attrs.push(("role".to_string(), role.to_string()));
        }
    }
    // TASK-6.3: surface pair — inject CSS vars and a data-vox-surface attr for validation.
    if let Some(surface) = &emission.surface_ref {
        attrs.push(("data-vox-surface".to_string(), surface.clone()));
        let style_val = format!(
            "--fg: var(--vox-surface-{surface}-fg); --bg: var(--vox-surface-{surface}-bg)"
        );
        if let Some(pos) = attrs.iter().position(|(k, _)| k == "style") {
            let existing = attrs[pos].1.clone();
            attrs[pos].1 = format!("{style_val}; {existing}");
        } else {
            attrs.push(("style".to_string(), style_val));
        }
    }
    // TASK-6.4: overlay markers — add data-vox-overlay, data-vox-z, data-vox-pos for validator.
    match tag {
        "overlay" => {
            attrs.push(("data-vox-overlay".to_string(), "true".to_string()));
        }
        "toast" | "drawer" | "modal" => {
            if let Some(z_val) = unquoted.iter().find(|(k, _)| k == "z").map(|(_, v)| v.clone()) {
                attrs.push(("data-vox-z".to_string(), z_val));
            }
            if let Some(pos_val) = unquoted.iter().find(|(k, _)| k == "position").map(|(_, v)| v.clone()) {
                attrs.push(("data-vox-pos".to_string(), pos_val));
            }
        }
        _ => {}
    }
    (emission.html_tag.to_string(), attrs)
}

/// handler bodies stay as stringified TS expressions. Dedicated [`BehaviorNode::EventHandler`] rows are
/// reserved for future binding tables — Phase 1 keeps behavior on the DOM edge for parity with `hir_emit`.
///
/// `bind={…}` expands to `value` + `onChange` like [`crate::codegen_ts::jsx::expand_bind_attribute`].
fn lower_jsx_attr_pair(
    attr: &HirJsxAttr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> Vec<(String, String)> {
    if attr.name == "bind" {
        let (value_str, onchange_str) =
            expand_bind_hir_attribute(&attr.value, state_names, island_names);
        return vec![
            ("value".to_string(), value_str),
            ("onChange".to_string(), onchange_str),
        ];
    }
    let name = map_jsx_attr_name(&attr.name).to_string();
    let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &name);
    vec![(name, val)]
}

fn lower_route_contract_entry(
    e: &crate::ast::decl::RouteEntry,
    parent_id: &str,
    idx: usize,
) -> RouteContract {
    let id = if parent_id.is_empty() {
        format!("route_{idx}")
    } else {
        format!("{parent_id}_c{idx}")
    };
    let mut meta = json!({ "component": e.component_name });
    if let Some(l) = &e.loader_name {
        meta["loader"] = json!(l.clone());
    }
    if let Some(p) = &e.pending_component_name {
        meta["pending"] = json!(p.clone());
    }
    let children: Vec<RouteContract> = e
        .children
        .iter()
        .enumerate()
        .map(|(i, c)| lower_route_contract_entry(c, &id, i))
        .collect();
    RouteContract {
        id,
        pattern: e.path.clone(),
        meta,
        children,
    }
}



fn lower_client_routes(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    for rd in &hir.client_routes {
        let routes: Vec<RouteContract> = rd
            .entries
            .iter()
            .enumerate()
            .map(|(i, e)| lower_route_contract_entry(e, "", i))
            .collect();
        m.route_nodes.push(RouteNode::RouteTree {
            routes,
            span: None,
        });
        summary.client_route_trees += 1;
    }
}

fn qualify(component: &str, name: &str) -> String {
    format!("{component}::{name}")
}

fn fn_signature_for_contract(sf: &HirEndpointFn) -> String {
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

fn mutation_payload_type(sf: &HirEndpointFn) -> String {
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

fn parse_style_selector(s: &str) -> StyleSelector {
    let s = s.trim();
    if s.is_empty() {
        return StyleSelector::Unparsed(s.to_string());
    }

    if s.contains(' ') || s.contains('>') || s.contains('+') || s.contains('~') {
        return StyleSelector::Unparsed(s.to_string());
    }

    if s.contains(':') {
        let parts: Vec<&str> = s.splitn(2, ':').collect();
        if parts.len() == 2 {
            let base = parse_style_selector(parts[0]);
            return StyleSelector::Pseudo {
                base: Box::new(base),
                pseudo: parts[1].to_string(),
            };
        }
    }

    if let Some(rest) = s.strip_prefix('.') {
        StyleSelector::Class(rest.to_string())
    } else if let Some(rest) = s.strip_prefix('#') {
        StyleSelector::Id(rest.to_string())
    } else {
        if s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') {
            StyleSelector::Element(s.to_string())
        } else {
            StyleSelector::Unparsed(s.to_string())
        }
    }
}

fn compute_specificity(sel: &StyleSelector) -> (u8, u8, u8) {
    match sel {
        StyleSelector::Id(_) => (1, 0, 0),
        StyleSelector::Class(_) => (0, 1, 0),
        StyleSelector::Element(_) => (0, 0, 1),
        StyleSelector::Pseudo { base, pseudo } => {
            let (a, b, c) = compute_specificity(base);
            if pseudo.starts_with(':') {
                (a, b, c + 1)
            } else {
                (a, b + 1, c)
            }
        }
        StyleSelector::Compound(parts) => parts.iter().fold((0, 0, 0), |(a1, b1, c1), p| {
            let (a2, b2, c2) = compute_specificity(p);
            (a1 + a2, b1 + b2, c1 + c2)
        }),
        StyleSelector::Unparsed(_) => (0, 0, 0),
    }
}

fn parse_css_value(prop: &str, val: &str) -> StyleDeclarationValue {
    use crate::web_ir::{CssColor, LengthUnit};
    let val = val.trim();

    if val.starts_with("tokens.") {
        let token_name = val.strip_prefix("tokens.").unwrap().replace('.', "-");
        return StyleDeclarationValue::TokenRef(format!("vox-{}", token_name));
    }

    if prop.ends_with("color") || prop == "background" || prop == "fill" || prop == "stroke" {
        if val.starts_with('#') {
            return StyleDeclarationValue::Color(CssColor::Hex(val.to_string()));
        } else if val.starts_with("rgb(") || val.starts_with("rgba(") {
            return StyleDeclarationValue::Color(CssColor::Rgba(0, 0, 0, 1.0));
        } else if val.starts_with("hsl(") || val.starts_with("hsla(") {
            return StyleDeclarationValue::Color(CssColor::Hsl(0.0, 0.0, 0.0));
        } else if val.starts_with("var(") {
            return StyleDeclarationValue::Color(CssColor::Var(val.to_string()));
        } else if val.chars().all(|c| c.is_ascii_alphabetic()) {
            return StyleDeclarationValue::Color(CssColor::Named(val.to_string()));
        }
    }

    if val.ends_with("px")
        || val.ends_with("rem")
        || val.ends_with("em")
        || val.ends_with("%")
        || val.ends_with("vw")
        || val.ends_with("vh")
    {
        let (num_str, unit) = if let Some(stripped) = val.strip_suffix("px") {
            (stripped, LengthUnit::Px)
        } else if let Some(stripped) = val.strip_suffix("rem") {
            (stripped, LengthUnit::Rem)
        } else if let Some(stripped) = val.strip_suffix("em") {
            (stripped, LengthUnit::Em)
        } else if let Some(stripped) = val.strip_suffix("%") {
            (stripped, LengthUnit::Percent)
        } else if let Some(stripped) = val.strip_suffix("vw") {
            (stripped, LengthUnit::Vw)
        } else if let Some(stripped) = val.strip_suffix("vh") {
            (stripped, LengthUnit::Vh)
        } else {
            (val, LengthUnit::Px)
        };

        if let Ok(num) = num_str.parse::<f64>() {
            return StyleDeclarationValue::Length(num, unit);
        }
    }

    if let Ok(num) = val.parse::<f64>() {
        return StyleDeclarationValue::Number(num);
    }

    StyleDeclarationValue::Raw(val.to_string())
}

fn lower_styles_from_classic_components(
    hir: &HirModule,
    m: &mut WebIrModule,
    summary: &mut WebIrLowerSummary,
) {
    let push_blocks = |blocks: &[crate::ast::decl::fundecl::StyleBlock],
                       m: &mut WebIrModule,
                       summary: &mut WebIrLowerSummary| {
        for block in blocks {
            let declarations: Vec<(String, StyleDeclarationValue)> = block
                .properties
                .iter()
                .map(|(prop, val)| (prop.clone(), parse_css_value(prop, val)))
                .collect();
            let selector = parse_style_selector(&block.selector);
            let specificity = compute_specificity(&selector);
            m.style_nodes.push(StyleNode::Rule {
                selector,
                declarations,
                specificity,
                is_raw_css: block.is_raw_css,
                span: None,
            });
            summary.style_rules_lowered += 1;
        }
    };


    for rc in &hir.components {
        push_blocks(&rc.styles, m, summary);
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

fn lower_endpoint_contracts(
    hir: &HirModule,
    m: &mut WebIrModule,
    summary: &mut WebIrLowerSummary,
) {
    for sf in &hir.endpoint_fns {
        match sf.kind {
            HirEndpointKind::Server | HirEndpointKind::Query => {
                m.route_nodes
                    .push(RouteNode::ServerFnContract(ServerFnContract {
                        name: sf.name.clone(),
                        export_path: sf.route_path.clone(),
                        signature: fn_signature_for_contract(sf),
                        span: None,
                    }));
                if sf.kind == HirEndpointKind::Server {
                    summary.server_fn_contracts += 1;
                } else {
                    summary.query_fn_contracts += 1;
                }
            }
            HirEndpointKind::Mutation => {
                m.route_nodes
                    .push(RouteNode::MutationContract(MutationContract {
                        name: sf.name.clone(),
                        payload_type: mutation_payload_type(sf),
                        span: None,
                    }));
                summary.mutation_contracts += 1;
            }
        }
    }
}

fn lower_scheduled_jobs(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    for f in &hir.functions {
        if let Some(interval) = f.schedule_interval.clone() {
            m.scheduled_jobs.push(ScheduledJobSpec {
                name: f.name.clone(),
                interval,
                span: None,
            });
            summary.scheduled_jobs_lowered += 1;
        }
    }
}

fn note_lowering_gaps(hir: &HirModule, m: &mut WebIrModule, summary: &mut WebIrLowerSummary) {
    summary.classic_components_deferred = 0;
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



    // Stage R — client `routes { }` blocks + HTTP handlers + RPC-shaped endpoints from HIR
    lower_client_routes(hir, &mut m, &mut summary);
    lower_http_routes(hir, &mut m, &mut summary);
    lower_endpoint_contracts(hir, &mut m, &mut summary);
    lower_scheduled_jobs(hir, &mut m, &mut summary);

    // Stage S — classic `@component` scoped CSS (AST-retained)
    lower_styles_from_classic_components(hir, &mut m, &mut summary);

    let mut arena = DomArena::new();

    // Stage B + D — Path C reactive components
    summary.components = hir.components.len();
    for rc in &hir.components {
        let state_names = reactive_component_name_set_for_web_ir(rc);

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
                HirReactiveMember::Stmt(_) => {}
            }
        }

        if let Some(view) = &rc.view {
            let root = arena.lower_expr(view, &state_names, &island_names);
            m.view_roots.push((rc.name.clone(), root));
        }
    }



    summary.dom_expr_fallbacks = arena.expr_fallback_count;
    m.dom_nodes = arena.nodes;

    note_lowering_gaps(hir, &mut m, &mut summary);

    accumulate_route_manifest_summary(hir, &mut summary);

    (m, summary)
}

fn accumulate_route_manifest_summary(_hir: &HirModule, _summary: &mut WebIrLowerSummary) {
    use crate::ast::decl::RouteEntry;

    #[allow(dead_code)]
    fn walk_entry(e: &RouteEntry, loaders: &mut usize, pending: &mut usize) {
        if e.loader_name.is_some() {
            *loaders += 1;
        }
        if e.pending_component_name.is_some() {
            *pending += 1;
        }
        for c in &e.children {
            walk_entry(c, loaders, pending);
        }
    }


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
