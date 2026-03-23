//! `vox populi` — the unified AI/ML command surface for Vox.
//!
//! All model training, serving, and corpus management lives here.
//! This is the canonical entry point; the deprecated top-level `vox train` remains
//! for Together / legacy native paths (see registry + `commands::ai::train`).
//!
//! ## Subcommands
//!
//! ```text
//! vox populi train      — Fine-tune: Burn LoRA (default) or Candle QLoRA (`--backend qlora` + HF tokenizer)
//! vox populi serve      — HTTP inference (build `vox-cli` with `--features execution-api`)
//! vox populi corpus     — Training data pipeline (extract, validate, mix, eval…)
//! vox populi probe      — Detect GPU capabilities and print recommended LoRA training config
//! vox populi status     — Show training run status from the latest telemetry log
//! vox populi eval-local — Evaluate a trained model against the heldout benchmark set
//! vox populi oratio   — Speech-to-text (Oratio) and transcript fixtures
//! ```

/// Latency and throughput benchmarking for completions.
pub mod bench_completion;
pub(crate) mod eval_gate;
#[cfg(feature = "gpu")]
mod eval_local;
mod eval_local_prompt;
#[cfg(feature = "gpu")]
mod merge_qlora;
mod merge_weights;
#[cfg(feature = "populi-oratio")]
pub mod oratio_cmd;
#[cfg(feature = "populi-base")]
mod pipeline;
/// AI-agent planning sessions and task decomposition.
pub mod plan;
#[cfg(feature = "gpu")]
mod probe;
#[cfg(feature = "gpu")]
mod process_priority;
mod status;
mod system_prompt_template;
#[cfg(feature = "gpu")]
mod train;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

use crate::commands::corpus::CorpusAction;

/// CLI mapping for `vox populi train --backend` → [`vox_populi::PopuliTrainBackend`].
#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum PopuliTrainBackendCli {
    /// Burn + wgpu LoRA on VoxTokenizer JSONL (default).
    #[default]
    Lora,
    /// Candle + qlora-rs NF4 on HF safetensors (`--tokenizer hf`, `--model`, CUDA/Metal optional).
    Qlora,
}

#[cfg(feature = "gpu")]
impl From<PopuliTrainBackendCli> for vox_populi::PopuliTrainBackend {
    fn from(value: PopuliTrainBackendCli) -> Self {
        match value {
            PopuliTrainBackendCli::Lora => Self::BurnLora,
            PopuliTrainBackendCli::Qlora => Self::CandleQlora,
        }
    }
}

/// CLI mapping for `vox populi train --tokenizer` → [`vox_populi::PopuliTokenizerMode`].
#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum PopuliTokenizerCli {
    /// Corpus VoxTokenizer JSONL (Burn `--backend lora`).
    #[default]
    Vox,
    /// Hugging Face `tokenizer.json` (required for native `--backend qlora` preflight).
    Hf,
}

#[cfg(feature = "gpu")]
impl From<PopuliTokenizerCli> for vox_populi::PopuliTokenizerMode {
    fn from(value: PopuliTokenizerCli) -> Self {
        match value {
            PopuliTokenizerCli::Vox => Self::Vox,
            PopuliTokenizerCli::Hf => Self::Hf,
        }
    }
}

/// CLI mapping for `vox populi train --deployment-target` → [`vox_populi::TrainingDeploymentTarget`].
#[cfg(feature = "gpu")]
#[derive(Clone, Copy, Debug, Default, clap::ValueEnum)]
pub enum TrainingDeploymentTargetCli {
    /// Default workstation / server Populi path.
    #[default]
    Workstation,
    /// Mobile edge export profile (`--device cpu` required; planner gates).
    MobileEdge,
}

#[cfg(feature = "gpu")]
impl From<TrainingDeploymentTargetCli> for vox_populi::TrainingDeploymentTarget {
    fn from(value: TrainingDeploymentTargetCli) -> Self {
        match value {
            TrainingDeploymentTargetCli::Workstation => Self::Workstation,
            TrainingDeploymentTargetCli::MobileEdge => Self::MobileEdge,
        }
    }
}

