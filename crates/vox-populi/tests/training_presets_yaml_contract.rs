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

/// Third leg of the SSOT chain: does the independent `gpu-specs.yaml`
/// resolver (`spoke_base_resolver::pick_base`, reading `train_bases.agentic_default`)
/// agree with `training-presets.v1.yaml::default_base_model` /
/// `vox_populi::mens::DEFAULT_MODEL_ID` at the 16GB-tier floor? The two YAML
/// files are edited independently and nothing else gates their agreement.
#[test]
fn gpu_specs_agentic_default_16g_floor_matches_training_presets_default() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root");

    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&workspace_root)
        .expect("load mens/config/gpu-specs.yaml train_bases");
    let rungs = overlay
        .get("agentic_default")
        .expect("agentic_default tag present in gpu-specs.yaml");
    // The "16GB-tier floor" is the floor_mb of the rung matching DEFAULT_MODEL_ID
    // itself (per the SSOT comment: "agentic_default 16GB-tier rung") — not the
    // lowest floor_mb in the list, which belongs to a different (Coder-7B) rung.
    let floor_mb = rungs
        .iter()
        .find(|b| b.hf_id == vox_populi::mens::DEFAULT_MODEL_ID)
        .unwrap_or_else(|| {
            panic!(
                "agentic_default has no rung matching DEFAULT_MODEL_ID ({}); rungs: {rungs:?}",
                vox_populi::mens::DEFAULT_MODEL_ID
            )
        })
        .floor_mb;

    let base = vox_populi::mens::tensor::spoke_base_resolver::pick_base(
        &overlay,
        "agentic_default",
        floor_mb,
    )
    .unwrap_or_else(|e| panic!("pick_base at the 16GB-tier floor ({floor_mb} MB): {e}"));

    assert_eq!(
        base.hf_id,
        vox_populi::mens::DEFAULT_MODEL_ID,
        "gpu-specs.yaml agentic_default's floor rung ({floor_mb} MB) must match \
         vox_populi::mens::DEFAULT_MODEL_ID (and training-presets.v1.yaml::default_base_model)"
    );

    let path = manifest_dir.join("../../contracts/mens/training-presets.v1.yaml");
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    let root: Root = serde_yaml::from_str(&raw).expect("parse training-presets YAML");
    assert_eq!(
        base.hf_id, root.default_base_model,
        "gpu-specs.yaml agentic_default's floor rung ({floor_mb} MB) must match \
         training-presets.v1.yaml::default_base_model"
    );
}

/// Every Qwen3 rung's `hf_id` in `agentic_default` must carry a real-shaped
/// pinned revision: `@` followed by exactly 40 hex characters (a git commit
/// SHA), not the literal placeholder text `ensure_not_placeholder()` checks
/// for. Scoped to Qwen3 entries — the legacy Qwen2.5-Coder rungs are
/// intentionally left floating to `main` and were never claimed to be pinned.
#[test]
fn gpu_specs_agentic_default_qwen3_hf_ids_have_real_sha_format() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("canonicalize workspace root");
    let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&workspace_root)
        .expect("load mens/config/gpu-specs.yaml train_bases");
    let rungs = overlay
        .get("agentic_default")
        .expect("agentic_default tag present in gpu-specs.yaml");

    let qwen3_rungs: Vec<_> = rungs.iter().filter(|b| b.hf_id.contains("Qwen3")).collect();
    assert!(
        !qwen3_rungs.is_empty(),
        "expected at least one Qwen3 rung in agentic_default"
    );
    for base in qwen3_rungs {
        let (_, sha) = base
            .hf_id
            .split_once('@')
            .unwrap_or_else(|| panic!("Qwen3 hf_id missing '@<sha>' pin: {}", base.hf_id));
        assert_eq!(
            sha.len(),
            40,
            "hf_id revision must be a 40-char git SHA, got {}-char '{sha}' in {}",
            sha.len(),
            base.hf_id
        );
        assert!(
            sha.chars().all(|c| c.is_ascii_hexdigit()),
            "hf_id revision must be hex, got '{sha}' in {}",
            base.hf_id
        );
    }
}
