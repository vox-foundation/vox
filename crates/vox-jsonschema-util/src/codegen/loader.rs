use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractIndex {
    pub schema_version: i64,
    pub contracts: Vec<ContractEntry>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContractEntry {
    pub id: String,
    pub path: String,
    pub owner: String,
    pub kind: String,
    pub description: Option<String>,
    pub enforced_by: Option<Vec<String>>,
}

pub fn load_contracts(root: &Path) -> Result<ContractIndex> {
    let index_path = root.join("contracts/index.yaml");
    let text = std::fs::read_to_string(&index_path)
        .with_context(|| format!("failed to read {}", index_path.display()))?;
    let index: ContractIndex = serde_yaml::from_str(&text)
        .with_context(|| format!("failed to parse {}", index_path.display()))?;
    Ok(index)
}
