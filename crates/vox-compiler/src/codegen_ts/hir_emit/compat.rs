//! JSX attribute names and HIR scalar types mapped for TypeScript / React emission.
//!
//! Web IR lowering and TSX preview use the same matrices via re-exports from [`super`] so Path C,
//! AST JSX, and `web_ir::lower` stay aligned (ADR 012 Phase 0).

use crate::hir::HirType;

#[must_use]
pub fn map_hir_type_to_ts(ty: &HirType) -> String {
    match ty {
        HirType::Named(name) => match name.as_str() {
            "int" | "float" => "number".to_string(),
            "str" | "dec" => "string".to_string(),
            "bool" => "boolean".to_string(),
            other => other.to_string(),
        },
        HirType::Generic(name, args) => {
            let args_str: Vec<String> = args.iter().map(map_hir_type_to_ts).collect();
            format!("{}<{}>", name, args_str.join(", "))
        }
        HirType::Decimal => "string".to_string(),
        _ => "any".to_string(),
    }
}

#[must_use]
pub fn map_jsx_attr_name(name: &str) -> &str {
    match name {
        "class" | "className" => "className",
        "on:click" | "on_click" => "onClick",
        "on:change" | "on_change" => "onChange",
        "on:input" | "on_input" => "onInput",
        "on:submit" | "on_submit" => "onSubmit",
        "on:keydown" | "on_keydown" => "onKeyDown",
        "on:keyup" | "on_keyup" => "onKeyUp",
        "on:mouseenter" | "on_mouseenter" => "onMouseEnter",
        "on:mouseleave" | "on_mouseleave" => "onMouseLeave",
        "for" => "htmlFor",
        "tab_index" | "tabIndex" => "tabIndex",
        // SVG snake_case → camelCase aliases (mirrors class/on:click pattern)
        "view_box" => "viewBox",
        "stroke_width" => "strokeWidth",
        "stroke_linecap" => "strokeLinecap",
        "stroke_linejoin" => "strokeLinejoin",
        "stroke_dasharray" => "strokeDasharray",
        "stroke_dashoffset" => "strokeDashoffset",
        "stroke_opacity" => "strokeOpacity",
        "fill_opacity" => "fillOpacity",
        "fill_rule" => "fillRule",
        "clip_path" => "clipPath",
        "clip_rule" => "clipRule",
        "gradient_units" => "gradientUnits",
        "gradient_transform" => "gradientTransform",
        "pattern_units" => "patternUnits",
        "pattern_content_units" => "patternContentUnits",
        "preserve_aspect_ratio" => "preserveAspectRatio",
        "text_anchor" => "textAnchor",
        "stop_color" => "stopColor",
        "stop_opacity" => "stopOpacity",
        "vector_effect" => "vectorEffect",
        "font_family" => "fontFamily",
        "font_size" => "fontSize",
        "font_weight" => "fontWeight",
        "letter_spacing" => "letterSpacing",
        "xmlns_xlink" => "xmlnsXlink",
        _ => name,
    }
}

/// Map SVG snake_case tag names to their React-required camelCase equivalents.
///
/// Mirrors the allowlist pattern of [`map_jsx_attr_name`] for tag names.
/// Back-compat: camelCase inputs pass through unchanged.
#[must_use]
pub fn map_jsx_tag(tag: &str) -> &str {
    match tag {
        "radial_gradient" => "radialGradient",
        "linear_gradient" => "linearGradient",
        "clip_path" => "clipPath",
        "foreign_object" => "foreignObject",
        "fe_gaussian_blur" => "feGaussianBlur",
        _ => tag,
    }
}
