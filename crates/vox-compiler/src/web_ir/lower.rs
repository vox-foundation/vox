//! Lower [`crate::hir::HirModule`] into [`super::WebIrModule`] (ADR 012 Phase 1).

use std::collections::HashSet;

use serde_json::json;

use crate::codegen_ts::hir_emit::{emit_hir_expr, emit_hir_expr_attr_value, map_jsx_attr_name};
use crate::codegen_ts::island_emit::island_data_prop_attr;
use crate::hir::{
    HirExpr, HirJsxAttr, HirJsxElement, HirJsxSelfClosing, HirModule, HirReactiveMember, HirRoutes,
};
use crate::web_ir::{
    BehaviorNode, DomNode, DomNodeId, FieldOptionality, RouteContract, RouteNode, WebIrModule,
    WebIrVersion,
};

struct DomArena {
    nodes: Vec<DomNode>,
}

impl DomArena {
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
            return self.lower_island(&el.tag, &el.attributes, el.children.len(), state_names, island_names);
        }
        let mut attrs = Vec::new();
        for attr in &el.attributes {
            let name = map_jsx_attr_name(&attr.name).to_string();
            let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &name);
            attrs.push((name, val));
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
            let name = map_jsx_attr_name(&attr.name).to_string();
            let val = emit_hir_expr_attr_value(&attr.value, state_names, island_names, &name);
            attrs.push((name, val));
        }
        self.push(DomNode::Element {
            id: DomNodeId(0),
            tag: el.tag.clone(),
            attrs,
            children: vec![],
            span: None,
        })
    }

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

/// Build a [`WebIrModule`] from lowered HIR (reactive views + `routes:` contracts + behaviors).
#[must_use]
pub fn lower_hir_to_web_ir(hir: &HirModule) -> WebIrModule {
    let island_names = crate::codegen_ts::island_emit::collect_island_names(hir);

    let mut m = WebIrModule {
        version: WebIrVersion::V0_1,
        ..Default::default()
    };

    for r in &hir.client_routes {
        m.route_nodes.push(lower_routes(r));
    }

    let mut arena = DomArena { nodes: Vec::new() };

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

    m.dom_nodes = arena.nodes;
    m
}

/// Lower only the `view:` expression of a single reactive component (tests / tools).
#[must_use]
pub fn lower_hir_view_expr(
    expr: &HirExpr,
    state_names: &HashSet<String>,
    island_names: &HashSet<String>,
) -> (Vec<DomNode>, DomNodeId) {
    let mut arena = DomArena { nodes: Vec::new() };
    let root = arena.lower_expr(expr, state_names, island_names);
    (arena.nodes, root)
}
