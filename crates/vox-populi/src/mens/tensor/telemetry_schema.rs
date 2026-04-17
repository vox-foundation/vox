//! Stable telemetry **event names and payload keys** (kernel-agnostic where possible).
//!
//! **Sensitivity (Mens / local research telemetry):** each constant below is tagged **S0** (coarse numeric /
//! non-identifying) or **S1** (may embed paths, run ids, or config fingerprints useful to operators).
//! Taxonomy alignment: `docs/src/architecture/telemetry-trust-ssot.md`,
//! `docs/src/architecture/telemetry-taxonomy-contracts-ssot.md`.

/// Top-level JSONL line: `event` field.
pub mod events {
    /// **S1** — run boundary; correlates with `run_id` / workspace-adjacent training state.
    pub const TRAIN_START: &str = "train_start";
    /// **S1** — per-step loop; may include loss/lr tied to a specific run.
    pub const TRAIN_STEP: &str = "step";
    /// **S1** — run completion summary.
    pub const TRAIN_COMPLETE: &str = "train_complete";
    /// **S1** — hardware/kernel fallback signal (diagnostic).
    pub const GPU_FALLBACK: &str = "gpu_fallback";
}

/// Common payload keys across Burn and Candle.
pub mod keys {
    /// **S1** — path or URI-like training script reference.
    pub const TRAIN_FILE: &str = "train_file";
    /// **S1** — output directory (workspace-adjacent).
    pub const OUTPUT_DIR: &str = "output";
    /// **S0** — numeric seed.
    pub const SEED: &str = "seed";
    /// **S0** — gradient accumulation steps.
    pub const GRAD_ACCUM: &str = "grad_accum";
    /// **S0** — dataset size metric.
    pub const PAIRS_LOADED: &str = "pairs_loaded";
    /// **S1** — execution stack label (Burn vs Candle, device class).
    pub const EXECUTION_KERNEL: &str = "execution_kernel";
    /// **S1** — config / manifest digest (fingerprint).
    pub const CONTRACT_DIGEST: &str = "contract_digest";
    /// **S0** — schema version integer for this key namespace.
    pub const TELEMETRY_SCHEMA: &str = "telemetry_schema";
    /// **S0** — Candle compatibility flag.
    pub const CANDLE_COMPAT_MODE: &str = "candle_compat_mode";
    /// **S0** — epoch index.
    pub const EPOCH: &str = "epoch";
    /// **S0** — step index.
    pub const STEP: &str = "step";
    /// **S0** — training loss scalar.
    pub const LOSS: &str = "loss";
    /// **S0** — learning rate (alias key).
    pub const LR: &str = "lr";
    /// **S0** — learning rate (canonical key).
    pub const LEARNING_RATE: &str = "learning_rate";
    /// **S0** — throughput estimate.
    pub const TOKENS_PER_SEC: &str = "tokens_per_sec";
    /// **S0** — whether `tokens_per_sec` is a proxy metric.
    pub const TOKENS_PER_SEC_IS_PROXY: &str = "tokens_per_sec_is_proxy";
    /// **S0** — token counts from batch.
    pub const VALID_TOKENS: &str = "valid_tokens";
    /// **S0** — upper-bound token accounting.
    pub const THEORETICAL_TOKENS: &str = "theoretical_tokens";
    /// **S0** — supervised fraction (%).
    pub const SUPERVISED_RATIO_PCT: &str = "supervised_ratio_pct";
    /// **S0** — Candle QLoRA: optimizer micro-steps expected per epoch if no vocab/hidden skips (upper bound).
    pub const PLANNED_STEPS_PER_EPOCH: &str = "planned_steps_per_epoch";
    /// **S0** — `planned_steps_per_epoch` × `epochs` (upper bound over the full run).
    pub const PLANNED_STEPS_TOTAL: &str = "planned_steps_total";
    /// **S0** — epochs configured for the run (`LoraTrainingConfig.epochs`).
    pub const EPOCHS: &str = "epochs";
    /// **S0** — Candle QLoRA: ETA (seconds) from smoothed steps/sec (interval EMA); `null` during warm-up or if unknown.
    pub const ETA_SECONDS_REMAINING: &str = "eta_seconds_remaining";
    /// **S0** — `global_step / planned_steps_total` when planned > 0.
    pub const PROGRESS_FRACTION: &str = "progress_fraction";
    /// **S0** — smoothed steps/sec (EMA over progress intervals) for ETA; `null` until first interval sample.
    pub const STEPS_PER_SEC_EMA: &str = "steps_per_sec_ema";
    /// **S0** — NNT small-world routing efficiency metric (0.0-1.0).
    pub const ROUTING_EFFICIENCY: &str = "routing_efficiency";
}

/// **S0** — current telemetry schema version written alongside events (integer, not content).
pub const TELEMETRY_SCHEMA_VERSION: u32 = 1;
