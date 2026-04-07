//! CLI argument parsing and dispatch for `vox-schola`.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

use anyhow::Result;

/// Raw argv from clap (subcommand optional — default is [`Cmd::default_train`]).
#[derive(Parser)]
#[command(
    name = "vox-schola",
    about = "Vox ML: train/serve/probe — GPU always enabled, no compiler stack",
    long_about = "Standalone GPU-native binary for the Vox Mens ML subsystem.\n\
                  Equivalent to `vox mens …` but ~3× faster to compile (no lexer/parser/codegen).\n\n\
                  With no subcommand, **`train`** runs with the same defaults as `vox-schola train`.\n\
                  Pass flags on `train` (e.g. `vox-schola train --device cuda --preset 4080`).\n\n\
                  Quick start:\n\
                  \n  vox-schola   # default `--model`: Qwen/Qwen3.5-4B (VoxMens SSOT; forwards to `vox` when found)\
                  \n  vox-schola train --device cuda --preset 4080 --model Qwen/Qwen2.5-Coder-1.5B-Instruct\
                  \n  vox-schola serve --model mens/runs/latest\
                  \n  vox-schola probe",
    version,
    subcommand_required = false,
    arg_required_else_help = false
)]
pub struct Cli {
    #[command(subcommand)]
    cmd: Option<Cmd>,
    /// Enable verbose tracing output.
    #[arg(long, global = true)]
    pub verbose: bool,
    /// Emit JSON where supported.
    #[arg(long, global = true)]
    pub json: bool,
}

/// Resolved CLI: always has a concrete subcommand (default: train).
pub struct Args {
    /// Active subcommand (`train` if argv omitted it).
    pub cmd: Cmd,
    pub verbose: bool,
    pub json: bool,
}

impl Args {
    /// Parse argv; bare `vox-schola` → `train` with defaults (Mens QLoRA).
    pub fn parse() -> Self {
        let c = Cli::parse();
        Self {
            cmd: c.cmd.unwrap_or_else(Cmd::default_train),
            verbose: c.verbose,
            json: c.json,
        }
    }
}

#[derive(Subcommand)]
#[allow(clippy::large_enum_variant)] // Clap CLI subcommands carry many path/buf fields by design.
pub enum Cmd {
    /// Fine-tune a HuggingFace model with Candle QLoRA (NF4).
    Train {
        /// HuggingFace model repo (e.g. Qwen/Qwen2.5-Coder-1.5B-Instruct). Downloads weights.
        #[arg(long, default_value = vox_populi::mens::DEFAULT_MODEL_ID)]
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
        /// Max sequence length (tokens). Omit to use preset/device default.
        #[arg(long)]
        seq_len: Option<usize>,
        /// Steps between mid-epoch checkpoints (default: 500).
        #[arg(long, default_value = "500")]
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
        /// Load `checkpoint_state.json` from `--output-dir` when present (default: start a fresh run).
        #[arg(long, visible_alias = "continue-run", action = clap::ArgAction::SetTrue)]
        resume_checkpoint: bool,
        /// Delete prior checkpoints/adapters in `--output-dir` and ignore checkpoint state.
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
        /// On `--device cuda`, `vox mens train` defaults this on unless `--qlora-allow-partial-proxy-stack` is set.
        #[arg(long)]
        qlora_require_full_proxy_stack: bool,
        /// Opt out of CUDA full proxy-stack preflight (forwarded to `vox mens train`).
        #[arg(long, default_value_t = false)]
        qlora_allow_partial_proxy_stack: bool,
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
        /// Provenance: coarse family label for upstream lineage.
        #[arg(long)]
        base_model_family: Option<String>,
        /// Provenance: explicit upstream model id.
        #[arg(long)]
        upstream_model_id: Option<String>,
        /// Provenance: license class label.
        #[arg(long)]
        license_class: Option<String>,
        /// Provenance: mark downstream publication as attribution-required.
        #[arg(long, default_value_t = false)]
        attribution_required: bool,
        /// Enable trajectory-aware weighting for tool/failure rows.
        #[arg(long, default_value_t = false)]
        trajectory_weighting_enabled: bool,
        /// Multiplier for tool-trace rows.
        #[arg(long, default_value_t = 1.1)]
        trajectory_tool_trace_boost: f32,
        /// Multiplier for failure/error rows.
        #[arg(long, default_value_t = 1.15)]
        trajectory_failure_category_boost: f32,
        /// Minimum quality floor (1-5) for quality boost.
        #[arg(long)]
        trajectory_quality_floor: Option<u8>,
        /// Multiplier when quality floor is met.
        #[arg(long, default_value_t = 1.05)]
        trajectory_quality_boost: f32,
    },
    /// Serve a trained adapter via HTTP (OpenAI-compatible ChatCompletion).
    Serve {
        /// Path to the model output directory (contains adapter_meta_v2.json).
        #[arg(long)]
        model: PathBuf,
        /// Port to listen on.
        #[arg(long, default_value_t = 11434)]
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
        /// Override model id for `/api/tags` and Ollama clients (default: directory name). Match `POPULI_MODEL` / orchestrator registry.
        #[arg(long)]
        model_name: Option<String>,
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

impl Cmd {
    /// Defaults for `vox-schola` with no subcommand (must match `Train`’s clap defaults).
    #[must_use]
    pub fn default_train() -> Self {
        Cmd::Train {
            model: Some(vox_populi::mens::DEFAULT_MODEL_ID.to_string()),
            device: "best".to_string(),
            data_dir: PathBuf::from("target/dogfood"),
            output_dir: PathBuf::from("mens/runs/latest"),
            preset: None,
            rank: None,
            alpha: None,
            seq_len: None,
            checkpoint_every: Some(500),
            batch_size: None,
            grad_accum: None,
            epochs: None,
            lr: None,
            warmup: None,
            seed: 42,
            min_rating: None,
            resume: None,
            resume_checkpoint: false,
            force_restart: false,
            adapter_tag: None,
            context_filter: None,
            vram_limit_fraction: None,
            background: false,
            log_dir: None,
            skip_corpus_mix: false,
            qlora_no_double_quant: false,
            qlora_require_full_proxy_stack: false,
            qlora_allow_partial_proxy_stack: false,
            qlora_lm_head_only: false,
            qlora_max_skip_rate: None,
            qlora_proxy_max_layers: None,
            qlora_ce_last_k: 64,
            base_model_family: None,
            upstream_model_id: None,
            license_class: None,
            attribution_required: false,
            trajectory_weighting_enabled: false,
            trajectory_tool_trace_boost: 1.1,
            trajectory_failure_category_boost: 1.15,
            trajectory_quality_floor: None,
            trajectory_quality_boost: 1.05,
        }
    }
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
    let Args { cmd, verbose, json } = Args::parse();
    match cmd {
        cmd @ Cmd::Train { .. } => crate::train::run(Args { cmd, verbose, json }).await,
        cmd @ Cmd::Serve { .. } => crate::serve::run(Args { cmd, verbose, json }).await,
        Cmd::Probe => crate::probe::run(),
        Cmd::Status { run_dir } => crate::status::run(run_dir),
        cmd @ Cmd::Merge { .. } => crate::merge::run(Args { cmd, verbose, json }),
    }
}
