use std::path::PathBuf;
use std::fs;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustedNode {
    pub node_id: String,
    pub pubkey_hex: String,
    pub label: Option<String>,
    pub added_at: String,
}

pub struct TrustedNodeRegistry {
    path: PathBuf,
}

impl TrustedNodeRegistry {
    pub fn new() -> Self {
        let mut path = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        path.push(".vox");
        path.push("trusted_nodes.json");
        Self { path }
    }

    fn load(&self) -> Result<HashMap<String, TrustedNode>> {
        if !self.path.exists() {
            return Ok(HashMap::new());
        }
        let data = fs::read_to_string(&self.path)?;
        let nodes: Vec<TrustedNode> = serde_json::from_str(&data)?;
        Ok(nodes.into_iter().map(|n| (n.node_id.clone(), n)).collect())
    }

    fn save(&self, nodes: &HashMap<String, TrustedNode>) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let vec: Vec<&TrustedNode> = nodes.values().collect();
        let data = serde_json::to_string_pretty(&vec)?;
        fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn add(&self, node_id: String, pubkey_hex: String, label: Option<String>) -> Result<()> {
        let mut nodes = self.load()?;
        let now = chrono::Utc::now().to_rfc3339();
        nodes.insert(node_id.clone(), TrustedNode {
            node_id,
            pubkey_hex,
            label,
            added_at: now,
        });
        self.save(&nodes)
    }

    pub fn remove(&self, node_id: &str) -> Result<bool> {
        let mut nodes = self.load()?;
        let removed = nodes.remove(node_id).is_some();
        if removed {
            self.save(&nodes)?;
        }
        Ok(removed)
    }

    pub fn is_trusted(&self, node_id: &str) -> Result<bool> {
        let nodes = self.load()?;
        Ok(nodes.contains_key(node_id))
    }

    pub fn list(&self) -> Result<Vec<TrustedNode>> {
        let nodes = self.load()?;
        let mut vec: Vec<TrustedNode> = nodes.into_values().collect();
        vec.sort_by(|a, b| a.added_at.cmp(&b.added_at));
        Ok(vec)
    }
}

impl Default for TrustedNodeRegistry {
    fn default() -> Self {
        Self::new()
    }
}
