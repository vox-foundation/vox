//! CLI argument parsing and dispatch for `vox-schola`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use anyhow::Result;

/// Standalone Vox ML binary — train, serve, probe. GPU always enabled.
#[derive(Parser)]
#[command(
    name = "vox-schola",
    about = "Vox ML: train/serve/probe — GPU always enabled, no compiler stack",
    long_about = "Standalone GPU-native binary for the Vox Mens ML subsystem.\n\
                  Equivalent to `vox mens …` but ~3× faster to compile (no lexer/parser/codegen).\n\n\
                  Quick start:\n\
                  \n  vox-schola train --model Qwen/Qwen2.5-Coder-1.5B-Instruct\
                  \n  vox-schola serve --model mens/runs/latest\
                  \n  vox-schola probe",
    version
)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Cmd,
    /// Enable verbose tracing output.
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Emit JSON where supported.
    #[arg(long, global = true)]
    pub json: bool,
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // Clap CLI subcommands carry many path/buf fields by design.
pub enum Cmd {
    /// Fine-tune a HuggingFace model with Candle QLoRA (NF4).
    Train {
        /// HuggingFace model repo (e.g. Qwen/Qwen2.5-Coder-1.5B-Instruct). Downloads weights.
        #[arg(long)]
        model: Option<String>,
        /// GPU backend: best | cuda | cpu | metal | vulkan | dx12.
        #[arg(long, default_value = "best")]
        device: String,
        /// Directory containing train.jsonl.
        #[arg(long, default_value = "target/dogfood")]
        data_dir: PathBuf,
        /// Adapter and checkpoint output directory.
        #[arg(long, default_value = "mens/runs/latest")]
        output_dir: PathBuf,
        /// Hardware preset: tiny | safe | 4080 | a100 | mobile_edge.
        #[arg(long)]
        preset: Option<String>,
        /// LoRA rank (higher = more expressiveness, more VRAM).
        #[arg(long)]
        rank: Option<usize>,
        /// LoRA alpha (usually 2× rank).
        #[arg(long)]
        alpha: Option<f32>,
        /// Max sequence length (tokens).
        #[arg(long, default_value_t = 512)]
        seq_len: usize,
        /// Steps between mid-epoch checkpoints (default: 500).
        #[arg(long)]
        checkpoint_every: Option<usize>,
        /// Batch size per step.
        #[arg(long)]
        batch_size: Option<usize>,
        /// Gradient accumulation steps.
        #[arg(long)]
        grad_accum: Option<usize>,
        /// Number of training epochs.
        #[arg(long)]
        epochs: Option<usize>,
        /// Learning rate.
        #[arg(long)]
        lr: Option<f64>,
        /// Warmup steps.
        #[arg(long)]
        warmup: Option<usize>,
        /// Random seed (0 = random).
        #[arg(long, default_value_t = 42)]
        seed: u64,
        /// Minimum quality rating to include (1–5; 0 = all).
        #[arg(long)]
        min_rating: Option<u8>,
        /// Resume from an output directory that has a checkpoint_state.json.
        #[arg(long)]
        resume: Option<PathBuf>,
        /// Ignore existing checkpoint and restart from scratch.
        #[arg(long)]
        force_restart: bool,
        /// Dual-mode adapter tag (target | meta). Sets output directory suffix.
        #[arg(long)]
        adapter_tag: Option<String>,
        /// Filter training records by context: target | meta | both.
        #[arg(long)]
        context_filter: Option<String>,
        /// Cap VRAM fraction (0.0–1.0; unset = adaptive 85%).
        #[arg(long)]
        vram_limit_fraction: Option<f32>,
        /// Run as low-priority background process.
        #[arg(long)]
        background: bool,
        /// Log training output to a file in this directory (non-blocking).
        #[arg(long)]
        log_dir: Option<PathBuf>,
        /// Skip running corpus mix before training.
        #[arg(long)]
        skip_corpus_mix: bool,
        /// Disable QLoRA double quantization (default: on).
        #[arg(long)]
        qlora_no_double_quant: bool,
        /// Require full proxy stack (abort if o_proj layers are missing).
        #[arg(long)]
        qlora_require_full_proxy_stack: bool,
        /// Train LM-head adapter only (no o_proj layers).
        #[arg(long)]
        qlora_lm_head_only: bool,
        /// Max skip rate per epoch before aborting (0.0–1.0).
        #[arg(long)]
        qlora_max_skip_rate: Option<f32>,
        /// Max middle o_proj layers in the proxy stack.
        #[arg(long)]
        qlora_proxy_max_layers: Option<usize>,
        /// Next-token CE over the last K positions per row (default: 64).
        #[arg(long, default_value_t = 64)]
        qlora_ce_last_k: usize,
    },
    /// Serve a trained adapter via HTTP (OpenAI-compatible ChatCompletion).
    Serve {
        /// Path to the model output directory (contains adapter_meta_v2.json).
        #[arg(long)]
        model: PathBuf,
        /// Port to listen on.
        #[arg(long, default_value_t = 8080)]
        port: u16,
        /// Host to bind.
        #[arg(long, default_value = "127.0.0.1")]
        host: String,
        /// Max tokens per response.
        #[arg(long, default_value_t = 512)]
        max_tokens: usize,
        /// Sampling temperature (0 = greedy).
        #[arg(long, default_value_t = 0.7)]
        temperature: f64,
        /// GPU backend: best | cuda | cpu | metal.
        #[arg(long, default_value = "best")]
        device: String,
    },
    /// Probe GPU capabilities and print recommended training profile.
    Probe,
    /// Show training run status and telemetry from an output directory.
    Status {
        /// Run directory (default: mens/runs/latest).
        #[arg(long)]
        run_dir: Option<PathBuf>,
    },
    /// Merge a QLoRA adapter into base weights (produce standalone safetensors subset).
    Merge {
        /// One or more base model safetensors shard paths.
        #[arg(long, required = true)]
        base_shard: Vec<PathBuf>,
        /// Path to candle_qlora_adapter.safetensors.
        #[arg(long)]
        adapter: PathBuf,
        /// Path to adapter_meta_v2.json (or populi_adapter_manifest_v3.json).
        #[arg(long)]
        meta: PathBuf,
        /// Output path for merged safetensors subset.
        #[arg(long)]
        output: PathBuf,
    },
}

/// Backend enum (mirrors vox-cli PopuliTrainBackendCli for standalone use).
#[derive(Clone, Copy, Debug, Default, ValueEnum)]
pub enum BackendCli {
    /// Candle QLoRA (NF4) — default.
    #[default]
    Qlora,
    /// Burn LoRA (deprecated).
    Lora,
}

pub async fn run() -> Result<()> {
    let args = Args::parse();
    match args.cmd {
        Cmd::Train { .. } => crate::train::run(args).await,
        Cmd::Serve { .. } => crate::serve::run(args).await,
        Cmd::Probe => crate::probe::run(),
        Cmd::Status { run_dir } => crate::status::run(run_dir),
        Cmd::Merge { .. } => crate::merge::run(args),
    }
}
