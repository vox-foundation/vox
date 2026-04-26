//! TASK-5.3 — Accessibility validator (a11y).
//!
//! Checks structural accessibility rules against the DOM arena:
//!
//! - `<img>` without `alt` or `aria-hidden="true"` → error.
//! - `<button>` without accessible name (text child / aria-label / aria-labelledby) → error.
//! - `<a href=...>` without accessible name → error.
//! - Any element with `role="button"` without accessible name → error.
//! - Any element with `role="button"` without a keyboard handler → error.
//!
//! All codes are under `web_ir_validate.a11y.*`.

use super::{BehaviorNode, DomNode, DomNodeId, WebIrDiagnostic, WebIrModule};

/// Run structural a11y checks on the DOM arena.
pub fn validate_a11y(module: &WebIrModule, out: &mut Vec<WebIrDiagnostic>) {
    for node in &module.dom_nodes {
        let DomNode::Element {
            id: elem_id,
            tag,
            attrs,
            children,
            ..
        } = node
        else {
            continue;
        };

        let get_attr = |name: &str| -> Option<&str> {
            attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
        };

        match tag.as_str() {
            "img" => {
                let has_alt = attrs.iter().any(|(k, _)| k == "alt");
                let aria_hidden = get_attr("aria-hidden").map_or(false, |v| v == "true");
                if !has_alt && !aria_hidden {
                    out.push(WebIrDiagnostic {
                        code: "web_ir_validate.a11y.img_missing_alt".to_string(),
                        message: "img element requires an `alt` attribute or `aria-hidden=\"true\"` for decorative images".to_string(),
                        span: None,
                        category: Some("a11y".to_string()),
                    });
                }
            }
            "button" => {
                check_accessible_name(module, tag, attrs, children, out);
            }
            "a" => {
                let has_href = attrs.iter().any(|(k, _)| k == "href" || k == "to");
                if has_href {
                    check_accessible_name(module, tag, attrs, children, out);
                }
            }
            _ => {
                if let Some(role) = get_attr("role") {
                    if role == "button" {
                        check_accessible_name(module, tag, attrs, children, out);
                        check_keyboard_handler(module, *elem_id, out);
                    }
                }
            }
        }
    }
}

fn check_accessible_name(
    module: &WebIrModule,
    tag: &str,
    attrs: &[(String, String)],
    children: &[DomNodeId],
    out: &mut Vec<WebIrDiagnostic>,
) {
    let has_aria = attrs
        .iter()
        .any(|(k, _)| k == "aria-label" || k == "aria-labelledby");
    if has_aria {
        return;
    }
    if has_non_empty_text_child(module, children) {
        return;
    }
    out.push(WebIrDiagnostic {
        code: "web_ir_validate.a11y.interactive_missing_label".to_string(),
        message: format!(
            "`{}` element has no accessible name — add text content, `aria-label`, or `aria-labelledby`",
            tag
        ),
        span: None,
        category: Some("a11y".to_string()),
    });
}

/// For `role="button"` elements: require at least one keyboard event handler in the module
/// targeting this element (or a module-level catch-all keyboard handler).
fn check_keyboard_handler(
    module: &WebIrModule,
    elem_id: DomNodeId,
    out: &mut Vec<WebIrDiagnostic>,
) {
    let is_keyboard = |event: &str| {
        event == "keydown" || event == "keyup" || event == "keypress"
    };

    let has_handler = module.behavior_nodes.iter().any(|b| {
        if let BehaviorNode::EventHandler { event, target_dom, .. } = b {
            if !is_keyboard(event) {
                return false;
            }
            // Accept: handler targeting this element OR a catch-all (None target).
            target_dom.map_or(true, |t| t == elem_id)
        } else {
            false
        }
    });

    if !has_handler {
        out.push(WebIrDiagnostic {
            code: "web_ir_validate.a11y.role_button_missing_keyboard".to_string(),
            message: "element with `role=\"button\"` requires a `keydown` or `keyup` handler for keyboard accessibility".to_string(),
            span: None,
            category: Some("a11y".to_string()),
        });
    }
}