/// Top-level subcommand enum for `vox populi` (AI/ML surfaces).
#[derive(Parser)]
#[allow(clippy::large_enum_variant)]
#[command(
    name = "populi",
    about = "Vox AI/ML: train, serve, and corpus management",
    long_about = "The Vox Populi subsystem handles everything related to AI model\n\
                  training, inference serving, and training data pipelines.\n\
                  \n\
                  Quick start:\n\
                  \n  vox populi corpus extract .    # extract corpus from vox files\
                  \n  vox populi corpus pairs ...    # generate training pairs\
                  \n  vox populi train               # Burn LoRA or Candle QLoRA (`--backend`)\
                  \n  vox populi pipeline            # Corpus → eval → optional native train\
                  \n  vox populi serve --model ...   # HTTP serve (needs `cargo build -p vox-cli --features execution-api`)"
)]
pub enum PopuliAction {
    /// Corpus extract/validate/pairs/eval and optional `vox populi train` (dogfood pipeline).
    #[cfg(feature = "populi-base")]
    Pipeline {
        /// Directory for `train.jsonl` (default matches legacy PS1 `target/dogfood`).
        #[arg(long, default_value = vox_corpus::training::CANONICAL_TRAIN_DATA_DIR)]
        data_dir: PathBuf,
        /// Run outputs (eval JSON, train checkpoints).
        #[arg(long, default_value = "populi/runs/v1")]
        output_dir: PathBuf,
        /// Stop after corpus eval (no train stage).
        #[arg(long, default_value_t = false)]
        skip_train: bool,
        /// Stricter eval gate env for the train stage (`VOX_EVAL_STRICT`, min pass rate).
        #[arg(long, default_value_t = false)]
        strict_gate: bool,
        /// Passed to `vox populi train --device` when training runs (default `best` in the pipeline runner).
        #[arg(long)]
        device: Option<String>,
    },
    /// Fine-tune: Burn LoRA (`--backend lora`) or Candle HF-embed adapter (`--backend qlora` + `--tokenizer hf`).
    #[cfg(feature = "gpu")]
    Train {
        /// HuggingFace model repo to fine-tune (e.g. Qwen/Qwen2.5-Coder-1.5B-Instruct).
        /// When set, weights are downloaded natively via hf-hub before training.
        #[arg(long)]
        model: Option<String>,
        /// GPU backend: cpu, best, vulkan, dx12, or metal
        #[arg(long, default_value = "best")]
        device: String,
        /// Trainer: `lora` = Burn+wGPU + `--tokenizer vox` (default) or `--tokenizer hf` (GPT-2-shaped HF config + optional embed warm-start); `qlora` = Candle qlora-rs NF4 (needs `--tokenizer hf`, `--model`, safetensors).
        #[arg(long, value_enum, default_value_t = PopuliTrainBackendCli::Lora)]
        backend: PopuliTrainBackendCli,
        /// Directory containing train.jsonl (produced by `vox populi corpus pairs`).
        /// Canonical path: target/dogfood (matches corpus merge output).
        #[arg(long, default_value = vox_corpus::training::CANONICAL_TRAIN_DATA_DIR)]
        data_dir: PathBuf,
        /// Where to save the trained adapter / checkpoint
        #[arg(long, default_value = "populi/runs/latest")]
        output_dir: PathBuf,
        /// LoRA rank (r). Higher = more expressiveness. Default: 16 (or auto-tuned)
        #[arg(long)]
        rank: Option<usize>,
        /// LoRA alpha scaling factor. Default: 32 (or r*2)
        #[arg(long)]
        alpha: Option<f32>,
        /// Maximum sequence length. Default: 512 (or auto-tuned)
        #[arg(long)]
        seq_len: Option<usize>,
        /// Batch size per step. Default: 4 (or auto-tuned)
        #[arg(long)]
        batch_size: Option<usize>,
        /// Gradient accumulation steps. Default: 4
        #[arg(long)]
        grad_accum: Option<usize>,
        /// Resume from a checkpoint directory
        #[arg(long)]
        resume: Option<PathBuf>,
        /// Number of epochs to train (default: 3)
        #[arg(long)]
        epochs: Option<usize>,
        /// Learning rate (default: 2e-4)
        #[arg(long)]
        lr: Option<f64>,
        /// Warmup steps (default: 100 or 10% of total)
        #[arg(long)]
        warmup: Option<usize>,
        /// Random seed for reproducibility (0 = random)
        #[arg(long, default_value = "42")]
        seed: u64,
        /// Minimum quality rating (1-5) to include. 0 = all. Default 3 matches mix.
        #[arg(long)]
        min_rating: Option<u8>,
        /// O029: Preset: tiny, safe, 4080 / qwen_4080_16g, mobile_edge (implies `--deployment-target mobile_edge`), …
        #[arg(long)]
        preset: Option<String>,
        /// Train for workstation (default) or mobile edge export (requires `--device cpu`; see mobile-edge-ai SSOT).
        #[arg(long, value_enum, default_value_t = TrainingDeploymentTargetCli::Workstation)]
        deployment_target: TrainingDeploymentTargetCli,
        /// Process priority: low (BELOW_NORMAL/nice 10) or normal. Default: normal. Use low when training while browsing.
        #[arg(long, default_value = "normal")]
        process_priority: String,
        /// Cap VRAM usage at this fraction (0.0–1.0). E.g. 0.8 = 80%. Unset = adaptive 85%. Use with --process-priority low for background training.
        #[arg(long)]
        vram_limit_fraction: Option<f32>,
        /// Convenience: implies --process-priority low and --vram-limit-fraction 0.8 for training while browsing.
        #[arg(long)]
        background: bool,
        /// Run training in background and write all output to this directory (e.g. populi/runs/logs or a temp path).
        /// Creates `train_<timestamp>.log`. Use to avoid IDE timeouts; tail the log to monitor progress.
        #[arg(long)]
        log_dir: Option<PathBuf>,
        /// Dual-mode adapter tag. Sets the output directory suffix (e.g. "target" → runs/lora-target/).
        /// Use "target" for Vox app-code mode, "meta" for codebase reasoning mode.
        /// When set, automatically applies a matching context filter unless --context-filter is also specified.
        #[arg(long)]
        adapter_tag: Option<String>,
        /// Filter training records by context field. One of: target, meta, both.
        /// Records tagged "both" always pass any filter. Defaults to the adapter_tag if set.
        #[arg(long)]
        context_filter: Option<String>,
        /// Tokenizer mode: `vox` for corpus JSONL (default; Burn LoRA). `hf` for Hugging Face `tokenizer.json` (native qlora preflight).
        #[arg(long, value_enum, default_value_t = PopuliTokenizerCli::Vox)]
        tokenizer: PopuliTokenizerCli,
        /// Disable qlora-rs double quantization of NF4 scales (default: double quant **on** for smaller VRAM).
        #[arg(long)]
        qlora_no_double_quant: bool,
        /// Candle QLoRA: abort preflight unless every expected per-layer output projection (`o_proj` / GPT-2 `c_proj`) exists in safetensors (full proxy stack vs LM-head-only).
        #[arg(long)]
        qlora_require_full_proxy_stack: bool,
        /// Candle QLoRA: skip the `o_proj` proxy stack; train the tied LM-head adapter only (stable CE on dogfood; see populi-training-ssot.md).
        #[arg(long)]
        qlora_lm_head_only: bool,
        /// Candle QLoRA: abort an epoch if skipped pairs / pair visits exceed this rate (0.0–1.0).
        #[arg(long)]
        qlora_max_skip_rate: Option<f32>,
        /// Candle QLoRA: max middle `o_proj` layers in the proxy stack (ablation / VRAM). Omit for full stack when keys are complete; `0` = LM-head-only.
        #[arg(long)]
        qlora_proxy_max_layers: Option<usize>,
        /// Candle QLoRA: next-token CE on the last **K** positions per JSONL row (default 1). Capped by effective `--seq-len` and 64.
        #[arg(long, default_value_t = 1)]
        qlora_ce_last_k: usize,
    },

