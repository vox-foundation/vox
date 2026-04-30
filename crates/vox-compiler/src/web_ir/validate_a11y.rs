//! Compile-time accessibility (a11y) validator for Web IR (TASK-5.3).
//!
//! Walks the DOM arena and emits diagnostics for common accessibility violations.
//! Diagnostic codes use the `web_ir_a11y.*` prefix.
//!
//! ## Rules implemented
//!
//! | Code | Severity | Rule |
//! |------|----------|------|
//! | `web_ir_a11y.img.missing_alt`          | Error   | `<img>` without `alt` or `aria-hidden="true"` |
//! | `web_ir_a11y.button.missing_label`      | Error   | `<button>` with no text content, `aria-label`, or `aria-labelledby` |
//! | `web_ir_a11y.anchor.missing_href`       | Warning | `<a>` without `href` |
//! | `web_ir_a11y.interactive.missing_keyboard` | Warning | `role="button"` element without `onclick`/`onkeydown` |
//! | `web_ir_a11y.input.missing_label`       | Warning | `<input>` without `aria-label`, `aria-labelledby`, or `title` |
//!
//! ## Escape hatches (Phase 6)
//!
//! Until the `decorative: true` / `aria_hidden: true` attribute annotations
//! land on `DomNode::Element`, use explicit `aria-hidden="true"` in the
//! source attrs to suppress `img.missing_alt`.
//!
//! ## IR embedding (Phase 6)
//!
//! `AriaNode` / `Role` types are internal to this module for now. When
//! `DomNode::Element` gains `aria: Option<AriaNode>` in Phase 6, the
//! inference logic here moves into the lowering pass.

use std::collections::HashMap;

use crate::web_ir::{DomNode, DomNodeId, WebIrDiagnostic, WebIrDiagnosticSeverity, WebIrModule};

// ---------------------------------------------------------------------------
// Internal aria inference types (not yet embedded in DomNode — Phase 6)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq)]
enum AriaRole {
    Button,
    Link,
    Img,
    TextInput,
    Checkbox,
    Radio,
    Combobox,
    Generic,
}

/// Derive the implicit ARIA role from an element tag, following the
/// [HTML-AAM](https://www.w3.org/TR/html-aam/) mapping.
fn implicit_role(tag: &str) -> AriaRole {
    match tag.to_ascii_lowercase().as_str() {
        "button" | "summary" => AriaRole::Button,
        "a" | "area" => AriaRole::Link,
        "img" => AriaRole::Img,
        "input" => AriaRole::TextInput, // simplified; real mapping depends on type attr
        "select" => AriaRole::Combobox,
        _ => AriaRole::Generic,
    }
}

/// Check whether an attrs list contains a specific attribute name (case-insensitive key).
fn has_attr(attrs: &[(String, String)], name: &str) -> bool {
    attrs.iter().any(|(k, _)| k.eq_ignore_ascii_case(name))
}

/// Get the value of a named attribute (case-insensitive key).
fn attr_value<'a>(attrs: &'a [(String, String)], name: &str) -> Option<&'a str> {
    attrs
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

/// True if the element carries an explicit accessible name via aria attributes.
fn has_aria_name(attrs: &[(String, String)]) -> bool {
    has_attr(attrs, "aria-label") || has_attr(attrs, "aria-labelledby")
}

