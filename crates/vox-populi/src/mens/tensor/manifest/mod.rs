//! Run manifests and architecture params for checkpoints / serve validation.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::lora::{DEFAULT_D_MODEL, DEFAULT_N_HEADS, DEFAULT_N_LAYERS};
use vox_tensor::data::VOCAB_SIZE;

/// Bumped when new required semantics appear; readers use [`load_manifest`] (serde defaults).
pub const TRAINING_MANIFEST_SCHEMA_VERSION: u32 = 4;

/// Architecture loaded from disk or defaulted for scratch training.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchParams {
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
}

impl Default for ArchParams {
    fn default() -> Self {
        Self {
            vocab_size: VOCAB_SIZE,
            d_model: DEFAULT_D_MODEL,
            n_heads: DEFAULT_N_HEADS,
            n_layers: DEFAULT_N_LAYERS,
        }
    }
}

/// Checkpoint flavor for validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckpointKind {
    Lora,
    Merged,
}

/// Parameters passed to [`validate_checkpoint_manifest`].
#[derive(Debug, Clone)]
pub struct ValidateParams {
    pub kind: Option<CheckpointKind>,
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
}

impl ArchParams {
    pub fn from_manifest(run_dir: &Path) -> anyhow::Result<Self> {
        if let Ok(Some(m)) = load_manifest(run_dir) {
            return Ok(Self {
                vocab_size: m.vocab_size,
                d_model: m.d_model,
                n_heads: m.n_heads,
                n_layers: m.n_layers,
            });
        }
        Ok(Self::default())
    }

    pub fn to_validate_params(&self, kind: Option<CheckpointKind>) -> ValidateParams {
        ValidateParams {
            kind,
            vocab_size: self.vocab_size,
            d_model: self.d_model,
            n_heads: self.n_heads,
            n_layers: self.n_layers,
        }
    }
}

/// On-disk training manifest (written by `run_mens_training`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingManifest {
    pub vocab_size: usize,
    pub d_model: usize,
    pub n_heads: usize,
    pub n_layers: usize,
    pub base_model: Option<String>,
    pub tokenizer_path: Option<String>,
    pub train_file: String,
    pub rank: usize,
    pub alpha: f32,
    pub seq_len: usize,
    pub epochs: usize,
    pub run_id: Option<String>,
    pub git_sha: Option<String>,
    pub device_profile: Option<String>,
    /// Optional run label; included in checkpoint filenames when set.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adapter_tag: Option<String>,
    /// Training RNG seed (JSONL row order per epoch after shuffle).
    #[serde(default)]
    pub seed: u64,
    /// Optimizer gradient accumulation steps (micro-batches per step).
    #[serde(default)]
    pub grad_accum: usize,
    /// Optional `category` substring filter applied to JSONL rows.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context_filter: Option<String>,
    /// Hint only: compared against VRAM probe for warnings.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_vram_fraction: Option<f32>,
    /// Manifest format generation (default 1 = legacy implicit).
    #[serde(default = "default_manifest_schema_v1")]
    pub manifest_schema_version: u32,
    /// Execution kernel identity at write time (`burn_lora`, `candle_qlora`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub execution_kernel: Option<String>,
    /// Hex digest of the full [`super::finetune_contract::FineTuneContract`] at plan time (`finetune_contract_digest`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finetune_contract_digest: Option<String>,
    /// Candle QLoRA: optimizer steps that completed (`training_step_lm` success).
    #[serde(default)]
    pub candle_qlora_training_steps_executed: u64,
    /// Candle QLoRA: pairs skipped because last token id was out of vocab.
    #[serde(default)]
    pub candle_qlora_skips_bad_vocab: u64,
    /// Candle QLoRA: pairs skipped because last hidden could not be built from embeddings.
    #[serde(default)]
    pub candle_qlora_skips_last_hidden: u64,
    /// Candle QLoRA: pairs skipped because encoded context was shorter than 2 tokens (cannot form a next-token target).
    #[serde(default)]
    pub candle_qlora_skips_short_seq: u64,
    /// Candle QLoRA: whether full middle projection stack was used (same as trainer `use_o_proj_stack`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candle_qlora_proxy_stack_complete: Option<bool>,
    /// Candle QLoRA: bounded graph id (`proxy_stack_v1_residual` vs `lm_head_only`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candle_qlora_graph_id: Option<String>,
    /// Candle QLoRA: middle projection layers active in the stacked forward (`0` = LM-head-only path).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub candle_qlora_middle_layers_active: Option<usize>,
    /// Candle QLoRA: suffix CE — last **K** token positions per row (`1` = last token only).
    #[serde(default = "default_candle_qlora_ce_last_k")]
    pub candle_qlora_ce_last_k: usize,
    /// Training objective hint for operators (kernel-specific).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_objective_note: Option<String>,
    /// `workstation` or `mobile_edge` when set (export handoff profile).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_deployment_target: Option<String>,
    /// Operator note for edge conversion (see mobile-edge-ai SSOT).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub training_deployment_note: Option<String>,
    /// Reserved: baseline vs adapter delta (populate when eval harness exists).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub eval_baseline_delta_note: Option<String>,
}

