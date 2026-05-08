//! Vox type → TypeScript type mapping helper.
//!
//! What remains here after the classic `@component fn` (Path A) retirement: a single
//! type-mapping helper used by sibling codegen modules (e.g., `activity.rs`). The
//! AST-path component emitters (`generate_component`, `generate_component_from_web_ir`)
//! and their JSX scaffolding were removed in the frontend convergence cleanup — Path C
//! reactive components ([`super::reactive`]) are the canonical TSX emit path.

use crate::ast::scalar_mapping::VoxScalar;

/// Map a Vox type expression to a TypeScript type string.
pub fn map_vox_type_to_ts(ty: &crate::ast::types::TypeExpr) -> String {
    match ty {
        crate::ast::types::TypeExpr::Named { name, .. } => {
            if let Some(s) = VoxScalar::parse(name) {
                s.as_ts_primitive().to_string()
            } else {
                match name.as_str() {
                    "Element" => "React.ReactElement".to_string(),
                    "Unit" => "void".to_string(),
                    other => other.to_string(),
                }
            }
        }
        crate::ast::types::TypeExpr::Generic { name, args, .. } => {
            let args_str: Vec<String> = args.iter().map(map_vox_type_to_ts).collect();
            match name.as_str() {
                "list" => format!("{}[]", args_str.join(", ")),
                "Result" => format!("Result<{}>", args_str.join(", ")),
                "Option" => format!("{} | undefined", args_str.join(", ")),
                _ => format!("{}<{}>", name, args_str.join(", ")),
            }
        }
        crate::ast::types::TypeExpr::Function {
            params,
            return_type,
            ..
        } => {
            let params_str: Vec<String> = params
                .iter()
                .enumerate()
                .map(|(i, p)| format!("arg{i}: {}", map_vox_type_to_ts(p)))
                .collect();
            format!(
                "({}) => {}",
                params_str.join(", "),
                map_vox_type_to_ts(return_type)
            )
        }
        crate::ast::types::TypeExpr::Tuple { elements, .. } => {
            let elems: Vec<String> = elements.iter().map(map_vox_type_to_ts).collect();
            format!("[{}]", elems.join(", "))
        }
        crate::ast::types::TypeExpr::Unit { .. } => "void".to_string(),
        crate::ast::types::TypeExpr::Infer { .. } => "any".to_string(),
        crate::ast::types::TypeExpr::Decimal { .. } => "string".to_string(),
    }
}