/// True if the element has non-empty text or expression children in the arena.
///
/// `DomNodeId` values in `children` lists serve a dual role:
/// - For `Element` nodes the `.id` field matches the `DomNodeId`.
/// - For non-element nodes (`Text`, `Expr`, `Fragment`, …) there is no `.id`;
///   the `DomNodeId` is used as a direct arena-position index.
/// We therefore try element-id lookup first, then fall back to positional.
fn has_accessible_child_content(node_id: DomNodeId, arena: &[DomNode], index: &HashMap<u32, usize>) -> bool {
    // Resolve the parent node via element-id index, then fall back to positional.
    let node = index
        .get(&node_id.0)
        .and_then(|&i| arena.get(i))
        .or_else(|| arena.get(node_id.0 as usize));
    let children = match node {
        Some(DomNode::Element { children, .. }) => children.as_slice(),
        _ => return false,
    };
    for &child_id in children {
        // Try element-id lookup first, then positional fallback.
        let child = index
            .get(&child_id.0)
            .and_then(|&i| arena.get(i))
            .or_else(|| arena.get(child_id.0 as usize));
        match child {
            Some(DomNode::Text { content, .. }) if !content.trim().is_empty() => return true,
            Some(DomNode::Expr { .. }) => return true, // dynamic expression — assume it provides content
            Some(DomNode::Fragment { children: fc, .. }) => {
                // recurse into fragments
                for fid in fc.clone() {
                    if has_accessible_child_content(fid, arena, index) {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    false
}

// ---------------------------------------------------------------------------
// Per-element checks
// ---------------------------------------------------------------------------

fn check_img(attrs: &[(String, String)], out: &mut Vec<WebIrDiagnostic>) {
    // aria-hidden="true" is the explicit escape hatch.
    if attr_value(attrs, "aria-hidden").is_some_and(|v| v == "true") {
        return;
    }
    let alt = attr_value(attrs, "alt");
    // Missing alt entirely.
    if alt.is_none() {
        out.push(WebIrDiagnostic {
            code: "web_ir_a11y.img.missing_alt".to_string(),
            message: "<img> element is missing an `alt` attribute. \
                      Add alt=\"\" for decorative images or a descriptive alt text for informative ones."
                .to_string(),
            span: None,
            category: Some("a11y".to_string()),
            severity: WebIrDiagnosticSeverity::Error,
        });
    }
    // alt="" is valid for decorative images — no diagnostic.
}

fn check_button(
    node_id: DomNodeId,
    attrs: &[(String, String)],
    arena: &[DomNode],
    index: &HashMap<u32, usize>,
    out: &mut Vec<WebIrDiagnostic>,
) {
    // Accessible name via: aria-label, aria-labelledby, or text/expr content.
    if has_aria_name(attrs) {
        return;
    }
    if has_accessible_child_content(node_id, arena, index) {
        return;
    }
    out.push(WebIrDiagnostic {
        code: "web_ir_a11y.button.missing_label".to_string(),
        message: "<button> element has no accessible name. \
                  Add visible text content, aria-label, or aria-labelledby."
            .to_string(),
        span: None,
        category: Some("a11y".to_string()),
        severity: WebIrDiagnosticSeverity::Error,
    });
}

fn check_anchor(attrs: &[(String, String)], out: &mut Vec<WebIrDiagnostic>) {
    // <a> without href is an interactive element masquerading as a link.
    if !has_attr(attrs, "href") && !has_attr(attrs, "to") {
        out.push(WebIrDiagnostic {
            code: "web_ir_a11y.anchor.missing_href".to_string(),
            message: "<a> element without `href` or `to` is not keyboard-reachable. \
                      Add href/to or use <button> instead."
                .to_string(),
            span: None,
            category: Some("a11y".to_string()),
            severity: WebIrDiagnosticSeverity::Warning,
        });
    }
}

fn check_role_button(attrs: &[(String, String)], out: &mut Vec<WebIrDiagnostic>) {
    // `role="button"` on a non-button element requires keyboard handlers.
    let has_keyboard = has_attr(attrs, "onkeydown")
        || has_attr(attrs, "onkeyup")
        || has_attr(attrs, "onkeypress")
        || has_attr(attrs, "tabIndex")
        || has_attr(attrs, "tabindex");
    if !has_keyboard {
        out.push(WebIrDiagnostic {
            code: "web_ir_a11y.interactive.missing_keyboard".to_string(),
            message: "Element with role=\"button\" must be keyboard-accessible. \
                      Add onKeyDown handler and tabIndex=\"0\"."
                .to_string(),
            span: None,
            category: Some("a11y".to_string()),
            severity: WebIrDiagnosticSeverity::Warning,
        });
    }
}

fn check_input(attrs: &[(String, String)], out: &mut Vec<WebIrDiagnostic>) {
    // Hidden inputs don't need labels.
    if attr_value(attrs, "type").is_some_and(|t| t.eq_ignore_ascii_case("hidden")) {
        return;
    }
    if has_aria_name(attrs) || has_attr(attrs, "title") || has_attr(attrs, "id") {
        // `id` may be referenced by a <label for="..."> elsewhere — treat as labelled.
        return;
    }
    out.push(WebIrDiagnostic {
        code: "web_ir_a11y.input.missing_label".to_string(),
        message: "<input> element has no accessible label. \
                  Add aria-label, aria-labelledby, title, or an associated <label for=\"...\">."
            .to_string(),
        span: None,
        category: Some("a11y".to_string()),
        severity: WebIrDiagnosticSeverity::Warning,
    });
}

// ---------------------------------------------------------------------------
// Main validator entry point
// ---------------------------------------------------------------------------

/// Build a fast `DomNodeId.0 → arena index` lookup map.
fn build_node_index(arena: &[DomNode]) -> HashMap<u32, usize> {
    let mut map = HashMap::with_capacity(arena.len());
    for (idx, node) in arena.iter().enumerate() {
        let id = match node {
            DomNode::Element { id, .. } => id.0,
            _ => continue,
        };
        map.insert(id, idx);
    }
    map
}

/// Walk the entire DOM arena and emit a11y diagnostics.
///
/// This is a pure function over the existing Web IR — it does not require
/// `AriaNode` to be embedded in `DomNode::Element` (that is Phase 6 work).
pub fn validate_a11y(module: &WebIrModule, out: &mut Vec<WebIrDiagnostic>) {
    let index = build_node_index(&module.dom_nodes);

    for node in &module.dom_nodes {
        let DomNode::Element { id, tag, attrs, .. } = node else {
            continue;
        };

        // Check explicit role="button" on non-button elements.
        if let Some(role) = attr_value(attrs, "role") {
            if role.eq_ignore_ascii_case("button") {
                let tag_lower = tag.to_ascii_lowercase();
                if tag_lower != "button" && tag_lower != "summary" {
                    check_role_button(attrs, out);
                }
            }
        }

        // Per-element structural checks.
        match implicit_role(tag) {
            AriaRole::Img => check_img(attrs, out),
            AriaRole::Button => {
                check_button(*id, attrs, &module.dom_nodes, &index, out);
            }
            AriaRole::Link => check_anchor(attrs, out),
            AriaRole::TextInput => check_input(attrs, out),
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::web_ir::{DomNode, DomNodeId, WebIrDiagnosticSeverity, WebIrModule};

    fn run(nodes: Vec<DomNode>) -> Vec<WebIrDiagnostic> {
        let mut m = WebIrModule::default();
        m.dom_nodes = nodes;
        let mut out = Vec::new();
        validate_a11y(&m, &mut out);
        out
    }

    // ── <img> ────────────────────────────────────────────────────────────────

    #[test]
    fn img_with_alt_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "img".to_string(),
            attrs: vec![
                ("src".to_string(), "/logo.png".to_string()),
                ("alt".to_string(), "Company logo".to_string()),
            ],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "img with alt should pass: {diags:?}");
    }

    #[test]
    fn img_with_empty_alt_passes_decorative() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "img".to_string(),
            attrs: vec![
                ("src".to_string(), "/deco.png".to_string()),
                ("alt".to_string(), String::new()),
            ],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "img with empty alt (decorative) should pass");
    }

    #[test]
    fn img_without_alt_is_error() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "img".to_string(),
            attrs: vec![("src".to_string(), "/logo.png".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "web_ir_a11y.img.missing_alt");
        assert_eq!(diags[0].severity, WebIrDiagnosticSeverity::Error);
    }

    #[test]
    fn img_with_aria_hidden_suppresses_alt_check() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "img".to_string(),
            attrs: vec![
                ("src".to_string(), "/bg.svg".to_string()),
                ("aria-hidden".to_string(), "true".to_string()),
            ],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "aria-hidden='true' suppresses alt check");
    }

    // ── <button> ─────────────────────────────────────────────────────────────

    #[test]
    fn button_with_text_child_passes() {
        let nodes = vec![
            DomNode::Element {
                id: DomNodeId(0),
                tag: "button".to_string(),
                attrs: vec![],
                children: vec![DomNodeId(1)],
                span: None,
            },
            DomNode::Text {
                content: "Submit".to_string(),
                span: None,
            },
        ];
        let diags = run(nodes);
        assert!(diags.is_empty(), "button with text child should pass");
    }

    #[test]
    fn button_with_aria_label_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "button".to_string(),
            attrs: vec![("aria-label".to_string(), "Close dialog".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "button with aria-label should pass");
    }

    #[test]
    fn empty_button_is_error() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "button".to_string(),
            attrs: vec![],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "web_ir_a11y.button.missing_label");
        assert_eq!(diags[0].severity, WebIrDiagnosticSeverity::Error);
    }

    // ── <a> ──────────────────────────────────────────────────────────────────

    #[test]
    fn anchor_with_href_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "a".to_string(),
            attrs: vec![("href".to_string(), "/home".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "anchor with href should pass");
    }

    #[test]
    fn anchor_without_href_warns() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "a".to_string(),
            attrs: vec![("class".to_string(), "tab".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "web_ir_a11y.anchor.missing_href");
        assert_eq!(diags[0].severity, WebIrDiagnosticSeverity::Warning);
    }

    // ── role="button" ────────────────────────────────────────────────────────

    #[test]
    fn div_role_button_without_keyboard_warns() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "div".to_string(),
            attrs: vec![
                ("role".to_string(), "button".to_string()),
                ("onclick".to_string(), "handleClick()".to_string()),
            ],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "web_ir_a11y.interactive.missing_keyboard");
    }

    #[test]
    fn div_role_button_with_keyboard_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "div".to_string(),
            attrs: vec![
                ("role".to_string(), "button".to_string()),
                ("onclick".to_string(), "handleClick()".to_string()),
                ("onkeydown".to_string(), "handleKey()".to_string()),
                ("tabindex".to_string(), "0".to_string()),
            ],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "div role=button with keyboard handler should pass");
    }

    // ── <input> ──────────────────────────────────────────────────────────────

    #[test]
    fn input_with_aria_label_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "input".to_string(),
            attrs: vec![("aria-label".to_string(), "Search".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "input with aria-label should pass");
    }

    #[test]
    fn input_hidden_no_label_passes() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "input".to_string(),
            attrs: vec![("type".to_string(), "hidden".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert!(diags.is_empty(), "hidden input needs no label");
    }

    #[test]
    fn input_without_label_warns() {
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "input".to_string(),
            attrs: vec![("type".to_string(), "text".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        assert_eq!(diags.len(), 1);
        assert_eq!(diags[0].code, "web_ir_a11y.input.missing_label");
        assert_eq!(diags[0].severity, WebIrDiagnosticSeverity::Warning);
    }

    // ── button native element doesn't trigger role="button" check ────────────

    #[test]
    fn button_element_with_role_button_not_double_warned() {
        // A real <button> with role="button" explicitly: should get button label check,
        // not the interactive keyboard check (it's already a button).
        let nodes = vec![DomNode::Element {
            id: DomNodeId(0),
            tag: "button".to_string(),
            attrs: vec![("role".to_string(), "button".to_string())],
            children: vec![],
            span: None,
        }];
        let diags = run(nodes);
        // Only button.missing_label, NOT interactive.missing_keyboard.
        assert!(!diags.iter().any(|d| d.code == "web_ir_a11y.interactive.missing_keyboard"),
            "native <button> with role=button should not get keyboard check");
        assert!(diags.iter().any(|d| d.code == "web_ir_a11y.button.missing_label"),
            "native <button> with no content should still get label check");
    }
}
