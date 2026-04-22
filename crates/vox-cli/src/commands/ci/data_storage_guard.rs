use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;
use crate::commands::ci::cmd_enums::GuardOpts;

#[derive(Debug, Clone, serde::Serialize)]
pub struct GuardReport {
    pub violations: Vec<String>,
    pub files_scanned: u32,
}

impl GuardReport {
    pub fn empty() -> Self {
        Self {
            violations: Vec::new(),
            files_scanned: 0,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct DataStoragePolicy {
    pub version: u32,
    #[serde(rename = "x-vox-version")]
    pub x_vox_version: u32,
}

pub fn load_policy(path: &Path) -> Result<DataStoragePolicy> {
    let yaml = std::fs::read_to_string(path).context("read policy yaml")?;
    let val: serde_json::Value = serde_yaml::from_str(&yaml).context("parse yaml")?;
    
    let schema_path = path.parent().unwrap().join("data-storage-policy.v1.schema.json");
    let schema_str = std::fs::read_to_string(&schema_path).context("read policy schema")?;
    let schema_val: serde_json::Value = serde_json::from_str(&schema_str).context("parse schema")?;
    
    let validator = vox_jsonschema_util::compile_validator(&schema_val, schema_path.display())?;
    vox_jsonschema_util::validate(&val, &validator, "data storage policy")?;
    
    serde_json::from_value(val).context("deserialize policy")
}

pub fn run(opts: &GuardOpts) -> Result<GuardReport> {
    let root = std::env::current_dir()?;
    let policy_path = root.join("contracts/db/data-storage-policy.v1.yaml");
    
    if opts.check_policy_only {
        let _ = load_policy(&policy_path)?;
        if !opts.json {
            println!("Policy valid.");
        }
        return Ok(GuardReport::empty());
    }

    let mut report = GuardReport::empty();
    
    let schemas_path = root.join("schemas");
    if schemas_path.exists() {
        report.violations.push("schemas-dir-absent: schemas directory exists but is forbidden by policy.".to_string());
    }

    if !opts.json {
        if report.violations.is_empty() {
            println!("DataStorageGuard check passed.");
        } else {
            println!("DataStorageGuard check failed with violations: {:#?}", report.violations);
        }
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bad_policy_fails() {
        let root = std::env::current_dir().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
        // Since we are running in crates/vox-cli, the workspace root is `../../`
        // Wait, current_dir() during test is `crates/vox-cli`. 
        // Let's use env!("CARGO_MANIFEST_DIR") and go up.
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let bad_policy_path = manifest.join("../../tests/fixtures/bad-data-storage-policy.yaml");
        let res = load_policy(&bad_policy_path);
        assert!(res.is_err(), "Bad policy should fail schema validation");
    }
}
