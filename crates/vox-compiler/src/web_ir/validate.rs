//! WebIR validation pass (ADR 012) — structural checks before target emitters.

use super::{DomNode, DomNodeId, WebIrDiagnostic, WebIrModule};

fn check_dom_id(out: &mut Vec<WebIrDiagnostic>, len: usize, id: DomNodeId, ctx: &str) -> bool {
    if (id.0 as usize) >= len {
        out.push(WebIrDiagnostic {
            code: "web_ir_dom_id_oob".to_string(),
            message: format!("{ctx}: DomNodeId({}) out of range (len {len})", id.0),
            span: None,
        });
        return false;
    }
    true
}

fn walk_dom_edges(out: &mut Vec<WebIrDiagnostic>, module: &WebIrModule, id: DomNodeId) {
    let len = module.dom_nodes.len();
    if !check_dom_id(out, len, id, "walk") {
        return;
    }
    let Some(node) = module.dom_nodes.get(id.0 as usize) else {
        return;
    };
    let child_ids: Vec<DomNodeId> = match node {
        DomNode::Element { children, .. } | DomNode::Fragment { children, .. } => {
            children.clone()
        }
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
        walk_dom_edges(out, module, c);
    }
}

/// Run structural checks that should hold before any target emitter.
pub fn validate_web_ir(module: &WebIrModule) -> Vec<WebIrDiagnostic> {
    let mut out = Vec::new();

    if module.dom_nodes.len() > 1_000_000 {
        out.push(WebIrDiagnostic {
            code: "web_ir_dom_arena_too_large".to_string(),
            message: "dom node arena exceeds implementation limit".to_string(),
            span: None,
        });
    }

    for (name, root) in &module.view_roots {
        if !check_dom_id(
            &mut out,
            module.dom_nodes.len(),
            *root,
            &format!("view root '{name}'"),
        ) {
            continue;
        }
        walk_dom_edges(&mut out, module, *root);
    }

    out
}
