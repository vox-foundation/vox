//! Pure, GPU-independent resolution of a spoke's training selection
//! (base model + preset + training backend, or a "skip"/"unwired" outcome).
//! Extracted from pipeline.rs so it is unit-testable and dry-run-observable
//! WITHOUT the `gpu` feature (AGH-0012 F1).
use std::path::Path;
use vox_populi::mens::PopuliTrainBackend;
use vox_populi::mens::tensor::domain_profiles::{EffectiveDomainProfile, TrainMethod};
use vox_populi::mens::tensor::finetune_contract::AdapterMethod;
use vox_populi::mens::tensor::finetune_registry::AdapterMethodRegistry;

#[derive(Debug, PartialEq)]
pub enum TrainingSelection {
    /// Train this base+preset with this backend.
    Train {
        model: Option<String>,
        preset: String,
        backend: PopuliTrainBackend,
    },
    /// Spoke is RagOnly/PromptOnly — skip the training stage.
    Skip { reason: String },
}

/// Resolve a spoke's training selection. `vram_mb_override` is the injected
/// free-memory reading. An absent model uses the measured qwen3_code QLoRA
/// picker and errors when the reading is missing. CLI overrides win.
pub fn resolve_training_selection(
    root: &Path,
    profile: Option<&str>,
    cli_model: Option<&str>,
    cli_preset: Option<&str>,
    vram_mb_override: Option<u32>,
    env_default_model: Option<&str>,
) -> anyhow::Result<TrainingSelection> {
    let eff = profile.and_then(|n| EffectiveDomainProfile::load_domain_profile(n, Some(root)).ok());
    let method = eff
        .as_ref()
        .and_then(|e| e.base.as_ref().map(|b| b.method))
        .unwrap_or(TrainMethod::Qlora);
    if matches!(method, TrainMethod::RagOnly | TrainMethod::PromptOnly) {
        return Ok(TrainingSelection::Skip {
            reason: format!("{method:?}"),
        });
    }
    let backend = match method {
        TrainMethod::Qlora => AdapterMethodRegistry::builtin()
            .resolve(AdapterMethod::Qlora)
            .map(|r| r.default_kernel)
            .ok_or_else(|| anyhow::anyhow!("AdapterMethodRegistry missing Qlora kernel"))?,
        TrainMethod::FullSft | TrainMethod::Dpo | TrainMethod::Orpo => {
            anyhow::bail!("training method {method:?} has no wired backend")
        }
        TrainMethod::RagOnly | TrainMethod::PromptOnly => unreachable!(),
    };

    // Absent model: one measured qwen3_code QLoRA size for every spoke.
    // A size spelling is explicit and is not stepped down. A capability tag
    // stays on resolve_base_model / pick_base and keeps the profile preset.
    let profile_preset = eff
        .as_ref()
        .and_then(|e| e.base.as_ref().and_then(|b| b.preset.clone()));
    let pinned = env_default_model
        .map(str::trim)
        .filter(|value| !value.is_empty());
    // CLI --model wins. The live selector runs only when both names are absent.
    let named = cli_model.or(pinned);
    let (model, size_preset) = if let Some(m) = named {
        if m.contains('/') {
            (Some(m.to_string()), None)
        } else if vox_populi::mens::tensor::spoke_base_resolver::is_qwen3_size_spelling(m) {
            let free_mb = vram_mb_override.ok_or_else(|| {
                anyhow::anyhow!("no free-memory reading; refusing explicit Qwen3 size '{m}'")
            })?;
            let overlay = vox_populi::mens::tensor::spoke_base_resolver::load_overlay(root)?;
            let pick =
                vox_populi::mens::tensor::spoke_base_resolver::explicit_qwen3_size_for_free_mb(
                    &overlay, m, free_mb,
                )?;
            (Some(pick.hf_id), Some(pick.preset))
        } else {
            let id = vox_populi::mens::tensor::spoke_base_resolver::resolve_base_model(
                root,
                m,
                vram_mb_override,
            )?;
            (Some(id), None)
        }
    } else if let Some(free_mb) = vram_mb_override {
        let pick =
            vox_populi::mens::tensor::spoke_base_resolver::select_qwen3_code_qlora_for_free_mb(
                free_mb,
            )?;
        (Some(pick.hf_id), Some(pick.preset))
    } else {
        anyhow::bail!("no free-memory reading; cannot select a qwen3_code QLoRA size")
    };
    // preset: CLI wins; else the selector preset; else the spoke profile preset.
    let preset = cli_preset
        .map(str::to_string)
        .or(size_preset)
        .or(profile_preset)
        .unwrap_or_else(|| "qwen_4080_16g".to_string());
    Ok(TrainingSelection::Train {
        model,
        preset,
        backend,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn root() -> std::path::PathBuf {
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .ancestors()
            .nth(2)
            .unwrap()
            .to_path_buf()
    }
    #[test]
    fn rust_resolves_qwen_qlora() {
        // "rust-expert" renamed to "rust" by B0.1; test updated by B0.2 audit.
        let sel = resolve_training_selection(&root(), Some("rust"), None, None, Some(16384), None)
            .unwrap();
        match sel {
            TrainingSelection::Train {
                model,
                preset,
                backend,
            } => {
                assert!(model.unwrap().contains("Qwen"));
                assert!(!preset.is_empty());
                assert!(matches!(backend, PopuliTrainBackend::CandleQlora));
            }
            _ => panic!("expected Train"),
        }
    }
    #[test]
    fn cli_model_overrides() {
        // "rust-expert" renamed to "rust" by B0.1; updated by B0.2 audit.
        let sel = resolve_training_selection(
            &root(),
            Some("rust"),
            Some("org/Manual"),
            None,
            Some(16384),
            None,
        )
        .unwrap();
        if let TrainingSelection::Train { model, .. } = sel {
            assert_eq!(model.as_deref(), Some("org/Manual"));
        } else {
            panic!()
        }
    }
    #[test]
    fn env_pin_wins_over_an_injected_free_memory_reading() {
        let fourteen = "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18";
        let pin = "org/Pinned-Not-Qwen";
        // 25_000 MB selects 14B when no name is pinned.
        let sel =
            resolve_training_selection(&root(), Some("rust"), None, None, Some(25_000), Some(pin))
                .expect("pin");
        match sel {
            TrainingSelection::Train { model, .. } => {
                assert_eq!(model.as_deref(), Some(pin));
                assert_ne!(model.as_deref(), Some(fourteen));
            }
            other => panic!("expected Train, got {other:?}"),
        }
        let blank = resolve_training_selection(
            &root(),
            Some("rust"),
            None,
            None,
            Some(25_000),
            Some("   "),
        )
        .expect("blank pin falls through");
        match blank {
            TrainingSelection::Train { model, .. } => {
                assert_eq!(model.as_deref(), Some(fourteen));
            }
            other => panic!("expected Train, got {other:?}"),
        }
        let cli_wins = resolve_training_selection(
            &root(),
            Some("rust"),
            Some("org/Cli"),
            None,
            Some(25_000),
            Some(pin),
        )
        .expect("cli wins");
        match cli_wins {
            TrainingSelection::Train { model, .. } => {
                assert_eq!(model.as_deref(), Some("org/Cli"));
            }
            other => panic!("expected Train, got {other:?}"),
        }
    }

    #[test]
    fn unnamed_size_without_a_reading_is_an_error() {
        let err = resolve_training_selection(&root(), Some("rust"), None, None, None, None)
            .expect_err("missing free-memory reading");
        let msg = err.to_string();
        assert!(
            msg.contains("free-memory"),
            "expected a missing-reading error, got {msg}"
        );
    }

    #[test]
    fn unnamed_size_uses_one_qwen3_code_size_for_every_spoke() {
        let spokes = ["vox-lang", "rust", "tool-selection", "argument-generation"];
        let fourteen = "Qwen/Qwen3-14B@40c069824f4251a91eefaf281ebe4c544efd3e18";
        let eight = "Qwen/Qwen3-8B@b968826d9c46dd6066d109eabc6255188de91218";
        for spoke in spokes {
            let sel =
                resolve_training_selection(&root(), Some(spoke), None, None, Some(25_000), None)
                    .unwrap_or_else(|e| panic!("{spoke}: {e}"));
            match sel {
                TrainingSelection::Train { model, preset, .. } => {
                    assert_eq!(model.as_deref(), Some(fourteen), "{spoke}");
                    assert_eq!(preset, "qwen3_24g", "{spoke}");
                }
                other => panic!("{spoke}: expected Train, got {other:?}"),
            }
            let at_16 =
                resolve_training_selection(&root(), Some(spoke), None, None, Some(16_384), None)
                    .unwrap_or_else(|e| panic!("{spoke} 16g: {e}"));
            match at_16 {
                TrainingSelection::Train { model, preset, .. } => {
                    assert_eq!(model.as_deref(), Some(eight), "{spoke}");
                    assert_eq!(preset, "qwen3_16g", "{spoke}");
                }
                other => panic!("{spoke}: expected Train, got {other:?}"),
            }
        }
        let kept_preset = resolve_training_selection(
            &root(),
            Some("rust"),
            None,
            Some("a100"),
            Some(16_384),
            None,
        )
        .expect("cli preset");
        match kept_preset {
            TrainingSelection::Train { model, preset, .. } => {
                assert_eq!(model.as_deref(), Some(eight));
                assert_eq!(preset, "a100");
            }
            other => panic!("expected Train, got {other:?}"),
        }
        let manual = resolve_training_selection(
            &root(),
            Some("rust"),
            Some("org/Manual"),
            None,
            Some(16_384),
            None,
        )
        .expect("concrete id");
        match manual {
            TrainingSelection::Train { model, .. } => {
                assert_eq!(model.as_deref(), Some("org/Manual"));
            }
            other => panic!("expected Train, got {other:?}"),
        }
        let small = resolve_training_selection(
            &root(),
            Some("rust"),
            Some("small_code_default"),
            None,
            Some(100_000),
            None,
        )
        .expect("small_code_default");
        match small {
            TrainingSelection::Train { model, preset, .. } => {
                assert_eq!(model.as_deref(), Some(eight));
                assert_eq!(preset, "qwen3_16g");
            }
            other => panic!("expected Train, got {other:?}"),
        }
        let strong = resolve_training_selection(
            &root(),
            Some("rust"),
            Some("strong_code_default"),
            None,
            Some(16_384),
            None,
        )
        .expect("strong_code_default");
        match strong {
            TrainingSelection::Train { model, .. } => {
                assert_eq!(model.as_deref(), Some(eight));
            }
            other => panic!("expected Train, got {other:?}"),
        }
        assert!(
            resolve_training_selection(
                &root(),
                Some("rust"),
                Some("32B"),
                None,
                Some(25_000),
                None
            )
            .is_err()
        );
        assert!(
            resolve_training_selection(
                &root(),
                Some("rust"),
                Some("1.7B"),
                None,
                Some(100_000),
                None
            )
            .is_err()
        );
        assert!(resolve_training_selection(&root(), Some("rust"), None, None, None, None).is_err());
    }
    #[test]
    fn cli_preset_overrides() {
        let sel = resolve_training_selection(
            &root(),
            Some("rust"),
            None,
            Some("a100"),
            Some(16384),
            None,
        )
        .unwrap();
        if let TrainingSelection::Train { preset, .. } = sel {
            assert_eq!(preset, "a100");
        } else {
            panic!()
        }
    }
    #[test]
    fn all_live_spokes_resolve_to_trainable_selection() {
        // "agents" retired by B0.1 → replaced by "tool-selection" and "argument-generation".
        let root = root();
        for spoke in ["vox-lang", "rust", "tool-selection", "argument-generation"] {
            let sel = resolve_training_selection(&root, Some(spoke), None, None, Some(16384), None)
                .unwrap_or_else(|e| panic!("{spoke}: {e}"));
            match sel {
                TrainingSelection::Train { model, preset, .. } => {
                    assert!(
                        model.as_deref().unwrap_or("").contains("Qwen"),
                        "{spoke}: {model:?}"
                    );
                    assert!(!preset.is_empty(), "{spoke}");
                }
                other => panic!("{spoke}: expected Train, got {other:?}"),
            }
        }
    }
}
