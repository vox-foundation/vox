//! TASK-6.1 — Vox GUI semantic primitive set.
//!
//! Primitives replace JSX tags as the authoring surface.  The parser already
//! accepts custom tag names inside `view:` blocks; when a tag name matches a
//! known primitive, `web_ir/lower.rs` delegates here to obtain the canonical
//! HTML tag and Tailwind class list instead of emitting the tag verbatim.
//!
//! ## Initial primitive set (10 highest-usage)
//!
//! Layout: `stack`, `row`, `column`, `wrap`
//! Content: `text`, `heading`, `link`, `image`
//! Interactive: `button`
//! Structural: `panel`, `card`, `list`, `list_item`, `route_outlet`
//!
//! Each primitive:
//! - Maps to a fixed HTML tag.
//! - Carries a base Tailwind class list.
//! - Declares its accessibility role where non-trivial.
//! - Accepts typed prop overrides (`gap`, `size`, `weight`, `surface`).
//!
//! **Prop conventions** (shared across applicable primitives):
//! - `gap` — spacing token name; resolves to `gap-<token>` Tailwind class.
//! - `size` — typography scale: `xs` | `sm` | `base` | `lg` | `xl` | `2xl`.
//! - `weight` — `normal` | `medium` | `semibold` | `bold`.
//! - `align` — `start` | `center` | `end`.
//! - `wrap` — `true` enables `flex-wrap`.

/// A resolved primitive emission: the HTML tag + ordered Tailwind class list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrimitiveEmission {
    /// HTML tag to emit (e.g. `"div"`, `"p"`, `"button"`).
    pub html_tag: &'static str,
    /// Base Tailwind classes applied unconditionally.
    pub base_classes: Vec<String>,
    /// WAI-ARIA role override when different from the HTML implicit role.
    pub aria_role: Option<&'static str>,
    /// Surface pair name from `surface` attr (e.g. `"primary"`).
    /// Lowering will inject CSS vars `--fg` / `--bg` and add a `data-vox-surface` attr for validation.
    pub surface_ref: Option<String>,
}

impl PrimitiveEmission {
    /// Returns the `class` attribute value as a space-joined string.
    pub fn class_string(&self) -> String {
        self.base_classes.join(" ")
    }
}

