//! TASK-5.4 — Pre-flight validation of v0.dev TSX output.
//!
//! Uses regex-based structural scanning rather than a full TSX parser
//! (`swc_ecma_parser` is not yet in the workspace; regex covers the same
//! accessibility and design-token classes as the Web IR validators 5.1–5.3).
//!
//! Checks:
//! - `<img` without `alt=` or `aria-hidden` → `v0.a11y.img_missing_alt`
//! - `<button` with no text content or `aria-label=` → `v0.a11y.interactive_missing_label`
//! - `<a href=` with no text content or `aria-label=` → `v0.a11y.interactive_missing_label`
//! - Hex color literals (`#rrggbb`) in JSX / inline styles → `v0.style.literal_color`
//! - `rgb(...)` / `hsl(...)` literals → `v0.style.literal_color`
//! - Pixel / rem dimension literals in inline `style={{...}}` → `v0.style.literal_dimension`

use regex::Regex;
use std::sync::OnceLock;

/// A single detected violation in v0-generated TSX.
#[derive(Debug, Clone)]
pub struct V0Violation {
    pub code: &'static str,
    pub message: String,
}

impl std::fmt::Display for V0Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.code, self.message)
    }
}

/// Scan `tsx` for structural violations.  Returns an empty Vec when the output
/// is clean enough to write verbatim.
#[must_use]
pub fn scan_tsx_violations(tsx: &str) -> Vec<V0Violation> {
    let mut out = Vec::new();
    check_img_alt(tsx, &mut out);
    check_interactive_labels(tsx, &mut out);
    check_literal_colors(tsx, &mut out);
    check_literal_dimensions(tsx, &mut out);
    out
}

// ---------------------------------------------------------------------------
// A11y checks
// ---------------------------------------------------------------------------

fn check_img_alt(tsx: &str, out: &mut Vec<V0Violation>) {
    // Match <img ... > / <img ... /> tags; check for alt= or aria-hidden inside.
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"(?s)<img\b([^>]*?)(/?>)").unwrap());
    for cap in re.captures_iter(tsx) {
        let attrs = cap.get(1).map_or("", |m| m.as_str());
        let has_alt = attrs.contains("alt=");
        let is_hidden = attrs.contains("aria-hidden");
        if !has_alt && !is_hidden {
            out.push(V0Violation {
                code: "v0.a11y.img_missing_alt",
                message: "img element is missing an `alt` attribute (add alt=\"\" for decorative images)".to_string(),
            });
        }
    }
}

