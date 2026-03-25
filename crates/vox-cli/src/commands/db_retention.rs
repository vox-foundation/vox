//! `vox db prune-plan` / `prune-apply` — read [`contracts/db/retention-policy.yaml`](../../../../contracts/db/retention-policy.yaml).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use crate::commands::ci::bounded_read::read_utf8_path_capped;

#[derive(Debug, Deserialize)]
pub struct RetentionPolicyFile {
    pub tables: HashMap<String, RetentionTableRule>,
}

#[derive(Debug, Deserialize)]
pub struct RetentionTableRule {
    pub kind: String,
    #[serde(default)]
    pub days: Option<u32>,
    #[serde(default)]
    pub time_column: Option<String>,
}

pub fn default_policy_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../contracts/db/retention-policy.yaml")
}

pub fn load_policy(path: &Path) -> anyhow::Result<RetentionPolicyFile> {
    let raw = read_utf8_path_capped(path)
        .map_err(|e| anyhow::anyhow!("read retention policy {}: {e}", path.display()))?;
    serde_yaml::from_str(&raw).map_err(|e| anyhow::anyhow!("parse retention policy: {e}"))
}

pub(crate) fn sqlite_quote_ident(name: &str) -> String {
    let mut s = String::with_capacity(name.len() + 2);
    s.push('"');
    for c in name.chars() {
        if c == '"' {
            s.push_str("\"\"");
        } else {
            s.push(c);
        }
    }
    s.push('"');
    s
}
