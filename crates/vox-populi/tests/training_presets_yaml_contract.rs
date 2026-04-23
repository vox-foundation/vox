//! `contracts/mens/training-presets.v1.yaml` lists every `KNOWN_PRESETS` id (SSOT parity).

#![cfg(feature = "mens-train")]

use serde::Deserialize;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
struct Root {
    default_base_model: String,
    presets: Vec<PresetRow>,
}

#[derive(Debug, Deserialize)]
struct PresetRow {
    id: String,
    #[serde(default)]
    aliases: Vec<String>,
}

#[test]
fn training_presets_yaml_covers_known_presets() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let path = manifest_dir.join("../../contracts/mens/training-presets.v1.yaml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let root: Root = serde_yaml::from_str(&raw).expect("parse training-presets YAML");

    assert_eq!(
        root.default_base_model,
        vox_populi::mens::DEFAULT_MODEL_ID,
        "training-presets default_base_model must match vox_populi::mens::DEFAULT_MODEL_ID"
    );

    let mut ids: HashSet<String> = HashSet::new();
    for row in &root.presets {
        ids.insert(row.id.clone());
        for a in &row.aliases {
            ids.insert(a.clone());
        }
    }

    for preset in vox_populi::mens::KNOWN_PRESETS {
        assert!(
            ids.contains(*preset),
            "KNOWN_PRESETS entry `{preset}` missing from {} (add id or alias)",
            path.display()
        );
    }
}
