//! Accessibility and structure validation for TSX returned by v0.dev (TASK-5.4).
//!
//! Performs a lightweight regex-based JSX element scan and feeds the result
//! into the Web IR a11y validator (`validate_a11y`).  No full parse tree is
//! required — we only need enough structure to surface the most common a11y
//! violations before the generated file is committed to `islands/src/`.
//!
//! ## What is checked
//!
//! All rules from `web_ir::validate_a11y` (TASK-5.3):
//! - `<img>` without `alt` (Error)
//! - `<button>` with no accessible label (Error)
//! - `<a>` without `href` (Warning)
//! - `role="button"` on non-button elements without keyboard handler (Warning)
//! - `<input>` without accessible label (Warning)
//!
//! ## Limitations
//!
//! The regex extractor handles standard JSX attribute syntax but will miss:
//! - Spread attributes (`{...props}`)
//! - Computed attribute names (rare in v0 output)
//! - Multi-line attr values spanning more than ~3 lines
//!
//! These edge cases are acceptable — we emit warnings conservatively and never
//! block generation on false positives (only `Error` severity items trigger a
//! mandatory user decision; `Warning` items are printed but auto-accepted).

use regex::Regex;
use vox_compiler::web_ir::{DomNode, DomNodeId, WebIrDiagnostic, WebIrDiagnosticSeverity, WebIrModule};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Validate TSX source for a11y issues.  Returns diagnostics sorted by code.
///
/// Errors indicate unambiguous violations (missing `alt`, unlabelled
/// buttons).  Warnings indicate likely violations that may have off-screen
/// labelling the regex cannot see.
pub fn validate_tsx_a11y(tsx: &str) -> Vec<WebIrDiagnostic> {
    let module = tsx_to_web_ir_module(tsx);
    let mut out = Vec::new();
    vox_compiler::web_ir::validate_a11y::validate_a11y(&module, &mut out);
    out.sort_by(|a, b| a.code.cmp(&b.code));
    out
}

/// Format diagnostics for CLI display.  Returns `None` if the slice is empty.
pub fn format_diagnostics(diags: &[WebIrDiagnostic], component_name: &str) -> Option<String> {
    if diags.is_empty() {
        return None;
    }
    let mut buf = format!(
        "⚠️  {component_name}: {n} accessibility issue(s) detected in v0 output:\n",
        n = diags.len(),
    );
    for d in diags {
        let icon = match d.severity {
            WebIrDiagnosticSeverity::Error => "❌",
            WebIrDiagnosticSeverity::Warning => "⚠️ ",
        };
        buf.push_str(&format!("  {icon} [{}] {}\n", d.code, d.message));
    }
    buf.push_str("\nFix these issues in the generated TSX before shipping, or add explicit aria attributes.");
    Some(buf)
}

/// True if any diagnostic has [`WebIrDiagnosticSeverity::Error`] severity.
pub fn has_errors(diags: &[WebIrDiagnostic]) -> bool {
    diags.iter().any(|d| d.severity == WebIrDiagnosticSeverity::Error)
}

// ---------------------------------------------------------------------------
// TSX → WebIrModule (regex-based, heuristic)
// ---------------------------------------------------------------------------

