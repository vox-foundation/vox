//! Drift-proofing for the spoke SSOT (mens/config/domain-profiles.yaml).
//! Exposed through vox ci spoke-check CI gate.

use crate::mens::tensor::domain_profiles::{DomainProfilesFile, TrainMethod};
use std::path::Path;

/// One human-readable validation problem.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct SpokeViolation(pub String);

#[derive(serde::Deserialize)]
struct MixLaneProbe {
    #[serde(default)]
    include_lanes: Vec<String>,
}

/// Validate every spoke in `DomainProfilesFile`. Returns all violations.
/// Rules: (1) a fine-tune method (`base:` present) requires `mix_config`; (2) a fine-tune
/// method requires `base.preset`; (3) `eval_gate`, if set, must exist on disk;
/// (4) `mix_config`, if set, must exist on disk (when `workspace_root` exists) and
/// declare non-empty `include_lanes`.
pub fn validate(file: &DomainProfilesFile, workspace_root: &Path) -> Vec<SpokeViolation> {
    let mut v = Vec::new();
    let fine_tune = |m: TrainMethod| {
        matches!(
            m,
            TrainMethod::Qlora | TrainMethod::FullSft | TrainMethod::Dpo | TrainMethod::Orpo
        )
    };
    for (name, p) in &file.profiles {
        if let Some(mc) = &p.mix_config {
            let mix_path = workspace_root.join(mc);
            if mix_path.is_file() {
                match std::fs::read_to_string(&mix_path) {
                    Ok(raw) => match serde_yaml::from_str::<MixLaneProbe>(&raw) {
                        Ok(probe) => {
                            if probe.include_lanes.is_empty()
                                || probe.include_lanes.iter().any(|l| l.trim().is_empty())
                            {
                                v.push(SpokeViolation(format!(
                                    "spoke '{name}': mix_config '{mc}' must declare non-empty include_lanes without blank entries"
                                )));
                            }
                        }
                        Err(e) => v.push(SpokeViolation(format!(
                            "spoke '{name}': failed to parse mix_config '{mc}' as YAML: {e}"
                        ))),
                    },
                    Err(e) => v.push(SpokeViolation(format!(
                        "spoke '{name}': failed to read mix_config '{mc}': {e}"
                    ))),
                }
            } else if p.base.is_none() && workspace_root.exists() {
                v.push(SpokeViolation(format!(
                    "spoke '{name}': mix_config '{mc}' not found"
                )));
            }
        }

        let Some(base) = &p.base else { continue };
        if fine_tune(base.method) {
            if p.mix_config.is_none() {
                v.push(SpokeViolation(format!(
                    "spoke '{name}': fine-tune method requires mix_config"
                )));
            }
            match &base.preset {
                None => v.push(SpokeViolation(format!(
                    "spoke '{name}': fine-tune method requires base.preset"
                ))),
                Some(preset)
                    if !crate::mens::tensor::spoke_base_resolver::KNOWN_PRESETS
                        .contains(&preset.as_str()) =>
                {
                    v.push(SpokeViolation(format!(
                        "spoke '{name}': base.preset '{preset}' is not in KNOWN_PRESETS — \
                         add it to spoke_base_resolver::KNOWN_PRESETS and \
                         contracts/mens/training-presets.v1.yaml, or fix the typo"
                    )));
                }
                Some(_) => {}
            }
        }
        if let Some(mc) = &p.mix_config
            && !workspace_root.join(mc).is_file()
        {
            v.push(SpokeViolation(format!(
                "spoke '{name}': mix_config '{mc}' not found"
            )));
        }
        if let Some(eg) = &p.eval_gate
            && !workspace_root.join(eg).is_file()
        {
            v.push(SpokeViolation(format!(
                "spoke '{name}': eval_gate '{eg}' not found"
            )));
        }
        // Rule 5: base.model resolvability check
        if !base.model.contains('/') {
            match crate::mens::tensor::spoke_base_resolver::load_overlay(workspace_root) {
                Ok(overlay) => {
                    if !overlay.contains_key(&base.model) {
                        v.push(SpokeViolation(format!(
                            "spoke '{name}': base.model capability tag '{}' not found in gpu-specs train_bases",
                            base.model
                        )));
                    }
                }
                Err(e) => {
                    v.push(SpokeViolation(format!(
                        "spoke '{name}': failed to load gpu-specs train_bases overlay: {e}"
                    )));
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mens::tensor::domain_profiles::DomainProfilesFile;

    #[test]
    fn flags_finetune_without_mix_config() {
        let yaml = r#"
profiles:
  broken:
    description: "x"
    base: { model: m, method: qlora }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let v = validate(&file, std::path::Path::new("/nonexistent"));
        assert!(
            v.iter().any(|x| x.0.contains("requires mix_config")),
            "got {v:?}"
        );
    }

    #[test]
    fn rag_only_spoke_needs_no_mix() {
        let yaml = r#"
profiles:
  docs:
    description: "x"
    base: { model: m, method: rag_only }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let v = validate(&file, std::path::Path::new("/nonexistent"));
        assert!(
            !v.iter().any(|x| x.0.contains("requires mix_config")),
            "got {v:?}"
        );
    }

    #[test]
    fn bogus_base_model_tag_violates() {
        let yaml = r#"
profiles:
  broken:
    description: "x"
    base: { model: bogus_tag, method: rag_only }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        // Since load_overlay will fail to read from /nonexistent, it will produce a violation
        let v = validate(&file, std::path::Path::new("/nonexistent"));
        assert!(
            v.iter()
                .any(|x| x.0.contains("failed to load") || x.0.contains("not found")),
            "got {v:?}"
        );
    }

    #[test]
    fn flags_unknown_preset() {
        let yaml = r#"
profiles:
  typo:
    description: "x"
    mix_config: mens/config/mix-vox-lang.yaml
    base: { model: strong_code_default, method: qlora, preset: qwen3_16gb }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let v = validate(&file, root);
        assert!(
            v.iter().any(|x| x.0.contains("not in KNOWN_PRESETS")),
            "a typo'd preset must be rejected, got {v:?}"
        );
    }

    #[test]
    fn accepts_known_preset() {
        let yaml = r#"
profiles:
  ok:
    description: "x"
    mix_config: mens/config/mix-vox-lang.yaml
    base: { model: strong_code_default, method: qlora, preset: qwen3_16g }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let v = validate(&file, root);
        assert!(
            !v.iter().any(|x| x.0.contains("KNOWN_PRESETS")),
            "a real preset must pass, got {v:?}"
        );
    }

    #[test]
    fn every_shipped_spoke_preset_is_known() {
        // Drift guard: the real domain-profiles.yaml must never reference a
        // preset the trainer would silently default on.
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let raw = std::fs::read_to_string(root.join("mens/config/domain-profiles.yaml")).unwrap();
        let file: DomainProfilesFile = serde_yaml::from_str(&raw).unwrap();
        let v = validate(&file, root);
        assert!(v.is_empty(), "shipped spoke SSOT has violations: {v:?}");
    }

    #[test]
    fn real_base_model_tag_passes() {
        let yaml = r#"
profiles:
  ok_spoke:
    description: "x"
    base: { model: strong_code_default, method: rag_only }
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let v = validate(&file, root);
        assert!(v.is_empty(), "got violations: {v:?}");
    }

    #[test]
    fn every_shipped_mix_declares_explicit_include_lanes() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap();
        let raw = std::fs::read_to_string(root.join("mens/config/domain-profiles.yaml")).unwrap();
        let file: DomainProfilesFile = serde_yaml::from_str(&raw).unwrap();
        let v = validate(&file, root);
        assert!(
            v.is_empty(),
            "shipped domain profiles & mix configs have violations: {v:?}"
        );

        let agents_raw = std::fs::read_to_string(root.join("mens/config/mix-agents.yaml")).unwrap();
        let probe: MixLaneProbe = serde_yaml::from_str(&agents_raw).unwrap();
        assert!(
            !probe.include_lanes.is_empty(),
            "mix-agents.yaml must declare non-empty include_lanes"
        );
    }

    #[test]
    fn flags_mix_config_missing_include_lanes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mix-empty.yaml"),
            "output: target/dogfood/out.jsonl\nsources: []\n",
        )
        .unwrap();
        let yaml = r#"
profiles:
  no_lanes:
    description: "x"
    mix_config: mix-empty.yaml
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let v = validate(&file, dir.path());
        assert!(
            v.iter()
                .any(|x| x.0.contains("must declare non-empty include_lanes")),
            "expected include_lanes violation, got {v:?}"
        );
    }

    #[test]
    fn flags_invalid_yaml_and_whitespace_only_include_lanes() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("mix-bad-yaml.yaml"),
            "include_lanes: [unclosed\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("mix-blank-lane.yaml"),
            "include_lanes:\n  - \"  \"\n",
        )
        .unwrap();
        let yaml = r#"
profiles:
  bad_yaml:
    description: "x"
    mix_config: mix-bad-yaml.yaml
  blank_lane:
    description: "y"
    mix_config: mix-blank-lane.yaml
"#;
        let file: DomainProfilesFile = serde_yaml::from_str(yaml).unwrap();
        let v = validate(&file, dir.path());
        assert!(
            v.iter().any(|x| x.0.contains("failed to parse mix_config")),
            "expected YAML parse violation, got {v:?}"
        );
        assert!(
            v.iter().any(|x| x.0.contains("without blank entries")),
            "expected blank-lane violation, got {v:?}"
        );
    }
}
