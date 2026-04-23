//! Load [`CapabilityRegistryDoc`](crate::document::CapabilityRegistryDoc) from disk.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use crate::document::CapabilityRegistryDoc;

/// Repo-relative path to the capability registry YAML (SSOT).
pub const CAPABILITY_REGISTRY_REL: &str = "contracts/capability/capability-registry.yaml";

/// Load and parse the capability registry from `repo_root`.
pub fn load_document(repo_root: &Path) -> Result<CapabilityRegistryDoc> {
    let p = repo_root.join(CAPABILITY_REGISTRY_REL);
    let raw = fs::read_to_string(&p).with_context(|| format!("read {}", p.display()))?;
    serde_yaml::from_str(&raw).with_context(|| format!("parse {}", p.display()))
}
