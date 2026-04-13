use syn::{Item, ItemStruct, ItemEnum, ItemImpl};
use std::collections::HashMap;

#[derive(Debug)]
pub struct TranslationPair {
    pub instruction: String,
    pub input_rust: String,
    pub output_vox: String,
    pub confidence: f32,
}

pub fn extract_translations(rust_source: &str) -> Vec<TranslationPair> {
    let mut pairs = Vec::new();
    
    let file = match syn::parse_file(rust_source) {
        Ok(f) => f,
        Err(_) => return pairs,
    };

    let mut structs = HashMap::new();
    let mut impls = HashMap::new();

    for item in &file.items {
        match item {
            Item::Struct(s) => {
                let name = s.ident.to_string();
                structs.insert(name, s.clone());
            }
            Item::Impl(i) => {
                if let syn::Type::Path(p) = &*i.self_ty {
                    if let Some(segment) = p.path.segments.last() {
                        let name = segment.ident.to_string();
                        impls.entry(name).or_insert_with(Vec::new).push(i.clone());
                    }
                }
            }
            Item::Enum(e) => {
                if let Some(pair) = translate_enum(e) {
                    pairs.push(pair);
                }
            }
            _ => {}
        }
    }

    // Try to synthesize actors or tables
    for (name, s) in structs {
        if let Some(impl_blocks) = impls.get(&name) {
            // Treat as Actor if it has methods
            if let Some(pair) = translate_to_actor(&s, impl_blocks) {
                pairs.push(pair);
            }
        } else {
            // Treat as Table if it has no methods but might have derives
            if let Some(pair) = translate_to_table(&s) {
                pairs.push(pair);
            }
        }
    }

    pairs
}

fn translate_enum(e: &ItemEnum) -> Option<TranslationPair> {
    let name = e.ident.to_string();
    let original = quote::quote!(#e).to_string();
    
    let mut vox_code = format!("pub type {} = \n", name);
    for variant in &e.variants {
        let v_name = variant.ident.to_string();
        vox_code.push_str(&format!("    | {}", v_name));
        
        match &variant.fields {
            syn::Fields::Named(named) => {
                vox_code.push_str(" { ");
                let mut fields = Vec::new();
                for f in &named.named {
                    let f_name = f.ident.as_ref().unwrap().to_string();
                    fields.push(format!("{}: any", f_name)); // Simplify type
                }
                vox_code.push_str(&fields.join(", "));
                vox_code.push_str(" }");
            }
            syn::Fields::Unnamed(unnamed) => {
                // Vox tagged unions prefer named structs or empty, for unnamed we map to a unified 'value: any' or similar.
                if unnamed.unnamed.len() == 1 {
                    vox_code.push_str(" { value: any }");
                }
            }
            syn::Fields::Unit => {}
        }
        vox_code.push('\n');
    }

    Some(TranslationPair {
        instruction: "Translate this Rust enum to a Vox tagged union type.".into(),
        input_rust: original,
        output_vox: vox_code,
        confidence: 0.8,
    })
}

fn translate_to_actor(s: &ItemStruct, impls: &[ItemImpl]) -> Option<TranslationPair> {
    let name = s.ident.to_string();
    
    let mut orig_tokens = quote::quote!(#s);
    for i in impls {
        let i_tokens = quote::quote!(#i);
        orig_tokens.extend(i_tokens);
    }
    let original = orig_tokens.to_string();

    let mut vox_code = format!("actor {} {{\n", name);
    for block in impls {
        for item in &block.items {
            if let syn::ImplItem::Fn(f) = item {
                let meth_name = f.sig.ident.to_string();
                if meth_name == "new" { continue; }
                
                vox_code.push_str(&format!("    on {}() to any {{\n        pass\n    }}\n", meth_name));
            }
        }
    }
    vox_code.push_str("}\n");

    Some(TranslationPair {
        instruction: "Translate this Rust struct and its methods into a Vox actor declaration. Omit implementations and use 'pass'.".into(),
        input_rust: original,
        output_vox: vox_code,
        confidence: 0.9,
    })
}

fn translate_to_table(s: &ItemStruct) -> Option<TranslationPair> {
    let name = s.ident.to_string();
    let original = quote::quote!(#s).to_string();

    let is_serde = s.attrs.iter().any(|a| {
        a.path().is_ident("derive") && quote::quote!(#a).to_string().contains("Serialize")
    });
    
    if !is_serde {
        return None;
    }

    let mut vox_code = format!("@table type {} {{\n", name);
    if let syn::Fields::Named(named) = &s.fields {
        for f in &named.named {
            let f_name = f.ident.as_ref().unwrap().to_string();
             vox_code.push_str(&format!("    {}: any\n", f_name));
        }
    }
    vox_code.push_str("}\n");

    Some(TranslationPair {
        instruction: "Translate this serializable Rust struct into a Vox @table type.".into(),
        input_rust: original,
        output_vox: vox_code,
        confidence: 0.85,
    })
}
