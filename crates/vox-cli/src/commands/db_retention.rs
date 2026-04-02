//! `vox db prune-plan` / `prune-apply` — read [`contracts/db/retention-policy.yaml`](../../../../contracts/db/retention-policy.yaml).
//!
//! Supported `kind` values consumed by the CLI today: `days` (datetime column), `ms_days` (Unix millis),
//! `expires_lt_now` (TTL TEXT column versus `datetime('now')`).
//! Other kinds are listed in the YAML for documentation only.
//!
//! Architecture notes (sensitivity, `ci_completion_*`, etc.): `docs/src/architecture/telemetry-retention-sensitivity-ssot.md`.

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_retention_policy_parses_and_includes_completion_tables() {
        let path = default_policy_path();
        assert!(
            path.is_file(),
            "expected retention policy at {}",
            path.display()
        );
        let pol = load_policy(&path).expect("parse retention policy");
        let run = pol
            .tables
            .get("ci_completion_run")
            .expect("ci_completion_run rule");
        assert_eq!(run.kind, "days");
        assert_eq!(run.days, Some(365));
        assert_eq!(run.time_column.as_deref(), Some("finished_at"));

        let sup = pol
            .tables
            .get("ci_completion_suppression")
            .expect("ci_completion_suppression rule");
        assert_eq!(sup.kind, "expires_lt_now");
        assert_eq!(sup.time_column.as_deref(), Some("expires_at"));
    }
}
