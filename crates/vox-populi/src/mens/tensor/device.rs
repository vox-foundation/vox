//! Device selection, GPU probing (best-effort), and VRAM heuristics for Mens training.

/// CLI / env device intent for Burn (wgpu) and Candle qlora backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceKind {
    /// Prefer host CPU / software paths.
    Cpu,
    /// Let the stack pick (wgpu default adapter; Candle CUDA → Metal → CPU when available).
    Best,
    /// Prefer NVIDIA CUDA when the Candle build supports it.
    Cuda,
    /// Prefer Apple Metal when the Candle build supports it (macOS).
    Metal,
}

/// Best-effort local GPU description (VRAM may be **0** when OS APIs are unavailable).
#[derive(Debug, Clone)]
pub struct GpuInfo {
    /// Marketing or driver-reported model name.
    pub model_name: String,
    /// Dedicated video memory in megabytes (0 = unknown / integrated / CPU-only).
    pub vram_mb: u64,
    /// Coarse vendor bucket: `nvidia`, `amd`, `intel`, `apple`, `cpu`, or `unknown`.
    pub vendor: String,
}

/// High-level training capacity hint derived from available VRAM (heuristic).
#[derive(Debug, Clone)]
pub struct TrainProfile {
    pub label: &'static str,
    pub max_seq_len: usize,
    pub suggested_batch: usize,
    pub suggested_rank: usize,
}

/// Parse `--device` / dispatch strings into a [`DeviceKind`].
pub fn normalize_device(raw: &str) -> Result<DeviceKind, String> {
    match raw.trim().to_lowercase().as_str() {
        "" | "cpu" => Ok(DeviceKind::Cpu),
        "best" | "auto" | "default" | "vulkan" | "dx12" | "d3d12" => Ok(DeviceKind::Best),
        "cuda" | "gpu" | "nvidia" => Ok(DeviceKind::Cuda),
        "metal" | "mps" => Ok(DeviceKind::Metal),
        other => Err(format!(
            "unknown device '{other}': expected cpu, best, cuda, metal, vulkan, or dx12"
        )),
    }
}

/// Apply process-level hints before Burn / Candle init (currently conservative / mostly no-op).
pub fn apply_backend_env(kind: DeviceKind) {
    tracing::debug!(?kind, "apply_backend_env");
}

/// Infer vendor string from a free-form GPU model name.
pub fn detect_gpu_vendor() -> String {
    let g = probe_gpu();
    g.vendor
}

/// Probe GPU metadata.
#[must_use]
pub fn probe_gpu() -> GpuInfo {
    // New SSOT: use the hardware registry
    let summary = futures::executor::block_on(crate::mens::hardware::probe());

    GpuInfo {
        model_name: summary.model_name.clone(),
        vram_mb: summary.vram_mb,
        vendor: format!("{:?}", summary.vendor).to_lowercase(),
    }
}

/// Very rough **megabyte** estimate for transformer-style training (activations + weights, FP32-ish).
#[must_use]
pub fn estimate_training_vram_mb(
    d_model: usize,
    n_heads: usize,
    n_layers: usize,
    vocab: usize,
    batch: usize,
    seq_len: usize,
) -> u64 {
    let _ = n_heads;
    let d = d_model as u64;
    let l = n_layers.max(1) as u64;
    let s = seq_len.max(1) as u64;
    let b = batch.max(1) as u64;
    let v = vocab.max(1) as u64;
    let embed = v.saturating_mul(d).saturating_mul(4);
    let block = d
        .saturating_mul(d)
        .saturating_mul(12)
        .saturating_mul(4)
        .saturating_mul(l);
    let acts = b
        .saturating_mul(s)
        .saturating_mul(d)
        .saturating_mul(l)
        .saturating_mul(16);
    let bytes = embed.saturating_add(block).saturating_add(acts);
    (bytes / (1024 * 1024)).max(128)
}

/// Peak-memory estimate (MiB) for the **Candle QLoRA** trainer, which runs one
/// row of up to `seq_len` tokens per micro-step (batch rows are sequential).
///
/// Calibrated on Qwen3-0.6B Metal runs (320 S2), where the pool keeps every
/// op's power-of-two buffer: resident weights (NF4 + BF16 dequant cache, F32
/// embedding, BF16 head) plus ~3 GiB load overhead; per layer ~1 KB per token
/// per hidden unit; the `[s, vocab]` head at ~230 B per token per vocab entry.
///
/// Checkpointed is the Metal plugin's path: one layer, the layer boundaries,
/// one 256-row head chunk (`HEAD_CHUNK_ROWS`) and query-chunked attention,
/// whose backward holds ~150 B per head per (query row, key) of one 512-row
/// block (`attention::QUERY_BLOCK`), so attention grows with `s`, not `s²`.
/// Eager models the unchunked attention every layer keeps on the tape (~95 B
/// per head per token pair, plus ~200 B while one layer's backward runs), as
/// measured before chunking. Rows are assumed to reach `seq_len`, so this is
/// the worst case for the run.
#[must_use]
pub fn estimate_training_vram_mb_qlora(
    d_model: usize,
    n_heads: usize,
    n_layers: usize,
    vocab: usize,
    seq_len: usize,
    gradient_checkpointing: bool,
) -> u64 {
    let (d, h, l, v) = (
        d_model as u64,
        n_heads as u64,
        n_layers.max(1) as u64,
        vocab as u64,
    );
    let s = seq_len.max(1) as u64;
    let resident = v * d * 6 + 12 * d * d * l * 5 / 2 + (3 << 30);
    let head = |rows: u64| rows * v * 230;
    let bytes = if gradient_checkpointing {
        let attention_block = h * s.min(512) * s * 150;
        resident + s * d * 1000 + attention_block + l * s * d * 4 + head(s.min(256))
    } else {
        let layer = s * d * 1000 + h * s * s * 95;
        resident + l * layer + h * s * s * 200 + head(s)
    };
    bytes / (1024 * 1024)
}

