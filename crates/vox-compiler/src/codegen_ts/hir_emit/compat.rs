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
        _ => name,
    }
}