/// Resolve a JSX tag name to a `PrimitiveEmission` if it is a known primitive.
/// Returns `None` for ordinary HTML tags or unrecognised names.
///
/// `attrs` are the raw `(name, value)` pairs from the JSX element, allowing
/// prop-driven class augmentation (e.g. `gap="4"` → `gap-4`).
#[must_use]
pub fn resolve(tag: &str, attrs: &[(String, String)]) -> Option<PrimitiveEmission> {
    let get_attr = |name: &str| -> Option<&str> {
        attrs.iter().find(|(k, _)| k == name).map(|(_, v)| v.as_str())
    };
    let surface_name: Option<String> = get_attr("surface").map(|s| s.to_string());

    match tag {
        // ── Layout ────────────────────────────────────────────────────────
        "stack" | "column" => {
            let mut classes = vec!["flex".to_string(), "flex-col".to_string()];
            apply_gap(get_attr("gap"), &mut classes);
            apply_align(get_attr("align"), &mut classes, false);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        "row" => {
            let mut classes = vec!["flex".to_string(), "flex-row".to_string()];
            apply_gap(get_attr("gap"), &mut classes);
            apply_align(get_attr("align"), &mut classes, true);
            if get_attr("wrap").map_or(false, |v| v == "true") {
                classes.push("flex-wrap".to_string());
            }
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        "wrap" => {
            let mut classes = vec![
                "flex".to_string(),
                "flex-row".to_string(),
                "flex-wrap".to_string(),
            ];
            apply_gap(get_attr("gap"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        // ── Content ───────────────────────────────────────────────────────
        "text" => {
            let mut classes = Vec::new();
            apply_text_size(get_attr("size"), &mut classes);
            apply_weight(get_attr("weight"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "p",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        "heading" => {
            let level = get_attr("level").and_then(|v| v.parse::<u8>().ok()).unwrap_or(2);
            let tag = match level {
                1 => "h1",
                2 => "h2",
                3 => "h3",
                4 => "h4",
                5 => "h5",
                _ => "h6",
            };
            let mut classes = Vec::new();
            let default_size = match level {
                1 => "3xl",
                2 => "2xl",
                3 => "xl",
                4 => "lg",
                _ => "base",
            };
            apply_text_size(Some(get_attr("size").unwrap_or(default_size)), &mut classes);
            apply_weight(Some(get_attr("weight").unwrap_or("semibold")), &mut classes);
            Some(PrimitiveEmission {
                html_tag: tag,
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        "link" => {
            let mut classes = vec![
                "text-primary".to_string(),
                "underline-offset-4".to_string(),
                "hover:underline".to_string(),
            ];
            apply_text_size(get_attr("size"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "a",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        "image" => Some(PrimitiveEmission {
            html_tag: "img",
            base_classes: vec!["max-w-full".to_string(), "h-auto".to_string()],
            aria_role: None,
            surface_ref: surface_name.clone(),
        }),
        // ── Interactive ───────────────────────────────────────────────────
        "button" => {
            let variant = get_attr("variant").unwrap_or("default");
            let mut classes = vec![
                "inline-flex".to_string(),
                "items-center".to_string(),
                "justify-center".to_string(),
                "rounded-md".to_string(),
                "text-sm".to_string(),
                "font-medium".to_string(),
                "ring-offset-background".to_string(),
                "transition-colors".to_string(),
                "focus-visible:outline-none".to_string(),
                "focus-visible:ring-2".to_string(),
                "focus-visible:ring-ring".to_string(),
                "focus-visible:ring-offset-2".to_string(),
                "disabled:pointer-events-none".to_string(),
                "disabled:opacity-50".to_string(),
            ];
            match variant {
                "outline" => {
                    classes.push("border".to_string());
                    classes.push("border-input".to_string());
                    classes.push("bg-background".to_string());
                    classes.push("hover:bg-accent".to_string());
                    classes.push("hover:text-accent-foreground".to_string());
                }
                "ghost" => {
                    classes.push("hover:bg-accent".to_string());
                    classes.push("hover:text-accent-foreground".to_string());
                }
                "destructive" => {
                    classes.push("bg-destructive".to_string());
                    classes.push("text-destructive-foreground".to_string());
                    classes.push("hover:bg-destructive/90".to_string());
                }
                _ => {
                    // "default" variant
                    classes.push("bg-primary".to_string());
                    classes.push("text-primary-foreground".to_string());
                    classes.push("hover:bg-primary/90".to_string());
                }
            }
            apply_size_padding(get_attr("size"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "button",
                base_classes: classes,
                aria_role: None,
                surface_ref: surface_name.clone(),
            })
        }
        // ── Structural ────────────────────────────────────────────────────
        "panel" => {
            let mut classes = vec![
                "bg-background".to_string(),
                "rounded-lg".to_string(),
                "border".to_string(),
                "border-border".to_string(),
                "p-4".to_string(),
            ];
            apply_gap(get_attr("gap"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: Some("region"),
                surface_ref: surface_name.clone(),
            })
        }
        "card" => {
            let mut classes = vec![
                "bg-card".to_string(),
                "text-card-foreground".to_string(),
                "rounded-xl".to_string(),
                "border".to_string(),
                "shadow-sm".to_string(),
                "p-6".to_string(),
            ];
            apply_gap(get_attr("gap"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: Some("region"),
                surface_ref: surface_name.clone(),
            })
        }
        "list" => Some(PrimitiveEmission {
            html_tag: "ul",
            base_classes: vec!["list-none".to_string(), "space-y-1".to_string()],
            aria_role: None,
            surface_ref: surface_name.clone(),
        }),
        "list_item" | "list-item" => Some(PrimitiveEmission {
            html_tag: "li",
            base_classes: vec![],
            aria_role: None,
            surface_ref: surface_name.clone(),
        }),
        "route_outlet" | "route-outlet" => Some(PrimitiveEmission {
            html_tag: "div",
            base_classes: vec!["contents".to_string()],
            aria_role: Some("main"),
            surface_ref: surface_name.clone(),
        }),
        // ── Overlay ───────────────────────────────────────────────────────
        "overlay" => Some(PrimitiveEmission {
            html_tag: "div",
            base_classes: vec!["relative".to_string()],
            aria_role: Some("presentation"),
            surface_ref: surface_name.clone(),
        }),
        "toast" => {
            let mut classes = vec!["fixed".to_string()];
            apply_overlay_position(get_attr("position"), &mut classes);
            apply_z_index(get_attr("z"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: Some("alert"),
                surface_ref: surface_name.clone(),
            })
        }
        "drawer" => {
            let mut classes = vec!["fixed".to_string()];
            apply_overlay_position(get_attr("position"), &mut classes);
            apply_z_index(get_attr("z"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: Some("dialog"),
                surface_ref: surface_name.clone(),
            })
        }
        "modal" => {
            let mut classes = vec![
                "fixed".to_string(),
                "inset-0".to_string(),
                "flex".to_string(),
                "items-center".to_string(),
                "justify-center".to_string(),
            ];
            apply_z_index(get_attr("z"), &mut classes);
            Some(PrimitiveEmission {
                html_tag: "div",
                base_classes: classes,
                aria_role: Some("dialog"),
                surface_ref: surface_name.clone(),
            })
        }
        _ => None,
    }
}

/// Returns `true` if the tag is in the known primitive set.
#[must_use]
pub fn is_primitive(tag: &str) -> bool {
    resolve(tag, &[]).is_some()
}

// ---------------------------------------------------------------------------
// Prop-to-class helpers
// ---------------------------------------------------------------------------

fn apply_gap(gap: Option<&str>, classes: &mut Vec<String>) {
    if let Some(g) = gap {
        classes.push(format!("gap-{g}"));
    }
}

fn apply_align(align: Option<&str>, classes: &mut Vec<String>, is_row: bool) {
    match align {
        Some("center") => {
            classes.push("items-center".to_string());
            if is_row {
                classes.push("justify-center".to_string());
            }
        }
        Some("end") => {
            classes.push("items-end".to_string());
        }
        Some("start") | None => {}
        _ => {}
    }
}

fn apply_text_size(size: Option<&str>, classes: &mut Vec<String>) {
    let cls = match size {
        Some("xs") => "text-xs",
        Some("sm") => "text-sm",
        Some("base") | None => "text-base",
        Some("lg") => "text-lg",
        Some("xl") => "text-xl",
        Some("2xl") => "text-2xl",
        Some("3xl") => "text-3xl",
        _ => "text-base",
    };
    classes.push(cls.to_string());
}

fn apply_weight(weight: Option<&str>, classes: &mut Vec<String>) {
    let cls = match weight {
        Some("normal") => "font-normal",
        Some("medium") => "font-medium",
        Some("semibold") => "font-semibold",
        Some("bold") => "font-bold",
        None => return,
        _ => return,
    };
    classes.push(cls.to_string());
}

fn apply_size_padding(size: Option<&str>, classes: &mut Vec<String>) {
    match size {
        Some("sm") => {
            classes.push("h-8".to_string());
            classes.push("px-3".to_string());
            classes.push("text-xs".to_string());
        }
        Some("lg") => {
            classes.push("h-11".to_string());
            classes.push("px-8".to_string());
        }
        Some("icon") => {
            classes.push("h-10".to_string());
            classes.push("w-10".to_string());
        }
        _ => {
            classes.push("h-10".to_string());
            classes.push("px-4".to_string());
            classes.push("py-2".to_string());
        }
    }
}

fn apply_overlay_position(position: Option<&str>, classes: &mut Vec<String>) {
    match position {
        Some("top_right") | Some("top-right") => {
            classes.push("top-0".to_string());
            classes.push("right-0".to_string());
        }
        Some("top_left") | Some("top-left") => {
            classes.push("top-0".to_string());
            classes.push("left-0".to_string());
        }
        Some("bottom_right") | Some("bottom-right") => {
            classes.push("bottom-0".to_string());
            classes.push("right-0".to_string());
        }
        Some("bottom_left") | Some("bottom-left") => {
            classes.push("bottom-0".to_string());
            classes.push("left-0".to_string());
        }
        Some("top") => {
            classes.push("top-0".to_string());
            classes.push("left-0".to_string());
            classes.push("right-0".to_string());
        }
        Some("bottom") => {
            classes.push("bottom-0".to_string());
            classes.push("left-0".to_string());
            classes.push("right-0".to_string());
        }
        Some("left") => {
            classes.push("top-0".to_string());
            classes.push("bottom-0".to_string());
            classes.push("left-0".to_string());
        }
        Some("right") => {
            classes.push("top-0".to_string());
            classes.push("bottom-0".to_string());
            classes.push("right-0".to_string());
        }
        Some("center") => {
            classes.push("inset-0".to_string());
            classes.push("m-auto".to_string());
        }
        _ => {}
    }
}

fn apply_z_index(z: Option<&str>, classes: &mut Vec<String>) {
    if let Some(val) = z {
        // Tailwind JIT arbitrary z-index: z-[100]
        classes.push(format!("z-[{val}]"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn stack_emits_flex_col() {
        let e = resolve("stack", &[]).unwrap();
        assert_eq!(e.html_tag, "div");
        assert!(e.base_classes.contains(&"flex".to_string()));
        assert!(e.base_classes.contains(&"flex-col".to_string()));
    }

    #[test]
    fn stack_with_gap_appends_gap_class() {
        let e = resolve("stack", &attrs(&[("gap", "4")])).unwrap();
        assert!(e.base_classes.contains(&"gap-4".to_string()), "{:?}", e.base_classes);
    }

    #[test]
    fn row_emits_flex_row() {
        let e = resolve("row", &[]).unwrap();
        assert_eq!(e.html_tag, "div");
        assert!(e.base_classes.contains(&"flex-row".to_string()));
        assert!(!e.base_classes.contains(&"flex-wrap".to_string()));
    }

    #[test]
    fn row_with_wrap_adds_flex_wrap() {
        let e = resolve("row", &attrs(&[("wrap", "true")])).unwrap();
        assert!(e.base_classes.contains(&"flex-wrap".to_string()));
    }

    #[test]
    fn text_emits_p() {
        let e = resolve("text", &[]).unwrap();
        assert_eq!(e.html_tag, "p");
    }

    #[test]
    fn text_with_size_sm() {
        let e = resolve("text", &attrs(&[("size", "sm")])).unwrap();
        assert!(e.base_classes.contains(&"text-sm".to_string()));
    }

    #[test]
    fn heading_level_1_emits_h1() {
        let e = resolve("heading", &attrs(&[("level", "1")])).unwrap();
        assert_eq!(e.html_tag, "h1");
        assert!(e.base_classes.contains(&"text-3xl".to_string()));
    }

    #[test]
    fn heading_default_level_emits_h2() {
        let e = resolve("heading", &[]).unwrap();
        assert_eq!(e.html_tag, "h2");
    }

    #[test]
    fn button_default_variant_has_primary_classes() {
        let e = resolve("button", &[]).unwrap();
        assert_eq!(e.html_tag, "button");
        assert!(e.base_classes.contains(&"bg-primary".to_string()));
        assert!(e.base_classes.contains(&"text-primary-foreground".to_string()));
    }

    #[test]
    fn button_outline_variant() {
        let e = resolve("button", &attrs(&[("variant", "outline")])).unwrap();
        assert!(e.base_classes.contains(&"border".to_string()));
        assert!(e.base_classes.contains(&"bg-background".to_string()));
    }

    #[test]
    fn link_emits_a_with_primary_color() {
        let e = resolve("link", &[]).unwrap();
        assert_eq!(e.html_tag, "a");
        assert!(e.base_classes.contains(&"text-primary".to_string()));
    }

    #[test]
    fn panel_emits_with_aria_region() {
        let e = resolve("panel", &[]).unwrap();
        assert_eq!(e.html_tag, "div");
        assert_eq!(e.aria_role, Some("region"));
    }

    #[test]
    fn card_emits_card_bg() {
        let e = resolve("card", &[]).unwrap();
        assert!(e.base_classes.contains(&"bg-card".to_string()));
    }

    #[test]
    fn list_emits_ul() {
        let e = resolve("list", &[]).unwrap();
        assert_eq!(e.html_tag, "ul");
    }

    #[test]
    fn route_outlet_emits_main_role() {
        let e = resolve("route_outlet", &[]).unwrap();
        assert_eq!(e.aria_role, Some("main"));
    }

    #[test]
    fn unknown_tag_returns_none() {
        assert!(resolve("div", &[]).is_none());
        assert!(resolve("span", &[]).is_none());
        assert!(resolve("custom-unknown", &[]).is_none());
    }

    #[test]
    fn is_primitive_recognizes_all_10() {
        for tag in &["stack", "row", "column", "text", "button", "link", "panel", "card", "list", "route_outlet"] {
            assert!(is_primitive(tag), "{tag} should be a primitive");
        }
        assert!(!is_primitive("div"));
        assert!(!is_primitive("span"));
    }

    #[test]
    fn class_string_joins_with_space() {
        let e = resolve("stack", &[]).unwrap();
        let s = e.class_string();
        assert!(s.contains("flex"), "{s}");
        assert!(s.contains("flex-col"), "{s}");
    }
}