/// Build a minimal [`WebIrModule`] from TSX source by scanning opening JSX tags.
///
/// Only element types relevant to the a11y rules are extracted:
/// `img`, `button`, `a`, `input`, `div`, `span`, `section`, `article`,
/// `header`, `footer`, `main`, `nav`, `aside`, `li`, `td`, `th`.
///
/// Text content is approximated: if the raw TSX between the opening tag and the
/// matching close tag contains non-whitespace characters other than JSX
/// expressions, a `DomNode::Text` child is injected.
fn tsx_to_web_ir_module(tsx: &str) -> WebIrModule {
    // Match self-closing or open tags: <tagName ... /> or <tagName ...>
    // We capture: group 1 = tag name, group 2 = attrs string
    // Matches opening JSX tags and their attribute strings.
    // Group 1 = tag name, group 2 = raw attrs text.
    // Uses r#"..."# so literal double-quotes can appear in the pattern.
    let re_tag = Regex::new(
        r#"(?x)
        <                       # open bracket
        ([A-Za-z][A-Za-z0-9]*)  # 1: tag name
        (                        # 2: attrs block
          (?:
            \s+
            (?:
              [A-Za-z_][\w\-:]*  # attr key
              (?:\s*=\s*(?:
                "[^"]*"          # double-quoted value
                |'[^']*'         # single-quoted value
                |\{[^}]*\}       # JSX expression
                |[^\s/>"'{]+     # bare value
              ))?
            )
          )*
        )
        \s*
        (?:/>|>)
        "#,
    )
    .expect("static JSX tag regex");

    // Attr extractor: key="val", key={'val'}, key={expr}, key (boolean).
    // Group 1 = name, 2 = double-quoted, 3 = single-quoted, 4 = JSX expr, 5 = bare.
    let re_attr = Regex::new(
        r#"(?x)
        \s+
        ([A-Za-z_][\w\-:]*)  # 1: attr name
        (?:\s*=\s*
          (?:
            "([^"]*)"          # 2: double-quoted value
            |'([^']*)'         # 3: single-quoted value
            |\{([^}]*)\}       # 4: JSX expr content
            |([^\s/>"'{]+)     # 5: bare value
          )
        )?
        "#,
    )
    .expect("static attr regex");

    let mut module = WebIrModule::default();
    let mut id_counter: u32 = 0;

    // Inline tags we actually care about for a11y checks.
    const RELEVANT: &[&str] = &[
        "img", "button", "a", "input", "div", "span", "section", "article", "header", "footer",
        "main", "nav", "aside", "li", "td", "th", "summary",
    ];

    for cap in re_tag.captures_iter(tsx) {
        let tag_raw = cap.get(1).map(|m| m.as_str()).unwrap_or("");
        let tag = tag_raw.to_ascii_lowercase();
        if !RELEVANT.contains(&tag.as_str()) {
            continue;
        }

        let attrs_str = cap.get(2).map(|m| m.as_str()).unwrap_or("");
        let mut attrs: Vec<(String, String)> = Vec::new();

        for a in re_attr.captures_iter(attrs_str) {
            let key = a.get(1).map(|m| m.as_str()).unwrap_or("").to_string();
            // Pick the first non-None value group (2,3,4,5); boolean attrs get empty string.
            let val = a
                .get(2)
                .or_else(|| a.get(3))
                .or_else(|| a.get(4))
                .or_else(|| a.get(5))
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_string();
            if !key.is_empty() {
                attrs.push((key, val));
            }
        }

        let elem_id = DomNodeId(id_counter);
        id_counter += 1;

        // Heuristic: look for text content after the opening tag close (>)
        // up to ~120 chars; if non-trivially non-empty, inject a Text child.
        let after_open = cap.get(0).map(|m| m.end()).unwrap_or(0);
        let snippet_end = (after_open + 120).min(tsx.len());
        let snippet = tsx.get(after_open..snippet_end).unwrap_or("");
        let text_content = extract_apparent_text_content(snippet);

        // Push element FIRST so its arena position matches elem_id.0 for
        // the positional fallback in has_accessible_child_content.  We set
        // children to a placeholder vec and will patch it below if text is found.
        let elem_arena_idx = module.dom_nodes.len();
        module.dom_nodes.push(DomNode::Element {
            id: elem_id,
            tag,
            attrs,
            children: vec![], // filled in below
            span: None,
        });

        // Push text child AFTER the element; its arena position = elem_arena_idx + 1
        // which equals id_counter (since we haven't incremented yet).
        if !text_content.trim().is_empty() {
            let text_arena_pos = module.dom_nodes.len() as u32; // == id_counter
            id_counter += 1;
            module.dom_nodes.push(DomNode::Text {
                content: text_content,
                span: None,
            });
            // Patch the element's children list.
            if let Some(DomNode::Element { children, .. }) = module.dom_nodes.get_mut(elem_arena_idx) {
                children.push(DomNodeId(text_arena_pos));
            }
        }
    }

    module
}

