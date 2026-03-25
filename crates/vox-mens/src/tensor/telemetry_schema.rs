//! Stable telemetry **event names and payload keys** (kernel-agnostic where possible).

/// Top-level JSONL line: `event` field.
pub mod events {
    pub const TRAIN_START: &str = "train_start";
    pub const TRAIN_STEP: &str = "step";
    pub const TRAIN_COMPLETE: &str = "train_complete";
    pub const GPU_FALLBACK: &str = "gpu_fallback";
}

/// Common payload keys across Burn and Candle.
pub mod keys {
    pub const TRAIN_FILE: &str = "train_file";
    pub const OUTPUT_DIR: &str = "output";
    pub const SEED: &str = "seed";
    pub const GRAD_ACCUM: &str = "grad_accum";
    pub const PAIRS_LOADED: &str = "pairs_loaded";
    pub const EXECUTION_KERNEL: &str = "execution_kernel";
    pub const CONTRACT_DIGEST: &str = "contract_digest";
    pub const TELEMETRY_SCHEMA: &str = "telemetry_schema";
    pub const CANDLE_COMPAT_MODE: &str = "candle_compat_mode";
    pub const EPOCH: &str = "epoch";
    pub const STEP: &str = "step";
    pub const LOSS: &str = "loss";
    pub const LR: &str = "lr";
    /// Candle QLoRA: optimizer micro-steps expected per epoch if no vocab/hidden skips (upper bound).
    pub const PLANNED_STEPS_PER_EPOCH: &str = "planned_steps_per_epoch";
    /// `planned_steps_per_epoch` × `epochs` (upper bound over the full run).
    pub const PLANNED_STEPS_TOTAL: &str = "planned_steps_total";
    /// Epochs configured for the run (`LoraTrainingConfig.epochs`).
    pub const EPOCHS: &str = "epochs";
    /// Candle QLoRA: ETA (seconds) from **smoothed** steps/sec (interval EMA); `null` during warm-up or if unknown.
    pub const ETA_SECONDS_REMAINING: &str = "eta_seconds_remaining";
    /// `global_step / planned_steps_total` when planned > 0.
    pub const PROGRESS_FRACTION: &str = "progress_fraction";
    /// Smoothed steps/sec (EMA over progress intervals) for ETA; `null` until first interval sample.
    pub const STEPS_PER_SEC_EMA: &str = "steps_per_sec_ema";
}

/// Current telemetry schema version written alongside events.
pub const TELEMETRY_SCHEMA_VERSION: u32 = 1;