    /// Run UV-managed Python quantized training (bitsandbytes/Unsloth-style).
    ///
    /// Invokes `uv run python populi/scripts/quantized_train.py` with env vars.
    /// Emits structured logs; writes manifest/metrics to output dir.
    TrainUv {
        /// HuggingFace model ID (e.g. Qwen/Qwen2.5-Coder-3B-Instruct)
        #[arg(long)]
        model: Option<String>,
        /// Directory containing train.jsonl
        #[arg(long, default_value = vox_corpus::training::CANONICAL_TRAIN_DATA_DIR)]
        data_dir: PathBuf,
        /// Where to write run_manifest.json, metrics.jsonl
        #[arg(long, default_value = "populi/runs/uv_output")]
        output_dir: PathBuf,
        /// LoRA rank (default 16)
        #[arg(long)]
        rank: Option<usize>,
        /// LoRA alpha (default 32)
        #[arg(long)]
        alpha: Option<f32>,
        /// Number of epochs (default 3)
        #[arg(long)]
        epochs: Option<usize>,
    },

    /// Serve a trained Populi checkpoint via HTTP (OpenAI-compatible API)
    #[cfg(all(feature = "gpu", feature = "execution-api"))]
    Serve {
        /// Path to model checkpoint (.bin from `vox populi train`)
        #[arg(long, required = true)]
        model: PathBuf,
        /// HTTP port to listen on
        #[arg(long, default_value_t = crate::commands::ai::inference_defaults::DEFAULT_INFERENCE_PORT)]
        port: u16,
        /// Host to bind (use 0.0.0.0 for network access)
        #[arg(long, default_value = crate::commands::ai::inference_defaults::DEFAULT_INFERENCE_HOST)]
        host: String,
        /// Maximum tokens to generate per request
        #[arg(
            long,
            default_value_t = crate::commands::ai::inference_defaults::DEFAULT_INFERENCE_MAX_TOKENS
        )]
        max_tokens: usize,
        /// Sampling temperature (0.0 = greedy, 1.0 = random)
        #[arg(
            long,
            default_value_t = crate::commands::ai::inference_defaults::DEFAULT_INFERENCE_TEMPERATURE
        )]
        temperature: f32,
    },

    /// Training data pipeline: extract, validate, mix, eval, audit…
    #[command(subcommand)]
    Corpus(CorpusAction),

    /// Detect GPU capabilities and print recommended LoRA training configuration
    #[cfg(feature = "gpu")]
    Probe,

    /// Show training run status or BYOK quota usage
    Status {
        /// Path to telemetry JSONL (default: populi/runs/latest/telemetry.jsonl)
        #[arg(long)]
        run_dir: Option<PathBuf>,
        /// Show BYOK quota usage vs limits (Wave 2)
        #[arg(long, default_value = "false")]
        quotas: bool,
        /// Show current agent/orchestrator inference configuration (Wave 7)
        #[arg(long, default_value = "false")]
        config: bool,
    },

    /// Merge LoRA adapter weights into the base model for faster inference
    ///
    /// Takes a LoRA checkpoint (LoraVoxTransformer) and folds A/B matrices
    /// into the base weights, producing a standalone VoxTransformer checkpoint
    /// that can be served without adapter overhead.
    #[cfg(feature = "gpu")]
    MergeWeights {
        /// Path to LoRA checkpoint (.bin from `vox populi train`)
        #[arg(long, required = true)]
        model: PathBuf,
        /// Where to save the merged model (default: <model_dir>/model_merged.bin)
        #[arg(long)]
        output: Option<PathBuf>,
        /// LoRA rank used during training
        #[arg(long, default_value = "16")]
        rank: usize,
        /// LoRA alpha used during training
        #[arg(long, default_value = "32")]
        alpha: f32,
    },

    /// Merge Candle QLoRA adapter v2 (`candle_qlora_adapter*.safetensors`) into base f32 weights (writes merged keys only).
    /// Canonical: `merge-qlora`. Alias: `merge-adapter` (same flags).
    #[cfg(feature = "gpu")]
    #[command(name = "merge-qlora", visible_alias = "merge-adapter")]
    MergeQlora {
        /// Base model safetensors shard path (repeat `--base-shard` for each file).
        #[arg(long = "base-shard", required = true)]
        base_shard: Vec<PathBuf>,
        /// `candle_qlora_adapter.safetensors` from a qlora-rs training run.
        #[arg(long, required = true)]
        adapter: PathBuf,
        /// `candle_qlora_adapter_meta.json` (format v2).
        #[arg(long, required = true)]
        meta: PathBuf,
        /// Output safetensors path (subset of merged keys).
        #[arg(long, required = true)]
        output: PathBuf,
    },

    /// AI-powered code generation from a natural language prompt
    #[cfg(feature = "populi-dei")]
    Generate {
        /// Natural language prompt
        prompt: String,
        /// Path to write the generated code
        #[arg(short = 'o', long)]
        output: Option<std::path::PathBuf>,
        /// Skip syntactic validation of the output
        #[arg(long, default_value = "false")]
        no_validate: bool,
        /// Custom inference server URL
        #[arg(long)]
        server_url: Option<String>,
        /// Max retries on network failure
        #[arg(long)]
        max_retries: Option<u32>,
        /// P015: Output mode (strict_json, jsonl_records, tool_args_json) for constrained decoding
        #[arg(long)]
        output_mode: Option<String>,
        /// P016: JSON schema path for post-generation validation (requires output_mode)
        #[arg(long)]
        schema: Option<std::path::PathBuf>,
        /// Context assembly mode: minimal | repo-aware | schema-only | graph-aware | full
        #[arg(long, default_value = "minimal")]
        context_mode: String,
        /// Conversation ID for graph-aware context (pulls versions/edges from Codex)
        #[arg(long)]
        conversation_id: Option<i64>,
        /// Track job in Codex task_jobs (queue-aware, sync execution). Fallback to direct when Codex unavailable.
        #[arg(long, default_value = "false")]
        queue: bool,
        /// Execution mode: efficient | fast | verbose | precision
        #[arg(long)]
        mode: Option<String>,
    },

    /// AI-powered code review for one or more paths
    #[cfg(feature = "populi-dei")]
    Review {
        /// Path(s) to files or directories to review
        #[arg(default_value = ".")]
        targets: Vec<std::path::PathBuf>,
        /// LLM model override
        #[arg(long)]
        model: Option<String>,
        /// Output format (text, json, markdown)
        #[arg(long)]
        format: Option<String>,
        /// Minimum severity to report
        #[arg(long)]
        severity: Option<String>,
        /// Restrict to free providers
        #[arg(long, default_value = "false")]
        free_only: bool,
        /// Review the current git diff only
        #[arg(long, default_value = "false")]
        diff: bool,
        /// CI mode: exit non-zero on issues
        #[arg(long, default_value = "false")]
        ci: bool,
        /// Add review as PR comments (GitHub/GitLab)
        #[arg(long, default_value = "false")]
        pr_comment: bool,
        /// Git base ref for diff-based review
        #[arg(long)]
        diff_base: Option<String>,
        /// Execution mode profile
        #[arg(long)]
        mode: Option<String>,
    },

    /// Inspect and run Vox workflow definitions
    #[cfg(feature = "populi-dei")]
    #[command(subcommand)]
    Workflow(crate::cli_actions::WorkflowAction),

    /// AI-powered code check for potential bugs or anti-patterns (alias: verify)
    #[cfg(feature = "populi-dei")]
    #[command(alias = "verify")]
    Check {
        /// File to check
        #[arg(required = true)]
        file: std::path::PathBuf,
    },

    /// Attempt to automatically fix compiler errors using AI
    #[cfg(feature = "populi-dei")]
    Fix {
        /// File to fix
        #[arg(required = true)]
        file: std::path::PathBuf,
        /// Optional: specific compiler errors to fix (omit to re-run check)
        #[arg(long)]
        errors: Option<String>,
    },

    /// Evaluate a trained model checkpoint against the heldout benchmark set.
    ///
    /// Runs inference on each sample in the benchmark directory and reports
    /// per-sample completion quality and pass rates by category.
    #[cfg(feature = "gpu")]
    #[command(name = "eval-local")]
    EvalLocal {
        /// Path to model checkpoint (.bin from `vox populi train`)
        #[arg(long, required = true)]
        model: PathBuf,
        /// Path to heldout benchmark directory (default: populi/data/heldout_bench)
        #[arg(long, default_value = "populi/data/heldout_bench")]
        bench: PathBuf,
        /// Maximum tokens to generate per sample
        #[arg(long, default_value = "128")]
        max_tokens: usize,
        /// Sampling temperature (0.0 = greedy)
        #[arg(long, default_value = "0.0")]
        temperature: f32,
        /// Output JSON results to this path
        #[arg(short = 'o', long)]
        output: Option<PathBuf>,
    },

    /// Check training/eval run against eval-gate policy thresholds.
    ///
    /// Reads populi/config/eval-gates.yaml and validates run artifacts.
    /// Exits 1 if any blocking gate fails.
    #[command(name = "eval-gate")]
    EvalGate {
        /// Run directory (manifest.json, metrics.jsonl, eval_results.json)
        #[arg(long, default_value = "populi/runs/v1")]
        run_dir: PathBuf,
        /// Policy file (default: populi/config/eval-gates.yaml)
        #[arg(long)]
        policy: Option<PathBuf>,
    },

    /// Benchmark FIM completion server latency
    BenchCompletion {
        /// URL of the completions API Endpoint
        #[arg(long, default_value = "http://127.0.0.1:8080/v1/completions")]
        url: String,
        /// Number of benchmark iteration runs
        #[arg(short = 'c', long, default_value = "100")]
        count: usize,
        /// Number of initial warmup requests
        #[arg(short = 'w', long, default_value = "5")]
        warmup: usize,
    },

    /// Generate, replan, and query AI planning sessions backed by Codex
    ///
    /// # Examples
    ///
    ///   vox populi plan new "Add OAuth2 support"
    ///   vox populi plan replan <session_id> "Auth library changed"
    ///   vox populi plan status <session_id>
    #[command(subcommand, name = "plan", next_help_heading = "Planning")]
    Plan(plan::PlanAction),

    /// Generate the canonical system prompt template for IDE integration (Cursor, Claude, etc.)
    #[command(name = "system-prompt-template")]
    SystemPromptTemplate {
        /// Optional: write the template to a file instead of stdout
        #[arg(short, long)]
        output: Option<PathBuf>,
        /// Optional: wrap in IDE-specific format: text (default), cursor, claude, copilot, or wind-pro (Windsurf)
        #[arg(long, default_value = "text")]
        format: String,
    },

    /// Speech-to-text and transcript pipeline (Oratio)
    #[cfg(feature = "populi-oratio")]
    #[command(subcommand, name = "oratio", alias = "speech")]
    Oratio(oratio_cmd::OratioAction),
}

