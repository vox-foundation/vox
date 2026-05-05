use std::env;
use std::fs;
use std::path::Path;

#[derive(serde::Deserialize)]
struct RoutingYaml {
    tiers: Vec<String>,
    strengths: Vec<String>,
    task_categories: Vec<String>,
    strength_inference: Vec<serde_yaml::Value>,
}

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let yaml_path =
        Path::new(&manifest_dir).join("../../contracts/orchestration/model-routing.v1.yaml");

    println!("cargo:rerun-if-changed={}", yaml_path.display());

    let yaml_content =
        fs::read_to_string(&yaml_path).expect("Failed to read model-routing.v1.yaml");
    let config: RoutingYaml = serde_yaml::from_str(&yaml_content).expect("Failed to parse YAML");

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("generated.rs");

    let mut out = String::new();
    out.push_str("// AUTO-GENERATED from contracts/orchestration/model-routing.v1.yaml\n");
    out.push_str("// DO NOT EDIT DIRECTLY.\n\n");

    // Generate ModelTier
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize, Default)]\n");
    out.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out.push_str("pub enum ModelTier {\n");
    for tier in &config.tiers {
        if tier == "Unknown" {
            out.push_str("    #[default]\n");
        }
        out.push_str(&format!("    {},\n", tier));
    }
    out.push_str("}\n\n");

    out.push_str("impl std::fmt::Display for ModelTier {\n");
    out.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    out.push_str("        match self {\n");
    for tier in &config.tiers {
        out.push_str(&format!(
            "            Self::{} => write!(f, \"{}\"),\n",
            tier,
            tier.to_lowercase()
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Generate StrengthTag
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize, Default)]\n");
    out.push_str("#[serde(rename_all = \"snake_case\")]\n");
    out.push_str("pub enum StrengthTag {\n");
    out.push_str("    #[default]\n");
    out.push_str("    Unknown,\n");
    let mut strength_variants = Vec::new();
    for strength in &config.strengths {
        let pascal = strength
            .split(['-', '_'])
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>();
        out.push_str(&format!("    {},\n", pascal));
        strength_variants.push((pascal, strength.clone()));
    }
    out.push_str("}\n\n");

    out.push_str("impl std::fmt::Display for StrengthTag {\n");
    out.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    out.push_str("        match self {\n");
    out.push_str("            Self::Unknown => write!(f, \"unknown\"),\n");
    for (pascal, raw) in &strength_variants {
        out.push_str(&format!(
            "            Self::{} => write!(f, \"{}\"),\n",
            pascal, raw
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("impl std::str::FromStr for StrengthTag {\n");
    out.push_str("    type Err = ();\n");
    out.push_str("    fn from_str(s: &str) -> Result<Self, Self::Err> {\n");
    out.push_str("        match s {\n");
    for (pascal, raw) in &strength_variants {
        out.push_str(&format!(
            "            \"{}\" => Ok(Self::{}),\n",
            raw, pascal
        ));
    }
    out.push_str("            _ => Ok(Self::Unknown),\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Generate TaskCategory
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize, Default)]\n");
    out.push_str("pub enum TaskCategory {\n");
    for cat in &config.task_categories {
        if cat == "General" {
            out.push_str("    #[default]\n");
        }
        out.push_str(&format!("    {},\n", cat));
    }
    out.push_str("}\n\n");

    out.push_str("impl std::fmt::Display for TaskCategory {\n");
    out.push_str("    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {\n");
    out.push_str("        match self {\n");
    for cat in &config.task_categories {
        out.push_str(&format!(
            "            Self::{} => write!(f, \"{}\"),\n",
            cat, cat
        ));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    out.push_str("impl std::str::FromStr for TaskCategory {\n");
    out.push_str("    type Err = ();\n");
    out.push_str("    fn from_str(s: &str) -> Result<Self, Self::Err> {\n");
    out.push_str("        match s.to_lowercase().as_str() {\n");
    for cat in &config.task_categories {
        out.push_str(&format!(
            "            \"{}\" => Ok(Self::{}),\n",
            cat.to_lowercase(),
            cat
        ));
    }
    out.push_str("            _ => Ok(Self::General),\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Generate infer_strengths()
    out.push_str("#[must_use]\n");
    out.push_str("pub fn infer_strengths(id: &str, description: Option<&str>, supported_parameters: &[String]) -> Vec<StrengthTag> {\n");
    out.push_str("    let mut strengths = std::collections::BTreeSet::new();\n");
    out.push_str("    let provider_prefix = id.split('/').next().unwrap_or(\"\");\n");
    out.push_str("    let mut haystack = id.to_ascii_lowercase();\n");
    out.push_str("    if let Some(desc) = description {\n");
    out.push_str("        haystack.push(' ');\n");
    out.push_str("        haystack.push_str(&desc.to_ascii_lowercase());\n");
    out.push_str("    }\n\n");

    // Parse the yaml loosely using serde_yaml::Value
    let empty_vec = vec![];
    for block in &config.strength_inference {
        if let Some(mapping) = block.as_mapping() {
            let type_str = mapping
                .get(serde_yaml::Value::String("type".to_string()))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let rules = mapping
                .get(serde_yaml::Value::String("rules".to_string()))
                .and_then(|v| v.as_sequence())
                .map(|v| v.as_slice())
                .unwrap_or(&[]);

            match type_str {
                "parameter_graph" => {
                    for rule in rules {
                        let params = rule
                            .get("parameters")
                            .and_then(|v| v.as_sequence())
                            .unwrap_or(&empty_vec);
                        let strengths_to_add = rule
                            .get("strengths")
                            .and_then(|v| v.as_sequence())
                            .unwrap_or(&empty_vec);
                        out.push_str("    let has_param = ");
                        let mut param_checks = Vec::new();
                        for p in params {
                            if let Some(p_str) = p.as_str() {
                                param_checks.push(format!(
                                    "supported_parameters.iter().any(|p| p == \"{}\")",
                                    p_str
                                ));
                            }
                        }
                        if param_checks.is_empty() {
                            out.push_str("false;\n");
                        } else {
                            out.push_str(&param_checks.join(" || "));
                            out.push_str(";\n");
                            out.push_str("    if has_param {\n");
                            for s in strengths_to_add {
                                if let Some(s_str) = s.as_str()
                                    && let Some((pascal, _)) =
                                        strength_variants.iter().find(|(_, raw)| raw == s_str)
                                {
                                    out.push_str(&format!(
                                        "        strengths.insert(StrengthTag::{});\n",
                                        pascal
                                    ));
                                }
                            }
                            out.push_str("    }\n");
                        }
                    }
                }
                "provider_family" => {
                    out.push_str("    match provider_prefix {\n");
                    for rule in rules {
                        if let Some(match_str) = rule.get("match").and_then(|v| v.as_str()) {
                            let strengths_to_add = rule
                                .get("strengths")
                                .and_then(|v| v.as_sequence())
                                .unwrap_or(&empty_vec);
                            out.push_str(&format!("        \"{}\" => {{\n", match_str));
                            for s in strengths_to_add {
                                if let Some(s_str) = s.as_str()
                                    && let Some((pascal, _)) =
                                        strength_variants.iter().find(|(_, raw)| raw == s_str)
                                {
                                    out.push_str(&format!(
                                        "            strengths.insert(StrengthTag::{});\n",
                                        pascal
                                    ));
                                }
                            }
                            out.push_str("        }\n");
                        }
                    }
                    out.push_str("        _ => {}\n");
                    out.push_str("    }\n");
                }
                "name_regex" => {
                    out.push_str("    if strengths.is_empty() {\n");
                    for rule in rules {
                        if let Some(pattern) = rule.get("pattern").and_then(|v| v.as_str()) {
                            let strengths_to_add = rule
                                .get("strengths")
                                .and_then(|v| v.as_sequence())
                                .unwrap_or(&empty_vec);
                            let inner = pattern.replace(['(', ')'], "");
                            let parts: Vec<&str> = inner.split('|').collect();
                            out.push_str("        if ");
                            let checks: Vec<String> = parts
                                .iter()
                                .map(|p| format!("haystack.contains(\"{}\")", p))
                                .collect();
                            out.push_str(&checks.join(" || "));
                            out.push_str(" {\n");
                            for s in strengths_to_add {
                                if let Some(s_str) = s.as_str()
                                    && let Some((pascal, _)) =
                                        strength_variants.iter().find(|(_, raw)| raw == s_str)
                                {
                                    out.push_str(&format!(
                                        "            strengths.insert(StrengthTag::{});\n",
                                        pascal
                                    ));
                                }
                            }
                            out.push_str("        }\n");
                        }
                    }
                    out.push_str("    }\n");
                }
                "default" => {
                    out.push_str("    if strengths.is_empty() {\n");
                    for rule in rules {
                        let strengths_to_add = rule
                            .get("strengths")
                            .and_then(|v| v.as_sequence())
                            .unwrap_or(&empty_vec);
                        for s in strengths_to_add {
                            if let Some(s_str) = s.as_str()
                                && let Some((pascal, _)) =
                                    strength_variants.iter().find(|(_, raw)| raw == s_str)
                            {
                                out.push_str(&format!(
                                    "        strengths.insert(StrengthTag::{});\n",
                                    pascal
                                ));
                            }
                        }
                    }
                    out.push_str("    }\n");
                }
                _ => {}
            }
        }
    }

    out.push_str("    strengths.into_iter().collect()\n");
    out.push_str("}\n");

    fs::write(&dest_path, out).unwrap();
}
