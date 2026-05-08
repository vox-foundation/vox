//! JSX attribute names and HIR scalar types mapped for TypeScript / React emission.
//!
//! Web IR lowering and TSX preview use the same matrices via re-exports from [`super`] so Path C,
//! AST JSX, and `web_ir::lower` stay aligned (ADR 012 Phase 0).

use crate::hir::HirType;

/// Quote a Vox string for emission as a TypeScript/JavaScript double-quoted string literal.
///
/// Uses `serde_json::to_string`, which produces a string that's simultaneously valid JSON
/// and valid JS/TS — escapes inner `"`, `\`, and control characters. Falls back to a naive
/// quote on the (impossible-in-practice) serde_json failure path.
#[must_use]
pub fn ts_string_literal(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
}

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
        "clip_path"              => "clipPath", // also valid as a tag — see map_jsx_tag
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
        "std_deviation" => "stdDeviation",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ts_string_literal_escapes_inner_double_quotes() {
        // Bug C: emitting `{"k":"v"}` as a TS literal must escape inner quotes.
        let out = ts_string_literal(r#"{"mood_score":3}"#);
        assert_eq!(out, r#""{\"mood_score\":3}""#);
    }

    #[test]
    fn ts_string_literal_escapes_backslashes_and_controls() {
        assert_eq!(ts_string_literal("a\\b"), r#""a\\b""#);
        assert_eq!(ts_string_literal("a\nb"), r#""a\nb""#);
        assert_eq!(ts_string_literal("a\tb"), r#""a\tb""#);
    }

    #[test]
    fn ts_string_literal_plain_strings_unchanged() {
        assert_eq!(ts_string_literal("hello"), r#""hello""#);
        assert_eq!(ts_string_literal(""), r#""""#);
    }
}