/// Log GPU summary via [`tracing`] (`target = "vox_mens_gpu"`).
pub fn print_gpu_summary() {
    let g = probe_gpu();
    tracing::info!(
        target: "vox_mens_gpu",
        model = %g.model_name,
        vendor = %g.vendor,
        vram_mb = g.vram_mb,
        "[mens-gpu] {} | vendor={} | vram_mb={}",
        g.model_name,
        g.vendor,
        g.vram_mb,
    );
}

/// Same as [`print_gpu_summary`] with a caller prefix in the message.
pub fn print_gpu_summary_for(prefix: &str) {
    let g = probe_gpu();
    tracing::info!(
        target: "vox_mens_gpu",
        prefix,
        model = %g.model_name,
        vendor = %g.vendor,
        vram_mb = g.vram_mb,
        "{prefix} {} | vendor={} | vram_mb={}",
        g.model_name,
        g.vendor,
        g.vram_mb,
    );
}

/// VRAM currently in use — not available without driver APIs.
/// NVML probing moved to vox-plugin-nvml-probe.
#[must_use]
#[rustfmt::skip] // keeps the toestub-ignore comment pinned to the fn signature line
pub fn sample_vram_used_mb() -> Option<u64> { // toestub-ignore(skeleton/hollow-fn): honest "unknown"; the probe lives in vox-plugin-nvml-probe
    None
}

/// User-facing hint when OOM / allocation fails.
#[must_use]
pub fn oom_guidance() -> &'static str {
    "Try `--preset safe`, lower `--seq-len` / `--batch-size`, or `--device cpu`. See docs/src/architecture/mens-training-ssot.md."
}

/// Map available VRAM to a coarse profile.
#[must_use]
pub fn recommend_config(vram_mb: u64) -> TrainProfile {
    if vram_mb >= 24_000 {
        TrainProfile {
            label: "ultra",
            max_seq_len: 1024,
            suggested_batch: 4,
            suggested_rank: 32,
        }
    } else if vram_mb >= 12_000 {
        TrainProfile {
            label: "high",
            max_seq_len: 512,
            suggested_batch: 4,
            suggested_rank: 16,
        }
    } else if vram_mb >= 7_000 {
        TrainProfile {
            label: "mid",
            max_seq_len: 512,
            suggested_batch: 2,
            suggested_rank: 12,
        }
    } else {
        TrainProfile {
            label: "low",
            max_seq_len: 256,
            suggested_batch: 1,
            suggested_rank: 8,
        }
    }
}

/// Preset-name helper for docs and older call sites.
#[must_use]
pub fn recommend_config_for_profile(preset_name: &str) -> TrainProfile {
    match preset_name {
        "tiny" | "safe" | "4080_safe" => recommend_config(4_000),
        "a100" => recommend_config(40_000),
        _ => recommend_config(16_000),
    }
}

#[cfg(test)]
mod vram_estimate_tests {
    use super::*;

    /// 320 S2: this estimate said 1618 MB for a Qwen3-0.6B row of 3072 tokens;
    /// the eager Metal run reached ~374 GB at 2333. It must carry the attention
    /// and head terms (measured peaks below, MiB) and drop with checkpointing.
    #[test]
    fn qlora_estimate_tracks_measured_metal_peaks() {
        let q06 = |s, gc| estimate_training_vram_mb_qlora(1024, 16, 28, 151_936, s, gc);
        for (s, measured) in [(128, 15_945), (256, 23_632), (512, 39_415), (2333, 357_056)] {
            let ratio = q06(s, false) as f64 / measured as f64;
            assert!((0.7..1.3).contains(&ratio), "seq {s}: ratio {ratio}");
        }
        // Checkpointed (256-row head chunks) with 512-row query-chunked
        // attention: the --seq-len 3072 run peaked at 19,024 MiB, longest row
        // 2508 (320-debug-chunked-s3072); before chunking that run was 43,862.
        let ratio = q06(2508, true) as f64 / 19_024.0;
        assert!(
            (0.9..1.15).contains(&ratio),
            "checkpointed seq 2508: ratio {ratio}"
        );
        assert!(q06(3072, true) * 5 < q06(3072, false));
    }
}