/// Pull apparent text from a short snippet following a JSX opening tag.
///
/// Strips JSX expressions (`{…}`) and child element tags (`<…>`); if the
/// remaining text has more than 2 non-whitespace characters we treat it as
/// real text content.
fn extract_apparent_text_content(snippet: &str) -> String {
    // Remove JSX expressions
    let re_expr = Regex::new(r"\{[^}]*\}").expect("static expr regex");
    let cleaned = re_expr.replace_all(snippet, " EXPR ");
    // Remove nested tags
    let re_tag = Regex::new(r"<[^>]*>").expect("static tag regex");
    let cleaned = re_tag.replace_all(&cleaned, " ");
    // Take up to the first '<' boundary as a heuristic sentence
    let up_to_next = cleaned.split('<').next().unwrap_or("").trim().to_string();
    // If the result contains JSX expression placeholder or non-whitespace content treat as content
    if up_to_next.contains("EXPR") || up_to_next.chars().filter(|c| !c.is_whitespace()).count() > 2
    {
        up_to_next
    } else {
        String::new()
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use vox_compiler::web_ir::WebIrDiagnosticSeverity;

    #[test]
    fn img_without_alt_is_caught() {
        let tsx = r#"
export function Card() {
  return <div><img src="/logo.png" /></div>;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            diags.iter().any(|d| d.code == "web_ir_a11y.img.missing_alt"),
            "expected img.missing_alt, got: {diags:?}"
        );
    }

    #[test]
    fn img_with_alt_passes() {
        let tsx = r#"
export function Card() {
  return <img src="/logo.png" alt="Company logo" />;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            !diags.iter().any(|d| d.code == "web_ir_a11y.img.missing_alt"),
            "img with alt should pass"
        );
    }

    #[test]
    fn empty_button_is_caught() {
        let tsx = r#"
export function Btn() {
  return <button onClick={handleClick} />;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            diags.iter().any(|d| d.code == "web_ir_a11y.button.missing_label"),
            "expected button.missing_label, got: {diags:?}"
        );
    }

    #[test]
    fn button_with_text_passes() {
        let tsx = r#"
export function Btn() {
  return <button onClick={handleClick}>Submit</button>;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            !diags.iter().any(|d| d.code == "web_ir_a11y.button.missing_label"),
            "button with text should pass, got: {diags:?}"
        );
    }

    #[test]
    fn anchor_without_href_warns() {
        let tsx = r#"
export function Nav() {
  return <a className="tab">Home</a>;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            diags.iter().any(|d| d.code == "web_ir_a11y.anchor.missing_href"),
            "expected anchor.missing_href, got: {diags:?}"
        );
    }

    #[test]
    fn anchor_with_href_passes() {
        let tsx = r#"
export function Nav() {
  return <a href="/home">Home</a>;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            !diags.iter().any(|d| d.code == "web_ir_a11y.anchor.missing_href"),
            "anchor with href should pass"
        );
    }

    #[test]
    fn input_without_label_warns() {
        let tsx = r#"
export function Form() {
  return <input type="text" placeholder="Search" />;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            diags.iter().any(|d| d.code == "web_ir_a11y.input.missing_label"),
            "expected input.missing_label, got: {diags:?}"
        );
    }

    #[test]
    fn input_with_aria_label_passes() {
        let tsx = r#"
export function Form() {
  return <input type="text" aria-label="Search" />;
}
"#;
        let diags = validate_tsx_a11y(tsx);
        assert!(
            !diags.iter().any(|d| d.code == "web_ir_a11y.input.missing_label"),
            "input with aria-label should pass"
        );
    }

    #[test]
    fn has_errors_false_for_warnings_only() {
        let tsx = r#"<a className="link">Click</a>"#;
        let diags = validate_tsx_a11y(tsx);
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == WebIrDiagnosticSeverity::Error)
            .collect();
        // Only anchor.missing_href which is Warning
        assert!(errors.is_empty(), "anchor.missing_href is Warning not Error");
    }

    #[test]
    fn format_diagnostics_none_when_empty() {
        assert!(format_diagnostics(&[], "Widget").is_none());
    }

    #[test]
    fn format_diagnostics_includes_component_name() {
        let tsx = r#"<img src="/x.png" />"#;
        let diags = validate_tsx_a11y(tsx);
        let msg = format_diagnostics(&diags, "MyWidget").unwrap();
        assert!(msg.contains("MyWidget"), "should mention component name");
        assert!(msg.contains("web_ir_a11y.img.missing_alt"));
    }
}
