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
        preset: Option<String>,
        backend: PopuliTrainBackend,
    },
    /// Spoke is RagOnly/PromptOnly — skip the training stage.
    Skip { reason: String },
}

/// Resolve a spoke's training selection. CLI overrides win.
///
/// `fixed_tier_mb: None` is live: the run's one size pick happens in
/// `run_train` (D-06), so the CLI model and preset pass through and the spoke's
/// `base.model` tag and `base.preset` are not consulted for size (D-02, D-03).
/// `Some(mb)` keeps the fixed-tier tag sizing used only by the cloud path (D-08).
pub fn resolve_training_selection(
    root: &Path,
    profile: Option<&str>,
    cli_model: Option<&str>,
    cli_preset: Option<&str>,
    fixed_tier_mb: Option<u32>,
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
    let Some(tier_mb) = fixed_tier_mb else {
        return Ok(TrainingSelection::Train {
            model: cli_model.map(str::to_string),
            preset: cli_preset.map(str::to_string),
            backend,
        });
    };

    // model: CLI wins; else resolve the spoke base.model (tag→tier fit / concrete id);
    // on failure fall back to None.
    let model = if let Some(m) = cli_model {
        Some(m.to_string())
    } else if let Some(tag) = eff
        .as_ref()
        .and_then(|e| e.base.as_ref().map(|b| b.model.clone()))
    {
        vox_populi::mens::tensor::spoke_base_resolver::resolve_base_model(root, &tag, Some(tier_mb))
            .ok()
    } else {
        None
    };
    // preset: CLI wins; else base.preset; else default.
    let preset = cli_preset
        .map(str::to_string)
        .or_else(|| {
            eff.as_ref()
                .and_then(|e| e.base.as_ref().and_then(|b| b.preset.clone()))
        })
        .unwrap_or_else(|| "qwen_4080_16g".to_string());
    Ok(TrainingSelection::Train {
        model,
        preset: Some(preset),
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
    /// The Qwen3-8B QLoRA floor: every alias tag lists that rung, so a tier at
    /// this floor resolves every tagged spoke to a Qwen3 base.
    fn qwen3_8b_tier_mb() -> u32 {
        vox_populi::mens::tensor::spoke_base_resolver::load_overlay(&root())
            .expect("overlay")["agentic_default"]
            .iter()
            .find(|b| b.hf_id == vox_populi::mens::DEFAULT_MODEL_ID)
            .expect("Qwen3-8B rung")
            .floor_mb
    }
    #[test]
    fn rust_resolves_qwen_qlora() {
        // "rust-expert" renamed to "rust" by B0.1; test updated by B0.2 audit.
        let sel =
            resolve_training_selection(&root(), Some("rust"), None, None, Some(qwen3_8b_tier_mb()))
                .unwrap();
        match sel {
            TrainingSelection::Train {
                model,
                preset,
                backend,
            } => {
                assert!(model.unwrap().contains("Qwen3"));
                assert!(!preset.unwrap().is_empty());
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
        )
        .unwrap();
        if let TrainingSelection::Train { model, .. } = sel {
            assert_eq!(model.as_deref(), Some("org/Manual"));
        } else {
            panic!()
        }
    }
    #[test]
    fn live_mode_leaves_size_and_preset_to_the_selector() {
        // The rust spoke has a base.model tag and base.preset qwen3_16g; live
        // mode consults neither (D-02, D-06).
        let sel = resolve_training_selection(&root(), Some("rust"), None, None, None).unwrap();
        assert_eq!(
            sel,
            TrainingSelection::Train {
                model: None,
                preset: None,
                backend: PopuliTrainBackend::CandleQlora,
            }
        );
    }
    #[test]
    fn live_mode_keeps_cli_values() {
        let sel =
            resolve_training_selection(&root(), Some("rust"), Some("Qwen3-4B"), Some("a100"), None)
                .unwrap();
        let TrainingSelection::Train { model, preset, .. } = sel else {
            panic!("expected Train")
        };
        assert_eq!(model.as_deref(), Some("Qwen3-4B"));
        assert_eq!(preset.as_deref(), Some("a100"));
    }
    #[test]
    fn cli_preset_overrides() {
        let sel =
            resolve_training_selection(&root(), Some("rust"), None, Some("a100"), Some(16384))
                .unwrap();
        if let TrainingSelection::Train { preset, .. } = sel {
            assert_eq!(preset.as_deref(), Some("a100"));
        } else {
            panic!()
        }
    }
    #[test]
    fn all_live_spokes_resolve_to_trainable_selection() {
        // "agents" retired by B0.1 → replaced by "tool-selection" and "argument-generation".
        let root = root();
        for spoke in ["vox-lang", "rust", "tool-selection", "argument-generation"] {
            let sel = resolve_training_selection(
                &root,
                Some(spoke),
                None,
                None,
                Some(qwen3_8b_tier_mb()),
            )
            .unwrap_or_else(|e| panic!("{spoke}: {e}"));
            match sel {
                TrainingSelection::Train { model, preset, .. } => {
                    assert!(
                        model.as_deref().unwrap_or("").contains("Qwen3"),
                        "{spoke}: {model:?}"
                    );
                    assert!(!preset.unwrap_or_default().is_empty(), "{spoke}");
                }
                other => panic!("{spoke}: expected Train, got {other:?}"),
            }
            // Below every Qwen3 rung the tag resolves nothing (never a retired
            // Qwen2.5 base); the cloud path then falls back to DEFAULT_MODEL_ID.
            let TrainingSelection::Train { model, .. } =
                resolve_training_selection(&root, Some(spoke), None, None, Some(16384))
                    .unwrap_or_else(|e| panic!("{spoke}: {e}"))
            else {
                panic!("{spoke}: expected Train");
            };
            assert_eq!(model, None, "{spoke}");
        }
    }
}
