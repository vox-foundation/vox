use anyhow::{Context, Result, anyhow};
use std::path::Path;
use super::loader::ContractIndex;

pub fn validate(index: &ContractIndex, root: &Path) -> Result<()> {
    let schema_path = root.join("contracts/index.schema.json");
    let schema_text = std::fs::read_to_string(&schema_path)
        .with_context(|| format!("failed to read {}", schema_path.display()))?;
    let validator = crate::compile_validator_from_utf8(&schema_text, &schema_path)?;

    let index_json = serde_json::to_value(index)
        .with_context(|| "failed to serialize ContractIndex to JSON Value")?;
    crate::validate(&index_json, &validator, "contracts/index.yaml")?;

    for contract in &index.contracts {
        let p = root.join(&contract.path);
        if !p.is_file() {
            return Err(anyhow!("contract file not found: {}", contract.path));
        }

        if contract.kind == "json-schema" {
            let text = std::fs::read_to_string(&p)
                .with_context(|| format!("failed to read schema file: {}", contract.path))?;
            let schema_val: serde_json::Value = serde_json::from_str(&text)
                .with_context(|| format!("failed to parse JSON schema: {}", contract.path))?;
            
            if let Some(x_vox_version) = schema_val.get("x-vox-version").and_then(|v| v.as_i64()) {
                // filename parity check. if path is "contracts/operations/catalog.v1.schema.json"
                let file_name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let re = regex::Regex::new(r"\.v(\d+)\.").expect("regex");
                if let Some(caps) = re.captures(file_name) {
                    if let Ok(v_match) = caps[1].parse::<i64>() {
                        if v_match != x_vox_version {
                            return Err(anyhow!(
                                "{}: x-vox-version ({}) does not match filename version v{}",
                                contract.path, x_vox_version, v_match
                            ));
                        }
                    }
                }
            }
        }
    }
    
    Ok(())
}
