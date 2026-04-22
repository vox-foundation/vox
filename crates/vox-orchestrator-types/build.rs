use std::env;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
struct ProviderDef {
    name: String,
    description: String,
    is_tuple: Option<bool>,
    default_route_kind: String,
}

#[derive(serde::Deserialize)]
struct ProvidersYaml {
    providers: Vec<ProviderDef>,
}

#[derive(serde::Deserialize)]
struct RoutingYaml {
    tiers: Vec<String>,
    strengths: Vec<String>,
    task_categories: Vec<String>,
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();

    let providers_path = Path::new(&manifest_dir).join("../../contracts/orchestration/providers.v1.yaml");
    println!("cargo:rerun-if-changed={}", providers_path.display());
    
    let prov_content = fs::read_to_string(&providers_path).expect("Failed to read providers.v1.yaml");
    let prov_config: ProvidersYaml = serde_yaml::from_str(&prov_content).expect("Failed to parse providers YAML");
    
    let dest_prov_path = Path::new(&out_dir).join("generated_providers.rs");
    let mut out_p = String::new();
    out_p.push_str("// AUTO-GENERATED from contracts/orchestration/providers.v1.yaml\n");
    out_p.push_str("// DO NOT EDIT DIRECTLY.\n\n");

    out_p.push_str("#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]\n");
    out_p.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out_p.push_str("pub enum ProviderType {\n");
    for prov in &prov_config.providers {
        out_p.push_str(&format!("    /// {}\n", prov.description));
        if prov.is_tuple.unwrap_or(false) {
            out_p.push_str(&format!("    {}(String),\n", prov.name));
        } else {
            out_p.push_str(&format!("    {},\n", prov.name));
        }
    }
    out_p.push_str("}\n\n");
    
    out_p.push_str("impl ProviderType {\n");
    out_p.push_str("    pub fn default_backend(&self) -> crate::ChatRouteBackend {\n");
    out_p.push_str("        match self {\n");
    for prov in &prov_config.providers {
        let backend = match prov.default_route_kind.as_str() {
            "GeminiDirect" => "crate::ChatRouteBackend::GeminiDirect",
            "OpenRouter" => "crate::ChatRouteBackend::OpenRouter",
            "Ollama" => "crate::ChatRouteBackend::Ollama",
            "PopuliMesh" => "crate::ChatRouteBackend::PopuliMesh",
            _ => "crate::ChatRouteBackend::CascadeFallback",
        };
        if prov.is_tuple.unwrap_or(false) {
            out_p.push_str(&format!("            Self::{}(_) => {},\n", prov.name, backend));
        } else {
            out_p.push_str(&format!("            Self::{} => {},\n", prov.name, backend));
        }
    }
    out_p.push_str("        }\n");
    out_p.push_str("    }\n");
    out_p.push_str("}\n\n");

    // Model Routing Enums
    let routing_path = Path::new(&manifest_dir).join("../../contracts/orchestration/model-routing.v1.yaml");
    println!("cargo:rerun-if-changed={}", routing_path.display());
    let routing_content = fs::read_to_string(&routing_path).expect("Failed to read model-routing.v1.yaml");
    let routing_config: RoutingYaml = serde_yaml::from_str(&routing_content).expect("Failed to parse routing YAML");

    // ModelTier
    out_p.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, Default)]\n");
    out_p.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out_p.push_str("pub enum ModelTier {\n");
    for tier in &routing_config.tiers {
        if tier == "Unknown" { out_p.push_str("    #[default]\n"); }
        out_p.push_str(&format!("    {},\n", tier));
    }
    out_p.push_str("}\n\n");

    // StrengthTag
    out_p.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize, Default)]\n");
    out_p.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out_p.push_str("pub enum StrengthTag {\n");
    out_p.push_str("    #[default]\n    Unknown,\n");
    let mut strength_variants = Vec::new();
    for s in &routing_config.strengths {
        let pascal = s.split(['-', '_']).map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            }
        }).collect::<String>();
        out_p.push_str(&format!("    {},\n", pascal));
        strength_variants.push((pascal, s.clone()));
    }
    out_p.push_str("}\n\n");

    out_p.push_str("impl std::fmt::Display for StrengthTag {\n");
    out_p.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    out_p.push_str("        match self {\n            Self::Unknown => write!(f, \"unknown\"),\n");
    for (pascal, raw) in &strength_variants {
        out_p.push_str(&format!("            Self::{} => write!(f, \"{}\"),\n", pascal, raw));
    }
    out_p.push_str("        }\n    }\n}\n\n");

    out_p.push_str("impl std::str::FromStr for StrengthTag {\n    type Err = ();\n    fn from_str(s: &str) -> Result<Self, Self::Err> {\n        match s {\n");
    for (pascal, raw) in &strength_variants {
        out_p.push_str(&format!("            \"{}\" => Ok(Self::{}),\n", raw, pascal));
    }
    out_p.push_str("            _ => Ok(Self::Unknown),\n        }\n    }\n}\n\n");

    // TaskCategory
    out_p.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize, Default)]\n");
    out_p.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out_p.push_str("pub enum TaskCategory {\n    #[default]\n    General,\n");
    let mut category_variants = Vec::new();
    for c in &routing_config.task_categories {
        if c == "General" { continue; }
        out_p.push_str(&format!("    {},\n", c));
        category_variants.push((c.clone(), c.to_lowercase()));
    }
    out_p.push_str("}\n\n");

    out_p.push_str("impl std::fmt::Display for TaskCategory {\n");
    out_p.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    out_p.push_str("        match self {\n            Self::General => write!(f, \"general\"),\n");
    for (pascal, raw) in &category_variants {
        out_p.push_str(&format!("            Self::{} => write!(f, \"{}\"),\n", pascal, raw));
    }
    out_p.push_str("        }\n    }\n}\n\n");

    out_p.push_str("impl std::str::FromStr for TaskCategory {\n    type Err = ();\n    fn from_str(s: &str) -> Result<Self, Self::Err> {\n        match s.to_lowercase().as_str() {\n");
    out_p.push_str("            \"general\" => Ok(Self::General),\n");
    for (pascal, raw) in &category_variants {
        out_p.push_str(&format!("            \"{}\" => Ok(Self::{}),\n", raw, pascal));
    }
    out_p.push_str("            _ => Ok(Self::General),\n        }\n    }\n}\n\n");

    fs::write(&dest_prov_path, out_p).unwrap();
}