/// Snapshot of run settings for [`initial_training_manifest`].
///
/// With the `train` feature, build via [`InitialManifestRun::from_lora_config`]. When
/// `LoraTrainingConfig` gains manifest-relevant fields, update that constructor and this struct together.
#[derive(Debug, Clone)]
pub struct InitialManifestRun {
    pub base_model: Option<String>,
    pub rank: usize,
    pub alpha: f32,
    pub seq_len: usize,
    pub epochs: usize,
    pub run_id: Option<String>,
    pub git_sha: Option<String>,
    pub device_profile: Option<String>,
    pub adapter_tag: Option<String>,
    pub seed: u64,
    pub grad_accum: usize,
    pub context_filter: Option<String>,
    pub max_vram_fraction: Option<f32>,
    pub finetune_contract_digest: Option<String>,
    /// `Some("mobile_edge")` when training for edge export (see SSOT).
    pub training_deployment_target: Option<String>,
    /// Operator note when `training_deployment_target` is set.
    pub training_deployment_note: Option<String>,
}

#[cfg(feature = "mens-train")]
impl InitialManifestRun {
    #[must_use]
    pub fn from_lora_config(c: &super::training_config::LoraTrainingConfig) -> Self {
        Self {
            base_model: c.base_model.clone(),
            rank: c.rank,
            alpha: c.alpha,
            seq_len: c.seq_len,
            epochs: c.epochs,
            run_id: c.run_id.clone(),
            git_sha: c.git_sha.clone(),
            device_profile: c.device_profile.clone(),
            adapter_tag: c.adapter_tag.clone(),
            seed: c.seed,
            grad_accum: c.grad_accum.max(1),
            context_filter: c.context_filter.clone(),
            max_vram_fraction: c.max_vram_fraction,
            finetune_contract_digest: c.finetune_contract_digest.clone(),
            training_deployment_target: (c.deployment_target
                == super::training_config::TrainingDeploymentTarget::MobileEdge)
                .then(|| c.deployment_target.as_str().to_string()),
            training_deployment_note: (c.deployment_target
                == super::training_config::TrainingDeploymentTarget::MobileEdge)
                .then(|| super::operator_messages::MOBILE_EDGE_TRAINING_MANIFEST_NOTE.to_string()),
        }
    }
}