fn check_interactive_labels(tsx: &str, out: &mut Vec<V0Violation>) {
    // <button ...> ... </button> — check for aria-label or non-whitespace text content.
    static BTN: OnceLock<Regex> = OnceLock::new();
    let btn_re = BTN.get_or_init(|| Regex::new(r"(?s)<button\b([^>]*)>(.*?)</button>").unwrap());
    for cap in btn_re.captures_iter(tsx) {
        let attrs = cap.get(1).map_or("", |m| m.as_str());
        let body = cap.get(2).map_or("", |m| m.as_str());
        let has_label = attrs.contains("aria-label") || attrs.contains("aria-labelledby");
        let has_text = body.chars().any(|c| !c.is_whitespace() && c != '{' && c != '}');
        if !has_label && !has_text {
            out.push(V0Violation {
                code: "v0.a11y.interactive_missing_label",
                message: "button element has no accessible name (add text content or aria-label)".to_string(),
            });
        }
    }

    // <a href=...> ... </a> — same check.
    static ANCHOR: OnceLock<Regex> = OnceLock::new();
    let a_re = ANCHOR.get_or_init(|| Regex::new(r#"(?s)<a\b([^>]*\bhref=)[^>]*>(.*?)</a>"#).unwrap());
    for cap in a_re.captures_iter(tsx) {
        let attrs = cap.get(1).map_or("", |m| m.as_str());
        let body = cap.get(2).map_or("", |m| m.as_str());
        let has_label = attrs.contains("aria-label") || attrs.contains("aria-labelledby");
        let has_text = body.chars().any(|c| !c.is_whitespace() && c != '{' && c != '}');
        if !has_label && !has_text {
            out.push(V0Violation {
                code: "v0.a11y.interactive_missing_label",
                message: "a[href] element has no accessible name (add text content or aria-label)".to_string(),
            });
        }
    }
}

// ---------------------------------------------------------------------------
// Design-token checks (mirrors TASK-5.1 Web IR checks)
// ---------------------------------------------------------------------------

fn check_literal_colors(tsx: &str, out: &mut Vec<V0Violation>) {
    // Hex colors: #rgb / #rgba / #rrggbb / #rrggbbaa inside string literals or style props.
    static HEX: OnceLock<Regex> = OnceLock::new();
    let hex_re = HEX.get_or_init(|| {
        Regex::new(r#"["'\s:,]#([0-9a-fA-F]{3,8})\b"#).unwrap()
    });
    let hex_count = hex_re.find_iter(tsx).count();
    if hex_count > 0 {
        out.push(V0Violation {
            code: "v0.style.literal_color",
            message: format!(
                "{hex_count} hex color literal(s) found — replace with design tokens"
            ),
        });
    }

    // Functional colors: rgb(...) / rgba(...) / hsl(...) / hsla(...)
    static FUNC: OnceLock<Regex> = OnceLock::new();
    let func_re = FUNC.get_or_init(|| {
        Regex::new(r"\b(?:rgb|rgba|hsl|hsla)\s*\(").unwrap()
    });
    let func_count = func_re.find_iter(tsx).count();
    if func_count > 0 {
        out.push(V0Violation {
            code: "v0.style.literal_color",
            message: format!(
                "{func_count} functional color literal(s) (rgb/hsl) found — replace with design tokens"
            ),
        });
    }
}

fn check_literal_dimensions(tsx: &str, out: &mut Vec<V0Violation>) {
    // Pixel/rem/em literals inside inline style objects: e.g. fontSize: 14, padding: "16px"
    // Only flag inside style={{ ... }} blocks to avoid false positives in comments/strings.
    static STYLE_BLOCK: OnceLock<Regex> = OnceLock::new();
    let style_re = STYLE_BLOCK.get_or_init(|| Regex::new(r"(?s)style=\{\{(.*?)\}\}").unwrap());

    static DIM: OnceLock<Regex> = OnceLock::new();
    let dim_re =
        DIM.get_or_init(|| Regex::new(r#""\s*\d+(?:\.\d+)?\s*(?:px|rem|em|%|vh|vw)""#).unwrap());

    let mut dim_count = 0usize;
    for cap in style_re.captures_iter(tsx) {
        let block = cap.get(1).map_or("", |m| m.as_str());
        dim_count += dim_re.find_iter(block).count();
        // Also count bare numeric values for pixel-ish props (fontSize: 14 → 14px in CSS)
        static BARE: OnceLock<Regex> = OnceLock::new();
        let bare_re = BARE.get_or_init(|| {
            Regex::new(r"\b(?:fontSize|padding|margin|width|height|gap|borderRadius|lineHeight)\s*:\s*\d+\b").unwrap()
        });
        dim_count += bare_re.find_iter(block).count();
    }
    if dim_count > 0 {
        out.push(V0Violation {
            code: "v0.style.literal_dimension",
            message: format!(
                "{dim_count} literal dimension value(s) in inline style — replace with design tokens"
            ),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn img_without_alt_is_flagged() {
        let tsx = r#"<img src="logo.png" />"#;
        let v = scan_tsx_violations(tsx);
        assert!(v.iter().any(|x| x.code == "v0.a11y.img_missing_alt"), "{v:?}");
    }

    #[test]
    fn img_with_alt_is_clean() {
        let tsx = r#"<img src="logo.png" alt="Logo" />"#;
        let v = scan_tsx_violations(tsx);
        assert!(v.iter().all(|x| x.code != "v0.a11y.img_missing_alt"), "{v:?}");
    }

    #[test]
    fn img_with_aria_hidden_is_clean() {
        let tsx = r#"<img src="deco.svg" aria-hidden />"#;
        let v = scan_tsx_violations(tsx);
        assert!(v.iter().all(|x| x.code != "v0.a11y.img_missing_alt"), "{v:?}");
    }

    #[test]
    fn empty_button_is_flagged() {
        let tsx = r#"<button onClick={fn}></button>"#;
        let v = scan_tsx_violations(tsx);
        assert!(
            v.iter().any(|x| x.code == "v0.a11y.interactive_missing_label"),
            "{v:?}"
        );
    }

    #[test]
    fn button_with_text_is_clean() {
        let tsx = r#"<button onClick={fn}>Submit</button>"#;
        let v = scan_tsx_violations(tsx);
        assert!(
            v.iter().all(|x| x.code != "v0.a11y.interactive_missing_label"),
            "{v:?}"
        );
    }

    #[test]
    fn button_with_aria_label_is_clean() {
        let tsx = r#"<button aria-label="Close" onClick={fn}></button>"#;
        let v = scan_tsx_violations(tsx);
        assert!(
            v.iter().all(|x| x.code != "v0.a11y.interactive_missing_label"),
            "{v:?}"
        );
    }

    #[test]
    fn hex_color_in_style_is_flagged() {
        let tsx = r##"style={{ color: "#ff0000" }}"##;
        let v = scan_tsx_violations(tsx);
        assert!(v.iter().any(|x| x.code == "v0.style.literal_color"), "{v:?}");
    }

    #[test]
    fn rgb_color_is_flagged() {
        let tsx = r#"color: rgb(255, 0, 0)"#;
        let v = scan_tsx_violations(tsx);
        assert!(v.iter().any(|x| x.code == "v0.style.literal_color"), "{v:?}");
    }

    #[test]
    fn inline_px_dimension_is_flagged() {
        let tsx = r#"style={{ padding: "16px" }}"#;
        let v = scan_tsx_violations(tsx);
        assert!(
            v.iter().any(|x| x.code == "v0.style.literal_dimension"),
            "{v:?}"
        );
    }

    #[test]
    fn clean_tsx_has_no_violations() {
        let tsx = r#"
export function Card({ title }: { title: string }) {
  return (
    <div className="bg-background p-4">
      <img src={src} alt={title} />
      <button aria-label="Close" onClick={onClose}>
        <X size={16} />
      </button>
      <p>{title}</p>
    </div>
  );
}"#;
        let v = scan_tsx_violations(tsx);
        assert!(v.is_empty(), "unexpected violations: {v:?}");
    }
}