/// Recursively check whether a set of child DOM nodes contains any non-empty text content
/// or expression nodes (which may produce text at runtime).
fn has_non_empty_text_child(module: &WebIrModule, child_ids: &[DomNodeId]) -> bool {
    for child_id in child_ids {
        let Some(node) = module.dom_nodes.get(child_id.0 as usize) else {
            continue;
        };
        match node {
            DomNode::Text { content, .. } => {
                if !content.trim().is_empty() {
                    return true;
                }
            }
            DomNode::Element { children, .. } => {
                if has_non_empty_text_child(module, children) {
                    return true;
                }
            }
            // Expression nodes ({label}, {count}, etc.) may produce text at runtime;
            // treat their presence as satisfying the accessible-name requirement.
            DomNode::Expr { .. } => return true,
            _ => {}
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web_ir::{DomNodeId, StyleNode, StyleSelector, WebIrModule};

    fn elem(id: u32, tag: &str, attrs: Vec<(&str, &str)>, children: Vec<u32>) -> DomNode {
        DomNode::Element {
            id: DomNodeId(id),
            tag: tag.to_string(),
            attrs: attrs.into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            children: children.into_iter().map(DomNodeId).collect(),
            span: None,
        }
    }

    fn text(content: &str) -> DomNode {
        DomNode::Text {
            content: content.to_string(),
            span: None,
        }
    }

    fn module_with_nodes(nodes: Vec<DomNode>) -> WebIrModule {
        WebIrModule {
            dom_nodes: nodes,
            ..Default::default()
        }
    }

    #[test]
    fn img_without_alt_is_error() {
        let m = module_with_nodes(vec![elem(0, "img", vec![("src", "logo.png")], vec![])]);
        let diags = {
            let mut out = Vec::new();
            validate_a11y(&m, &mut out);
            out
        };
        assert!(
            diags.iter().any(|d| d.code == "web_ir_validate.a11y.img_missing_alt"),
            "expected img_missing_alt: {diags:?}"
        );
    }

    #[test]
    fn img_with_alt_is_ok() {
        let m = module_with_nodes(vec![elem(0, "img", vec![("src", "x.png"), ("alt", "Logo")], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(out.is_empty(), "unexpected: {out:?}");
    }

    #[test]
    fn img_with_aria_hidden_is_ok() {
        let m = module_with_nodes(vec![elem(0, "img", vec![("src", "deco.svg"), ("aria-hidden", "true")], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(out.is_empty(), "unexpected: {out:?}");
    }

    #[test]
    fn button_without_label_is_error() {
        let m = module_with_nodes(vec![elem(0, "button", vec![], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(
            out.iter().any(|d| d.code == "web_ir_validate.a11y.interactive_missing_label"),
            "expected interactive_missing_label: {out:?}"
        );
    }

    #[test]
    fn button_with_text_child_is_ok() {
        let m = module_with_nodes(vec![
            elem(0, "button", vec![], vec![1]),
            text("Submit"),
        ]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(out.is_empty(), "unexpected: {out:?}");
    }

    #[test]
    fn button_with_aria_label_is_ok() {
        let m = module_with_nodes(vec![elem(0, "button", vec![("aria-label", "Close")], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(out.is_empty(), "unexpected: {out:?}");
    }

    #[test]
    fn anchor_without_href_skips_check() {
        let m = module_with_nodes(vec![elem(0, "a", vec![], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(out.is_empty(), "anchor without href should not be checked: {out:?}");
    }

    #[test]
    fn anchor_with_href_and_no_label_is_error() {
        let m = module_with_nodes(vec![elem(0, "a", vec![("href", "/about")], vec![])]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(
            out.iter().any(|d| d.code == "web_ir_validate.a11y.interactive_missing_label"),
            "expected interactive_missing_label: {out:?}"
        );
    }

    #[test]
    fn role_button_without_keyboard_handler_is_error() {
        let m = module_with_nodes(vec![elem(
            0,
            "div",
            vec![("role", "button"), ("aria-label", "Toggle")],
            vec![],
        )]);
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(
            out.iter().any(|d| d.code == "web_ir_validate.a11y.role_button_missing_keyboard"),
            "expected role_button_missing_keyboard: {out:?}"
        );
    }

    #[test]
    fn role_button_with_keyboard_handler_is_ok() {
        let mut m = module_with_nodes(vec![elem(
            0,
            "div",
            vec![("role", "button"), ("aria-label", "Toggle")],
            vec![],
        )]);
        m.behavior_nodes.push(BehaviorNode::EventHandler {
            target_dom: Some(DomNodeId(0)),
            event: "keydown".to_string(),
            handler: "handleKey".to_string(),
            span: None,
        });
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        assert!(
            out.iter().all(|d| d.code != "web_ir_validate.a11y.role_button_missing_keyboard"),
            "unexpected keyboard error: {out:?}"
        );
    }
}
