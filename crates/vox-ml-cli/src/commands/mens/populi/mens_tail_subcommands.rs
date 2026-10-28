//! Tail `vox mens` subcommands (eval-gate, bench, system prompt) — separate module for TOESTUB line budget.

use std::path::PathBuf;

use clap::Subcommand;

/// Eval-gate, completion bench, and system-prompt-template (flattened under [`PopuliAction`](super::PopuliAction)).
#[derive(Subcommand)]
pub enum PopuliMensTail {
    /// Check training/eval run against eval-gate policy thresholds.
    ///
    /// Reads mens/config/eval-gates.yaml and validates run artifacts.
    /// Exits 1 if any blocking gate fails.
    #[command(name = "eval-gate")]
    EvalGate {
        /// Run directory (manifest.json, metrics.jsonl, eval_results.json)
        #[arg(long, default_value = vox_scaling_policy::DEFAULT_MENS_RUNS_V1)]
        run_dir: PathBuf,
        /// Policy file (default: mens/config/eval-gates.yaml)
        #[arg(long)]
        policy: Option<PathBuf>,
        /// Run the 25-problem 5-spoke Mens before/after verification gate and write `mens_spoke_scorecard.json`.
        #[arg(long, default_value_t = false)]
        five_spokes: bool,
    },

    /// Capture a base-model BFCL baseline so the beat-base eval gate has a real
    /// point of comparison.
    ///
    /// Run the BFCL eval harness against the BASE model (no adapter) to produce
    /// `bfcl_results.json` in `--base-eval-dir`, then run this to write
    /// `baseline_report.json`. Without a captured baseline, beat-base silently
    /// skips.
    #[command(name = "baseline")]
    Baseline {
        /// Spoke identifier this baseline applies to (e.g. vox-lang, tool-selection).
        #[arg(long)]
        spoke: String,
        /// Directory containing the base-model `bfcl_results.json`.
        #[arg(long)]
        base_eval_dir: PathBuf,
        /// Output path for baseline_report.json
        /// (default: <base-eval-dir>/baseline_report.json).
        #[arg(long)]
        out: Option<PathBuf>,
    },

    /// Check for catastrophic forgetting against a static benchmark
    #[command(name = "eval-collateral-damage", visible_alias = "eval")]
    EvalCollateralDamage {
        /// Baseline score JSON path
        #[arg(long)]
        pre_score: PathBuf,
        /// Adapter path to evaluate
        #[arg(long, id = "post")]
        post_adapter: PathBuf,
    },

    /// Build collateral pre-score JSON (six reasoning keys) from a compared
    /// `eval-reasoning-compare` report.
    #[command(name = "baseline-from-compare")]
    BaselineFromCompare {
        /// Compared reasoning_before_after.v1 report path
        #[arg(long)]
        compare: PathBuf,
        /// Write baseline JSON here (default: stdout)
        #[arg(short = 'o', long)]
        out: Option<PathBuf>,
    },

    /// Score a pinned base and its adapter on the 32-item reasoning slices; writes an absence record when either is not on disk. Not a serve-gate input.
    #[command(name = "eval-reasoning-compare")]
    EvalReasoningCompare {
        /// Base: a Qwen3 size name (resolved through gpu-specs `train_bases`) or `hf_id@<40-hex sha>`
        /// (default: the pinned Qwen/Qwen3-8B)
        #[arg(long)]
        base: Option<String>,
        /// Adapter run directory trained on that base
        #[arg(long)]
        adapter: Option<PathBuf>,
        /// Hugging Face hub cache root (default, hf-hub order: HF_HUB_CACHE, HUGGINGFACE_HUB_CACHE,
        /// HF_HOME/hub, XDG_CACHE_HOME/huggingface/hub, ~/.cache/huggingface/hub)
        #[arg(long)]
        hub_root: Option<PathBuf>,
        /// Directory holding the `<slice>/reasoning.jsonl` files
        #[arg(long, default_value = "mens/data")]
        bench_root: PathBuf,
        /// Report path (default: mens/runs/reasoning_compare/<size>-<rev12>.json)
        #[arg(long)]
        out: Option<PathBuf>,
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

    /// Rank live cloud GPU offers for a training run and print the estimated bill.
    ///
    /// Read-only: resolves and ranks, never provisions. Requires VOX_VAST_API_KEY
    /// and/or VOX_RUNPOD_API_KEY for the rented rows; the local row needs neither.
    #[cfg(feature = "cloud")]
    #[command(name = "cloud-estimate")]
    CloudEstimate {
        /// Path to the model directory (read for layers / hidden / artifact bytes).
        #[arg(long)]
        model_dir: PathBuf,
        /// Provider to query: auto, vast, runpod, local.
        #[arg(long, default_value = "auto")]
        target: String,
        /// Refuse to list offers above this total run cost.
        #[arg(long, default_value_t = 100.0)]
        max_budget: f64,
        /// Micro-batch size (drives the memory-model prediction and the
        /// resolver's cost estimate). Matches `vox mens train`'s default.
        #[arg(long, default_value_t = 4)]
        batch_size: usize,
        /// Training sequence length. Matches `vox mens train`'s default.
        #[arg(long, default_value_t = 512)]
        seq_len: usize,
        /// Total number of training pairs (cost-estimate input only).
        #[arg(long, default_value_t = 5000)]
        num_samples: usize,
        /// Number of training epochs (cost-estimate input only).
        #[arg(long, default_value_t = 3)]
        epochs: usize,
    },

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
}
