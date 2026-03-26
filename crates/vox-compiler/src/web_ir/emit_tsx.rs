//! WebIR → TSX string preview (ADR 012 target emitter stub; use for tests + future `ReactTanStackEmitter`).

use crate::codegen_ts::island_emit::escape_html_attr;
use crate::web_ir::{DomNode, DomNodeId, WebIrModule};

/// Emit JSX for a reactive component `view:` root, if present in [`WebIrModule::view_roots`].
#[must_use]
pub fn emit_component_view_tsx(module: &WebIrModule, component_name: &str) -> Option<String> {
    let root_id = module
        .view_roots
        .iter()
        .find(|(n, _)| n == component_name)
        .map(|(_, id)| *id)?;
    Some(emit_node(module, root_id, 0))
}

fn emit_node(module: &WebIrModule, id: DomNodeId, indent: usize) -> String {
    let Some(node) = module.dom_nodes.get(id.0 as usize) else {
        return String::new();
    };
    let pad = "  ".repeat(indent);
    match node {
        DomNode::Element {
            tag,
            attrs,
            children,
            ..
        } => {
            let attr_str = attrs
                .iter()
                .map(|(k, v)| format!("{k}={{{v}}}"))
                .collect::<Vec<_>>()
                .join(" ");
            let open = if attr_str.is_empty() {
                format!("<{tag}")
            } else {
                format!("<{tag} {attr_str}")
            };
            if children.is_empty() {
                return format!("{pad}{open} />\n");
            }
            let mut inner = String::new();
            for c in children {
                inner.push_str(&emit_node(module, *c, indent + 1));
            }
            format!("{pad}{open}>\n{inner}{pad}</{tag}>\n")
        }
        DomNode::Text { content, .. } => {
            let lit = serde_json::to_string(content).unwrap_or_else(|_| "\"\"".into());
            format!("{pad}{{{lit}}}\n")
        }
        DomNode::Fragment { children, .. } => {
            let mut inner = String::new();
            for c in children {
                inner.push_str(&emit_node(module, *c, indent));
            }
            format!("{pad}<>\n{inner}{pad}</>\n")
        }
        DomNode::Slot { .. } => format!("{pad}{{/* slot */ null}}\n"),
        DomNode::Conditional {
            predicate,
            then_children,
            else_children,
            ..
        } => {
            let then_s: String = then_children
                .iter()
                .map(|c| emit_node(module, *c, indent + 1))
                .collect();
            let else_s: String = else_children
                .iter()
                .map(|c| emit_node(module, *c, indent + 1))
                .collect();
            format!(
                "{pad}{{({predicate}) ? (\n{then_s}{pad}) : (\n{else_s}{pad})}}\n"
            )
        }
        DomNode::Loop {
            iterator,
            body,
            ..
        } => {
            let body_s: String = body
                .iter()
                .map(|c| emit_node(module, *c, indent + 1))
                .collect();
            format!("{pad}{{{iterator}.map(() => (\n{body_s}{pad}))}}\n")
        }
        DomNode::IslandMount {
            island_name,
            props,
            ignored_child_count,
            ..
        } => {
            let mut parts = vec![format!(
                "data-vox-island=\"{}\"",
                escape_html_attr(island_name)
            )];
            for (k, v) in props {
                parts.push(format!("{k}={{{v}}}"));
            }
            let inner = format!("<div {} />", parts.join(" "));
            if *ignored_child_count == 0 {
                format!("{pad}{inner}\n")
            } else {
                format!(
                    "{pad}<>{{/* vox: @island `{island_name}` ignores {ignored_child_count} JSX child(ren); use `<{island_name} />` */}}{inner}</>\n"
                )
            }
        }
        DomNode::Expr { ts, .. } => format!("{pad}{{{ts}}}\n"),
    }
}