/// Dispatch `vox populi` subcommands to their feature-gated implementations.
pub async fn run(action: PopuliAction, _global_json: bool, _global_verbose: bool) -> Result<()> {
    match action {
        #[cfg(feature = "populi-base")]
        PopuliAction::Pipeline {
            data_dir,
            output_dir,
            skip_train,
            strict_gate,
            device,
        } => pipeline::run(data_dir, output_dir, skip_train, strict_gate, device).await,
        PopuliAction::TrainUv {
            model: _,
            data_dir: _,
            output_dir: _,
            rank: _,
            alpha: _,
            epochs: _,
        } => {
            anyhow::bail!(
                "`vox populi train-uv` is retired: `quantized_train.py` is not shipped in this repository.\n\
                 Use **`vox populi train --backend qlora --tokenizer hf`** (see docs/src/architecture/populi-training-ssot.md)."
            );
        }
        #[cfg(feature = "gpu")]
        PopuliAction::Train {
            model,
            device,
            backend,
            data_dir,
            output_dir,
            rank,
            alpha,
            seq_len,
            batch_size,
            grad_accum,
            resume,
            epochs,
            lr,
            warmup,
            seed,
            min_rating,
            preset,
            deployment_target,
            process_priority,
            vram_limit_fraction,
            background,
            log_dir,
            adapter_tag,
            context_filter,
            tokenizer,
            qlora_no_double_quant,
            qlora_require_full_proxy_stack,
            qlora_lm_head_only,
            qlora_max_skip_rate,
            qlora_proxy_max_layers,
            qlora_ce_last_k,
        } => {
            let process_priority = if background {
                "low".to_string()
            } else {
                process_priority
            };
            let vram_limit_fraction = if background {
                vram_limit_fraction.or(Some(0.8))
            } else {
                vram_limit_fraction
            };

            // Preflight auto-regen check
            let workspace_root = vox_corpus::training::contract::find_workspace_root();
            if let Some(ref root) = workspace_root {
                use owo_colors::OwoColorize;
                let current_fp = vox_corpus::corpus::preflight::compute_corpus_fingerprint(root);
                
                let is_fresh = if let Ok(db) = vox_db::VoxDb::open_from_env() {
                    db.is_corpus_fresh(&current_fp).await.unwrap_or(false)
                } else {
                    let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
                    vox_corpus::corpus::preflight::corpus_is_fresh(root, &fp_file)
                };

                let skip_regen = std::env::var("VOX_TRAIN_SKIP_CORPUS_MIX").map(|v| v == "1" || v.eq_ignore_ascii_case("true")).unwrap_or(false);
                if !is_fresh && !skip_regen {
                    eprintln!("  {} Stale corpus detected (fingerprint: {}). Regenerating...", "🔄".cyan(), current_fp);
                    let _ = vox_corpus::corpus::preflight::clean_corpus_targets(root);
                    
                    let cfg = vox_corpus::synthetic_gen::SyntheticGenConfig::default();
                    let out_path = root.join("populi/data/synthetic.jsonl");
                    if let Ok(pairs) = vox_corpus::synthetic_gen::generate_all(&cfg, &out_path) {
                        eprintln!("  {} Regenerated {} synthetic pairs", "✓".green(), pairs);
                        if let Ok(db) = vox_db::VoxDb::open_from_env() {
                            let _ = db.record_corpus_snapshot(&current_fp, pairs as i64, None).await;
                        } else {
                            let fp_file = vox_corpus::corpus::preflight::fingerprint_cache_path(root);
                            let _ = vox_corpus::corpus::preflight::write_fingerprint_snapshot(root, &fp_file);
                        }
                    }
                }
            }

            // If context_filter is not explicitly set, default it to adapter_tag
            // so --adapter-tag target automatically filters to target records.
            let context_filter = context_filter.or_else(|| adapter_tag.clone());
            if let Some(ref log_dir) = log_dir {
                return crate::commands::populi::train::spawn_train_with_log(log_dir.clone());
            }
            let deployment_target = if preset.as_deref() == Some("mobile_edge") {
                vox_populi::TrainingDeploymentTarget::MobileEdge
            } else {
                deployment_target.into()
            };
            train::run_train(
                backend.into(),
                model,
                device,
                data_dir,
                output_dir,
                rank,
                alpha,
                seq_len,
                batch_size,
                grad_accum,
                resume,
                epochs,
                lr,
                warmup,
                seed,
                min_rating,
                preset,
                deployment_target,
                process_priority,
                vram_limit_fraction,
                adapter_tag,
                context_filter,
                tokenizer.into(),
                qlora_no_double_quant,
                qlora_require_full_proxy_stack,
                qlora_max_skip_rate,
                qlora_lm_head_only,
                qlora_proxy_max_layers,
                qlora_ce_last_k,
            )
        }

        #[cfg(all(feature = "gpu", feature = "execution-api"))]
        PopuliAction::Serve {
            model,
            port,
            host,
            max_tokens,
            temperature,
        } => {
            // Serve is handled by main() early-exit (outside tokio) when execution-api is enabled.
            // When execution-api is disabled, run_serve is a stub that prints instructions.
            let config = crate::commands::ai::serve::ServeConfig {
                model_path: model,
                port,
                host,
                max_tokens,
                temperature,
                system_prompt: None,
            };
            crate::commands::ai::serve::run_serve(&config)
        }

        PopuliAction::Corpus(action) => crate::commands::corpus::run(action).await,

        #[cfg(feature = "gpu")]
        PopuliAction::Probe => {
            let _ = _global_verbose;
            probe::run_probe(_global_verbose)
        }

        PopuliAction::Status {
            run_dir,
            quotas,
            config,
        } => {
            let _ = _global_json;
            status::run_status(run_dir, _global_json, quotas, config).await
        }

        #[cfg(feature = "gpu")]
        PopuliAction::MergeWeights {
            model,
            output,
            rank,
            alpha,
        } => merge_weights::run_merge_weights(model, output, rank, alpha),

        #[cfg(feature = "gpu")]
        PopuliAction::MergeQlora {
            base_shard,
            adapter,
            meta,
            output,
        } => merge_qlora::run_merge_qlora(base_shard, adapter, meta, output),

        #[cfg(feature = "populi-dei")]
        PopuliAction::Generate {
            prompt,
            output,
            no_validate,
            server_url,
            max_retries,
            output_mode,
            schema,
            context_mode,
            conversation_id,
            queue,
            mode,
        } => {
            if let Some(ref m) = mode {
                // SAFETY: isolated env var for this process; no other threads read it during this block
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_DEI_MODE_PROFILE", m);
                }
            }
            // Run generate in a dedicated thread with its own runtime to avoid
            // "Cannot drop a runtime in a context where blocking is not allowed" during shutdown.
            let prompt = prompt.clone();
            let output = output.clone();
            let server_url = server_url.clone();
            let output_mode = output_mode.as_deref();
            let schema = schema.as_deref();
            let context_mode = context_mode.clone();
            tokio::task::block_in_place(|| {
                let rt = tokio::runtime::Runtime::new().expect("create runtime for generate");
                rt.block_on(crate::commands::ai::generate::run(
                    &prompt,
                    output,
                    no_validate,
                    server_url.as_deref(),
                    max_retries,
                    output_mode,
                    schema,
                    Some(&context_mode),
                    conversation_id,
                    queue,
                ))
            })
        }

        #[cfg(feature = "populi-dei")]
        PopuliAction::Review {
            targets,
            model,
            format,
            severity,
            free_only,
            diff,
            ci,
            pr_comment,
            diff_base,
            mode,
        } => {
            if let Some(ref m) = mode {
                // SAFETY: main-thread env set before spawning review; no concurrent readers
                #[allow(unsafe_code)]
                unsafe {
                    std::env::set_var("VOX_DEI_MODE_PROFILE", m);
                }
            }
            crate::commands::review::run(
                &targets,
                model.as_deref(),
                format.as_deref(),
                severity.as_deref(),
                free_only,
                diff,
                ci,
                pr_comment,
                diff_base.as_deref(),
            )
            .await
        }

        #[cfg(feature = "populi-dei")]
        PopuliAction::Workflow(action) => crate::commands::ai::workflow::run(action).await,

        #[cfg(feature = "populi-dei")]
        PopuliAction::Check { file } => {
            crate::dei_daemon::call(
                crate::dei_daemon::method::AI_CHECK,
                serde_json::json!({
                    "file": file,
                }),
                false,
            )
            .await?;
            Ok(())
        }

        #[cfg(feature = "populi-dei")]
        PopuliAction::Fix { file, errors } => {
            let code = std::fs::read_to_string(&file)?;
            let errors_val = if let Some(e) = errors {
                e
            } else {
                "".to_string()
            };
            crate::dei_daemon::call(
                crate::dei_daemon::method::AI_FIX,
                serde_json::json!({
                    "code": code,
                    "errors": errors_val,
                }),
                false,
            )
            .await?;
            Ok(())
        }

        #[cfg(feature = "gpu")]
        PopuliAction::EvalLocal {
            model,
            bench,
            max_tokens,
            temperature,
            output,
        } => eval_local::run_eval_local(model, bench, max_tokens, temperature, output),

        PopuliAction::EvalGate { run_dir, policy } => {
            let code = eval_gate::run_eval_gate(run_dir, policy)?;
            std::process::exit(code);
        }

        PopuliAction::BenchCompletion { url, count, warmup } => {
            bench_completion::run_bench(&url, count, warmup).await
        }

        PopuliAction::Plan(action) => plan::run(action).await,

        PopuliAction::SystemPromptTemplate { output, format } => {
            crate::commands::populi::system_prompt_template::run(output, &format).await
        }

        #[cfg(feature = "populi-oratio")]
        PopuliAction::Oratio(action) => oratio_cmd::run(action, _global_json),
    }
}

