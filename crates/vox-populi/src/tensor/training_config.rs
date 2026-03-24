//! Shared training run configuration for all Populi native trainers (`--backend`).

/// Where trained artifacts are intended to run (planner gates + manifest hints).
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum TrainingDeploymentTarget {
    /// Workstation / server Populi stack (default).
    #[default]
    Workstation,
    /// Export-oriented profile for phone / edge inference (train off-device).
    MobileEdge,
}

impl TrainingDeploymentTarget {
    /// Stable wire / manifest label.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workstation => "workstation",
            Self::MobileEdge => "mobile_edge",
        }
    }
}

/// Tokenization strategy for training pairs.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, Default, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum PopuliTokenizerMode {
    /// [`vox_tensor::data::VoxTokenizer`] (Burn LoRA default; corpus-native).
    #[default]
    Vox,
    /// Hugging Face `tokenizer.json` (`--tokenizer hf`; required for `--backend qlora`).
    Hf,
}

/// Full configuration for one LoRA / QLoRA training run.
#[derive(Debug, Clone)]
pub struct LoraTrainingConfig {
    pub base_model: Option<String>,
    pub base_model_paths: Option<(Vec<std::path::PathBuf>, std::path::PathBuf)>,
    pub tokenizer_path: Option<std::path::PathBuf>,
    pub train_file: Option<std::path::PathBuf>,
    pub rank: usize,
    pub alpha: f32,
    pub seq_len: usize,
    pub batch_size: usize,
    pub grad_accum: usize,
    pub resume_from: Option<std::path::PathBuf>,
    pub epochs: usize,
    pub learning_rate: f64,
    pub warmup_steps: usize,
    pub seed: u64,
    pub min_rating: u8,
    pub run_id: Option<String>,
    pub git_sha: Option<String>,
    pub device_profile: Option<String>,
    pub max_vram_fraction: Option<f32>,
    pub adapter_tag: Option<String>,
    pub context_filter: Option<String>,
    pub validation_split_ratio: Option<f64>,
    pub tokenizer_mode: PopuliTokenizerMode,
    /// When false, sets qlora-rs `QuantizationConfig.double_quant` off (debug / ablation). Default: true.
    pub qlora_double_quant: bool,
    /// Set by [`crate::tensor::lora_train::run_populi_training`] from the execution plan.
    pub finetune_contract_digest: Option<String>,
    /// Candle QLoRA: fail preflight when middle projection keys are incomplete (`--qlora-require-full-proxy-stack`).
    pub qlora_require_full_proxy_stack: bool,
    /// Candle QLoRA: abort training when skip rate (skipped pairs / pair visits) exceeds this value in an epoch.
    pub qlora_max_skip_rate: Option<f32>,
    /// Candle QLoRA: skip `o_proj` proxy stack; train tied LM-head `QuantizedLinear` only (stable CE on dogfood).
    pub qlora_lm_head_only: bool,
    /// Candle QLoRA: cap how many ordered middle `o_proj` layers are stacked before the LM head (`None` = all when stack is used; `0` = LM-head-only).
    pub qlora_proxy_max_layers: Option<usize>,
    /// Candle QLoRA: next-token CE over the last **K** positions per JSONL row (default 1).
    pub qlora_ce_last_k: usize,
    /// Steps between mid-epoch checkpoints. None means only epoch-boundary checkpoints.
    pub checkpoint_every: Option<usize>,
    /// Ignore existing checkpoints and force a fresh run.
    pub force_restart: bool,
    /// Intended deployment surface for trained artifacts (planner gates + manifest).
    pub deployment_target: TrainingDeploymentTarget,
}

impl Default for LoraTrainingConfig {
    fn default() -> Self {
        Self {
            base_model: None,
            base_model_paths: None,
            tokenizer_path: None,
            train_file: None,
            rank: 16,
            alpha: 32.0,
            seq_len: 256,
            batch_size: 4,
            grad_accum: 4,
            resume_from: None,
            epochs: 3,
            learning_rate: 2e-4,
            warmup_steps: 100,
            seed: 42,
            min_rating: 3,
            run_id: None,
            git_sha: None,
            device_profile: None,
            max_vram_fraction: None,
            adapter_tag: None,
            context_filter: None,
            validation_split_ratio: Some(0.05),
            tokenizer_mode: PopuliTokenizerMode::Hf,
            qlora_double_quant: true,
            finetune_contract_digest: None,
            qlora_require_full_proxy_stack: false,
            qlora_max_skip_rate: None,
            qlora_lm_head_only: false,
            qlora_proxy_max_layers: None,
            qlora_ce_last_k: 16,
            checkpoint_every: Some(500),
            force_restart: false,
            deployment_target: TrainingDeploymentTarget::default(),
        }
    }
}
