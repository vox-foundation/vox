use std::path::Path;

use serde::{Deserialize, Serialize};

pub const EXEMPTIONS_REL: &str = "contracts/gui/coverage-exemptions.v1.yaml";
pub const SCHEMA_REL: &str = "contracts/gui/coverage-exemptions.v1.schema.json";

/// A single coverage exemption entry from `contracts/gui/coverage-exemptions.v1.yaml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Exemption {
    pub group: String,
    pub owner: String,
    pub reason: String,
    pub review_by: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ExemptionsFile {
    #[allow(dead_code)]
    pub x_vox_version: u32,
    #[allow(dead_code)]
    pub schema_version: u32,
    pub exemptions: Vec<Exemption>,
}

/// Loads and validates `contracts/gui/coverage-exemptions.v1.yaml` against its schema.
/// Returns an error if the file is missing, does not match the schema, or cannot be parsed.
pub(crate) fn load_exemptions(repo_root: &Path) -> anyhow::Result<Vec<Exemption>> {
    let path = repo_root.join(EXEMPTIONS_REL);
    if !path.is_file() {
        anyhow::bail!("coverage exemptions file missing: {}", path.display());
    }
    let content = vox_bounded_fs::read_utf8_path_capped(&path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", path.display()))?;

    let schema_path = repo_root.join(SCHEMA_REL);
    let schema_str = vox_bounded_fs::read_utf8_path_capped(&schema_path)
        .map_err(|e| anyhow::anyhow!("read {}: {e}", schema_path.display()))?;
    let schema_json: serde_json::Value = serde_json::from_str(&schema_str)
        .map_err(|e| anyhow::anyhow!("parse schema {}: {e}", schema_path.display()))?;
    let val_json: serde_json::Value = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("parse yaml {}: {e}", path.display()))?;

    let validator = vox_jsonschema_util::compile_validator(&schema_json, schema_path.display())
        .map_err(|e| anyhow::anyhow!("compile exemptions schema: {e}"))?;
    vox_jsonschema_util::validate(&val_json, &validator, "coverage-exemptions schema")
        .map_err(|e| anyhow::anyhow!("exemptions schema validation failed: {e}"))?;

    let parsed: ExemptionsFile = serde_yaml::from_str(&content)
        .map_err(|e| anyhow::anyhow!("deserialize exemptions: {e}"))?;
    Ok(parsed.exemptions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_exemptions_missing_file_is_err() {
        let missing_root = Path::new("/nonexistent/repo/root");
        let res = load_exemptions(missing_root);
        assert!(res.is_err());
        assert!(res.unwrap_err().to_string().contains("missing"));
    }

    #[test]
    fn load_exemptions_validates_schema() {
        let dir = tempfile::tempdir().unwrap();
        let schema_path = dir.path().join(SCHEMA_REL);
        std::fs::create_dir_all(schema_path.parent().unwrap()).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let real_schema = std::fs::read_to_string(root.join(SCHEMA_REL)).unwrap();
        std::fs::write(&schema_path, real_schema).unwrap();

        let exemptions_path = dir.path().join(EXEMPTIONS_REL);
        // Invalid review_by date pattern
        let bad_yaml = "x_vox_version: 2\nschema_version: 1\nexemptions:\n  - group: stop\n    owner: vox-cli\n    reason: test\n    review_by: not-a-date\n";
        std::fs::write(&exemptions_path, bad_yaml).unwrap();
        assert!(load_exemptions(dir.path()).is_err());

        // Valid YAML
        let good_yaml = "x_vox_version: 2\nschema_version: 1\nexemptions:\n  - group: stop\n    owner: vox-cli\n    reason: test\n    review_by: 2027-01-31\n";
        std::fs::write(&exemptions_path, good_yaml).unwrap();
        let loaded = load_exemptions(dir.path()).expect("load valid exemptions");
        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].group, "stop");
    }

    #[test]
    fn load_repo_exemptions() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap();
        let loaded = load_exemptions(root).expect("load repo exemptions");
        assert_eq!(loaded.len(), 14);
    }
}