/// Which native kernel is writing the first `training_manifest.json` (kernel-specific notes / Candle fields).
#[derive(Debug, Clone, Copy)]
pub enum InitialTrainingKernel {
    BurnLora,
    /// `proxy_stack_complete` / `middle_layers_active` match the trainer stack; `ce_last_k` is the suffix CE width.
    CandleQlora {
        proxy_stack_complete: bool,
        middle_layers_active: usize,
        ce_last_k: usize,
    },
}
include!("part_build.rs");
include!("part_persist.rs");
include!("part_io.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use vox_tensor::data::VOCAB_SIZE;

    #[test]
    fn initial_training_manifest_burn_wires_kernel_and_candle_defaults() {
        let run = InitialManifestRun {
            base_model: Some("org/model".into()),
            rank: 4,
            alpha: 8.0,
            seq_len: 64,
            epochs: 2,
            run_id: Some("run-1".into()),
            git_sha: Some("deadbeef".into()),
            device_profile: Some("test-gpu".into()),
            adapter_tag: None,
            seed: 11,
            grad_accum: 3,
            context_filter: None,
            max_vram_fraction: None,
            finetune_contract_digest: None,
            training_deployment_target: None,
            training_deployment_note: None,
        };
        let m = initial_training_manifest(
            ArchParams {
                vocab_size: VOCAB_SIZE,
                d_model: 8,
                n_heads: 2,
                n_layers: 1,
            },
            "train.jsonl",
            run,
            None,
            InitialTrainingKernel::BurnLora,
        );
        assert_eq!(m.execution_kernel.as_deref(), Some("burn_lora"));
        assert_eq!(
            m.training_objective_note.as_deref(),
            Some("burn_lora_masked_chatml_ce")
        );
        assert_eq!(m.candle_qlora_proxy_stack_complete, None);
        assert_eq!(m.candle_qlora_graph_id.as_deref(), None);
        assert_eq!(m.candle_qlora_middle_layers_active, None);
        assert_eq!(m.candle_qlora_ce_last_k, 1);
        assert_eq!(m.candle_qlora_training_steps_executed, 0);
        assert_eq!(m.grad_accum, 3);
        assert_eq!(m.train_file, "train.jsonl");
        assert_eq!(m.base_model.as_deref(), Some("org/model"));
    }

    #[test]
    fn initial_training_manifest_candle_sets_proxy_and_objective() {
        let run = InitialManifestRun {
            base_model: None,
            rank: 8,
            alpha: 16.0,
            seq_len: 128,
            epochs: 1,
            run_id: None,
            git_sha: None,
            device_profile: None,
            adapter_tag: None,
            seed: 1,
            grad_accum: 2,
            context_filter: None,
            max_vram_fraction: None,
            finetune_contract_digest: Some("digest".into()),
            training_deployment_target: None,
            training_deployment_note: None,
        };
        let tok = Some("tokenizer.json".to_string());
        let m_stack = initial_training_manifest(
            ArchParams {
                vocab_size: 1000,
                d_model: 32,
                n_heads: 4,
                n_layers: 2,
            },
            "data/train.jsonl",
            run.clone(),
            tok.clone(),
            InitialTrainingKernel::CandleQlora {
                proxy_stack_complete: true,
                middle_layers_active: 3,
                ce_last_k: 1,
            },
        );
        assert_eq!(m_stack.execution_kernel.as_deref(), Some("candle_qlora"));
        assert_eq!(
            m_stack.training_objective_note.as_deref(),
            Some("candle_qlora_proxy_v1_k1")
        );
        assert_eq!(
            m_stack.candle_qlora_graph_id.as_deref(),
            Some("proxy_stack_v1_residual")
        );
        assert_eq!(m_stack.candle_qlora_middle_layers_active, Some(3));
        assert_eq!(m_stack.candle_qlora_ce_last_k, 1);
        assert_eq!(m_stack.candle_qlora_proxy_stack_complete, Some(true));
        assert_eq!(m_stack.tokenizer_path.as_deref(), Some("tokenizer.json"));

        let m_k8 = initial_training_manifest(
            ArchParams {
                vocab_size: 1000,
                d_model: 32,
                n_heads: 4,
                n_layers: 2,
            },
            "data/train.jsonl",
            run.clone(),
            tok.clone(),
            InitialTrainingKernel::CandleQlora {
                proxy_stack_complete: true,
                middle_layers_active: 2,
                ce_last_k: 8,
            },
        );
        assert_eq!(
            m_k8.training_objective_note.as_deref(),
            Some("candle_qlora_proxy_v1_k8")
        );
        assert_eq!(m_k8.candle_qlora_ce_last_k, 8);

        let m_lm = initial_training_manifest(
            ArchParams {
                vocab_size: 1000,
                d_model: 32,
                n_heads: 4,
                n_layers: 2,
            },
            "data/train.jsonl",
            run,
            tok,
            InitialTrainingKernel::CandleQlora {
                proxy_stack_complete: false,
                middle_layers_active: 0,
                ce_last_k: 1,
            },
        );
        assert_eq!(m_lm.candle_qlora_proxy_stack_complete, Some(false));
        assert_eq!(m_lm.candle_qlora_graph_id.as_deref(), Some("lm_head_only"));
        assert_eq!(m_lm.candle_qlora_middle_layers_active, Some(0));
    }

    #[cfg(feature = "mens-train")]
    #[test]
    fn initial_manifest_run_from_lora_config_grad_accum_clamped() {
        use super::super::training_config::LoraTrainingConfig;

        let c = LoraTrainingConfig {
            base_model: Some("hf/model".into()),
            rank: 11,
            grad_accum: 0,
            ..Default::default()
        };
        let snap = InitialManifestRun::from_lora_config(&c);
        assert_eq!(snap.grad_accum, 1);
        assert_eq!(snap.rank, 11);
        let m = initial_training_manifest(
            ArchParams::default(),
            "train.jsonl",
            snap,
            None,
            InitialTrainingKernel::BurnLora,
        );
        assert_eq!(m.grad_accum, 1);
        assert_eq!(m.base_model.as_deref(), Some("hf/model"));
    }

    #[cfg(feature = "mens-train")]
    #[test]
    fn initial_manifest_run_mobile_edge_sets_deployment_fields() {
        use super::super::training_config::{LoraTrainingConfig, TrainingDeploymentTarget};

        let c = LoraTrainingConfig {
            deployment_target: TrainingDeploymentTarget::MobileEdge,
            ..Default::default()
        };
        let snap = InitialManifestRun::from_lora_config(&c);
        assert_eq!(
            snap.training_deployment_target.as_deref(),
            Some("mobile_edge")
        );
        assert!(snap.training_deployment_note.is_some());
    }

    #[test]
    fn training_manifest_roundtrip_grad_accum() {
        let dir = tempdir().expect("tempdir");
        let m = TrainingManifest {
            vocab_size: VOCAB_SIZE,
            d_model: 8,
            n_heads: 2,
            n_layers: 1,
            base_model: None,
            tokenizer_path: None,
            train_file: "train.jsonl".into(),
            rank: 4,
            alpha: 8.0,
            seq_len: 64,
            epochs: 1,
            run_id: None,
            git_sha: None,
            device_profile: None,
            adapter_tag: None,
            seed: 0,
            grad_accum: 7,
            context_filter: None,
            max_vram_fraction: None,
            manifest_schema_version: TRAINING_MANIFEST_SCHEMA_VERSION,
            execution_kernel: None,
            finetune_contract_digest: None,
            candle_qlora_training_steps_executed: 0,
            candle_qlora_skips_bad_vocab: 0,
            candle_qlora_skips_last_hidden: 0,
            candle_qlora_skips_short_seq: 0,
            candle_qlora_proxy_stack_complete: None,
            candle_qlora_graph_id: None,
            candle_qlora_middle_layers_active: None,
            candle_qlora_ce_last_k: 1,
            training_objective_note: None,
            training_deployment_target: None,
            training_deployment_note: None,
            eval_baseline_delta_note: None,
        };
        write_training_manifest(dir.path(), m).expect("write");
        let loaded = load_manifest(dir.path()).expect("load").expect("some");
        assert_eq!(loaded.grad_accum, 7);
    }
}
