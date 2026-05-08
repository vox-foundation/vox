use crate::hir::{HirModule, HirType, HirTypeDef};

/// Generate TypeScript Zod schema definitions from Vox ADTs.
pub fn generate_zod_schemas(hir: &HirModule) -> String {
    let mut out = String::new();
    if hir.types.is_empty() && hir.tables.is_empty() {
        return out;
    }

    out.push_str("import { z } from \"zod\";\n\n");

    for typedef in &hir.types {
        out.push_str(&generate_zod_schema(typedef));
        out.push('\n');
    }

    out
}

fn generate_zod_schema(typedef: &HirTypeDef) -> String {
    let mut out = String::new();
    let name = &typedef.name;

    // Struct typedef → flat z.object with declared fields.
    if typedef.variants.is_empty() && !typedef.fields.is_empty() {
        out.push_str(&format!("export const {}Schema = z.object({{\n", name));
        for (fname, ftype) in &typedef.fields {
            out.push_str(&format!("  {}: {},\n", fname, map_type_to_zod(ftype)));
        }
        out.push_str("});\n");
        return out;
    }

    if typedef.variants.is_empty() {
        return format!("export const {}Schema = z.object({{}});\n", name);
    }

    if typedef.variants.len() == 1 {
        let variant = &typedef.variants[0];
        out.push_str(&format!("export const {}Schema = z.object({{\n", name));
        out.push_str(&format!("  _tag: z.literal(\"{}\"),\n", variant.name));
        for (fname, ftype) in &variant.fields {
            out.push_str(&format!("  {}: {},\n", fname, map_type_to_zod(ftype)));
        }
        out.push_str("});\n");
    } else {
        out.push_str(&format!(
            "export const {}Schema = z.discriminatedUnion(\"_tag\", [\n",
            name
        ));
        for variant in &typedef.variants {
            out.push_str(&format!(
                "  z.object({{\n    _tag: z.literal(\"{}\"),\n",
                variant.name
            ));
            for (fname, ftype) in &variant.fields {
                out.push_str(&format!("    {}: {},\n", fname, map_type_to_zod(ftype)));
            }
            out.push_str("  }),\n");
        }
        out.push_str("]);\n");
    }

    out
}

pub fn map_type_to_zod(ty: &HirType) -> String {
    match ty {
        HirType::Named(name) => match name.as_str() {
            "int" | "float" => "z.number()".to_string(),
            "str" => "z.string()".to_string(),
            "bool" => "z.boolean()".to_string(),
            // Recursive references will use lazy evaluation in a more robust implementation,
            // but for now we assume linear references.
            other => format!("{}Schema", other),
        },
        HirType::Generic(name, args) => {
            if name == "Option" && args.len() == 1 {
                format!("{}.optional()", map_type_to_zod(&args[0]))
            } else if (name == "list" || name == "Vec" || name == "Array") && args.len() == 1 {
                format!("z.array({})", map_type_to_zod(&args[0]))
            } else {
                "z.any()".to_string()
            }
        }
        _ => "z.any()".to_string(),
    }
}