#[cfg(all(test, feature = "gpu"))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn probe_runs_without_gpu() {
        let result = probe::run_probe(false);
        assert!(result.is_ok());
    }

    #[test]
    fn probe_verbose_runs_without_gpu() {
        let result = probe::run_probe(true);
        assert!(result.is_ok());
    }

    #[test]
    fn status_missing_dir_reports_gracefully() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(status::run_status(
            Some(PathBuf::from("/nonexistent/run/dir")),
            false,
            false,
            false,
        ));
        assert!(
            result.is_ok(),
            "missing telemetry should not error: {:?}",
            result
        );
    }

    #[test]
    fn status_json_missing_dir() {
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(status::run_status(
            Some(PathBuf::from("/nonexistent/run/dir")),
            true,
            false,
            false,
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn merge_weights_missing_checkpoint_errors() {
        let result = merge_weights::run_merge_weights(
            PathBuf::from("/nonexistent/model.bin"),
            None,
            16,
            32.0,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not found") || msg.contains("Checkpoint"),
            "expected checkpoint error: {msg}"
        );
    }

    #[test]
    fn merge_weights_rejects_candle_qlora_adapter_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let p = dir.path().join("candle_qlora_adapter.safetensors");
        std::fs::write(&p, []).expect("touch adapter");
        let result = merge_weights::run_merge_weights(p, None, 8, 16.0);
        assert!(result.is_err(), "expected rejection of Candle adapter path");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Candle") && msg.contains("merge"),
            "expected Candle merge guard: {msg}"
        );
        assert!(
            msg.contains("merge-qlora"),
            "expected pointer to merge-qlora: {msg}"
        );
    }

    #[test]
    fn merge_qlora_rejects_burn_bin_adapter() {
        let dir = tempfile::tempdir().expect("tempdir");
        let adapter = dir.path().join("latest.bin");
        std::fs::write(&adapter, [1u8, 2, 3]).expect("touch bin");
        let meta = dir.path().join("meta.json");
        std::fs::write(&meta, "{}").expect("meta");
        let base = dir.path().join("base.safetensors");
        std::fs::write(&base, []).expect("base shard");
        let out = dir.path().join("merged.safetensors");
        let result = merge_qlora::run_merge_qlora(vec![base], adapter, meta, out);
        assert!(result.is_err(), "expected rejection of Burn bin adapter");
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("merge-weights"),
            "expected pointer to merge-weights: {msg}"
        );
        assert!(
            msg.contains("safetensors") || msg.contains("Candle"),
            "expected Candle safetensors hint: {msg}"
        );
    }

    #[test]
    fn merge_qlora_cli_roundtrip_lm_head_subset() {
        use std::collections::HashMap;

        use safetensors::SafeTensors;
        use safetensors::tensor::{Dtype, TensorView};
        use serde_json::json;

        let dir = tempfile::tempdir().expect("tempdir");
        let d = 3usize;
        let vocab = 4usize;
        let rank = 2usize;
        let alpha = 4usize;
        let scale = (alpha as f64 / rank as f64) as f32;

        let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.1).collect();
        let mut wb = Vec::with_capacity(w.len() * 4);
        for x in &w {
            wb.extend_from_slice(&x.to_le_bytes());
        }
        let mut base_map: HashMap<String, TensorView<'_>> = HashMap::new();
        base_map.insert(
            "wte.weight".into(),
            TensorView::new(Dtype::F32, vec![vocab, d], wb.as_slice()).unwrap(),
        );
        let base_path = dir.path().join("base.safetensors");
        std::fs::write(
            &base_path,
            safetensors::serialize(&base_map, &None).unwrap(),
        )
        .unwrap();

        let fa = vec![1.0f32; rank * d];
        let fb = vec![1.0f32; vocab * rank];
        let mut ab = Vec::new();
        for x in &fa {
            ab.extend_from_slice(&x.to_le_bytes());
        }
        let mut bb = Vec::new();
        for x in &fb {
            bb.extend_from_slice(&x.to_le_bytes());
        }
        let mut ad_map: HashMap<String, TensorView<'_>> = HashMap::new();
        ad_map.insert(
            "lm_head.lora_a".into(),
            TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
        );
        ad_map.insert(
            "lm_head.lora_b".into(),
            TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
        );
        let ad_path = dir.path().join("adapter.safetensors");
        std::fs::write(&ad_path, safetensors::serialize(&ad_map, &None).unwrap()).unwrap();

        let meta_path = dir.path().join("meta.json");
        std::fs::write(
            &meta_path,
            serde_json::to_string_pretty(&json!({
                "format": "vox_populi_qlora_lora_only_v2",
                "version": 2,
                "embed_key": "wte.weight",
                "vocab": vocab,
                "d_model": d,
                "rank": rank,
                "alpha": alpha,
                "layer_order": ["lm_head"],
                "base_key_map": { "lm_head": "wte.weight" },
            }))
            .unwrap(),
        )
        .unwrap();

        let out_path = dir.path().join("merged.safetensors");
        merge_qlora::run_merge_qlora(vec![base_path], ad_path, meta_path, out_path.clone())
            .expect("merge-qlora");

        let mut delta = vec![0f32; vocab * d];
        for i in 0..vocab {
            for j in 0..d {
                let mut s = 0f32;
                for k in 0..rank {
                    s += fb[i * rank + k] * fa[k * d + j];
                }
                delta[i * d + j] = s * scale;
            }
        }
        let bytes = std::fs::read(&out_path).unwrap();
        let st = SafeTensors::deserialize(&bytes).unwrap();
        let tv = st.tensor("wte.weight").unwrap();
        assert_eq!(tv.dtype(), Dtype::F32);
        let sl = tv.data();
        for i in 0..vocab * d {
            let o = i * 4;
            let got = f32::from_le_bytes([sl[o], sl[o + 1], sl[o + 2], sl[o + 3]]);
            let exp = w[i] + delta[i];
            assert!(
                (got - exp).abs() < 1e-5,
                "idx {i}: expected {exp} got {got}"
            );
        }
    }

    #[test]
    fn merge_qlora_cli_roundtrip_lm_head_subset_adapter_manifest_v3() {
        use std::collections::HashMap;

        use safetensors::SafeTensors;
        use safetensors::tensor::{Dtype, TensorView};
        use vox_populi::tensor::adapter_schema_v3::PopuliAdapterManifestV3;
        use vox_populi::tensor::finetune_contract::{AdapterMethod, BaseQuantMode};

        let dir = tempfile::tempdir().expect("tempdir");
        let d = 3usize;
        let vocab = 4usize;
        let rank = 2usize;
        let alpha = 4usize;
        let scale = (alpha as f64 / rank as f64) as f32;

        let w: Vec<f32> = (0..vocab * d).map(|i| i as f32 * 0.1).collect();
        let mut wb = Vec::with_capacity(w.len() * 4);
        for x in &w {
            wb.extend_from_slice(&x.to_le_bytes());
        }
        let mut base_map: HashMap<String, TensorView<'_>> = HashMap::new();
        base_map.insert(
            "wte.weight".into(),
            TensorView::new(Dtype::F32, vec![vocab, d], wb.as_slice()).unwrap(),
        );
        let base_path = dir.path().join("base.safetensors");
        std::fs::write(
            &base_path,
            safetensors::serialize(&base_map, &None).unwrap(),
        )
        .unwrap();

        let fa = vec![1.0f32; rank * d];
        let fb = vec![1.0f32; vocab * rank];
        let mut ab = Vec::new();
        for x in &fa {
            ab.extend_from_slice(&x.to_le_bytes());
        }
        let mut bb = Vec::new();
        for x in &fb {
            bb.extend_from_slice(&x.to_le_bytes());
        }
        let mut ad_map: HashMap<String, TensorView<'_>> = HashMap::new();
        ad_map.insert(
            "lm_head.lora_a".into(),
            TensorView::new(Dtype::F32, vec![rank, d], ab.as_slice()).unwrap(),
        );
        ad_map.insert(
            "lm_head.lora_b".into(),
            TensorView::new(Dtype::F32, vec![vocab, rank], bb.as_slice()).unwrap(),
        );
        let ad_path = dir.path().join("adapter.safetensors");
        std::fs::write(&ad_path, safetensors::serialize(&ad_map, &None).unwrap()).unwrap();

        let mut base_key_map = HashMap::new();
        base_key_map.insert("lm_head".into(), "wte.weight".into());
        let v3 = PopuliAdapterManifestV3::new(
            AdapterMethod::Qlora,
            BaseQuantMode::Nf4,
            true,
            base_key_map,
            vec!["lm_head".into()],
            vocab,
            d,
            rank,
            alpha,
        );
        let meta_path = dir.path().join("meta_v3.json");
        std::fs::write(
            &meta_path,
            serde_json::to_string_pretty(&v3).expect("serialize v3 manifest"),
        )
        .unwrap();

        let out_path = dir.path().join("merged_v3.safetensors");
        merge_qlora::run_merge_qlora(vec![base_path], ad_path, meta_path, out_path.clone())
            .expect("merge-qlora v3 meta");

        let mut delta = vec![0f32; vocab * d];
        for i in 0..vocab {
            for j in 0..d {
                let mut s = 0f32;
                for k in 0..rank {
                    s += fb[i * rank + k] * fa[k * d + j];
                }
                delta[i * d + j] = s * scale;
            }
        }
        let bytes = std::fs::read(&out_path).unwrap();
        let st = SafeTensors::deserialize(&bytes).unwrap();
        let tv = st.tensor("wte.weight").unwrap();
        assert_eq!(tv.dtype(), Dtype::F32);
        let sl = tv.data();
        for i in 0..vocab * d {
            let o = i * 4;
            let got = f32::from_le_bytes([sl[o], sl[o + 1], sl[o + 2], sl[o + 3]]);
            let exp = w[i] + delta[i];
            assert!(
                (got - exp).abs() < 1e-5,
                "idx {i}: expected {exp} got {got}"
            );
        }
    }

    #[test]
    fn eval_local_missing_model_errors() {
        let result = eval_local::run_eval_local(
            PathBuf::from("/nonexistent/model.bin"),
            PathBuf::from("populi/data/heldout_bench"),
            32,
            0.0,
            None,
        );
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("not found") || msg.contains("Model"),
            "expected model not found: {msg}"
        );
    }
}
